//! Public value and error types for the local transfer history.

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

/// The full 256-bit identity fingerprint used to group history by peer.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HistoryPeer([u8; 32]);

impl HistoryPeer {
    /// Constructs the stable peer identifier from a canonical full fingerprint.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns all 32 bytes. History never groups by the 128-bit human display.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Which side moved the bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryDirection {
    /// This device sent the transfer.
    Sent,
    /// This device received the transfer.
    Received,
}

/// Terminal outcome retained in local history.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryStatus {
    /// Every selected item completed.
    Completed,
    /// A participant cancelled before completion.
    Cancelled,
    /// The transfer stopped on an error.
    Failed,
}

/// What opening the append-only file did to its tail.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryRepair {
    /// Every record was complete, valid and in time order.
    Clean,
    /// The first incomplete or corrupt record and everything after it was removed.
    TailDiscarded { bytes: u64 },
}

/// One terminal transfer summary. No paths or content names are retained.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryRecord {
    pub(crate) ended_at: i64,
    pub(crate) transfer_id: u64,
    pub(crate) peer: HistoryPeer,
    pub(crate) direction: HistoryDirection,
    pub(crate) status: HistoryStatus,
    pub(crate) item_count: u32,
    pub(crate) bytes_transferred: u64,
}

impl HistoryRecord {
    /// Terminal time as Unix UTC seconds.
    #[must_use]
    pub const fn ended_at(&self) -> i64 {
        self.ended_at
    }

    /// Transfer identifier from the authenticated session.
    #[must_use]
    pub const fn transfer_id(&self) -> u64 {
        self.transfer_id
    }

    /// Full peer fingerprint.
    #[must_use]
    pub const fn peer(&self) -> HistoryPeer {
        self.peer
    }

    /// Sent or received.
    #[must_use]
    pub const fn direction(&self) -> HistoryDirection {
        self.direction
    }

    /// Terminal status.
    #[must_use]
    pub const fn status(&self) -> HistoryStatus {
        self.status
    }

    /// Number of manifest items in the transfer.
    #[must_use]
    pub const fn item_count(&self) -> u32 {
        self.item_count
    }

    /// Bytes committed before the terminal status.
    #[must_use]
    pub const fn bytes_transferred(&self) -> u64 {
        self.bytes_transferred
    }
}

/// Why local history could not be opened or appended.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum HistoryError {
    /// The existing file is shorter than its 12-byte header.
    TruncatedHistoryHeader { found: usize },
    /// The magic is not `QYRO-HST`.
    NotTransferHistory,
    /// The file declares a format this build does not implement.
    UnsupportedHistoryVersion { found: u8 },
    /// A reserved header byte was non-zero.
    HistoryReservedNotZero,
    /// The file or a prospective append exceeds the 16 MiB safety bound.
    HistoryFileTooLarge { found: u64, maximum: u64 },
    /// A record used a negative Unix UTC time.
    InvalidTimestamp { found: i64 },
    /// Append would make the sequence decrease in time.
    OutOfOrder { previous: i64, found: i64 },
    /// A prior append failed; reopening is required to repair any partial tail.
    NeedsReopen,
    /// The operating system refused an operation.
    Io { code: i32 },
}

impl fmt::Display for HistoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedHistoryHeader { found } => {
                write!(
                    formatter,
                    "transfer history header is truncated at {found} bytes"
                )
            }
            Self::NotTransferHistory => {
                formatter.write_str("stored bytes are not Qyro transfer history")
            }
            Self::UnsupportedHistoryVersion { found } => {
                write!(
                    formatter,
                    "transfer history declares unsupported version {found}"
                )
            }
            Self::HistoryReservedNotZero => {
                formatter.write_str("transfer history has non-zero reserved bytes")
            }
            Self::HistoryFileTooLarge { found, maximum } => write!(
                formatter,
                "transfer history is {found} bytes, maximum is {maximum}"
            ),
            Self::InvalidTimestamp { found } => {
                write!(formatter, "transfer history timestamp is negative: {found}")
            }
            Self::OutOfOrder { previous, found } => write!(
                formatter,
                "transfer history time moved backwards from {previous} to {found}"
            ),
            Self::NeedsReopen => {
                formatter.write_str("transfer history must be reopened after a failed append")
            }
            Self::Io { code } => write!(formatter, "transfer history I/O failed: code {code}"),
        }
    }
}

impl core::error::Error for HistoryError {}

impl From<std::io::Error> for HistoryError {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            code: error.raw_os_error().unwrap_or(-1),
        }
    }
}
