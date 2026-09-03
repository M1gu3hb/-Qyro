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
use std::time::{Duration, Instant};

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
/// §5 — a transport failure and "still running" must not arrive through one
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
/// ADR-0032 §6 forbids anything there that needs freeing.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Progress {
    /// Bytes handed to the wire or to the disk so far.
    pub done: u64,
    /// Bytes the manifest declares in total.
    pub total: u64,
    /// Por que entrada del manifiesto va, **en base uno**. Cero solo antes de
    /// que empiece a moverse nada.
    ///
    /// QYR-0318 lo documento como «siempre cero, porque el motor no lo asigna» y
    /// lo dejo ahi. Describir un defecto con precision no es arreglarlo: el
    /// campo llevaba desde la fase 02 cruzando a Dart con un cero, y ADR-0050
    /// §4.1 pide «archivo N de M» sobre el.
    ///
    /// **La M no esta aqui.** Anadirla es un parametro mas en
    /// `QyroProgressFn`, y eso es la frontera C con su enmienda a ADR-0032.
    /// Va con su ceremonia, no de rebote en este arreglo.
    pub item: u32,
}

/// How many emissions a whole transfer is allowed, whatever its size.
///
/// ADR-0033 §4. The engine moves 64 KiB chunks, so one emission per chunk is
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
/// one of these, and everything on this side of the boundary — including the
/// tests that count emissions — stays ordinary Rust. ADR-0032 §2 still holds,
/// because `Progress` is three integers and names no `qyro_crypto` type.
pub type ProgressObserver = Box<dyn FnMut(Progress) + Send>;

/// The emission budget of ADR-0033 §4, and the state it needs.
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
    /// (ADR-0035 §3). `into_parts` drops the `qyro_net::Session`, so if this is
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
    /// Si el último paso del emisor no produjo un solo frame.
    ///
    /// QYR-0393. Un emisor sin nada que mandar no está midiendo la red: está
    /// esperando a que el otro lado conteste, y el otro lado puede ser una
    /// persona leyendo una pantalla.
    nothing_left_to_send: bool,
    /// Cuándo empezó el paso anterior, para saber si este lado estuvo ausente.
    ///
    /// QYR-0393. Un hueco grande entre dos pasos no es un par callado: es un
    /// consumidor que dejó de escuchar, casi siempre porque está preguntando
    /// algo a una persona.
    last_step_at: Instant,
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

/// Why a **bind** failed, which is not the same question as why a wire ended.
///
/// ADR-0041 §3 wants «that port is taken» to reach the person as its own fact,
/// with the offer to choose another. Every other reason a bind fails is the
/// caller's argument being wrong — an address this machine does not hold, a
/// malformed one — and telling somebody to try another port there would send
/// them looking in the wrong place.
///
/// `PermissionDenied` sits beside `AddrInUse` because of Windows: a bind inside
/// a range reserved for Hyper-V, WSL2 or Docker fails with `WSAEACCES` (10013),
/// not with «in use». Same fact to the person, same answer.
fn bind_error(error: NetError) -> SessionError {
    match error {
        NetError::SocketFailed {
            kind: std::io::ErrorKind::AddrInUse | std::io::ErrorKind::PermissionDenied,
            ..
        } => SessionError::PortUnavailable,
        _ => SessionError::BadArgument,
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

/// The **expectation** a pairing string carries: thirty-two lowercase hex.
///
/// **QYR-0381, and the comment above this was describing an intention.** It says
/// the fingerprint half «is an expectation to check against the authenticated
/// fingerprint», and ADR-0035 §2.1 is explicit: *«si la cadena llevaba una huella
/// y no coincide con la autenticada, la sesión se rechaza **sin preguntar a
/// nadie**. Quien escaneó ya contestó la pregunta, y preguntar otra vez es cómo
/// la gente aprende a decir que sí.»*
///
/// Nothing checked it. `parse_pairing` returned the address, threw the
/// fingerprint away, and no caller had any way to get it back — not the CLI, not
/// `qyro_pairing_parse`, which emits the address and nothing else. So the code a
/// person **typed by hand, comparing it character by character**, established
/// less than typing an `ip:port` and adding `--expect` afterwards.
///
/// Returned separately from [`parse_pairing`] rather than as a pair, so that
/// every existing caller keeps meaning what it meant: an address is what most of
/// them want, and a tuple would have made each of them decide what to do with a
/// second value they had not asked for.
///
/// # Errors
///
/// [`SessionError::BadArgument`], for exactly the same inputs as
/// [`parse_pairing`] — the two never disagree about whether a string is a
/// pairing string.
pub fn pairing_fingerprint(text: &str) -> Result<String, SessionError> {
    qyro_net::PairingEndpoint::parse(text)
        .map(|endpoint| {
            endpoint
                .fingerprint()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect()
        })
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

/// Cuánto hueco entre dos pasos cuenta como «este lado no estaba escuchando».
///
/// QYR-0393. Un bucle normal da un paso cada `READ_TIMEOUT` (250 ms) más lo que
/// tarde el trabajo, así que quince segundos no es un bucle lento: es un
/// consumidor que se paró. Elegido lejos de los dos extremos a propósito — no
/// tan corto que un disco lento lo dispare, no tan largo que se coma la ventana
/// de sesenta segundos que protege.
const SELF_ABSENCE: Duration = Duration::from_secs(15);

/// Cuánto se sigue vaciando el cable después de decir «cancelo».
///
/// **QYR-0400.** Lo suficiente para que el emisor termine de escribir lo que
/// tenía en vuelo y alcance a leer la despedida; lo bastante poco para que
/// cancelar siga sintiéndose inmediato. Son unos pocos `READ_TIMEOUT`.
const GOODBYE_DRAIN: Duration = Duration::from_millis(750);

impl Session {
    /// Opens a sending session against `address`, naming files relative to
    /// `root`.
    ///
    /// `root` is what gives the receiver its names. Every path in `files` must
    /// live under it, and the name that travels is the remainder — so sending
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
    /// ADR-0033 §2: the nullable pointer on the C side arrives here as `None`,
    /// and «no observer» must never be a second code path.
    pub fn open_sender(
        address: SocketAddr,
        root: &Path,
        files: &[PathBuf],
        observer: Option<ProgressObserver>,
    ) -> Result<Self, SessionError> {
        if files.is_empty() {
            return Err(SessionError::BadArgument);
        }
        // **Se cuentan los archivos, no las entradas** (ADR-0050 enmienda 1).
        //
        // El límite existe por los descriptores —ADR-0047 §3 lo dice con esas
        // palabras— y **una carpeta no abre ninguno**: se crea en el destino y
        // ya. Desde que las carpetas viajan, contarlas aquí rechazaría un árbol
        // de 200 archivos con 60 carpetas por un motivo que no le aplica.
        //
        // `is_dir()` es un `stat`, no un `open`, así que la negativa sigue
        // llegando **antes de abrir nada**, que es donde ADR-0047 §3 la quiere.
        let opening = files.iter().filter(|source| !source.is_dir()).count();
        if opening > MAX_FILES_PER_TRANSFER {
            return Err(SessionError::TooManyFiles {
                given: opening,
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
            last_step_at: Instant::now(),
            nothing_left_to_send: false,
        })
    }

    /// Opens a sending session over files that are **already open**.
    ///
    /// ADR-0034: on Android the Storage Access Framework hands out a descriptor
    /// and never a path, so there is nothing to open and nothing to reopen. The
    /// `File`s arrive owned — `qyro_ffi` did the one `unsafe` this needs, at the
    /// C boundary where `unsafe` already lives — and this session owns them from
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
            last_step_at: Instant::now(),
            nothing_left_to_send: false,
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
        let listener = Listener::bind(bind).map_err(bind_error)?;
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
            last_step_at: Instant::now(),
            nothing_left_to_send: false,
        })
    }

    /// The address of this end of the session.
    ///
    /// Needed because a receiver may be opened on port 0 and has to report which
    /// port the system chose. An accepted socket's local address carries that
    /// port, so the answer survives the `Listener` being dropped.
    ///
    /// **This returned `peer_addr` — the *far* end — until 2026-08-14**, and
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
    /// wait for the very step it is trying to interrupt (ADR-0032 §7).
    /// The peer's fingerprint, **formatted by the core**, ready to show.
    ///
    /// ADR-0035 §4. Not the raw bytes: if the interface formatted this itself,
    /// two devices could render the same fingerprint differently and comparing
    /// it out loud — the only thing a fingerprint is for — would prove nothing.
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
    /// Steps until the offer and its manifest have arrived, and no further.
    ///
    /// **QYR-0372, and the number is the finding: it takes two steps, not one.**
    /// `open_receiver` returns the moment the handshake completes, and the offer
    /// and the manifest come afterwards. Measured on the loopback, with
    /// `session_behaviour::what_is_offered_is_unknown_until_await_offer_and_known_after_it`:
    ///
    /// | after | `offered_files()` | `progress().total` |
    /// |---|---|---|
    /// | step 1 | empty | **0** |
    /// | step 2 | the manifest | the real total |
    ///
    /// Both consumers got that wrong, in the same direction and for the same
    /// reason. The terminal asked «accept from this device? [y/N]» having called
    /// [`Self::offered_files`] with no step at all, and printed «they have not
    /// said what they are sending yet» — the question with no object that
    /// QYR-0364 is recorded as having closed. The Dart worker took **one**
    /// `stepBlocking()` and sent `progress().total` along with the offer, so the
    /// dialog on the phone offered **0 bytes** — and 0 is not «unknown» to
    /// somebody reading it, it is «nothing», which is a different lie.
    ///
    /// So the number lives here, once, and neither consumer has to know it.
    ///
    /// # Why a bound and not a loop
    ///
    /// A peer that connects and then says nothing must not turn this into a
    /// hang: the read deadline inside `qyro_net` ends the wait, but a caller
    /// spinning on «not yet» would restart it forever. Eight is far above the
    /// two this needs and far below anything a stall could hide behind.
    ///
    /// # Errors
    ///
    /// Whatever the underlying step returned. A session that ends before the
    /// manifest arrives is **not** an error here — it returns `Ok` with nothing
    /// offered, and the caller's own «nothing was offered» path is the right one
    /// to take.
    pub fn await_offer(&mut self) -> Result<(), SessionError> {
        // Eight, and see the doc comment: two is what it takes.
        for _ in 0..8 {
            if !self.offered_files().is_empty() {
                return Ok(());
            }
            if self.step()? != SessionState::InProgress {
                return Ok(());
            }
        }
        Ok(())
    }

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
    /// ADR-0035 §3: the identity handed to the decision is the authenticated
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
    /// Once anything has failed, **the same error every time**: ADR-0032 §5
    /// freezes stickiness as returning the same code, which is why `error.rs`
    /// deliberately has no `AlreadyFailed` variant — a second `Ok` would let a
    /// caller believe a session recovered when its worker is dead (QYR-0319).
    pub fn step(&mut self) -> Result<SessionState, SessionError> {
        if let Some(failure) = self.failed {
            return Err(failure);
        }
        if self.cancel.load(Ordering::Acquire) {
            // **Y se lo decimos al otro lado antes de irnos.**
            //
            // Hasta aquí `cancel()` sólo ponía una bandera local: este paso
            // fallaba y el par se quedaba esperando hasta que vencía su reloj de
            // inactividad — sesenta segundos para leer «el otro aparato no
            // responde», que es el nombre equivocado de «alguien lo paró».
            //
            // `request_cancel()` existe desde la fase 04 y **no lo llamaba nada
            // de producción**. ADR-0050 §4.2 pide un cancelar que pare el lote
            // entero, y un lote tiene dos extremos.
            //
            // Si el frame no sale, no cambia nada: se falla igual. Un adiós que
            // no se pudo dar no es motivo para no irse.
            let farewell = match &mut self.role {
                Role::Sending { engine, .. } => engine.request_cancel().ok(),
                Role::Receiving { engine, .. } => engine.request_cancel().ok(),
            };
            if let Some(bytes) = farewell {
                self.outbound = vec![bytes];
                let _ = self.write_outbound();
                // **Un adiós que no se oye no es un adiós** (QYR-0400).
                //
                // Quien cancela deja de leer inmediatamente. Si el otro lado
                // estaba empujando un archivo, su ventana se llena, el búfer de
                // recepción de aquí se queda lleno con una aplicación que ya no
                // lo vacía, y el sistema contesta con un **RST**. En Windows un
                // RST **descarta lo que hubiera en el búfer del par**: el frame
                // de cancelación que se acaba de escribir se pierde en tránsito,
                // y el emisor lee «el otro aparato no responde» — el nombre
                // equivocado, que es justo lo que la fase 25 §5 existe para
                // evitar.
                //
                // En Linux los búferes son mayores y el par suele alcanzar a
                // leer el frame antes, así que esto se veía **sólo** en el
                // trabajador de Windows. La diferencia no era del protocolo:
                // era de cuánto aguanta un búfer.
                //
                // Así que se sigue vaciando el cable un momento después de
                // decir adiós. No se procesa nada de lo que llegue —la sesión
                // ya está cancelada— sólo se consume, para que el emisor pueda
                // terminar de escribir y llegue a leer la despedida.
                //
                // **Acotado y corto**: es una cortesía de salida, no una espera.
                // Si el emisor no se calla en este plazo, se sale igual: irse
                // tarde es peor que irse sin que te oigan.
                let until = Instant::now() + GOODBYE_DRAIN;
                while Instant::now() < until {
                    match self.stream.read_frame() {
                        Ok(Some(_)) => {}
                        // Venció la lectura y no llegó nada: el emisor ya se
                        // calló, que es exactamente lo que se esperaba.
                        Ok(None) => break,
                        Err(_) => break,
                    }
                }
            }
            return Err(self.fail(SessionError::Cancelled));
        }
        match self.advance() {
            Ok(state) => {
                // The terminal emission is forced, because a bar that stops at
                // 99% is the most common visible failure of this pattern
                // (ADR-0033 §4). An *error* deliberately emits nothing: the
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

    /// Cuánto silencio del otro lado tolera esta sesión **en el estado en que
    /// está ahora**.
    fn silence_budget(&self) -> Duration {
        let waiting_on_a_decision = match &self.role {
            // Un emisor **que no tiene nada que poner en el cable** está
            // esperando al otro lado, y no a la red. Medido: durante los 65 s
            // que tarda una persona en contestar, el emisor da 227 pasos en
            // `Transferring` sin producir un solo frame -- ya lo mandó todo y
            // espera acuses que no llegan porque nadie está leyendo.
            //
            // Las otras dos fases son el mismo caso escrito por su nombre:
            // `Negotiating` espera una aceptación, y `AwaitingIntegrity` espera
            // un SHA-256 sobre todo lo recibido, que sobre cuatro gigas pasa de
            // sesenta segundos **sin que nada vaya mal**.
            //
            // Mientras hay algo que mandar, el reloj vuelve a ser el de sesenta:
            // ahí el silencio del otro sí es una medida de la red.
            Role::Sending { engine, .. } => {
                self.nothing_left_to_send
                    || matches!(
                        engine.phase(),
                        Phase::Negotiating | Phase::AwaitingIntegrity
                    )
            }
            // Un receptor que negocia todavía no tiene manifiesto: lo que espera
            // es que el otro lado ofrezca. Su otra pausa -- la de preguntar a
            // una persona -- la cubre `SELF_ABSENCE`, porque ahí el que no
            // escucha es él.
            Role::Receiving { engine, .. } => matches!(engine.phase(), Phase::Negotiating),
        };
        if waiting_on_a_decision {
            qyro_net::DECISION_DEADLINE
        } else {
            qyro_net::IDLE_TIMEOUT
        }
    }

    /// El reloj de silencio que esta sesión está usando ahora mismo.
    ///
    /// Público para que una prueba pueda comprobar la política de QYR-0393 sin
    /// esperar un minuto: la prueba que la mide de verdad tarda 65 s y está
    /// `#[ignore]`, y una propiedad que sólo se comprueba a mano es una
    /// propiedad que se rompe sin que nadie lo note.
    #[must_use]
    pub const fn idle_deadline(&self) -> Duration {
        self.stream.idle_timeout()
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

        // **Mirar el cable antes de empujar** (QYR-0400), y sólo el emisor.
        //
        // Un emisor escribe hasta llenar el buffer del par y entonces **se
        // bloquea dentro de `write`**, donde no puede enterarse de nada. Si lo
        // que el par hizo fue cancelar, ese bloqueo dura hasta que el par
        // cierra — y en Windows un cierre con datos sin leer manda un RST, que
        // **descarta lo que hubiera en el buffer de recepción de este lado**.
        // La cancelación estaba ahí, entregada y a tiempo, y se pierde: el
        // emisor despierta con un reset y dice «el otro aparato no responde»
        // de alguien que se lo había dicho.
        //
        // En Linux no se veía porque 4 MiB caben en los buffers: la escritura
        // termina, el paso siguiente lee la cancelación, y el nombre sale bien.
        // **Una suite verde en Linux no prueba nada sobre esto.**
        //
        // Y no es sólo el nombre. Sin esto, un cancelar contra una transferencia
        // grande no se atiende hasta que el archivo entero está en el cable, que
        // es lo contrario de lo que pide ADR-0050 §4.2.
        //
        // Sólo el emisor: es el único que empuja volumen. Y va antes de
        // `write_outbound` porque después ya sería tarde — el bloqueo ocurre
        // ahí. Cuando esto entrega un frame que **no** termina la sesión, el
        // paso continúa normal: se escribe lo pendiente y no se lee dos veces.
        let mut delivered_early = false;
        if matches!(self.role, Role::Sending { .. }) {
            match self.stream.poll_frame() {
                Ok(Some(frame)) => {
                    let bytes = frame
                        .try_encode()
                        .map_err(|_| SessionError::TransferRefused)?;
                    if let Role::Sending { engine, .. } = &mut self.role {
                        engine
                            .deliver(&bytes)
                            .map_err(|_| SessionError::TransferRefused)?;
                    }
                    delivered_early = true;
                }
                Ok(None) => {}
                Err(error) => {
                    if self.finished() {
                        return Ok(self.verdict());
                    }
                    return Err(net_error(&error));
                }
            }
        }

        self.write_outbound()?;

        // **El único silencio que este protocolo produce a propósito es el de
        // una persona pensando** (QYR-0393).
        //
        // No hay latido: `MessageType::Heartbeat` existe en el formato y **nadie
        // lo emite**, así que los sesenta segundos de ADR-0028 §4.2 corren
        // enteros contra el tiempo que tarda alguien en leer una pantalla.
        // Medido: con 65 s de espera el emisor moría a los **60,11 s** con «el
        // otro aparato no responde», que es una acusación falsa contra una red
        // que funciona.
        //
        // Son **dos** reglas, porque los dos lados se quedan callados por
        // razones distintas.
        //
        // La primera: **este lado espera de verdad, y lo que espera no es un
        // aparato**. Un emisor en `Negotiating` espera una aceptación y uno en
        // `AwaitingIntegrity` ya lo ha puesto todo en el cable y espera un
        // veredicto — que es un SHA-256 sobre todo lo recibido, y sobre cuatro
        // gigas eso pasa de sesenta segundos **sin que nada vaya mal**. En esas
        // dos fases el reloj es el de una decisión. Mientras el contenido se
        // mueve vuelve a ser sesenta, porque ahí el silencio sí significa
        // muerto. Esto **no** es subir `IDLE_TIMEOUT`.
        self.stream.set_idle_timeout(self.silence_budget());

        // La segunda: **el reloj mide el silencio del otro, no la espera de
        // éste.** Cuando el consumidor deja de dar pasos —el receptor sale de
        // `await_offer` y pregunta a una persona— nadie estaba escuchando, así
        // que ese silencio no es prueba de nada y contarlo es culpar al par de
        // una pausa propia. Se reinicia la ventana; no se alarga.
        if self.last_step_at.elapsed() > SELF_ABSENCE {
            self.stream.mark_listening();
        }
        self.last_step_at = Instant::now();

        // Ya se entregó lo que había arriba: leer otra vez aquí sería esperar
        // `READ_TIMEOUT` por noticias que ya se tienen. Y **no cuenta como una
        // lectura vencida**: no venció nada, llegó antes. Sumarla ahí falsearía
        // la única cifra con la que QYR-0365 mide si este bucle espera de más.
        let inbound = if delivered_early {
            None
        } else {
            match self.stream.read_frame() {
                Ok(Some(frame)) => Some(
                    frame
                        .try_encode()
                        .map_err(|_| SessionError::TransferRefused)?,
                ),
                Ok(None) => {
                    // Venció el reloj de lectura sin un frame. **Es el suceso
                    // que QYR-0365 mide.**
                    self.expired_reads = self.expired_reads.saturating_add(1);
                    None
                }
                Err(error) => {
                    if self.finished() {
                        return Ok(self.verdict());
                    }
                    return Err(net_error(&error));
                }
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
                self.nothing_left_to_send = self.outbound.is_empty();
                self.progress.done = engine.bytes_sent();
                self.progress.item = engine.item_in_flight();
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
                // Lo que faltaba para que la barra del receptor no fuera un cero
                // fijo. Va fuera del `if let Some(bytes)` a proposito: un paso
                // sin nada que entregar sigue siendo un paso, y su emision debe
                // decir la verdad de ese momento.
                self.progress.done = engine.bytes_received();
                self.progress.item = engine.item_in_flight();
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
    /// says why, and it is the difference between «could not send it» and «they
    /// said no» on a screen.
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
    /// needs: without it the only «no» a person could express was a cancel,
    /// which says something else.
    ///
    /// Everything already written is released — `FileSink::abandon` — so a
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
    /// to verify on disk — two checks over the same bytes disagreeing means one
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
        // **QYR-0383: esto tenía dos `return` dentro del bucle.**
        //
        // Un solo ítem que fallara abandonaba **todos los que venían detrás en
        // el manifiesto**, con sus `.qyro-part` ya escritos enteros y
        // verificados, sin renombrar y sin que nadie los volviera a mirar.
        // Medido: tres archivos, el primero vacío, y llegan cero — los dos
        // llenos se quedan como `b-lleno.bin.qyro-part` y `c-lleno.bin.qyro-part`.
        //
        // Se recorre entero. Lo que se pudo materializar se materializa, y el
        // fallo se cuenta y se devuelve **al final**: la negativa sigue siendo
        // una negativa, y deja de llevarse por delante archivos que cruzaron
        // perfectamente.
        let mut refused = 0_u32;
        for (item_id, verdict) in &verdicts {
            match target.finish_item(*item_id) {
                Ok(_) if *verdict == ItemVerdict::Ok => {
                    materialised = materialised.saturating_add(1);
                }
                // Se pudo escribir y el veredicto del motor dice que no. El
                // archivo no vale, y el que viene detrás puede que sí.
                Ok(_) => refused = refused.saturating_add(1),
                // El veredicto dice que sí y el disco dice que no: nombre
                // tomado, digest que no cuadra, sitio que se acabó.
                Err(_) if *verdict == ItemVerdict::Ok => {
                    refused = refused.saturating_add(1);
                }
                // Los dos dicen que no. Nada que contar y nada que salvar.
                Err(_) => {}
            }
        }
        if refused > 0 {
            // **Sin número, y es una limitación conocida.** Esta firma devuelve
            // un contador o un error, no las dos cosas, y cambiarla es cambiar
            // `qyro_session_finish` en la frontera C. Lo que importa —que lo
            // salvable quede salvado— ya está hecho arriba; el llamante dice
            // «alguno no se pudo guardar» y la persona ve en la carpeta cuáles.
            return Err(SessionError::StorageRefused);
        }
        Ok(materialised)
    }
}

/// The emission budget, pinned rather than bounded.
///
/// `tests/session_behaviour.rs` proves the budget holds over a real socket, and
/// that is the test worth having. But a ceiling and a strict inequality between
/// two sizes are satisfied by **any** formula that stays under the ceiling and
/// grows with the file, so seven mutants inside `Emitter` survived it — `/`
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

    /// The size at which the fraction first ties the floor: 256 KiB × 100.
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
        // forced terminal emission exists to prevent (ADR-0033 §4).
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
        // The property ADR-0033 §1 exists for, checked on sizes no socket test
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
