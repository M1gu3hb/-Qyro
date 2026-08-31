//! A `TcpStream` that hands out whole frames.
//!
//! A socket read is not a message: it delivers whatever happens to have
//! arrived, which may be half a header, three frames and a bit, or one byte.
//! Reassembly is **not** solved here — `qyro_protocol::FrameDecoder` already
//! solves it, under a memory ceiling, with poisoning semantics this crate must
//! not re-decide. What is solved here is everything around it: how many bytes a
//! read may take, what a timeout means, and which of the several ways a
//! connection can stop this one was.
//!
//! See `docs/adr/ADR-0028-network-transport.md` §1, §2, §3.1, §4 and §5.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use qyro_protocol::{DecodedFrame, FrameDecoder};

use crate::error::{NetError, SocketOp};
use crate::limits::{IDLE_TIMEOUT, MAX_PREAUTH_BYTES, READ_BUFFER_LEN, READ_TIMEOUT};
use crate::listener::PendingSlot;

/// Whether an error is the read timeout doing its job.
///
/// **The portability trap of this crate, and it is a real one.** A read that
/// expires under `SO_RCVTIMEO` surfaces as `WouldBlock` on Linux and as
/// `TimedOut` on Windows. Handling only one of the two would work perfectly on
/// the platform it was written on and break every transfer on the other, which
/// is exactly the class of bug that only appears on the platform nobody ran.
/// ADR-0028 §8 lists this first among the known Windows risks.
pub(crate) const fn is_read_timeout(kind: io::ErrorKind) -> bool {
    matches!(kind, io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut)
}

/// Whether an error means the peer's socket stopped existing.
pub(crate) const fn is_peer_gone(kind: io::ErrorKind) -> bool {
    matches!(
        kind,
        io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::BrokenPipe
            | io::ErrorKind::NotConnected
    )
}

/// A framed connection: whole frames in, whole frames out.
#[derive(Debug)]
pub struct FrameStream {
    socket: TcpStream,
    decoder: FrameDecoder,
    /// Staging area between `read` and `FrameDecoder::push`.
    ///
    /// Sized to [`MAX_PREAUTH_BYTES`] while the peer is a stranger and grown to
    /// [`READ_BUFFER_LEN`] only once authenticated, so an unauthenticated peer
    /// never causes the larger allocation. ADR-0028 §3.1.
    buffer: Vec<u8>,
    /// Bytes taken from an unauthenticated peer so far.
    preauth_taken: usize,
    authenticated: bool,
    /// When a byte last arrived. The clock that decides slow against dead.
    last_byte_at: Instant,
    /// How long silence may last before the peer is declared dead.
    ///
    /// Defaults to [`IDLE_TIMEOUT`], which is the number ADR-0028 §4.2 froze. It
    /// is settable because a deadline nothing can reach is a deadline nothing
    /// tests: sixty seconds is right in production and useless in a test suite,
    /// and the alternative — trusting that the untested branch works — is how a
    /// project ends up with an ending nobody has ever seen produced.
    idle_timeout: Duration,
    /// The listener's reservation against `MAX_PENDING_HANDSHAKES`.
    ///
    /// `None` on a dialled connection, which no listener is holding a budget
    /// for. Released when the handshake authenticates the peer, or when this
    /// stream drops — whichever happens first, which is the point.
    pending_slot: Option<PendingSlot>,
    /// Largest `FrameDecoder::buffer_capacity()` ever observed.
    ///
    /// `cfg(test)`, never a feature. It exists because "an unauthenticated peer
    /// cannot make us reserve what it declares" is a security property, and a
    /// property nothing measures is a comment.
    ///
    /// It records what the decoder **actually** reserved, read back out of the
    /// decoder after each push. It is deliberately not set from a limit
    /// constant: a counter that stores the number you were hoping for and is
    /// then compared against that same number proves nothing, and this project
    /// has shipped that bug.
    #[cfg(test)]
    peak_decoder_capacity: usize,
    /// Total bytes handed back by `read`, summed.
    #[cfg(test)]
    bytes_read: usize,
    /// Reads that returned at least one byte.
    ///
    /// Counts the operation, not an expectation: it is incremented where `read`
    /// returned `Ok(count)` with `count > 0`, so it can only say what actually
    /// happened. It is what lets a test claim "this frame arrived across three
    /// reads" instead of assuming it because it wrote three times.
    #[cfg(test)]
    read_calls: usize,
}

impl FrameStream {
    /// Wraps a connected socket.
    ///
    /// # Errors
    ///
    /// [`NetError::SocketFailed`] with [`SocketOp::Configure`] if the socket
    /// refuses `nodelay` or the read timeout. Both are refused rather than
    /// ignored: `nodelay` is mandatory with go-back-N, because Nagle plus TCP's
    /// delayed acknowledgement produces pauses of hundreds of milliseconds that
    /// look like an engine fault and are not; and without the read timeout no
    /// parked reader could ever notice a cancellation.
    pub fn new(socket: TcpStream) -> Result<Self, NetError> {
        socket
            .set_nodelay(true)
            .map_err(|error| NetError::SocketFailed {
                operation: SocketOp::Configure,
                kind: error.kind(),
            })?;
        socket
            .set_read_timeout(Some(READ_TIMEOUT))
            .map_err(|error| NetError::SocketFailed {
                operation: SocketOp::Configure,
                kind: error.kind(),
            })?;
        // No write timeout, deliberately. ADR-0028 §4.3: a write that expires
        // mid-frame leaves a frame half written, and there is no
        // resynchronising from that. A stuck writer is freed by `shutdown`,
        // which the reader calls once it declares the peer dead.
        socket
            .set_write_timeout(None)
            .map_err(|error| NetError::SocketFailed {
                operation: SocketOp::Configure,
                kind: error.kind(),
            })?;

        Ok(Self {
            socket,
            decoder: FrameDecoder::new(),
            buffer: vec![0_u8; MAX_PREAUTH_BYTES],
            preauth_taken: 0,
            authenticated: false,
            last_byte_at: Instant::now(),
            idle_timeout: IDLE_TIMEOUT,
            pending_slot: None,
            #[cfg(test)]
            peak_decoder_capacity: 0,
            #[cfg(test)]
            bytes_read: 0,
            #[cfg(test)]
            read_calls: 0,
        })
    }

    /// Replaces the silence deadline of [`IDLE_TIMEOUT`].
    ///
    /// Shortening it makes [`NetError::PeerSilent`] reachable in a test. It does
    /// **not** shorten anything else: the read wakeup stays at
    /// [`READ_TIMEOUT`](crate::READ_TIMEOUT), so a heartbeat is still a
    /// heartbeat, and a value below that simply means the first wakeup is
    /// already late.
    pub const fn set_idle_timeout(&mut self, idle: Duration) {
        self.idle_timeout = idle;
    }

    /// Vuelve a empezar la ventana de silencio, sin declarar nada vivo.
    ///
    /// **QYR-0393.** El reloj de silencio mide **cuánto lleva callado el otro**,
    /// y sólo significa «muerto» si este lado estuvo escuchando todo ese rato.
    /// Cuando el consumidor deja de dar pasos —el receptor pregunta a una
    /// persona y se queda esperando su respuesta— nadie estaba escuchando, así
    /// que ese silencio no es prueba de nada.
    ///
    /// Esto **no** alarga el plazo ni afirma que el par esté vivo: lo reinicia.
    /// Un par de verdad muerto se descubre igual, sesenta segundos después de
    /// que este lado vuelva a escuchar de verdad.
    pub fn mark_listening(&mut self) {
        self.last_byte_at = Instant::now();
    }

    /// The silence deadline in force.
    #[must_use]
    pub const fn idle_timeout(&self) -> Duration {
        self.idle_timeout
    }

    /// Lifts the pre-authentication allowance.
    ///
    /// Called once the handshake has established a session, and never before:
    /// everything the allowance protects is protection against a peer whose
    /// identity is not yet known.
    ///
    /// The decoder is **not** replaced. Bytes of the first sealed frame can
    /// arrive in the same read as the last handshake message, and a fresh
    /// decoder would drop them.
    pub fn mark_authenticated(&mut self) {
        self.authenticated = true;
        self.buffer.resize(READ_BUFFER_LEN, 0);
        // The connection stops being *pending* the instant it is authenticated,
        // so the listener's budget is for strangers only. Dropping the slot
        // here is what stops a busy, healthy receiver from refusing new peers.
        self.pending_slot = None;
    }

    /// Attaches the listener's reservation to this connection.
    pub(crate) fn hold_pending_slot(&mut self, slot: PendingSlot) {
        self.pending_slot = Some(slot);
    }

    /// Whether the peer has authenticated.
    #[must_use]
    pub const fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// The address of the far end.
    ///
    /// # Errors
    ///
    /// [`NetError::SocketFailed`] with [`SocketOp::Address`].
    pub fn peer_addr(&self) -> Result<SocketAddr, NetError> {
        self.socket
            .peer_addr()
            .map_err(|error| NetError::SocketFailed {
                operation: SocketOp::Address,
                kind: error.kind(),
            })
    }

    /// The address of **this** end.
    ///
    /// The twin of [`Self::peer_addr`], and the one a receiver needs: an
    /// accepted socket's local address carries the port the listener actually
    /// bound, so a session opened on port 0 can say which port the system chose
    /// without holding on to the `Listener` (QYR-0314).
    ///
    /// # Errors
    ///
    /// [`NetError::SocketFailed`] with [`SocketOp::Address`].
    pub fn local_addr(&self) -> Result<SocketAddr, NetError> {
        self.socket
            .local_addr()
            .map_err(|error| NetError::SocketFailed {
                operation: SocketOp::Address,
                kind: error.kind(),
            })
    }

    /// Writes one whole frame.
    ///
    /// Takes the already-encoded bytes rather than a `Frame`, because a sealed
    /// frame is not a `Frame`: only `qyro_crypto` can produce one, and this
    /// crate must not be able to fabricate anything that claims to be sealed.
    ///
    /// # Errors
    ///
    /// [`NetError::PeerVanished`] if the peer's socket has gone, otherwise
    /// [`NetError::SocketFailed`] with [`SocketOp::Write`].
    pub fn write_frame(&mut self, encoded: &[u8]) -> Result<(), NetError> {
        match self.socket.write_all(encoded) {
            Ok(()) => Ok(()),
            Err(error) if is_peer_gone(error.kind()) => {
                Err(NetError::PeerVanished { kind: error.kind() })
            }
            Err(error) => Err(NetError::SocketFailed {
                operation: SocketOp::Write,
                kind: error.kind(),
            }),
        }
    }

    /// Flushes anything the socket is holding.
    ///
    /// # Errors
    ///
    /// [`NetError::SocketFailed`] with [`SocketOp::Write`].
    pub fn flush(&mut self) -> Result<(), NetError> {
        self.socket.flush().map_err(|error| NetError::SocketFailed {
            operation: SocketOp::Write,
            kind: error.kind(),
        })
    }

    /// Attempts to produce the next frame, returning `Ok(None)` on a heartbeat.
    ///
    /// `Ok(None)` means the read timeout expired with nothing to show and the
    /// peer is not yet late. **It is not an ending and not an error**: it is
    /// the caller's cue to check whether it has been asked to cancel, and then
    /// call again. This mirrors `FrameDecoder::next_frame`, where `Ok(None)`
    /// likewise means "more bytes needed", not "end of stream".
    ///
    /// # Errors
    ///
    /// Every ending in ADR-0028 §5 that a read can observe:
    /// [`NetError::PeerClosedEarly`], [`NetError::PeerClosedMidFrame`],
    /// [`NetError::PeerVanished`], [`NetError::PeerSilent`],
    /// [`NetError::PreAuthByteLimitExceeded`] and [`NetError::Framing`].
    pub fn read_frame(&mut self) -> Result<Option<DecodedFrame>, NetError> {
        loop {
            // A read may have delivered several frames at once, so the decoder
            // is drained before the socket is touched again. Skipping this is
            // how a peer that batches frames gets one of them stranded until
            // the next unrelated read.
            match self.decoder.next_frame() {
                Ok(Some(frame)) => return Ok(Some(frame)),
                Ok(None) => {}
                Err(error) => return Err(NetError::Framing(error)),
            }

            let window = self.read_window();
            if window == 0 {
                // Only reachable unauthenticated: an established connection's
                // window is the whole buffer, which is never empty.
                return Err(NetError::PreAuthByteLimitExceeded {
                    attempted: self.preauth_taken,
                    limit: MAX_PREAUTH_BYTES,
                });
            }
            let Some(slice) = self.buffer.get_mut(..window) else {
                return Err(NetError::SocketFailed {
                    operation: SocketOp::Read,
                    kind: io::ErrorKind::InvalidInput,
                });
            };

            match self.socket.read(slice) {
                Ok(0) => return Err(self.orderly_close()),
                Ok(count) => {
                    self.last_byte_at = Instant::now();
                    if !self.authenticated {
                        self.preauth_taken = self.preauth_taken.saturating_add(count);
                    }
                    #[cfg(test)]
                    {
                        self.bytes_read = self.bytes_read.saturating_add(count);
                        self.read_calls = self.read_calls.saturating_add(1);
                    }
                    let Some(fresh) = self.buffer.get(..count) else {
                        return Err(NetError::SocketFailed {
                            operation: SocketOp::Read,
                            kind: io::ErrorKind::InvalidData,
                        });
                    };
                    self.decoder.push(fresh).map_err(NetError::Framing)?;
                    #[cfg(test)]
                    {
                        // Read back out of the decoder: what it actually
                        // reserved, not what we hoped it would.
                        let reserved = self.decoder.buffer_capacity();
                        if reserved > self.peak_decoder_capacity {
                            self.peak_decoder_capacity = reserved;
                        }
                    }
                }
                // Interrupted is not a failure and not a heartbeat; the read
                // simply did not happen.
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) if is_read_timeout(error.kind()) => {
                    let idle = self.last_byte_at.elapsed();
                    if idle >= self.idle_timeout {
                        return Err(NetError::PeerSilent { idle });
                    }
                    return Ok(None);
                }
                Err(error) if is_peer_gone(error.kind()) => {
                    return Err(NetError::PeerVanished { kind: error.kind() });
                }
                Err(error) => {
                    return Err(NetError::SocketFailed {
                        operation: SocketOp::Read,
                        kind: error.kind(),
                    });
                }
            }
        }
    }

    /// Produces the next frame, waiting through heartbeats.
    ///
    /// For callers with nothing to check between wakeups — tests, and the
    /// handshake, which has its own deadline. A caller that must react to
    /// cancellation wants [`Self::read_frame`] instead, because this one only
    /// returns when there is a frame or the connection has ended.
    ///
    /// # Errors
    ///
    /// The same set as [`Self::read_frame`]. It cannot block for ever: with no
    /// bytes at all, [`IDLE_TIMEOUT`] turns into [`NetError::PeerSilent`].
    pub fn next_frame(&mut self) -> Result<DecodedFrame, NetError> {
        loop {
            if let Some(frame) = self.read_frame()? {
                return Ok(frame);
            }
        }
    }

    /// Unblocks both directions, freeing any thread parked in this socket.
    ///
    /// The forceful half of cancellation. A flag cannot wake a thread inside a
    /// blocking syscall; this can. ADR-0028 §6.1.
    ///
    /// # Errors
    ///
    /// [`NetError::SocketFailed`] with [`SocketOp::Shutdown`]. Shutting down a
    /// socket the peer has already dropped is *not* reported as an error:
    /// wanting it closed and finding it closed is success.
    pub fn shutdown(&self) -> Result<(), NetError> {
        match self.socket.shutdown(Shutdown::Both) {
            Ok(()) => Ok(()),
            Err(error) if is_peer_gone(error.kind()) => Ok(()),
            Err(error) => Err(NetError::SocketFailed {
                operation: SocketOp::Shutdown,
                kind: error.kind(),
            }),
        }
    }

    /// Clones the handle so another thread can write, or shut it down.
    ///
    /// # Errors
    ///
    /// [`NetError::SocketFailed`] with [`SocketOp::Configure`].
    pub fn try_clone_socket(&self) -> Result<TcpStream, NetError> {
        self.socket
            .try_clone()
            .map_err(|error| NetError::SocketFailed {
                operation: SocketOp::Configure,
                kind: error.kind(),
            })
    }

    /// How many bytes the next read may take.
    ///
    /// Unauthenticated, this is the remaining allowance — which is what makes
    /// [`MAX_PREAUTH_BYTES`] a bound on what the process *can* receive rather
    /// than a check applied to bytes it has already taken.
    fn read_window(&self) -> usize {
        if self.authenticated {
            self.buffer.len()
        } else {
            MAX_PREAUTH_BYTES
                .saturating_sub(self.preauth_taken)
                .min(self.buffer.len())
        }
    }

    /// Which orderly-close ending a `read` of zero means.
    ///
    /// Anything still buffered here is a partial frame: a complete one would
    /// have left through `next_frame` at the top of the loop.
    fn orderly_close(&self) -> NetError {
        let buffered = self.decoder.buffered_len();
        if buffered == 0 {
            NetError::PeerClosedEarly
        } else {
            NetError::PeerClosedMidFrame { buffered }
        }
    }

    /// Largest capacity the decoder ever reserved, measured.
    #[cfg(test)]
    pub(crate) const fn peak_decoder_capacity(&self) -> usize {
        self.peak_decoder_capacity
    }

    /// Total bytes this connection accepted from the socket, measured.
    #[cfg(test)]
    pub(crate) const fn bytes_read(&self) -> usize {
        self.bytes_read
    }

    /// Reads that returned at least one byte, measured.
    #[cfg(test)]
    pub(crate) const fn read_calls(&self) -> usize {
        self.read_calls
    }

    /// Size of the staging buffer, which the allowance governs before
    /// authentication.
    #[cfg(test)]
    pub(crate) fn read_buffer_len(&self) -> usize {
        self.buffer.len()
    }
}
