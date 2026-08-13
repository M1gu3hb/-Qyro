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
            Self::PeerUnreachable => "the peer could not be reached, or the wire ended",
            Self::NotAuthenticated => "the peer did not authenticate",
            Self::TransferRefused => "the transfer was refused",
            Self::StorageRefused => "the destination refused the content",
            Self::Cancelled => "the session was cancelled",
        })
    }
}

impl core::error::Error for SessionError {}
