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
fn open_part(path: &Path, append: bool) -> Result<File, FsError> {
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

    Ok(options.open(path)?)
}

/// `O_NOFOLLOW` as a literal.
///
/// Spelled out rather than pulled from `libc`, which this workspace does not
/// depend on and which would be a new package for one integer. The value is
/// fixed by the platform ABI, and `a_symlink_at_the_final_component_is_refused`
/// is what proves the number is the right one: a wrong constant makes that test
/// pass a write through the link, loudly.
#[cfg(unix)]
const fn libc_o_nofollow() -> i32 {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        0
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

    /// Reads into `out`, returning bytes read, or nothing on any failure.
    ///
    /// `ContentSource::read_at` has no error channel — it returns a count — so a
    /// failure here reads as a short read, and the engine's digest check is what
    /// turns that into a refusal. That is deliberate: the alternative is
    /// widening ADR-0026's trait, and a seam that has to change for its second
    /// implementation was the wrong seam.
    fn try_read(&self, item_id: u32, offset: u64, out: &mut [u8]) -> Option<usize> {
        let path = self.paths.get(&item_id)?;
        let mut handles = self.handles.borrow_mut();
        let file = match handles.entry(item_id) {
            std::collections::btree_map::Entry::Occupied(slot) => slot.into_mut(),
            std::collections::btree_map::Entry::Vacant(slot) => slot.insert(File::open(path).ok()?),
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
        self.peak_read.set(self.peak_read.get().max(out.len()));
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
        let mut plan = BTreeMap::new();
        for item in manifest.items() {
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
            root: root.to_path_buf(),
            plan,
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

            let handle = open_part(&resolved.part_path, false)?;
            let written = handle.metadata().map(|m| m.len()).unwrap_or(0);
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
        #[cfg(test)]
        {
            self.peak_write = self.peak_write.max(bytes.len());
        }
        let part = self.part_for(item_id)?;
        part.handle.seek(SeekFrom::Start(offset))?;
        part.handle.write_all(bytes)?;
        let end = offset.saturating_add(bytes.len() as u64);
        part.written = part.written.max(end);
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
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; HASH_BUFFER_LEN];
    loop {
        let count = file.read(&mut buffer)?;
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
