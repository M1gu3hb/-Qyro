//! One variant per way a session refuses, and none of them carries a key.
//!
//! Deliberately **not** a wrapper around the errors underneath. `NetError`,
//! `TransferError` and `FsError` are rich, and re-exporting them would put three
//! more crates' vocabularies into the surface `qyro_ffi` can name — which is the
//! surface ADR-0032 §2 bounds. They are collapsed into a small set that says
//! what the caller can act on, and the detail stays on the Rust side of the
//! boundary where a log can still print it.

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

/// Why a session refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SessionError {
    /// The address, port or path the caller supplied was not usable.
    BadArgument,
    /// More files than one transfer may carry (ADR-0047 §3).
    ///
    /// **Separate from `BadArgument` on purpose.** «Too many» is a number the
    /// person can act on — pick fewer, or send in two goes — and collapsing it
    /// into the generic refusal would produce the message this project has
    /// already been bitten by once: an argument error printed as if the network
    /// were at fault (QYR-0361).
    TooManyFiles { given: usize, limit: usize },
    /// The peer could not be reached, or the wire ended.
    ///
    /// Collapses every ending in ADR-0028 §5 that means "the peer stopped".
    /// The distinction matters inside `qyro_net` and does not change what a
    /// caller does about it.
    PeerUnreachable,
    /// The peer failed to prove who it is, or a frame did not authenticate.
    ///
    /// The one error a caller must never retry blindly.
    NotAuthenticated,
    /// The transfer itself refused: an unexpected message, a digest that did
    /// not match, a manifest the receiver would not accept.
    TransferRefused,
    /// The filesystem refused: a path that escapes the root, a symlink, a
    /// collision.
    StorageRefused,
    /// The session was cancelled by this end.
    Cancelled,
    /// There is no usable device identity for this process.
    ///
    /// ADR-0040. Three situations, deliberately one variant: nobody called
    /// `identity::open`, the stored blob would not open, or the caller asked
    /// for platform protection on a platform with no wrapper installed.
    ///
    /// **None of them generates a replacement.** A store that mints a new
    /// identity when it cannot read the old one is a device that silently
    /// becomes a stranger to every peer that trusted it, and that is the exact
    /// defect this variant exists to make impossible to reach by accident.
    IdentityUnreadable,
    /// The port could not be bound: somebody else holds it, or this machine
    /// will not hand it out.
    ///
    /// **ADR-0041 §3 decided the behaviour and the enum had no word for it.**
    /// The ADR says «si el puerto está ocupado: se dice, no se mueve. Qyro dice
    /// qué puerto está ocupado y ofrece elegir otro» — and `open_receiver`
    /// mapped *every* bind failure to [`Self::BadArgument`], whose message is
    /// «the address, port or path was not usable». Nothing above could tell «that
    /// port is taken» from «that path is wrong», so nothing above could offer
    /// another port. A decision written in an ADR that the code cannot express
    /// is a decision nobody implemented.
    ///
    /// **Two operating-system errors, one fact.** `AddrInUse` is the obvious
    /// one. The other is Windows-specific and is the one that will actually
    /// happen: Windows reserves TCP ranges for Hyper-V, WSL2 and Docker — visible
    /// with `netsh interface ipv4 show excludedportrange protocol=tcp` — and a
    /// bind inside one fails with **`WSAEACCES`, 10013**, which `std` surfaces as
    /// `PermissionDenied`, not as «in use». To the person holding the machine
    /// both mean *this port is not yours today*, and both have the same answer:
    /// choose another. Separating them here would buy a distinction that changes
    /// nothing anybody does.
    ///
    /// Deliberately **not** a silent move to the next free port. ADR-0041 §3
    /// refuses that: a port that moves on its own loses the two properties it
    /// was chosen fixed for — one firewall grant instead of one per session, and
    /// a pairing code somebody can predict — and loses them without saying so.
    PortUnavailable,
}

// There is deliberately no `AlreadyFailed`. ADR-0032 §5 freezes stickiness as
// *returning the same code*, so a session that failed with `PeerUnreachable`
// keeps answering `PeerUnreachable` — which is strictly more useful than a
// second variant meaning "something failed earlier, look it up". The draft of
// this enum had one, and the construction-site guard caught it: nothing could
// ever produce it, and a caller matching on it would have been matching on a
// state that does not exist.

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::BadArgument => "the address, port or path was not usable",
            Self::TooManyFiles { .. } => "more files than one transfer can carry",
            Self::PeerUnreachable => "the peer could not be reached, or the wire ended",
            Self::NotAuthenticated => "the peer did not authenticate",
            Self::TransferRefused => "the transfer was refused",
            Self::StorageRefused => "the destination refused the content",
            Self::Cancelled => "the session was cancelled",
            Self::IdentityUnreadable => "there is no usable identity for this process",
            Self::PortUnavailable => {
                "that port is already taken, or this machine will not give it out"
            }
        })
    }
}

impl core::error::Error for SessionError {}
