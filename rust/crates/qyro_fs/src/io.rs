//! The two seams, backed by real files.
//!
//! `FileSource` reads a chunk at a time; `FileSink` writes to a `.qyro-part`
//! and renames only on a verified digest. Neither the engine nor ADR-0026's
//! traits changed to make this fit.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use qyro_transfer::{ContentSink, ContentSource};
use sha2::{Digest, Sha256};

use crate::error::FsError;
use crate::resume::{ItemProgress, ResumeState};
use crate::safe_path::{self, Resolved};

/// Bytes read at a time when hashing a file.
///
/// The manifest builder and the resume rebuild both use it. Sized to be a small
/// multiple of a page and far below any file this will meet.
pub const HASH_BUFFER_LEN: usize = 65_536;

/// Opens a file for writing without following a link at the final component.
///
/// `O_NOFOLLOW` on Unix and `FILE_FLAG_OPEN_REPARSE_POINT` on Windows. Both come
/// from `std::os`, so the symlink policy of ADR-0027 §1 costs no dependency.
///
/// This is the half of the policy with **no** race: the check and the open are
/// one syscall, so nothing can be substituted between them. The intermediate
/// components are the half that still has one (QYR-0072).
pub(crate) fn open_part(root: &Path, path: &Path, append: bool) -> Result<File, FsError> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).append(append);
    if !append {
        options.truncate(false);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        // O_NOFOLLOW: if the final component is a symlink, fail rather than
        // write through it.
        options.custom_flags(libc_o_nofollow());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        // FILE_FLAG_OPEN_REPARSE_POINT: open the reparse point itself rather
        // than its target.
        options.custom_flags(0x0020_0000);
    }

    let file = match options.open(path) {
        Ok(file) if metadata_is_link_or_reparse_point(&file.metadata()?) => {
            return Err(final_component_link(path));
        }
        Ok(file) => file,
        Err(error) => match fs::symlink_metadata(path) {
            // Unix reports ELOOP before returning a handle. Classify that
            // refusal by inspecting the path *after* the atomic open failed;
            // this check only chooses the error variant and is not the control.
            Ok(metadata) if metadata_is_link_or_reparse_point(&metadata) => {
                return Err(final_component_link(path));
            }
            _ => return Err(error.into()),
        },
    };

    // ADR-0027 §1.5. This is deliberately after the handle exists and before
    // callers can truncate, delete or write. It catches a parent substitution
    // that persists through the check; QYR-0072 records why a double swap still
    // needs descriptor-relative operations to close the race completely.
    let parent = path.parent().ok_or_else(|| FsError::EscapesRoot {
        resolved: path.to_string_lossy().into_owned(),
    })?;
    let canonical_parent = fs::canonicalize(parent)?;
    if !canonical_parent.starts_with(root) {
        return Err(FsError::EscapesRoot {
            resolved: canonical_parent.to_string_lossy().into_owned(),
        });
    }

    Ok(file)
}

fn final_component_link(path: &Path) -> FsError {
    FsError::SymlinkInPath {
        component: path.to_string_lossy().into_owned(),
    }
}

#[cfg(unix)]
fn metadata_is_link_or_reparse_point(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(windows)]
fn metadata_is_link_or_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;

    // FILE_ATTRIBUTE_REPARSE_POINT. Looking at the metadata of the handle
    // opened with FILE_FLAG_OPEN_REPARSE_POINT binds this decision to the
    // object that would otherwise receive the write, not to a second path walk.
    metadata.file_attributes() & 0x0000_0400 != 0
}

/// `O_NOFOLLOW` as a literal.
///
/// Spelled out rather than pulled from `libc`, which this workspace does not
/// depend on and which would be a new package for one integer. The values are
/// fixed by each platform ABI. The final-component integration test proves the
/// value on every host where it runs: Linux and macOS in CI. Android and iOS
/// compile these constants but still lack runtime filesystem evidence.
#[cfg(unix)]
const fn libc_o_nofollow() -> i32 {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        0o400_000
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        0x0000_0100
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    )))]
    {
        0
    }
}

/// Reads item content from files on disk, a chunk at a time.
///
/// Holds paths, not contents. A hundred-megabyte file never becomes a
/// hundred-megabyte allocation because nothing here ever asks for one.
pub struct FileSource {
    paths: BTreeMap<u32, PathBuf>,
    /// Open handles, kept so a transfer does not reopen per chunk.
    handles: RefCell<BTreeMap<u32, File>>,
    /// Largest single read this source has served, counted under test.
    #[cfg(test)]
    pub(crate) peak_read: std::cell::Cell<usize>,
}

impl FileSource {
    /// Builds a source over `item_id -> path`.
    #[must_use]
    pub fn new(paths: BTreeMap<u32, PathBuf>) -> Self {
        Self {
            paths,
            handles: RefCell::new(BTreeMap::new()),
            #[cfg(test)]
            peak_read: std::cell::Cell::new(0),
        }
    }

    /// Builds a source over handles that are already open.
    ///
    /// ADR-0034's Android half: the Storage Access Framework hands out a
    /// descriptor, so there is no path to reopen and `paths` stays empty. The
    /// handles are the only way in, and dropping this source closes them.
    #[must_use]
    pub fn from_open_files(handles: BTreeMap<u32, File>) -> Self {
        Self {
            paths: BTreeMap::new(),
            handles: RefCell::new(handles),
            #[cfg(test)]
            peak_read: std::cell::Cell::new(0),
        }
    }

    /// Reads into `out`, returning bytes read, or nothing on any failure.
    ///
    /// `ContentSource::read_at` has no error channel — it returns a count — so a
    /// failure here reads as a short read, and the engine's digest check is what
    /// turns that into a refusal. That is deliberate: the alternative is
    /// widening ADR-0026's trait, and a seam that has to change for its second
    /// implementation was the wrong seam.
    fn try_read(&self, item_id: u32, offset: u64, out: &mut [u8]) -> Option<usize> {
        let mut handles = self.handles.borrow_mut();
        let file = match handles.entry(item_id) {
            std::collections::btree_map::Entry::Occupied(slot) => slot.into_mut(),
            // Only a path-backed source opens lazily. A descriptor-backed one
            // has no path to reopen, and an item it has no handle for is an
            // item it cannot serve -- which reads as a short read, exactly as
            // any other read failure does.
            std::collections::btree_map::Entry::Vacant(slot) => {
                let path = self.paths.get(&item_id)?;
                slot.insert(File::open(path).ok()?)
            }
        };
        file.seek(SeekFrom::Start(offset)).ok()?;

        let mut filled = 0usize;
        while filled < out.len() {
            let slice = out.get_mut(filled..)?;
            match file.read(slice) {
                Ok(0) => break,
                Ok(count) => filled = filled.checked_add(count)?,
                Err(_) => return None,
            }
        }
        Some(filled)
    }
}

impl ContentSource for FileSource {
    fn read_at(&self, item_id: u32, offset: u64, out: &mut [u8]) -> usize {
        let filled = self.try_read(item_id, offset, out).unwrap_or(0);
        #[cfg(test)]
        self.peak_read.set(self.peak_read.get().max(filled));
        filled
    }
}

/// One item being written.
struct PartFile {
    resolved: Resolved,
    handle: File,
    written: u64,
}

/// Writes verified content into a destination directory.
///
/// Content goes to a `.qyro-part` beside its destination. The final name appears
/// only after [`FileSink::finish_item`] has verified the digest.
pub struct FileSink {
    root: PathBuf,
    /// `item_id -> (relative path, declared size, expected digest)`.
    plan: BTreeMap<u32, (String, u64, Vec<u8>)>,
    /// `item_id -> ruta` de las entradas que son carpetas.
    ///
    /// Se crean al preparar el destino, asi que `finish_item` tiene que poder
    /// decir que si sobre ellas: devolver un error hace que `Session::finish`
    /// salga del bucle y deje sin materializar lo que venga despues.
    directories: BTreeMap<u32, PathBuf>,
    open: BTreeMap<u32, PartFile>,
    transfer_id: u64,
    /// Largest single write this sink accepted, counted under test.
    #[cfg(test)]
    pub(crate) peak_write: usize,
}

impl FileSink {
    /// Builds a sink for `manifest` rooted at `root`.
    ///
    /// # Errors
    ///
    /// [`FsError::Io`] when `root` cannot be canonicalised.
    pub fn new(root: &Path, manifest: &qyro_manifest::TransferManifest) -> Result<Self, FsError> {
        fs::create_dir_all(root)?;
        let root = fs::canonicalize(root)?;
        let mut plan = BTreeMap::new();
        let mut directories = BTreeMap::new();
        for item in manifest.items() {
            // **Una carpeta se crea aquí y no entra en el plan** (ADR-0050
            // enmienda 1). Nadie va a escribir en ella, así que dejarla en el
            // plan sería un elemento que nunca se completa.
            //
            // Se crea con el mismo `resolve_under` que todo lo demás: un
            // manifiesto llega por el cable, y una entrada de directorio es tan
            // capaz de intentar salirse de la raíz como una de archivo.
            if item.kind() == qyro_manifest::ItemKind::Directory {
                let resolved = crate::safe_path::resolve_under(&root, &item.path().to_string())?;
                match fs::create_dir(&resolved.final_path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(error.into()),
                }
                // **Se recuerda cuál era**, y no por contabilidad: `finish_item`
                // tiene que poder decir que sí sobre ella. Sin esto devolvía
                // `DigestMismatch` —nunca entró en `open`— y `Session::finish`
                // hace `return` en ese caso, dejando **sin materializar todo lo
                // que viniera después en el manifiesto**.
                directories.insert(item.item_id(), resolved.final_path);
                continue;
            }
            plan.insert(
                item.item_id(),
                (
                    item.path().to_string(),
                    item.size(),
                    item.hash().digest().to_vec(),
                ),
            );
        }
        Ok(Self {
            root,
            plan,
            directories,
            open: BTreeMap::new(),
            transfer_id: manifest.transfer_id(),
            #[cfg(test)]
            peak_write: 0,
        })
    }

    /// Where the resume metadata for this destination lives.
    #[must_use]
    pub fn resume_path(root: &Path) -> PathBuf {
        root.join(".qyro-resume")
    }

    /// Returns the committed boundary when the destination metadata belongs to
    /// this transfer and describes `item_id`.
    fn committed_progress(&self, item_id: u32) -> Result<Option<u64>, FsError> {
        let bytes = match fs::read(Self::resume_path(&self.root)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let state = ResumeState::decode(&bytes)?;
        if state.transfer_id != self.transfer_id {
            return Ok(None);
        }
        Ok(state.progress_of(item_id))
    }

    /// Opens (or reopens) the part file for `item_id`.
    fn part_for(&mut self, item_id: u32) -> Result<&mut PartFile, FsError> {
        if !self.open.contains_key(&item_id) {
            // Not an `entry`: building the value can fail, and `entry` has no
            // way to give up after taking the slot.
            let (relative, _, _) = self
                .plan
                .get(&item_id)
                .ok_or(FsError::DigestMismatch { item_id })?;
            let relative = relative.clone();
            let resolved = safe_path::resolve_under(&self.root, &relative)?;

            // A collision is refused, never overwritten (ADR-0027 §2).
            if fs::symlink_metadata(&resolved.final_path).is_ok() {
                return Err(FsError::DestinationExists {
                    path: resolved.final_path.to_string_lossy().into_owned(),
                });
            }

            let part_exists = match fs::symlink_metadata(&resolved.part_path) {
                Ok(_) => true,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
                Err(error) => return Err(error.into()),
            };
            let committed = if part_exists {
                self.committed_progress(item_id)?
            } else {
                None
            };
            let (handle, written) = if part_exists {
                // Opening first applies the atomic final-component link/reparse
                // guard before either truncating or removing the prior file.
                let handle = open_part(&self.root, &resolved.part_path, false)?;
                if let Some(bytes_committed) = committed {
                    handle.set_len(bytes_committed)?;
                    (handle, bytes_committed)
                } else {
                    drop(handle);
                    fs::remove_file(&resolved.part_path)?;
                    (open_part(&self.root, &resolved.part_path, false)?, 0)
                }
            } else {
                (open_part(&self.root, &resolved.part_path, false)?, 0)
            };
            self.open.insert(
                item_id,
                PartFile {
                    resolved,
                    handle,
                    written,
                },
            );
        }
        self.open
            .get_mut(&item_id)
            .ok_or(FsError::DigestMismatch { item_id })
    }

    /// Writes `bytes` for `item_id` at `offset`.
    ///
    /// # Errors
    ///
    /// Whatever the path resolution or the filesystem reports.
    pub fn put(&mut self, item_id: u32, offset: u64, bytes: &[u8]) -> Result<(), FsError> {
        {
            let part = self.part_for(item_id)?;
            part.handle.seek(SeekFrom::Start(offset))?;
            part.handle.write_all(bytes)?;
            let end = offset.saturating_add(bytes.len() as u64);
            part.written = part.written.max(end);
        }
        #[cfg(test)]
        {
            self.peak_write = self.peak_write.max(bytes.len());
        }
        Ok(())
    }

    /// How far each item has got, for the resume file.
    #[must_use]
    pub fn progress(&self) -> ResumeState {
        ResumeState {
            transfer_id: self.transfer_id,
            items: self
                .open
                .iter()
                .map(|(item_id, part)| ItemProgress {
                    item_id: *item_id,
                    bytes_committed: part.written,
                })
                .collect(),
        }
    }

    /// Writes the resume metadata for this destination.
    ///
    /// # Errors
    ///
    /// [`FsError::Io`].
    pub fn persist_progress(&self) -> Result<(), FsError> {
        let path = Self::resume_path(&self.root);
        fs::write(path, self.progress().encode())?;
        Ok(())
    }

    /// Verifies the digest and, only if it matches, renames into place.
    ///
    /// The order is ADR-0027 §4: verify, `sync_all` the part file, rename, then
    /// `sync_all` the directory on Unix. A mismatch **deletes the part file** and
    /// produces nothing — keeping it would leave bytes nobody can verify sitting
    /// next to a name that suggests they are a transfer in progress.
    ///
    /// # Errors
    ///
    /// [`FsError::DigestMismatch`] or [`FsError::Io`].
    pub fn finish_item(&mut self, item_id: u32) -> Result<PathBuf, FsError> {
        // Una carpeta ya está terminada: se creó al preparar el destino, antes
        // de que llegara un byte. Decir que sí es la verdad, no una excepción.
        if let Some(path) = self.directories.get(&item_id) {
            return Ok(path.clone());
        }
        let Some(part) = self.open.remove(&item_id) else {
            return Err(FsError::DigestMismatch { item_id });
        };
        let expected = self
            .plan
            .get(&item_id)
            .map(|(_, _, digest)| digest.clone())
            .unwrap_or_default();

        part.handle.sync_all()?;
        drop(part.handle);

        let actual = digest_of(&part.resolved.part_path)?;
        if actual != expected {
            // Nothing verifiable survives a mismatch.
            let _ = fs::remove_file(&part.resolved.part_path);
            return Err(FsError::DigestMismatch { item_id });
        }

        fs::rename(&part.resolved.part_path, &part.resolved.final_path)?;

        // Durability of the rename itself. Unix only; Windows has no direct
        // equivalent and ADR-0027 §4 says so rather than pretending otherwise.
        #[cfg(unix)]
        if let Some(parent) = part.resolved.final_path.parent() {
            if let Ok(directory) = File::open(parent) {
                let _ = directory.sync_all();
            }
        }

        Ok(part.resolved.final_path)
    }

    /// Abandons the transfer: every partial this sink opened is removed.
    ///
    /// QYR-0088. Until this existed, the only thing that deleted a `.qyro-part`
    /// was `finish_item` failing its digest check — so the way to abandon a
    /// transfer was to **ask it to finish knowing it would fail**. That works and
    /// it is the wrong shape: a caller had to know that «reject it and the
    /// cleanup falls out» is how you give up, which is a side effect, not an
    /// interface.
    ///
    /// Returns how many partials it removed, so a caller can assert on it rather
    /// than trust it.
    ///
    /// **Total on purpose.** A partial that cannot be deleted — a handle held by
    /// a virus scanner, a read-only directory — must not stop the others from
    /// being deleted, and there is no useful thing a caller could do with a
    /// per-file error here. What it can do is compare the count.
    ///
    /// Also removes the resume metadata: leaving it would describe a transfer
    /// whose parts no longer exist, and the next run would resume into nothing.
    pub fn abandon(&mut self) -> usize {
        let mut removed = 0_usize;
        for (_, part) in std::mem::take(&mut self.open) {
            let path = part.resolved.part_path.clone();
            // The handle first: Windows refuses to delete a file that is open,
            // and a `remove_file` that quietly failed would leave the partial
            // behind while this reported success.
            drop(part.handle);
            if fs::remove_file(&path).is_ok() {
                removed = removed.saturating_add(1);
            }
        }
        let _ = fs::remove_file(Self::resume_path(&self.root));
        removed
    }
}

impl ContentSink for FileSink {
    fn write_at(&mut self, item_id: u32, offset: u64, bytes: &[u8]) {
        // `ContentSink::write_at` has no error channel either. A failed write
        // leaves the part file short, and the digest check at close is what
        // refuses it — the same reasoning as `FileSource::try_read`.
        let _ = self.put(item_id, offset, bytes);
    }
}

/// SHA-256 of a file, read in [`HASH_BUFFER_LEN`] pieces.
///
/// Streaming, not slurping: the buffer is a constant, so the memory this uses
/// does not grow with the file. That is the property
/// `building_a_manifest_from_disk_does_not_load_the_file` measures.
///
/// # Errors
///
/// [`FsError::Io`].
pub fn digest_of(path: &Path) -> Result<Vec<u8>, FsError> {
    let mut file = File::open(path)?;
    digest_of_reader(&mut file)
}

/// The same streaming digest over anything readable.
///
/// Factored out for ADR-0034: on Android the bytes arrive as an already-open
/// descriptor and there is no path to reopen. One implementation rather than
/// two, so a path-backed transfer and a descriptor-backed one cannot disagree
/// about what SHA-256 they computed.
///
/// # Errors
///
/// [`FsError::Io`] when a read fails.
pub fn digest_of_reader<R: Read>(source: &mut R) -> Result<Vec<u8>, FsError> {
    let file = source;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; HASH_BUFFER_LEN];
    loop {
        let count = file.read(&mut buffer)?;
        #[cfg(test)]
        crate::manifest_builder::PEAK_BUILDER_READ.with(|peak| {
            peak.set(peak.get().max(count));
        });
        if count == 0 {
            break;
        }
        match buffer.get(..count) {
            Some(slice) => hasher.update(slice),
            None => break,
        }
    }
    Ok(hasher.finalize().to_vec())
}
