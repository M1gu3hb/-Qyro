//! Transfer manifests and the paths inside them.
//!
//! A manifest says what is about to be written to disk, and it arrives from a
//! peer. This crate is deliberately independent of the real filesystem: it never
//! opens, stats or creates anything. It turns untrusted bytes into a value whose
//! existence proves the paths are relative, free of traversal, and expressible
//! on every platform Qyro targets, and leaves the actual I/O to a layer that can
//! be given a root directory.
//!
//! A validated path is stored **verbatim**. This said "normalized" until sprint
//! 4C.2, and the field was named that way too, which contradicted the rule
//! immediately below it (QYR-0031).
//!
//! # Safety posture
//!
//! - [`RelativePath`] rejects instead of sanitising. Rewriting a hostile path
//!   usually just produces a different hostile path.
//! - Unix and Windows rules are enforced on every platform, so a manifest is
//!   accepted or refused identically everywhere.
//! - Counts and lengths are checked against the constants in this crate before
//!   they can drive an allocation.
//! - Sizes are summed with checked arithmetic, so a set of items engineered to
//!   wrap `u64` is an error rather than a small, believable total.
//! - The visible name is derived from the path, never sent separately, so
//!   `invoice.pdf.exe` cannot be presented as `invoice.pdf` (ADR-0019). Since
//!   sprint 4C.2 the Unicode format characters are refused as well, so
//!   `invoice<RLO>fdp.exe` cannot be rendered as `invoiceexe.pdf` either
//!   (QYR-0021).
//! - Two paths that a real filesystem would fold onto one file are rejected,
//!   including a file that is also another item's parent directory (QYR-0028).
//!
//! # Example
//!
//! ```
//! use qyro_manifest::{HashMetadata, ManifestItem, RelativePath, TransferManifest, codec};
//!
//! let path = RelativePath::parse("photos/summer/beach.jpg")?;
//! // Every file needs a final digest, including an empty one.
//! let hash = HashMetadata::new(qyro_manifest::HashAlgorithm::Sha256, vec![0x11; 32])?;
//! let item = ManifestItem::file(1, path, 2048, hash)?;
//! let manifest = TransferManifest::new(7, 1_760_000_000, vec![item])?;
//!
//! let bytes = codec::encode(&manifest)?;
//! assert_eq!(codec::decode(&bytes)?, manifest);
//!
//! // Traversal never becomes a path.
//! assert!(RelativePath::parse("../../etc/passwd").is_err());
//! # Ok::<(), Box<dyn core::error::Error>>(())
//! ```

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codec;
mod error;
mod limits;
mod model;
mod path;

pub use error::{ManifestError, ManifestField, PathError};
pub use limits::{
    MANIFEST_MAGIC, MANIFEST_VERSION, MAX_ENCODED_LEN, MAX_HASH_LEN, MAX_ITEMS, MAX_MIME_LEN,
    MAX_NAME_LEN, MAX_PATH_LEN, MAX_PATH_SEGMENTS, MAX_SEGMENT_LEN, MAX_TOTAL_BYTES,
};
pub use model::{
    Compression, HashAlgorithm, HashMetadata, ItemKind, ManifestItem, TransferManifest,
};
pub use path::{PortableCollisionKey, RelativePath, SEPARATOR};
