//! Public trust result types, separate from their construction sites.

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

use qyro_crypto::IdentityError;

use crate::StoreError;

/// The three trust outcomes. `New` is deliberately not a success boolean.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustVerdict {
    /// The selected record exists and its complete public identity matches.
    KnownAndMatches,
    /// The selected record exists but its complete public identity changed.
    KnownAndChanged,
    /// No record exists under the locally selected name.
    New,
}

/// A typed refusal while validating or opening a known-peer store.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum KnownPeerStoreError {
    /// A local name was empty.
    EmptyName,
    /// A local name exceeded 255 UTF-8 bytes.
    NameTooLong { found: usize },
    /// A local name contained a Unicode control character.
    NameContainsControl,
    /// Times were negative or `last_seen` preceded `first_seen`.
    InvalidTimestamps { first_seen: i64, last_seen: i64 },
    /// The store exceeded 4096 peers.
    TooManyPeers { found: usize },
    /// Two records used the same exact local name.
    DuplicateName,
    /// Two records used the same complete public identity.
    DuplicateIdentity,
    /// Fewer bytes remained than the field being read required.
    Truncated { found: usize, needed: usize },
    /// The outer magic was not `QYRO-KPS`.
    NotAKnownPeerStore,
    /// The outer version is newer or otherwise unsupported.
    UnsupportedKnownPeerVersion { found: u8 },
    /// The wrapper identifier is unknown to this build.
    UnsupportedKnownPeerWrap { found: u8 },
    /// An outer reserved byte was non-zero.
    ReservedNotZero,
    /// The wrapped body exceeded the fixed 2 MiB limit.
    WrappedTooLarge { found: usize },
    /// The declared wrapped length differed from the bytes present.
    LengthMismatch { declared: u32, present: usize },
    /// The file belongs to another platform wrapper.
    WrapMismatch { blob: u8, wrapper: u8 },
    /// The selected platform wrapper refused to wrap or unwrap.
    Wrapper(StoreError),
    /// The authenticated clear body exceeded its structural maximum.
    UnwrappedTooLarge { found: usize },
    /// A record length was outside 52 through 306 bytes.
    InvalidRecordLength { found: u32 },
    /// A record length did not equal its fixed fields plus its name.
    RecordLengthMismatch { declared: u32, expected: usize },
    /// A record carried a public identity that Qyro refuses.
    MalformedPublicIdentity(IdentityError),
    /// A record's declared name bytes were not UTF-8.
    MalformedName,
    /// Bytes remained after the declared record count.
    TrailingBytes { found: usize },
}

impl fmt::Display for KnownPeerStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("known-peer name is empty"),
            Self::NameTooLong { found } => {
                write!(
                    formatter,
                    "known-peer name is {found} bytes, maximum is 255"
                )
            }
            Self::NameContainsControl => {
                formatter.write_str("known-peer name contains a control character")
            }
            Self::InvalidTimestamps {
                first_seen,
                last_seen,
            } => write!(
                formatter,
                "known-peer times are invalid: first {first_seen}, last {last_seen}"
            ),
            Self::TooManyPeers { found } => {
                write!(
                    formatter,
                    "known-peer store has {found} records, maximum is 4096"
                )
            }
            Self::DuplicateName => formatter.write_str("known-peer name is duplicated"),
            Self::DuplicateIdentity => {
                formatter.write_str("known-peer public identity is duplicated")
            }
            Self::Truncated { found, needed } => write!(
                formatter,
                "known-peer store is truncated: {found} bytes remain, {needed} needed"
            ),
            Self::NotAKnownPeerStore => {
                formatter.write_str("stored bytes are not a Qyro known-peer store")
            }
            Self::UnsupportedKnownPeerVersion { found } => {
                write!(
                    formatter,
                    "known-peer store declares unsupported version {found}"
                )
            }
            Self::UnsupportedKnownPeerWrap { found } => write!(
                formatter,
                "known-peer store declares unsupported wrapper {found}"
            ),
            Self::ReservedNotZero => {
                formatter.write_str("known-peer store has non-zero reserved bytes")
            }
            Self::WrappedTooLarge { found } => write!(
                formatter,
                "known-peer wrapped body is {found} bytes, maximum is 2097152"
            ),
            Self::LengthMismatch { declared, present } => write!(
                formatter,
                "known-peer store declares {declared} wrapped bytes but carries {present}"
            ),
            Self::WrapMismatch { blob, wrapper } => write!(
                formatter,
                "known-peer store was wrapped by {blob} and was handed to wrapper {wrapper}"
            ),
            Self::Wrapper(error) => write!(formatter, "known-peer wrapper refused: {error}"),
            Self::UnwrappedTooLarge { found } => write!(
                formatter,
                "known-peer clear body is {found} bytes, above its structural maximum"
            ),
            Self::InvalidRecordLength { found } => {
                write!(
                    formatter,
                    "known-peer record declares invalid length {found}"
                )
            }
            Self::RecordLengthMismatch { declared, expected } => write!(
                formatter,
                "known-peer record declares {declared} bytes but its fields require {expected}"
            ),
            Self::MalformedPublicIdentity(error) => {
                write!(formatter, "known-peer public identity is invalid: {error}")
            }
            Self::MalformedName => formatter.write_str("known-peer name is not valid UTF-8"),
            Self::TrailingBytes { found } => {
                write!(formatter, "known-peer store has {found} trailing bytes")
            }
        }
    }
}

impl core::error::Error for KnownPeerStoreError {}
