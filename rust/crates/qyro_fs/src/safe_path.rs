//! Turning a validated manifest path into a place on this disk.
//!
//! `RelativePath` validates the string. Traversal bites somewhere else: at the
//! moment that string is joined to a root and opened. ADR-0027 §1.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::FsError;

/// Where a resolved manifest path lands, with its parent already created.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolved {
    /// The final file.
    pub final_path: PathBuf,
    /// The `.qyro-part` beside it. Same directory, so `rename` stays atomic.
    pub part_path: PathBuf,
}

/// Appends the part-file suffix to a file name.
fn part_name(final_path: &Path) -> PathBuf {
    let mut name = final_path.as_os_str().to_os_string();
    name.push(".qyro-part");
    PathBuf::from(name)
}

/// Resolves `relative` under `root`, creating the directories it needs.
///
/// Refuses if any existing component is a symbolic link, and refuses if the
/// result lands outside `root`. Directories are created one at a time with
/// [`fs::create_dir`] and never `create_dir_all`: creating the whole chain
/// delegates traversal of paths nobody checked.
///
/// # Errors
///
/// [`FsError::SymlinkInPath`], [`FsError::EscapesRoot`] or [`FsError::Io`].
pub fn resolve_under(root: &Path, relative: &str) -> Result<Resolved, FsError> {
    let canonical_root = fs::canonicalize(root)?;

    let mut here = canonical_root.clone();
    let segments: Vec<&str> = relative.split('/').filter(|s| !s.is_empty()).collect();
    let Some((file_name, directories)) = segments.split_last() else {
        return Err(FsError::EscapesRoot {
            resolved: relative.to_owned(),
        });
    };

    // A manifest path has no `.` or `..` — `RelativePath` refuses them — but
    // this does not take that on trust. What arrives here has crossed a wire.
    for segment in &segments {
        if *segment == "." || *segment == ".." {
            return Err(FsError::EscapesRoot {
                resolved: relative.to_owned(),
            });
        }
    }

    for segment in directories {
        here.push(segment);
        assert_not_a_symlink(&here)?;
        match fs::create_dir(&here) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }

    // The parent exists now, so it can be canonicalised and compared. Doing it
    // after creation rather than before is the point: before, it did not exist.
    let canonical_parent = fs::canonicalize(&here)?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(FsError::EscapesRoot {
            resolved: canonical_parent.to_string_lossy().into_owned(),
        });
    }

    let final_path = canonical_parent.join(file_name);
    assert_not_a_symlink(&final_path)?;

    Ok(Resolved {
        part_path: part_name(&final_path),
        final_path,
    })
}

/// Refuses a path that exists and is a symbolic link.
///
/// `symlink_metadata` is `lstat`: it does **not** follow. The link is never
/// followed to see where it points, because following one in order to judge it
/// is half the race.
fn assert_not_a_symlink(path: &Path) -> Result<(), FsError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(FsError::SymlinkInPath {
            component: path.to_string_lossy().into_owned(),
        }),
        Ok(_) => Ok(()),
        // Not existing yet is the common case: this is a destination.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Whether `candidate` stays inside `root` once both are canonical.
///
/// Exposed for the resolver's own tests and for callers that hold a path they
/// did not resolve themselves.
///
/// # Errors
///
/// [`FsError::Io`] when either path cannot be canonicalised.
pub fn is_inside(root: &Path, candidate: &Path) -> Result<bool, FsError> {
    let root = fs::canonicalize(root)?;
    let candidate = fs::canonicalize(candidate)?;
    Ok(candidate.starts_with(&root))
}

/// Whether a path has no `.` or `..` components.
///
/// A second opinion on what the manifest already refused, kept because the two
/// checks answer different questions: the manifest judges a string it received,
/// and this judges a path about to be opened.
#[must_use]
pub fn has_no_traversal(path: &Path) -> bool {
    path.components()
        .all(|component| matches!(component, Component::Normal(_)))
}
