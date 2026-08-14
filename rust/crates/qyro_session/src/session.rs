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
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use qyro_fs::{FileSink, FileSource, PlannedFile, manifest_from_disk};
use qyro_net::{FrameStream, Listener, NetError, dial, initiate, respond};
use qyro_transfer::{ContentSink, ItemVerdict, Phase, Receiver, Sender};

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
    /// Which manifest item is moving, one-based. Zero before the first.
    pub item: u32,
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
    role: Role,
    cancel: Arc<AtomicBool>,
    failed: Option<SessionError>,
    progress: Progress,
    outbound: Vec<Vec<u8>>,
}

impl core::fmt::Debug for Session {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Never the engine and never the stream: a Debug that prints a sealer
        // is a key in a log.
        f.debug_struct("Session")
            .field("progress", &self.progress)
            .field("failed", &self.failed)
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
    pub fn open_sender(
        address: SocketAddr,
        root: &Path,
        files: &[PathBuf],
    ) -> Result<Self, SessionError> {
        if files.is_empty() {
            return Err(SessionError::BadArgument);
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

        let identity = new_identity()?;
        let stream = dial(address).map_err(|error| net_error(&error))?;
        let established = initiate(stream, &identity).map_err(|error| net_error(&error))?;
        let (stream, sealer, opener) = established.into_parts();

        let mut engine = Box::new(Sender::new(sealer, opener, manifest));
        let opening = engine.open().map_err(|_| SessionError::TransferRefused)?;

        Ok(Self {
            stream,
            role: Role::Sending {
                engine,
                source: FileSource::new(paths),
            },
            cancel: Arc::new(AtomicBool::new(false)),
            failed: None,
            progress: Progress {
                done: 0,
                total,
                item: 0,
            },
            outbound: opening,
        })
    }

    /// Opens a receiving session on `port`, writing under `destination`.
    ///
    /// # Errors
    ///
    /// [`SessionError::BadArgument`] when the port cannot be bound, and the
    /// same authentication and reachability errors as [`Self::open_sender`].
    pub fn open_receiver(bind: SocketAddr, destination: &Path) -> Result<Self, SessionError> {
        let listener = Listener::bind(bind).map_err(|_| SessionError::BadArgument)?;
        let identity = new_identity()?;
        let accepted = listener.accept().map_err(|error| net_error(&error))?;
        let established = respond(accepted, &identity).map_err(|error| net_error(&error))?;
        let (stream, sealer, opener) = established.into_parts();

        Ok(Self {
            stream,
            role: Role::Receiving {
                engine: Box::new(Receiver::new(sealer, opener)),
                sink: None,
                destination: destination.to_path_buf(),
            },
            cancel: Arc::new(AtomicBool::new(false)),
            failed: None,
            progress: Progress::default(),
            outbound: Vec::new(),
        })
    }

    /// The address the session is bound to, once open.
    ///
    /// Needed because a receiver may be opened on port 0 and has to report
    /// which port the system chose.
    ///
    /// # Errors
    ///
    /// [`SessionError::PeerUnreachable`] if the socket cannot answer.
    pub fn local_addr(&self) -> Result<SocketAddr, SessionError> {
        self.stream
            .peer_addr()
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
    /// [`SessionError::AlreadyFailed`] once anything has failed — the failure is
    /// sticky, per ADR-0032 §5 — and otherwise whatever refused.
    pub fn step(&mut self) -> Result<SessionState, SessionError> {
        if let Some(failure) = self.failed {
            return Err(failure);
        }
        if self.cancel.load(Ordering::Acquire) {
            return Err(self.fail(SessionError::Cancelled));
        }
        match self.advance() {
            Ok(state) => Ok(state),
            Err(error) => Err(self.fail(error)),
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
        self.write_outbound()?;

        let inbound = match self.stream.read_frame() {
            Ok(Some(frame)) => Some(
                frame
                    .try_encode()
                    .map_err(|_| SessionError::TransferRefused)?,
            ),
            Ok(None) => None,
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
            Role::Sending { engine, .. } => engine.phase() == Phase::Done,
            Role::Receiving { engine, .. } => engine.phase() == Phase::Done,
        }
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

/// A fresh device identity for one session.
///
/// Deliberately not exposed and deliberately not a parameter: an identity is a
/// `qyro_crypto` type, and letting one cross this crate's public surface is
/// exactly what ADR-0032 §2 bounds. Persistent identity arrives in phase 06
/// through the platform stores, still on this side of the boundary.
fn new_identity() -> Result<qyro_crypto::DeviceIdentity, SessionError> {
    qyro_crypto::DeviceIdentity::generate().map_err(|_| SessionError::NotAuthenticated)
}
