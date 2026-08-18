//! Why a frame could not be read.
//!
//! In its own module, as every other crate in this workspace keeps its errors,
//! and the structural guard is the reason it is worth saying out loud: a variant
//! is only counted as real when something **other than its own declaration**
//! constructs it. Declaring and constructing in one file lets a variant be
//! written, matched on, and never produced — which reads in a `match` arm as if
//! the case were handled.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// Why a frame could not be read.
///
/// Every variant is a **real thing a camera produces**. A decoder that treated
/// these as impossible would panic on a smudged code, which on this channel is
/// the normal case rather than the exception.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireError {
    /// Fewer bytes than a header.
    TooShort,
    /// Not `QF`: some other protocol's QR code was in shot.
    NotAFrame,
    /// A version this build does not know.
    UnknownVersion(u8),
    /// The payload is not the length the header claims a block is.
    BlockSizeMismatch,
    /// A shape that cannot describe any payload.
    ImpossibleShape,
}
