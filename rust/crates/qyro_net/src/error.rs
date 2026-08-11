//! One variant per way a connection can stop, and none of them is called `Io`.
//!
//! ADR-0028 §5 is the table; this enum is that table in code. The rule it
//! encodes is worth stating once here, because every variant below is placed by
//! it:
//!
//! > **Poison what lied, not what stopped.**
//!
//! A frame whose tag does not verify, or whose framing is structurally invalid,
//! means the bytes are not what they claim to be, and nothing after them is
//! interpretable. A close, a reset or a silence claims nothing false: it says
//! the wire ended. Conflating the two turns "the Wi-Fi dropped" into "you are
//! being attacked", and buries the signal that matters.

// Same denials as every other crate that parses peer bytes. A panic here is a
// remote denial of service.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use core::fmt;
use core::time::Duration;
use std::io;

use qyro_crypto::HandshakeError;
use qyro_crypto::aead::AeadError;
use qyro_protocol::{FrameError, MessageType};

/// Which socket operation failed, for the cases nothing else explains.
///
/// This exists so that the catch-all variant still says *what was being done*.
/// An error called `Io(..)` tells a reader that something to do with a socket
/// went wrong, which they already knew.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum SocketOp {
    /// Binding a listener to an address.
    Bind,
    /// Accepting an inbound connection.
    Accept,
    /// Dialling a peer.
    Connect,
    /// Reading from an established socket.
    Read,
    /// Writing to an established socket.
    Write,
    /// Setting a socket option: `nodelay`, a timeout.
    Configure,
    /// Asking the socket for one of its own addresses.
    Address,
    /// Shutting a socket down to unblock a parked thread.
    Shutdown,
}

impl fmt::Display for SocketOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match self {
            Self::Bind => "bind",
            Self::Accept => "accept",
            Self::Connect => "connect",
            Self::Read => "read",
            Self::Write => "write",
            Self::Configure => "configure",
            Self::Address => "address",
            Self::Shutdown => "shutdown",
        };
        f.write_str(word)
    }
}

/// Why a connection stopped carrying frames.
///
/// `Copy`, like [`FrameError`], so that reporting one never has to decide who
/// owns it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum NetError {
    // ----------------------------------------------- the wire simply ended
    /// The peer closed the connection cleanly, on a frame boundary, before the
    /// transfer had a reason to finish.
    ///
    /// Nothing lied. Does not poison.
    PeerClosedEarly,

    /// The peer closed the connection cleanly, but partway through a frame.
    ///
    /// Distinct from [`Self::PeerClosedEarly`] because the bytes already
    /// buffered can never become a frame: ADR-0018 forbids resynchronising,
    /// since resynchronising is guessing. `buffered` is the measured number of
    /// bytes stranded in the decoder, not a constant.
    PeerClosedMidFrame { buffered: usize },

    /// The peer's socket stopped existing without an orderly close.
    ///
    /// A reset. Typical of a process that was killed with data still queued,
    /// but a firewall, an expired NAT entry or a cable produce it too — so the
    /// name says what was **observed**, not a cause that was inferred. ADR-0028
    /// §5.1 is explicit that TCP cannot tell these apart.
    PeerVanished { kind: io::ErrorKind },

    /// No byte arrived for [`IDLE_TIMEOUT`](crate::IDLE_TIMEOUT).
    ///
    /// Neither a close nor a reset: silence. Typical of a suspended machine or
    /// a dropped link, where nobody is left to send anything at all.
    PeerSilent { idle: Duration },

    // ------------------------------------ refusals before the peer is known
    /// An unauthenticated peer tried to push past its byte allowance.
    ///
    /// ADR-0028 §3.1. `attempted` is what the connection had actually taken in
    /// when the allowance ran out.
    PreAuthByteLimitExceeded { attempted: usize, limit: usize },

    /// The handshake did not finish inside its deadline.
    ///
    /// ADR-0028 §3.2. Whole-handshake, so a peer that dribbles cannot restart
    /// it.
    HandshakeDeadlineExceeded { limit: Duration },

    /// The listener already holds as many unauthenticated connections as it
    /// will.
    ///
    /// ADR-0028 §3.3. The listener accepts and closes rather than stopping
    /// accepting, so this is what the **dialling** end observes.
    TooManyPendingConnections { limit: usize },

    /// The far end did not answer the dial inside
    /// [`CONNECT_TIMEOUT`](crate::CONNECT_TIMEOUT).
    ConnectTimedOut { limit: Duration },

    /// The handshake refused: a signature that did not verify, a MAC that did
    /// not match, a message of the wrong length.
    ///
    /// Does **not** report `poisons()`, and the reason is not that it is
    /// harmless — it is that there is no session to poison. A handshake that
    /// fails leaves no session at all: `initiate` and `respond` consume the
    /// stream, so a peer that fails to prove who it is cannot be continued
    /// with. That is stronger than poisoning, not weaker.
    Handshake(HandshakeError),

    /// A frame arrived where a handshake message was expected.
    ///
    /// `None` means the frame was encrypted or of an unimplemented type, so it
    /// had no message type this version can name.
    UnexpectedHandshakeMessage { got: Option<MessageType> },

    /// This end could not seal a frame it was asked to send.
    ///
    /// Ours, not the peer's, which is why it is separate from
    /// [`Self::NotAuthenticated`].
    Sealing(AeadError),

    // ------------------------------------------------- the bytes lied: poison
    /// A sealed frame did not authenticate.
    ///
    /// Carries no detail on purpose, exactly as `TransferError::NotAuthenticated`
    /// does: a frame whose tag does not verify has **no known sender**, so
    /// nothing in it can be reported as fact — not its type, not its length, not
    /// which session it claimed to belong to.
    NotAuthenticated,

    /// Framing itself was refused.
    ///
    /// One of the two variants that poison. The decoder is poisoned by its own
    /// rules (ADR-0018) and `reset()` is never called, because there is nothing
    /// to resume: a stream whose framing is untrustworthy has no next frame.
    Framing(FrameError),

    // --------------------------------------------------------- the catch-all
    /// A socket operation failed for a reason none of the above describes.
    ///
    /// Deliberately **not** called `Io`: it names the operation, so a reader
    /// learns something from the variant alone.
    SocketFailed {
        operation: SocketOp,
        kind: io::ErrorKind,
    },
}

impl NetError {
    /// Whether this ending means the session must be treated as poisoned.
    ///
    /// True for the two ways bytes can lie — framing that is structurally
    /// invalid, and a tag that does not verify. See the module comment for why
    /// a close, a reset and a silence are not on this list. This is a method
    /// rather
    /// than a comment so that the rule can be tested, and so that adding a
    /// variant forces a decision about it instead of inheriting one.
    #[must_use]
    pub const fn poisons(self) -> bool {
        matches!(self, Self::Framing(_) | Self::NotAuthenticated)
    }

    /// Whether this ending means the peer stopped, as opposed to misbehaved.
    ///
    /// The four ways a wire ends without anything having lied.
    #[must_use]
    pub const fn is_peer_gone(self) -> bool {
        matches!(
            self,
            Self::PeerClosedEarly
                | Self::PeerClosedMidFrame { .. }
                | Self::PeerVanished { .. }
                | Self::PeerSilent { .. }
        )
    }
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PeerClosedEarly => {
                f.write_str("the peer closed the connection before the transfer finished")
            }
            Self::PeerClosedMidFrame { buffered } => write!(
                f,
                "the peer closed the connection with {buffered} bytes of an unfinished frame"
            ),
            Self::PeerVanished { kind } => {
                write!(f, "the peer's socket stopped existing ({kind:?})")
            }
            Self::PeerSilent { idle } => {
                write!(f, "no byte arrived from the peer in {idle:?}")
            }
            Self::PreAuthByteLimitExceeded { attempted, limit } => write!(
                f,
                "an unauthenticated peer sent {attempted} bytes against a limit of {limit}"
            ),
            Self::HandshakeDeadlineExceeded { limit } => {
                write!(f, "the handshake did not finish in {limit:?}")
            }
            Self::TooManyPendingConnections { limit } => {
                write!(f, "already holding {limit} unauthenticated connections")
            }
            Self::ConnectTimedOut { limit } => {
                write!(f, "the peer did not answer in {limit:?}")
            }
            Self::Handshake(error) => write!(f, "the handshake refused: {error}"),
            Self::UnexpectedHandshakeMessage { got } => match got {
                Some(message) => write!(f, "{message:?} arrived during the handshake"),
                None => f.write_str("an unreadable frame arrived during the handshake"),
            },
            Self::Sealing(error) => write!(f, "could not seal a frame: {error}"),
            Self::NotAuthenticated => f.write_str("the frame did not authenticate"),
            Self::Framing(error) => write!(f, "framing refused: {error}"),
            Self::SocketFailed { operation, kind } => {
                write!(f, "socket {operation} failed ({kind:?})")
            }
        }
    }
}

impl core::error::Error for NetError {}
