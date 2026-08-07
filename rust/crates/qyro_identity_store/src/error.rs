//! What can go wrong, kept apart on purpose.
//!
//! One variant per step of the read order in
//! `docs/security/identity-storage.md`, plus the write-side refusals.

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

/// A failure to store or retrieve a device identity.
///
/// The distinction this enum exists to hold is between [`Self::IdentityAbsent`]
/// and everything else. "There is no identity" and "there is one and it cannot
/// be read" lead to opposite actions: the first may generate one, the second
/// must never. Collapsing them into a single error is how a device silently
/// replaces an identity a peer had already trusted, which is the worst outcome
/// this crate can produce.
#[derive(Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum StoreError {
    /// Step 1. The store holds nothing. **Not** an I/O failure and not an
    /// ambiguous `None`: a caller is allowed to act on this one.
    IdentityAbsent,
    /// Step 2. Fewer bytes than a header needs.
    Truncated { found: usize },
    /// Step 3. The magic does not match; this is not a Qyro identity blob.
    NotAnIdentityBlob,
    /// Step 4. A version this build does not know, named rather than guessed.
    UnsupportedVersion { found: u8 },
    /// Step 5. A wrap algorithm this build does not know.
    UnsupportedWrap { found: u8 },
    /// Step 6. Reserved bytes were not zero.
    ReservedNotZero,
    /// Step 7. The declared length disagrees with the bytes present.
    LengthMismatch { declared: u32, present: usize },
    /// Step 8. The platform wrapper refused. Carries the platform's own code so
    /// a report can say which failure it was without this crate pretending to
    /// interpret it.
    Unwrap { code: u32 },
    /// Step 9. The wrapper returned something that is not a 32-byte seed.
    MalformedSecret { found: usize },
    /// Write side: the wrapper produced more than `u32::MAX` bytes. Refused
    /// rather than truncated.
    WrappedTooLarge { found: usize },
    /// The store could not be read or written at all.
    ///
    /// Deliberately distinct from [`Self::IdentityAbsent`]: a permissions
    /// failure is not an empty store, and treating it as one would generate a
    /// new identity on a machine that already had a perfectly good one it just
    /// could not open.
    Io { code: i32 },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdentityAbsent => f.write_str("no identity is stored"),
            Self::Truncated { found } => {
                write!(f, "stored blob is truncated: {found} bytes")
            }
            Self::NotAnIdentityBlob => f.write_str("stored bytes are not a Qyro identity blob"),
            Self::UnsupportedVersion { found } => {
                write!(f, "stored blob declares unsupported version {found}")
            }
            Self::UnsupportedWrap { found } => {
                write!(f, "stored blob declares unsupported wrap algorithm {found}")
            }
            Self::ReservedNotZero => f.write_str("stored blob has non-zero reserved bytes"),
            Self::LengthMismatch { declared, present } => write!(
                f,
                "stored blob declares {declared} wrapped bytes but carries {present}"
            ),
            Self::Unwrap { code } => {
                write!(f, "the platform store refused to unwrap: code {code}")
            }
            Self::MalformedSecret { found } => {
                write!(f, "unwrapped secret is {found} bytes, not 32")
            }
            Self::WrappedTooLarge { found } => {
                write!(f, "wrapped output does not fit a u32: {found} bytes")
            }
            Self::Io { code } => write!(f, "the identity store could not be accessed: {code}"),
        }
    }
}

impl core::error::Error for StoreError {}

impl StoreError {
    /// Whether this means "there is no identity" rather than "there is one and
    /// something went wrong".
    ///
    /// Exists so a caller does not have to re-derive the distinction with a
    /// `matches!` at every call site, and get it subtly wrong at one of them.
    #[must_use]
    pub const fn is_absent(&self) -> bool {
        matches!(self, Self::IdentityAbsent)
    }
}
