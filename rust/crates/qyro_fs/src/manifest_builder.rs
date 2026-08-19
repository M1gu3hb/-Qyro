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

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};

use qyro_manifest::{HashAlgorithm, HashMetadata, ManifestItem, RelativePath, TransferManifest};

use crate::error::FsError;
use crate::io::{digest_of, digest_of_reader};

/// One file to put in a manifest: where it is, and what to call it.
#[derive(Clone, Debug)]
pub struct PlannedFile {
    /// Where the file is now.
    pub source: PathBuf,
    /// The relative path the receiver will materialise, as manifest text.
    pub relative: String,
}

#[cfg(test)]
std::thread_local! {
    /// Largest single read the builder performed, for the memory test.
    ///
    /// Thread-local so parallel tests hashing unrelated files cannot inflate
    /// one another's measurement.
    pub(crate) static PEAK_BUILDER_READ: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

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
        let path = RelativePath::parse(&file.relative).map_err(|_| FsError::EscapesRoot {
            resolved: file.relative.clone(),
        })?;
        let item_id = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);

        // **Una carpeta se manda como carpeta** (ADR-0050 enmienda 1). El disco
        // ya sabe cuál es cuál, así que no hace falta un tipo nuevo en la API:
        // se pregunta.
        //
        // `ItemKind::Directory` lleva en el formato de cable desde siempre —con
        // su validación y sus contratos— y **nadie lo emitía**. Dos ADR
        // justificaron no mandar carpetas vacías diciendo que haría falta una
        // versión de protocolo, y el tipo ya estaba ahí.
        if file.source.is_dir() {
            let item =
                ManifestItem::directory(item_id, path).map_err(|_| FsError::EscapesRoot {
                    resolved: file.relative.clone(),
                })?;
            items.push(item);
            continue;
        }

        let size = file_size(&file.source)?;
        let digest = digest_of(&file.source)?;

        let hash =
            HashMetadata::new(HashAlgorithm::Sha256, digest).map_err(|_| FsError::EscapesRoot {
                resolved: file.relative.clone(),
            })?;
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

/// One already-open file to put in a manifest, and what to call it.
///
/// The Android half of ADR-0034: the Storage Access Framework hands out a
/// descriptor, never a path, and `qyro_fs` never sees the `content://` URI it
/// came from. The `File` arrives already owned — `qyro_ffi` did the one `unsafe`
/// this needs, at the C boundary where `unsafe` already lives, so this crate
/// keeps `#![forbid(unsafe_code)]`.
#[derive(Debug)]
pub struct PlannedOpenFile {
    /// The open file. Its `Drop` closes the descriptor.
    pub handle: File,
    /// The relative path the receiver will materialise, as manifest text.
    pub relative: String,
}

/// Builds a manifest from files that are already open, streaming each digest.
///
/// Rewinds every handle afterwards, because the digest pass consumed it and the
/// transfer is about to read the same bytes from offset zero. Forgetting that
/// would send an empty file with a correct digest — a transfer that verifies and
/// delivers nothing.
///
/// # Errors
///
/// [`FsError::Io`] when a handle cannot be read or rewound, or
/// [`FsError::EscapesRoot`] when a relative name is not one the manifest accepts.
pub fn manifest_from_open_files(
    transfer_id: u64,
    created_unix_seconds: i64,
    files: &mut [PlannedOpenFile],
) -> Result<TransferManifest, FsError> {
    let mut items = Vec::with_capacity(files.len());
    for (index, file) in files.iter_mut().enumerate() {
        let size = file.handle.seek(SeekFrom::End(0))?;
        file.handle.seek(SeekFrom::Start(0))?;
        let digest = digest_of_reader(&mut file.handle)?;
        file.handle.seek(SeekFrom::Start(0))?;

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
            resolved: String::new(),
        }
    })
}

/// Takes the handles out, keyed by the item id the manifest gave them.
///
/// The ids are positional and must match [`manifest_from_open_files`], which is
/// why both derive them the same way from the same slice.
#[must_use]
pub fn descriptors_by_item(files: Vec<PlannedOpenFile>) -> BTreeMap<u32, File> {
    let mut handles = BTreeMap::new();
    for (index, file) in files.into_iter().enumerate() {
        let item_id = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);
        handles.insert(item_id, file.handle);
    }
    handles
}
