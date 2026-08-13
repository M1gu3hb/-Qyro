//! Real files for the Qyro transfer engine.
//!
//! Specification: `docs/adr/ADR-0027-filesystem-materialisation.md`.
//!
//! # What this does and what it does not
//!
//! It reads item content from files and writes it back to files, behind the two
//! seams `qyro_transfer` already defined. **Neither seam changed to make this
//! fit**, which is the evidence they were in the right place.
//!
//! There is **no file picker** and **no network**. The caller says which files;
//! choosing them on Android and Windows crosses the FFI and is sprint 5B.2.
//!
//! # The part that is security
//!
//! `RelativePath` validates a string. Traversal bites when that string is joined
//! to a root and opened, and the destination may already contain symbolic links
//! the manifest cannot describe. See [`safe_path`] and ADR-0027 §1, including
//! what `O_NOFOLLOW` closes, the post-open containment mitigation and the
//! descriptor-relative race that remains outside its guarantee (QYR-0072).

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

mod error;
mod history;
mod history_types;
mod io;
mod manifest_builder;
mod resume;
pub mod safe_path;

#[cfg(test)]
mod guards;
#[cfg(test)]
mod tests;

pub use error::FsError;
pub use history::{MAX_HISTORY_FILE_LEN, TransferHistory};
pub use history_types::{
    HistoryDirection, HistoryError, HistoryPeer, HistoryRecord, HistoryRepair, HistoryStatus,
};
pub use io::{FileSink, FileSource, HASH_BUFFER_LEN, digest_of};
pub use manifest_builder::{PlannedFile, manifest_from_disk};
pub use resume::{ItemProgress, ResumeState};
