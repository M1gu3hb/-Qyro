//! Who listens, who dials, and the budget a stranger may spend.
//!
//! ADR-0028 §7 assigns the roles once for both layers: the end that **receives**
//! listens and is the handshake responder; the end that **sends** dials and is
//! the handshake initiator. There is deliberately no "TCP client but handshake
//! responder" combination for anyone to reason about.
//!
//! The port comes from the caller. Port `0` means "let the operating system
//! choose", and [`Listener::local_addr`] reports what it chose — which is not a
//! convenience but the thing that keeps network tests from being intermittent.
//! A fixed port fails the moment two tests run at once, and the reflex cure for
//! that intermittency is a longer `sleep`, which hides the problem instead of
//! removing it.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use qyro_protocol::{DecodedFrame, Frame, MessageType};

use crate::error::{NetError, SocketOp};
use crate::limits::{CONNECT_TIMEOUT, MAX_PENDING_HANDSHAKES};
use crate::stream::FrameStream;

/// Payload byte of the refusal a full listener sends before closing.
///
/// ADR-0028 §3.3 promises the dialling end learns *why* it was turned away. A
/// bare close cannot carry that: the dialler would observe
/// [`NetError::PeerClosedEarly`] and could not tell a full listener from a
/// crashed one. So the refusal is one plain `Error` frame and then the close.
///
/// It is unauthenticated, necessarily — there is no session yet — so an on-path
/// attacker can forge it and make a dialler give up. That is not a new power:
/// anyone who can inject this can already inject a reset. It carries no secret
/// and grants no access, so being forgeable costs a retry and nothing else.
pub const REFUSAL_TOO_MANY_PENDING: u8 = 1;

/// A reservation against [`MAX_PENDING_HANDSHAKES`], released when dropped.
///
/// Held by an accepted connection for exactly as long as it is *pending*: from
/// accept until either the handshake authenticates it or it dies. Releasing on
/// `Drop` is what makes the budget survive the paths nobody remembers —
/// a refused handshake, a panic, an early return.
#[derive(Debug)]
pub struct PendingSlot {
    counter: Arc<AtomicUsize>,
}

impl Drop for PendingSlot {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Accepts inbound connections under the budget of ADR-0028 §3.3.
#[derive(Debug)]
pub struct Listener {
    inner: TcpListener,
    pending: Arc<AtomicUsize>,
}

impl Listener {
    /// Binds to an address. Port `0` lets the system choose one.
    ///
    /// # Errors
    ///
    /// [`NetError::SocketFailed`] with [`SocketOp::Bind`].
    pub fn bind(addr: SocketAddr) -> Result<Self, NetError> {
        let inner = TcpListener::bind(addr).map_err(|error| NetError::SocketFailed {
            operation: SocketOp::Bind,
            kind: error.kind(),
        })?;
        Ok(Self {
            inner,
            pending: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// The address actually bound, including the port the system chose.
    ///
    /// # Errors
    ///
    /// [`NetError::SocketFailed`] with [`SocketOp::Address`].
    pub fn local_addr(&self) -> Result<SocketAddr, NetError> {
        self.inner
            .local_addr()
            .map_err(|error| NetError::SocketFailed {
                operation: SocketOp::Address,
                kind: error.kind(),
            })
    }

    /// Unauthenticated connections currently held.
    #[must_use]
    pub fn pending(&self) -> usize {
        self.pending.load(Ordering::Acquire)
    }

    /// Waits for a connection this listener has room for.
    ///
    /// Over budget, a connection is **accepted and closed**, and this keeps
    /// waiting. Refusing by not accepting would leave the kernel's backlog
    /// full, and the next peer to suffer for it would be a legitimate one —
    /// exactly backwards.
    ///
    /// # Errors
    ///
    /// [`NetError::SocketFailed`] with [`SocketOp::Accept`].
    pub fn accept(&self) -> Result<FrameStream, NetError> {
        loop {
            let (socket, _) = self
                .inner
                .accept()
                .map_err(|error| NetError::SocketFailed {
                    operation: SocketOp::Accept,
                    kind: error.kind(),
                })?;

            if self.pending.load(Ordering::Acquire) >= MAX_PENDING_HANDSHAKES {
                refuse(socket);
                continue;
            }
            self.pending.fetch_add(1, Ordering::AcqRel);
            let slot = PendingSlot {
                counter: Arc::clone(&self.pending),
            };

            let mut stream = FrameStream::new(socket)?;
            stream.hold_pending_slot(slot);
            return Ok(stream);
        }
    }
}

/// Tells a peer it was turned away, then lets the socket close.
///
/// Best effort throughout: the peer being refused is by definition one this
/// process owes nothing, and a refusal that itself blocks or fails is still a
/// refusal. The socket closes when it drops at the end of this function.
fn refuse(mut socket: TcpStream) {
    let Ok(frame) = Frame::new(MessageType::Error, vec![REFUSAL_TOO_MANY_PENDING]) else {
        return;
    };
    let _ = socket.write_all(&frame.encode());
    let _ = socket.flush();
}

/// Reads a refusal out of a frame that arrived before any session existed.
///
/// Returns `None` for anything that is not a refusal, so a caller can ask
/// without having to know the shape first.
#[must_use]
pub fn refusal_of(frame: &DecodedFrame) -> Option<NetError> {
    let plain = frame.as_plain()?;
    if plain.message_type() != MessageType::Error {
        return None;
    }
    match plain.payload().first() {
        Some(&REFUSAL_TOO_MANY_PENDING) => Some(NetError::TooManyPendingConnections {
            limit: MAX_PENDING_HANDSHAKES,
        }),
        _ => None,
    }
}

/// Dials a peer, giving up after [`CONNECT_TIMEOUT`].
///
/// # Errors
///
/// [`NetError::ConnectTimedOut`] when the far end does not answer in time,
/// [`NetError::PeerVanished`] when it actively refuses, otherwise
/// [`NetError::SocketFailed`] with [`SocketOp::Connect`].
pub fn dial(addr: SocketAddr) -> Result<FrameStream, NetError> {
    match TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) {
        Ok(socket) => FrameStream::new(socket),
        Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
            Err(NetError::ConnectTimedOut {
                limit: CONNECT_TIMEOUT,
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
            Err(NetError::PeerVanished { kind: error.kind() })
        }
        Err(error) => Err(NetError::SocketFailed {
            operation: SocketOp::Connect,
            kind: error.kind(),
        }),
    }
}
