//! One variant per way the filesystem side refuses.

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

/// Why a read, a write or a materialisation refused.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FsError {
    /// A component of the destination path exists and is a symbolic link.
    ///
    /// ADR-0027 §1. The link is not followed to see where it points: following
    /// one in order to judge it is half the race.
    SymlinkInPath { component: String },
    /// The resolved path left the destination root.
    ///
    /// Reached even for a manifest path the manifest itself accepted, because
    /// what the manifest validates is a string and what this checks is where
    /// that string lands on this disk.
    EscapesRoot { resolved: String },
    /// The final file already exists at the destination.
    ///
    /// Refused, never overwritten: overwriting is other people's data lost on
    /// the sender's say-so (ADR-0027 §2).
    DestinationExists { path: String },
    /// The content hashed to something other than the manifest said.
    DigestMismatch { item_id: u32 },
    /// Resume metadata this build does not know how to read.
    UnsupportedResumeVersion { found: u8 },
    /// Resume metadata that is not resume metadata.
    NotResumeMetadata,
    /// Resume metadata shorter than its fixed fields.
    ResumeTruncated { found: usize },
    /// Resume metadata whose reserved byte was not zero.
    ResumeReservedNotZero,
    /// The underlying filesystem call failed.
    ///
    /// Carries the raw OS code rather than a rendered string, so a report can
    /// say which failure it was without this crate pretending to interpret it.
    Io { code: i32 },
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SymlinkInPath { component } => {
                write!(f, "{component} is a symbolic link and will not be followed")
            }
            Self::EscapesRoot { resolved } => {
                write!(f, "{resolved} is outside the destination root")
            }
            Self::DestinationExists { path } => {
                write!(f, "{path} already exists and will not be overwritten")
            }
            Self::DigestMismatch { item_id } => {
                write!(f, "item {item_id} hashed to something else")
            }
            Self::UnsupportedResumeVersion { found } => {
                write!(f, "resume metadata declares unsupported version {found}")
            }
            Self::NotResumeMetadata => f.write_str("these bytes are not Qyro resume metadata"),
            Self::ResumeTruncated { found } => {
                write!(f, "resume metadata is {found} bytes, too short")
            }
            Self::ResumeReservedNotZero => {
                f.write_str("resume metadata has a non-zero reserved byte")
            }
            Self::Io { code } => write!(f, "the filesystem refused: code {code}"),
        }
    }
}

impl core::error::Error for FsError {}

impl From<std::io::Error> for FsError {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            code: error.raw_os_error().unwrap_or(-1),
        }
    }
}
