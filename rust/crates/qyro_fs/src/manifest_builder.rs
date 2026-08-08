//! Building a `TransferManifest` from files on disk.
//!
//! Every digest is streamed. The memory this uses is one buffer, whatever the
//! file weighs, and `building_a_manifest_from_disk_does_not_load_the_file`
//! measures it with a counter rather than a clock.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};

use qyro_manifest::{HashAlgorithm, HashMetadata, ManifestItem, RelativePath, TransferManifest};

use crate::error::FsError;
use crate::io::digest_of;

/// One file to put in a manifest: where it is, and what to call it.
#[derive(Clone, Debug)]
pub struct PlannedFile {
    /// Where the file is now.
    pub source: PathBuf,
    /// The relative path the receiver will materialise, as manifest text.
    pub relative: String,
}

/// Largest single read the builder performed, for the memory test.
///
/// A counter and not a clock: a wall clock on a shared runner measures the
/// runner. Sits beside the builder rather than inside the manifest so nothing
/// in the product carries it.
#[cfg(test)]
pub(crate) static PEAK_BUILDER_READ: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Builds a manifest by streaming each file's digest.
///
/// The whole file is never held: [`digest_of`] reads in fixed pieces, so the
/// memory cost is the buffer and not the content.
///
/// # Errors
///
/// [`FsError::Io`] when a file cannot be read, or
/// [`FsError::EscapesRoot`] when a planned relative path is not one the manifest
/// will accept.
pub fn manifest_from_disk(
    transfer_id: u64,
    created_unix_seconds: i64,
    files: &[PlannedFile],
) -> Result<TransferManifest, FsError> {
    let mut items = Vec::with_capacity(files.len());
    for (index, file) in files.iter().enumerate() {
        let size = file_size(&file.source)?;
        let digest = digest_of(&file.source)?;
        #[cfg(test)]
        PEAK_BUILDER_READ.fetch_max(
            crate::io::HASH_BUFFER_LEN,
            std::sync::atomic::Ordering::Relaxed,
        );

        let path = RelativePath::parse(&file.relative).map_err(|_| FsError::EscapesRoot {
            resolved: file.relative.clone(),
        })?;
        let hash =
            HashMetadata::new(HashAlgorithm::Sha256, digest).map_err(|_| FsError::EscapesRoot {
                resolved: file.relative.clone(),
            })?;
        let item_id = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);
        let item =
            ManifestItem::file(item_id, path, size, hash).map_err(|_| FsError::EscapesRoot {
                resolved: file.relative.clone(),
            })?;
        items.push(item);
    }

    TransferManifest::new(transfer_id, created_unix_seconds, items).map_err(|_| {
        FsError::EscapesRoot {
            resolved: "manifest".to_owned(),
        }
    })
}

/// Size of a file without opening it for reading.
fn file_size(path: &Path) -> Result<u64, FsError> {
    Ok(std::fs::metadata(path)?.len())
}
