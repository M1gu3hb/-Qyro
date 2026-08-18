//! A transfer, driven one step at a time, behind a surface with no key in it.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;
use std::fs::File;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use qyro_crypto::PublicIdentity;
use qyro_fs::{
    FileSink, FileSource, PlannedFile, PlannedOpenFile, descriptors_by_item, manifest_from_disk,
    manifest_from_open_files,
};
use qyro_net::{FrameStream, Listener, NetError, dial, initiate, respond};
use qyro_transfer::{
    ContentSink, ItemVerdict, Phase, Receiver, RejectReason as WireReject, Sender,
};

use crate::error::SessionError;

/// Where a session is.
///
/// Three states, not four: an error is the return value, not a state. ADR-0032
/// Ã‚Â§5 Ã¢â‚¬â€ a transport failure and "still running" must not arrive through one
/// channel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionState {
    /// Still moving bytes.
    InProgress,
    /// Finished, and every item verified.
    Completed,
    /// Finished, and at least one item did not verify.
    Rejected,
}

/// How far a session has got.
///
/// Plain integers on purpose: this is what crosses to Dart in phase 02, and
/// ADR-0032 Ã‚Â§6 forbids anything there that needs freeing.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Progress {
    /// Bytes handed to the wire or to the disk so far.
    pub done: u64,
    /// Bytes the manifest declares in total.
    pub total: u64,
    /// Which manifest item is moving, one-based -- **and always zero, because
    /// the engine never assigns it**.
    ///
    /// QYR-0318. This said "one-based, zero before the first", which read as
    /// "the transfer has not started" for the whole of every transfer. It
    /// crosses to Dart and no screen reads it: the bar is drawn from `done` and
    /// `total`. The field stays in the ABI because removing it changes
    /// `qyro_session_progress` and the progress callback, and the honest fix
    /// for a wrong sentence is the sentence.
    pub item: u32,
}

/// How many emissions a whole transfer is allowed, whatever its size.
///
/// ADR-0033 Ã‚Â§4. The engine moves 64 KiB chunks, so one emission per chunk is
/// 16 384 calls for a gigabyte, and every one of them enqueues a message on the
/// event loop of the isolate that has to draw the bar. The unacceptable part is
/// not the number, it is that it **grows with the file**.
const PROGRESS_TARGET_EMISSIONS: u64 = 100;

/// The floor under the emission step.
///
/// Without it, a small transfer would emit a hundred times for a few kilobytes.
const PROGRESS_MIN_STEP: u64 = 256 * 1024;

/// Something that wants to be told how far a session has got.
///
/// A boxed closure and not a C function pointer: `qyro_ffi` wraps its pointer in
/// one of these, and everything on this side of the boundary Ã¢â‚¬â€ including the
/// tests that count emissions Ã¢â‚¬â€ stays ordinary Rust. ADR-0032 Ã‚Â§2 still holds,
/// because `Progress` is three integers and names no `qyro_crypto` type.
pub type ProgressObserver = Box<dyn FnMut(Progress) + Send>;

/// The emission budget of ADR-0033 Ã‚Â§4, and the state it needs.
struct Emitter {
    sink: ProgressObserver,
    /// Bytes that must pass before the next emission. Zero until `total` is
    /// known, which for a receiver is not until the manifest arrives.
    step_bytes: u64,
    last: u64,
    opened: bool,
}

impl Emitter {
    fn new(sink: ProgressObserver) -> Self {
        Self {
            sink,
            step_bytes: 0,
            last: 0,
            opened: false,
        }
    }

    /// `max(256 KiB, total/100)`.
    ///
    /// Below about 25 MiB the floor decides and there are fewer than a hundred
    /// emissions; above it the fraction decides and there are exactly a hundred.
    /// Both branches are bounded and neither grows with the file.
    const fn step_for(total: u64) -> u64 {
        let fraction = total / PROGRESS_TARGET_EMISSIONS;
        if fraction > PROGRESS_MIN_STEP {
            fraction
        } else {
            PROGRESS_MIN_STEP
        }
    }

    /// Emits if the budget allows, or if `force` says this is an ending.
    ///
    /// The terminal emission is unconditional on purpose: without it the bar
    /// stops at 99%, which is the most common visible failure of this pattern.
    fn offer(&mut self, progress: Progress, force: bool) {
        if self.step_bytes == 0 && progress.total > 0 {
            self.step_bytes = Self::step_for(progress.total);
        }
        let opening = !self.opened && progress.total > 0;
        let stepped =
            self.step_bytes > 0 && progress.done.saturating_sub(self.last) >= self.step_bytes;
        if !(force || opening || stepped) {
            return;
        }
        self.opened = true;
        self.last = progress.done;
        (self.sink)(progress);
    }
}

/// A sink that refuses to be written to.
///
/// Used only for the frames that precede the manifest. It **records** rather
/// than swallowing: content arriving before the plan that describes it is a
/// contradiction, and a silent sink would turn it into a passing run.
#[derive(Default)]
struct RefusingSink {
    written: bool,
}

impl ContentSink for RefusingSink {
    fn write_at(&mut self, _item_id: u32, _offset: u64, _bytes: &[u8]) {
        self.written = true;
    }
}

enum Role {
    Sending {
        engine: Box<Sender>,
        source: FileSource,
    },
    Receiving {
        engine: Box<Receiver>,
        sink: Option<FileSink>,
        destination: PathBuf,
    },
}

/// One transfer.
///
/// Holds the socket and the engine. There is no accessor for either, and that
/// absence is the point: `qyro_ffi` can name this type and nothing underneath
/// it.
pub struct Session {
    stream: FrameStream,
    /// The public identity the peer proved it holds, kept so a trust decision
    /// can be made after the handshake and before a manifest crosses
    /// (ADR-0035 Ã‚Â§3). `into_parts` drops the `qyro_net::Session`, so if this is
    /// not taken here it is gone.
    peer_identity: PublicIdentity,
    role: Role,
    cancel: Arc<AtomicBool>,
    failed: Option<SessionError>,
    progress: Progress,
    outbound: Vec<Vec<u8>>,
    observer: Option<Emitter>,
    /// Cuántas veces `advance` entró, y cuántas de sus lecturas vencieron.
    ///
    /// **Diagnóstico, y existe porque QYR-0365 lo pidió por su nombre.** Es una
    /// ficha abierta de severidad alta —cada archivo pequeño cuesta ~1,2 s— y su
    /// propia entrada dice cuál es la medida que la cierra: por lado, cuántas
    /// veces entra `advance` y cuántas de esas lecturas vencen. Sin contarlo no
    /// hay forma de distinguir «el par no ha contestado todavía» de «contestó y
    /// nadie leyó», y las dos piden arreglos opuestos.
    ///
    /// Dos `u64` por sesión. No se emite a ningún sitio, no cruza la frontera C
    /// y no cambia una sola decisión: sólo se puede preguntar.
    steps: u64,
    expired_reads: u64,
}

impl core::fmt::Debug for Session {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Never the engine and never the stream: a Debug that prints a sealer
        // is a key in a log. The observer is a closure the caller owns and this
        // side cannot describe, so it is reported as present or absent and not
        // as itself.
        f.debug_struct("Session")
            .field("progress", &self.progress)
            .field("failed", &self.failed)
            .field("observed", &self.observer.is_some())
            .finish_non_exhaustive()
    }
}

const fn net_error(error: &NetError) -> SessionError {
    if error.poisons() {
        SessionError::NotAuthenticated
    } else {
        SessionError::PeerUnreachable
    }
}

/// Why a receiver refused, in this crate's words.
///
/// Owned rather than re-exported, for the same reason as [`crate::PeerTrust`]:
/// ADR-0032 §2 bounds what `qyro_ffi` can name to this crate's public API, and
/// republishing `qyro_transfer::RejectReason` would put that crate's vocabulary
/// on the C boundary along with every variant it ever adds. The guard
/// `qyro_session_re_exports_nothing_it_does_not_own` is what keeps that bound
/// real, and it refused this exact re-export while it was being written.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectReason {
    /// The person said no.
    Declined,
    /// There is no room for what was offered.
    NoRoom,
    /// The manifest itself was refused — a name, a size, a count.
    UnacceptableManifest,
    /// A reason this build does not know, including a peer that sent none.
    Unspecified,
}

impl RejectReason {
    /// The stable integer the C boundary carries. Written out, not derived.
    #[must_use]
    pub const fn code(self) -> i32 {
        match self {
            Self::Declined => 0,
            Self::NoRoom => 1,
            Self::UnacceptableManifest => 2,
            Self::Unspecified => 3,
        }
    }

    const fn to_wire(self) -> WireReject {
        match self {
            Self::Declined => WireReject::Declined,
            Self::NoRoom => WireReject::NoRoom,
            Self::UnacceptableManifest => WireReject::UnacceptableManifest,
            Self::Unspecified => WireReject::Unspecified,
        }
    }

    const fn from_wire(wire: WireReject) -> Self {
        match wire {
            WireReject::Declined => Self::Declined,
            WireReject::NoRoom => Self::NoRoom,
            WireReject::UnacceptableManifest => Self::UnacceptableManifest,
            WireReject::Unspecified => Self::Unspecified,
        }
    }
}

/// Validates a pairing string and returns its address half as text.
///
/// ADR-0035 §2. Lives here rather than being re-exported from `qyro_net`,
/// because `qyro_ffi` may name only this crate and `qyro_core` — the same reason
/// [`crate::PeerTrust`] and [`crate::RejectReason`] are owned rather than
/// republished.
///
/// The fingerprint half is deliberately **not** returned: it is an expectation
/// to check against the authenticated fingerprint, not a value to display as if
/// it were established (ADR-0035 §2.1).
///
/// # Errors
///
/// [`SessionError::BadArgument`] for anything that is not a valid pairing
/// string — a wrong prefix, a field count that is not three, an address nothing
/// can dial, or a fingerprint that is not thirty-two lowercase hex characters.
pub fn parse_pairing(text: &str) -> Result<String, SessionError> {
    qyro_net::PairingEndpoint::parse(text)
        .map(|endpoint| endpoint.address().to_string())
        .map_err(|_| SessionError::BadArgument)
}

/// How many files one transfer may carry.
///
/// **ADR-0047 §3, and the reason is Android, not taste.** The per-process limit
/// on open file descriptors is hard, and on Android the picker hands back
/// **descriptors**, not paths (ADR-0034) — so a selection of a few thousand is
/// not a slow transfer, it is an exhausted process. 256 sits far under any
/// reasonable `RLIMIT_NOFILE` and above anything anybody picks by hand.
///
/// **Refused before anything is opened.** Running out of descriptors halfway
/// arrives as a system error with a file already in flight; a counted refusal
/// arrives before the first byte and says the number.
pub const MAX_FILES_PER_TRANSFER: usize = 256;

impl Session {
    /// Opens a sending session against `address`, naming files relative to
    /// `root`.
    ///
    /// `root` is what gives the receiver its names. Every path in `files` must
    /// live under it, and the name that travels is the remainder Ã¢â‚¬â€ so sending
    /// `docs/a.txt` and `notes/a.txt` from a common root sends two distinct
    /// names, where naming by file name alone would send `a.txt` twice and make
    /// the receiver arbitrate a collision that the sender created.
    ///
    /// # Errors
    ///
    /// [`SessionError::BadArgument`] when `files` is empty, when a path is not
    /// under `root`, or when the remainder is empty,
    /// [`SessionError::PeerUnreachable`] when the peer does not answer,
    /// [`SessionError::NotAuthenticated`] when it fails to prove who it is.
    /// `observer` may be `None`, and a session without one behaves identically.
    /// ADR-0033 Ã‚Â§2: the nullable pointer on the C side arrives here as `None`,
    /// and Ã‚Â«no observerÃ‚Â» must never be a second code path.
    pub fn open_sender(
        address: SocketAddr,
        root: &Path,
        files: &[PathBuf],
        observer: Option<ProgressObserver>,
    ) -> Result<Self, SessionError> {
        if files.is_empty() {
            return Err(SessionError::BadArgument);
        }
        if files.len() > MAX_FILES_PER_TRANSFER {
            return Err(SessionError::TooManyFiles {
                given: files.len(),
                limit: MAX_FILES_PER_TRANSFER,
            });
        }
        let mut planned: Vec<PlannedFile> = Vec::with_capacity(files.len());
        for source in files {
            // A path outside the root is refused rather than quietly renamed to
            // its last component: the caller asked for something this cannot
            // express, and guessing produces a name they did not choose.
            let relative = source
                .strip_prefix(root)
                .map_err(|_| SessionError::BadArgument)?;
            let mut components = Vec::new();
            for component in relative.components() {
                match component {
                    Component::Normal(part) => components.push(part.to_string_lossy().into_owned()),
                    // `..`, `.`, a root or a prefix in the remainder would mean
                    // the strip succeeded on something that is not a plain
                    // descendant. qyro_manifest refuses these too; refusing here
                    // keeps the error the caller's, not the manifest's.
                    Component::ParentDir
                    | Component::CurDir
                    | Component::RootDir
                    | Component::Prefix(_) => return Err(SessionError::BadArgument),
                }
            }
            planned.push(PlannedFile {
                source: source.clone(),
                relative: components.join("/"),
            });
        }
        if planned.iter().any(|file| file.relative.is_empty()) {
            return Err(SessionError::BadArgument);
        }

        let manifest =
            manifest_from_disk(1, 0, &planned).map_err(|_| SessionError::StorageRefused)?;
        let total = manifest
            .items()
            .iter()
            .map(qyro_manifest::ManifestItem::size)
            .sum();

        let mut paths = BTreeMap::new();
        for (index, file) in planned.iter().enumerate() {
            let item_id = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);
            paths.insert(item_id, file.source.clone());
        }

        let identity = crate::identity::current()?;
        let stream = dial(address).map_err(|error| net_error(&error))?;
        let established = initiate(stream, identity).map_err(|error| net_error(&error))?;
        let peer_identity = established.peer_identity().clone();
        let (stream, sealer, opener) = established.into_parts();

        let mut engine = Box::new(Sender::new(sealer, opener, manifest));
        let opening = engine.open().map_err(|_| SessionError::TransferRefused)?;

        let progress = Progress {
            done: 0,
            total,
            item: 0,
        };
        let mut observer = observer.map(Emitter::new);
        // The opening emission, so the bar knows its scale before the first
        // byte. A sender knows `total` here; a receiver does not learn it until
        // the manifest arrives, and emits its opening then.
        if let Some(emitter) = observer.as_mut() {
            emitter.offer(progress, false);
        }

        Ok(Self {
            stream,
            peer_identity,
            role: Role::Sending {
                engine,
                source: FileSource::new(paths),
            },
            cancel: Arc::new(AtomicBool::new(false)),
            failed: None,
            progress,
            outbound: opening,
            observer,
            steps: 0,
            expired_reads: 0,
        })
    }

    /// Opens a sending session over files that are **already open**.
    ///
    /// ADR-0034: on Android the Storage Access Framework hands out a descriptor
    /// and never a path, so there is nothing to open and nothing to reopen. The
    /// `File`s arrive owned Ã¢â‚¬â€ `qyro_ffi` did the one `unsafe` this needs, at the
    /// C boundary where `unsafe` already lives Ã¢â‚¬â€ and this session owns them from
    /// here until it drops, which is what closes them.
    ///
    /// `files` carries the relative name each one travels under, because a
    /// descriptor has no name: the picker knew it and the kernel does not.
    ///
    /// # Errors
    ///
    /// [`SessionError::BadArgument`] when `files` is empty or a name is empty,
    /// [`SessionError::StorageRefused`] when a handle cannot be read or rewound,
    /// and the same reachability and authentication errors as
    /// [`Self::open_sender`].
    pub fn open_sender_files(
        address: SocketAddr,
        files: Vec<(String, File)>,
        observer: Option<ProgressObserver>,
    ) -> Result<Self, SessionError> {
        if files.is_empty() {
            return Err(SessionError::BadArgument);
        }
        if files.len() > MAX_FILES_PER_TRANSFER {
            return Err(SessionError::TooManyFiles {
                given: files.len(),
                limit: MAX_FILES_PER_TRANSFER,
            });
        }
        if files.iter().any(|(name, _)| name.is_empty()) {
            return Err(SessionError::BadArgument);
        }
        let mut planned: Vec<PlannedOpenFile> = files
            .into_iter()
            .map(|(relative, handle)| PlannedOpenFile { handle, relative })
            .collect();

        let manifest = manifest_from_open_files(1, 0, &mut planned)
            .map_err(|_| SessionError::StorageRefused)?;
        let total = manifest
            .items()
            .iter()
            .map(qyro_manifest::ManifestItem::size)
            .sum();
        let handles = descriptors_by_item(planned);

        let identity = crate::identity::current()?;
        let stream = dial(address).map_err(|error| net_error(&error))?;
        let established = initiate(stream, identity).map_err(|error| net_error(&error))?;
        let peer_identity = established.peer_identity().clone();
        let (stream, sealer, opener) = established.into_parts();

        let mut engine = Box::new(Sender::new(sealer, opener, manifest));
        let opening = engine.open().map_err(|_| SessionError::TransferRefused)?;

        let progress = Progress {
            done: 0,
            total,
            item: 0,
        };
        let mut observer = observer.map(Emitter::new);
        if let Some(emitter) = observer.as_mut() {
            emitter.offer(progress, false);
        }

        Ok(Self {
            stream,
            peer_identity,
            role: Role::Sending {
                engine,
                source: FileSource::from_open_files(handles),
            },
            cancel: Arc::new(AtomicBool::new(false)),
            failed: None,
            progress,
            outbound: opening,
            observer,
            steps: 0,
            expired_reads: 0,
        })
    }

    /// Opens a receiving session on `port`, writing under `destination`.
    ///
    /// # Errors
    ///
    /// [`SessionError::BadArgument`] when the port cannot be bound, and the
    /// same authentication and reachability errors as [`Self::open_sender`].
    /// `observer` may be `None`, exactly as in [`Self::open_sender`].
    pub fn open_receiver(
        bind: SocketAddr,
        destination: &Path,
        observer: Option<ProgressObserver>,
    ) -> Result<Self, SessionError> {
        let listener = Listener::bind(bind).map_err(|_| SessionError::BadArgument)?;
        let identity = crate::identity::current()?;
        let accepted = listener.accept().map_err(|error| net_error(&error))?;
        let established = respond(accepted, identity).map_err(|error| net_error(&error))?;
        let peer_identity = established.peer_identity().clone();
        let (stream, sealer, opener) = established.into_parts();

        Ok(Self {
            stream,
            peer_identity,
            role: Role::Receiving {
                engine: Box::new(Receiver::new(sealer, opener)),
                sink: None,
                destination: destination.to_path_buf(),
            },
            cancel: Arc::new(AtomicBool::new(false)),
            failed: None,
            progress: Progress::default(),
            outbound: Vec::new(),
            observer: observer.map(Emitter::new),
            steps: 0,
            expired_reads: 0,
        })
    }

    /// The address of this end of the session.
    ///
    /// Needed because a receiver may be opened on port 0 and has to report which
    /// port the system chose. An accepted socket's local address carries that
    /// port, so the answer survives the `Listener` being dropped.
    ///
    /// **This returned `peer_addr` Ã¢â‚¬â€ the *far* end Ã¢â‚¬â€ until 2026-08-14**, and
    /// nothing noticed because the C surface does not expose it and no test
    /// called it (QYR-0314). What remains, and is not a defect this function can
    /// fix: `open_receiver` blocks in `accept` before returning, so a caller
    /// still cannot learn the port *before* a peer connects. Binding on port 0
    /// to announce the port is therefore still out of reach, and that is the
    /// half of QYR-0314 this does not close.
    ///
    /// # Errors
    ///
    /// [`SessionError::PeerUnreachable`] if the socket cannot answer.
    pub fn local_addr(&self) -> Result<SocketAddr, SessionError> {
        self.stream
            .local_addr()
            .map_err(|_| SessionError::PeerUnreachable)
    }

    /// How far the session has got.
    #[must_use]
    pub const fn progress(&self) -> Progress {
        self.progress
    }

    /// Asks the session to stop at the next chunk boundary.
    ///
    /// Safe from any thread and never blocks: it raises a flag rather than
    /// taking the session's lock, because taking the lock would make cancel
    /// wait for the very step it is trying to interrupt (ADR-0032 Ã‚Â§7).
    /// The peer's fingerprint, **formatted by the core**, ready to show.
    ///
    /// ADR-0035 Ã‚Â§4. Not the raw bytes: if the interface formatted this itself,
    /// two devices could render the same fingerprint differently and comparing
    /// it out loud Ã¢â‚¬â€ the only thing a fingerprint is for Ã¢â‚¬â€ would prove nothing.
    /// What the peer is offering, once the manifest has arrived.
    ///
    /// **QYR-0364.** The receiver used to be asked «accept from this device?»
    /// with a fingerprint and nothing else — no names, no count, no sizes.
    /// ADR-0036 §1 says nothing is ever accepted on its own, and **a question
    /// with no object is not a decision, it is a formality**. The GUI showed the
    /// files; the terminal could not, because this accessor did not exist.
    ///
    /// Empty before a manifest crosses, which is a real state and not an error:
    /// the trust decision about *who* is connected happens first, on purpose, so
    /// that a name never gets a chance to argue for its own acceptance.
    ///
    /// The names come back **exactly as the peer sent them**. Sanitising here
    /// would be sanitising for a screen inside a function that also feeds
    /// filesystem code; ADR-0047 §6 puts the terminal rule at the drawing site
    /// and ADR-0027 keeps the stricter filesystem rules where they belong.
    #[must_use]
    pub fn offered_files(&self) -> Vec<(String, u64)> {
        match &self.role {
            Role::Sending { .. } => Vec::new(),
            Role::Receiving { engine, .. } => {
                let Some(manifest) = engine.manifest() else {
                    return Vec::new();
                };
                manifest
                    .items()
                    .iter()
                    // `display_name` and not the whole relative path: ADR-0019
                    // decided that a person is shown the file's name, and a
                    // relative path drawn in a confirmation is a place for a
                    // peer to write something that looks like a directory the
                    // person recognises.
                    .map(|item| (item.display_name().to_owned(), item.size()))
                    .collect()
            }
        }
    }

    #[must_use]
    pub fn peer_fingerprint(&self) -> String {
        crate::trust::fingerprint_text(&self.peer_identity)
    }

    /// What `book` says about the peer **this handshake authenticated**.
    ///
    /// ADR-0035 Ã‚Â§3: the identity handed to the decision is the authenticated
    /// one, never a name or a fingerprint that arrived in a pairing string. A
    /// pairing string sets an expectation; only this proves anything.
    ///
    /// # Errors
    ///
    /// [`SessionError::BadArgument`] for a name the peer store refuses.
    pub fn peer_trust(
        &self,
        book: &crate::TrustBook,
        name: &str,
    ) -> Result<crate::PeerTrust, SessionError> {
        book.verdict(name, &self.peer_identity)
    }

    /// Records this peer under `name`. Only a person may cause this.
    ///
    /// # Errors
    ///
    /// [`SessionError::BadArgument`] for a name the peer store refuses.
    pub fn remember_peer(
        &self,
        book: &mut crate::TrustBook,
        name: &str,
    ) -> Result<(), SessionError> {
        book.remember(name, &self.peer_identity)
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }

    /// Whether cancellation has been asked for.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }

    /// Moves the session forward, blocking until something happens.
    ///
    /// # Errors
    ///
    /// Once anything has failed, **the same error every time**: ADR-0032 Ã‚Â§5
    /// freezes stickiness as returning the same code, which is why `error.rs`
    /// deliberately has no `AlreadyFailed` variant Ã¢â‚¬â€ a second `Ok` would let a
    /// caller believe a session recovered when its worker is dead (QYR-0319).
    pub fn step(&mut self) -> Result<SessionState, SessionError> {
        if let Some(failure) = self.failed {
            return Err(failure);
        }
        if self.cancel.load(Ordering::Acquire) {
            return Err(self.fail(SessionError::Cancelled));
        }
        match self.advance() {
            Ok(state) => {
                // The terminal emission is forced, because a bar that stops at
                // 99% is the most common visible failure of this pattern
                // (ADR-0033 Ã‚Â§4). An *error* deliberately emits nothing: the
                // caller already got a stronger signal than a progress update.
                self.emit(state != SessionState::InProgress);
                Ok(state)
            }
            Err(error) => Err(self.fail(error)),
        }
    }

    /// Cuántos pasos ha dado esta sesión, y cuántas lecturas vencieron.
    ///
    /// **La medida que QYR-0365 pidió por su nombre.** Una lectura vencida es un
    /// `READ_TIMEOUT` entero —250 ms— gastado esperando a un par que no dijo
    /// nada. Si vencen muchas de un lado y ninguna del otro, el que espera es
    /// ese, y ahí está el defecto.
    ///
    /// No cruza la frontera C: es para una prueba y para quien lea el código.
    #[must_use]
    pub const fn step_tally(&self) -> (u64, u64) {
        (self.steps, self.expired_reads)
    }

    /// Offers the current progress to the observer, if there is one.
    fn emit(&mut self, terminal: bool) {
        let progress = self.progress;
        if let Some(emitter) = self.observer.as_mut() {
            emitter.offer(progress, terminal);
        }
    }

    fn fail(&mut self, error: SessionError) -> SessionError {
        self.failed = Some(error);
        error
    }

    /// Puts everything the engine has produced on the wire.
    ///
    /// Called at the start of a step *and* again when a step turns out to be the
    /// last one. Only at the start is not enough: the receiver produces its
    /// `IntegrityResult` in the same step that takes it to `Phase::Done`, and a
    /// step that returns a terminal state is the last one anybody calls. Those
    /// bytes are the sender's only way to learn the transfer verified, so
    /// leaving them in `outbound` makes a successful transfer end as
    /// `PeerUnreachable` on the sending side (QYR-0316).
    fn write_outbound(&mut self) -> Result<(), SessionError> {
        for frame in core::mem::take(&mut self.outbound) {
            self.stream
                .write_frame(&frame)
                .map_err(|error| net_error(&error))?;
        }
        self.stream.flush().map_err(|error| net_error(&error))
    }

    fn advance(&mut self) -> Result<SessionState, SessionError> {
        self.steps = self.steps.saturating_add(1);
        self.write_outbound()?;

        let inbound = match self.stream.read_frame() {
            Ok(Some(frame)) => Some(
                frame
                    .try_encode()
                    .map_err(|_| SessionError::TransferRefused)?,
            ),
            Ok(None) => {
                // Venció el reloj de lectura sin un frame. **Es el suceso que
                // QYR-0365 mide.**
                self.expired_reads = self.expired_reads.saturating_add(1);
                None
            }
            Err(error) => {
                if self.finished() {
                    return Ok(self.verdict());
                }
                return Err(net_error(&error));
            }
        };

        // Declared without a value on purpose: both arms are required to set it,
        // and a `false` default would let a future arm fall through to "not
        // finished" silently.
        let finished;
        match &mut self.role {
            Role::Sending { engine, source } => {
                if let Some(bytes) = inbound {
                    engine
                        .deliver(&bytes)
                        .map_err(|_| SessionError::TransferRefused)?;
                }
                let produced = engine
                    .pump(source)
                    .map_err(|_| SessionError::TransferRefused)?;
                self.outbound = produced;
                self.progress.done = engine.bytes_sent();
                finished = engine.phase() == Phase::Done;
            }
            Role::Receiving {
                engine,
                sink,
                destination,
            } => {
                if let Some(bytes) = inbound {
                    let mut refusing = RefusingSink::default();
                    let answers = match sink.as_mut() {
                        Some(target) => engine.deliver(&bytes, target),
                        None => engine.deliver(&bytes, &mut refusing),
                    }
                    .map_err(|_| SessionError::TransferRefused)?;
                    if refusing.written {
                        return Err(SessionError::TransferRefused);
                    }
                    if sink.is_none()
                        && let Some(manifest) = engine.manifest()
                    {
                        self.progress.total = manifest
                            .items()
                            .iter()
                            .map(qyro_manifest::ManifestItem::size)
                            .sum();
                        *sink = Some(
                            FileSink::new(destination, manifest)
                                .map_err(|_| SessionError::StorageRefused)?,
                        );
                    }
                    self.outbound = answers;
                }
                finished = engine.phase() == Phase::Done;
            }
        }
        if finished {
            // The step that ends the transfer is the last one a caller makes, so
            // anything the ending produced has to leave here rather than wait
            // for a step that never comes.
            self.write_outbound()?;
            return Ok(self.verdict());
        }
        Ok(SessionState::InProgress)
    }

    fn finished(&self) -> bool {
        match &self.role {
            Role::Sending { engine, .. } => {
                // `Rejected` ends the session as surely as `Done` does. Before
                // QYR-0089 there was nothing to end it: a refused sender kept
                // stepping against a peer that had stopped listening.
                matches!(engine.phase(), Phase::Done | Phase::Rejected)
            }
            Role::Receiving { engine, .. } => {
                matches!(engine.phase(), Phase::Done | Phase::Rejected)
            }
        }
    }

    /// Why the receiver refused, if it did.
    ///
    /// QYR-0089. `SessionState::Rejected` says the transfer did not happen; this
    /// says why, and it is the difference between Â«could not send itÂ» and Â«they
    /// said noÂ» on a screen.
    #[must_use]
    pub fn rejection(&self) -> Option<RejectReason> {
        match &self.role {
            Role::Sending { engine, .. } => engine.rejection().map(RejectReason::from_wire),
            Role::Receiving { .. } => None,
        }
    }

    /// Refuses the offered transfer, with a reason the sender will see.
    ///
    /// The receiving half of QYR-0089, and the operation the receive screen
    /// needs: without it the only Â«noÂ» a person could express was a cancel,
    /// which says something else.
    ///
    /// Everything already written is released â€” `FileSink::abandon` â€” so a
    /// refusal leaves the destination as it found it.
    ///
    /// # Errors
    ///
    /// [`SessionError::BadArgument`] on a sending session, and
    /// [`SessionError::TransferRefused`] when the engine is already finished.
    pub fn reject(&mut self, reason: RejectReason) -> Result<(), SessionError> {
        let Role::Receiving { engine, sink, .. } = &mut self.role else {
            return Err(SessionError::BadArgument);
        };
        let bytes = engine
            .reject_transfer(reason.to_wire())
            .map_err(|_| SessionError::TransferRefused)?;
        // The refusal goes on the wire before the parts are removed: a receiver
        // that cleaned up and then failed to send would leave the sender waiting
        // for chunks nobody will ever accept.
        self.stream
            .write_frame(&bytes)
            .map_err(|error| net_error(&error))?;
        // `Option`, because a receiver that never saw a manifest has no sink to
        // abandon — and refusing before the manifest is a legitimate «no».
        if let Some(sink) = sink.as_mut() {
            sink.abandon();
        }
        Ok(())
    }

    fn verdict(&mut self) -> SessionState {
        let all_ok = match &self.role {
            Role::Sending { engine, .. } => engine
                .integrity()
                .is_some_and(|items| items.iter().all(|(_, v)| *v == ItemVerdict::Ok)),
            Role::Receiving { engine, .. } => {
                let verdicts = engine.verdicts();
                !verdicts.is_empty() && verdicts.iter().all(|(_, v)| *v == ItemVerdict::Ok)
            }
        };
        if all_ok {
            SessionState::Completed
        } else {
            SessionState::Rejected
        }
    }

    /// Materialises what arrived, and releases what did not.
    ///
    /// A receiver that stops early leaves a `.qyro-part` per started item and
    /// nothing else removes it, so this is called on every ending and not only
    /// on the happy one (QYR-0087, QYR-0088).
    ///
    /// # Errors
    ///
    /// [`SessionError::StorageRefused`] when an item the engine accepted fails
    /// to verify on disk Ã¢â‚¬â€ two checks over the same bytes disagreeing means one
    /// of them is wrong.
    pub fn finish(&mut self) -> Result<u32, SessionError> {
        let Role::Receiving { engine, sink, .. } = &mut self.role else {
            return Ok(0);
        };
        let Some(target) = sink.as_mut() else {
            return Ok(0);
        };
        let verdicts = engine.verdicts();
        let mut materialised = 0_u32;
        for (item_id, verdict) in &verdicts {
            match target.finish_item(*item_id) {
                Ok(_) => {
                    if *verdict == ItemVerdict::Ok {
                        materialised = materialised.saturating_add(1);
                    } else {
                        return Err(SessionError::StorageRefused);
                    }
                }
                Err(_) => {
                    if *verdict == ItemVerdict::Ok {
                        return Err(SessionError::StorageRefused);
                    }
                }
            }
        }
        Ok(materialised)
    }
}

/// The emission budget, pinned rather than bounded.
///
/// `tests/session_behaviour.rs` proves the budget holds over a real socket, and
/// that is the test worth having. But a ceiling and a strict inequality between
/// two sizes are satisfied by **any** formula that stays under the ceiling and
/// grows with the file, so seven mutants inside `Emitter` survived it Ã¢â‚¬â€ `/`
/// swapped for `%` among them (QYR-0321).
///
/// The lesson, and the reason these live here: two sizes and a strict inequality
/// tell a measured value from a **constant**, and do not tell one measurement
/// from **another**. The arithmetic is pure, so it gets tested as arithmetic.
#[cfg(test)]
mod tests {
    #![allow(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "a test that cannot fail loudly is not a test"
    )]

    #[test]
    fn more_files_than_the_ceiling_are_refused_by_number_before_anything_opens() {
        // ADR-0047 §3. The refusal has to arrive **before** the first
        // descriptor, because running out halfway is a system error with a file
        // already in flight, and a counted refusal is a sentence somebody can
        // act on. The address is never dialled: the check runs first.
        let files: Vec<std::path::PathBuf> = (0..=crate::MAX_FILES_PER_TRANSFER)
            .map(|index| std::path::PathBuf::from(format!("/root/f{index}")))
            .collect();
        let outcome = crate::Session::open_sender(
            "127.0.0.1:1".parse().expect("a literal address"),
            std::path::Path::new("/root"),
            &files,
            None,
        );
        assert!(
            matches!(
                outcome,
                Err(crate::SessionError::TooManyFiles { given, limit })
                    if given == crate::MAX_FILES_PER_TRANSFER + 1
                        && limit == crate::MAX_FILES_PER_TRANSFER
            ),
            "one over the ceiling was not refused by number"
        );
    }

    #[test]
    fn and_exactly_the_ceiling_is_not_refused_for_being_too_many() {
        // The control. A check written `>=` would pass the test above and
        // silently move the real ceiling to 255 -- which nobody would notice
        // until somebody counted.
        let files: Vec<std::path::PathBuf> = (0..crate::MAX_FILES_PER_TRANSFER)
            .map(|index| std::path::PathBuf::from(format!("/root/f{index}")))
            .collect();
        let outcome = crate::Session::open_sender(
            "127.0.0.1:1".parse().expect("a literal address"),
            std::path::Path::new("/root"),
            &files,
            None,
        );
        assert!(
            !matches!(outcome, Err(crate::SessionError::TooManyFiles { .. })),
            "the ceiling itself was refused, so the real limit is one lower"
        );
    }

    use std::sync::{Arc, Mutex};

    use super::{
        Emitter, PROGRESS_MIN_STEP, PROGRESS_TARGET_EMISSIONS, Progress, ProgressObserver,
    };

    /// The size at which the fraction first ties the floor: 256 KiB Ãƒâ€” 100.
    const ELBOW: u64 = PROGRESS_MIN_STEP * PROGRESS_TARGET_EMISSIONS;

    type Log = Arc<Mutex<Vec<Progress>>>;

    fn recorder() -> (Log, ProgressObserver) {
        let log: Log = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&log);
        let observer: ProgressObserver = Box::new(move |progress| {
            if let Ok(mut entries) = sink.lock() {
                entries.push(progress);
            }
        });
        (log, observer)
    }

    fn seen(log: &Log) -> Vec<Progress> {
        log.lock()
            .map(|entries| entries.clone())
            .unwrap_or_default()
    }

    fn at(done: u64, total: u64) -> Progress {
        Progress {
            done,
            total,
            item: 0,
        }
    }

    #[test]
    fn the_step_is_the_floor_below_the_elbow_and_exactly_the_fraction_above_it() {
        // Below: the floor decides, so a tiny transfer does not emit a hundred
        // times for a few kilobytes.
        assert_eq!(Emitter::step_for(0), PROGRESS_MIN_STEP);
        assert_eq!(Emitter::step_for(1024), PROGRESS_MIN_STEP);
        assert_eq!(Emitter::step_for(ELBOW - 100), PROGRESS_MIN_STEP);

        // At the elbow the two agree, which is what makes `>` and `>=` the same
        // function here -- see the equivalence test below.
        assert_eq!(Emitter::step_for(ELBOW), PROGRESS_MIN_STEP);

        // One step-group past it the fraction takes over, and the value is
        // *exact*. `>` swapped for `==` returns the floor here instead.
        assert_eq!(Emitter::step_for(ELBOW + 100), PROGRESS_MIN_STEP + 1);

        // And far above, the step is precisely total/100. `/` swapped for `%`
        // gives 24 for this input, which is under the floor and would return the
        // floor -- a number this assertion refuses.
        const GIB: u64 = 1024 * 1024 * 1024;
        assert_eq!(Emitter::step_for(GIB), GIB / PROGRESS_TARGET_EMISSIONS);
        assert_eq!(Emitter::step_for(GIB), 10_737_418);
    }

    #[test]
    fn swapping_the_floor_comparison_for_a_non_strict_one_cannot_change_the_answer() {
        // Proved rather than excused, the way the `|` to `^` mutant in the FFI
        // handle was. The two branches return the same value whenever the
        // comparison differs -- which is only when fraction == floor, and then
        // both hand back that same number. So `>` to `>=` is an equivalent
        // mutant, and no test can kill it.
        for total in [0, 1024, ELBOW - 1, ELBOW, ELBOW + 1, 1024 * 1024 * 1024] {
            let fraction = total / PROGRESS_TARGET_EMISSIONS;
            let strict = if fraction > PROGRESS_MIN_STEP {
                fraction
            } else {
                PROGRESS_MIN_STEP
            };
            let loose = if fraction >= PROGRESS_MIN_STEP {
                fraction
            } else {
                PROGRESS_MIN_STEP
            };
            assert_eq!(
                strict, loose,
                "the two comparisons disagree at total={total}, so the mutant is \
                 not equivalent after all and needs a test rather than this proof"
            );
            assert_eq!(Emitter::step_for(total), strict);
        }
    }

    #[test]
    fn an_observer_hears_nothing_until_the_total_is_known() {
        // A receiver does not learn its total until the manifest arrives, and a
        // bar with no length is not progress. Every comparison against zero in
        // `offer` is what keeps this true: swap any `> 0` for `>= 0` on a u64
        // and it becomes always-true, so this starts emitting.
        let (log, sink) = recorder();
        let mut emitter = Emitter::new(sink);

        for _ in 0..5 {
            emitter.offer(at(0, 0), false);
        }

        // And still nothing once bytes have moved but the total has not arrived.
        // This is the case that separates the two guards in `offer`: with a
        // total of zero the step must stay unset, because a step computed from
        // nothing is the floor, and a floor's worth of bytes would then emit a
        // progress reading whose total is zero -- a bar that is 300 KiB along a
        // journey of unknown length.
        emitter.offer(at(PROGRESS_MIN_STEP + 1, 0), false);
        emitter.offer(at(PROGRESS_MIN_STEP * 4, 0), false);

        assert!(
            seen(&log).is_empty(),
            "emitted {} times with no total known: {:?}",
            seen(&log).len(),
            seen(&log)
        );
    }

    #[test]
    fn the_first_offer_that_knows_the_total_is_the_opening_emission() {
        let (log, sink) = recorder();
        let mut emitter = Emitter::new(sink);

        emitter.offer(at(0, 4 * 1024 * 1024), false);

        let entries = seen(&log);
        assert_eq!(entries.len(), 1, "the opening emission did not happen once");
        assert_eq!(entries[0], at(0, 4 * 1024 * 1024));
    }

    #[test]
    fn an_emission_lands_on_its_boundary_and_not_one_byte_early() {
        const TOTAL: u64 = 4 * 1024 * 1024;
        let (log, sink) = recorder();
        let mut emitter = Emitter::new(sink);
        emitter.offer(at(0, TOTAL), false);
        assert_eq!(seen(&log).len(), 1, "the opening emission is the baseline");

        // One byte short of the step: still silent.
        emitter.offer(at(PROGRESS_MIN_STEP - 1, TOTAL), false);
        assert_eq!(
            seen(&log).len(),
            1,
            "emitted one byte before the boundary, so the step is not the step"
        );

        // Exactly on it: emits.
        emitter.offer(at(PROGRESS_MIN_STEP, TOTAL), false);
        assert_eq!(seen(&log).len(), 2, "the boundary itself did not emit");

        // And the next boundary is measured from the last emission, not from
        // zero: a counter that measured from zero would fire again immediately.
        emitter.offer(at(PROGRESS_MIN_STEP + 1, TOTAL), false);
        assert_eq!(
            seen(&log).len(),
            2,
            "emitted again one byte after the last emission"
        );
    }

    #[test]
    fn an_ending_emits_even_when_it_is_nowhere_near_a_boundary() {
        // Without this the bar stops at 99%, which is the visible failure the
        // forced terminal emission exists to prevent (ADR-0033 Ã‚Â§4).
        const TOTAL: u64 = 4 * 1024 * 1024;
        let (log, sink) = recorder();
        let mut emitter = Emitter::new(sink);
        emitter.offer(at(0, TOTAL), false);

        emitter.offer(at(TOTAL, TOTAL), true);

        let entries = seen(&log);
        assert_eq!(entries.len(), 2);
        let last = entries[1];
        assert_eq!(last.done, last.total, "the ending did not report the total");
    }

    #[test]
    fn the_whole_budget_is_bounded_by_a_constant_and_not_by_the_file() {
        // The property ADR-0033 Ã‚Â§1 exists for, checked on sizes no socket test
        // could afford: a gigabyte must not cost more emissions than 4 MiB does.
        for total in [
            4 * 1024 * 1024_u64,
            100 * 1024 * 1024,
            1024 * 1024 * 1024,
            64 * 1024 * 1024 * 1024,
        ] {
            let (log, sink) = recorder();
            let mut emitter = Emitter::new(sink);
            let step = Emitter::step_for(total);
            let mut done = 0;
            emitter.offer(at(0, total), false);
            // Offer at every chunk boundary, which is what the engine does.
            while done < total {
                done = done.saturating_add(64 * 1024).min(total);
                emitter.offer(at(done, total), false);
            }
            emitter.offer(at(total, total), true);

            let count = seen(&log).len();
            assert!(
                count <= 102,
                "{count} emissions for {total} bytes at a step of {step}; the \
                 budget is 102 whatever the size"
            );
            assert!(
                count >= 2,
                "{count} emissions for {total} bytes: an opening and an ending \
                 are the minimum, so this measurement is not seeing them"
            );
        }
    }
}
