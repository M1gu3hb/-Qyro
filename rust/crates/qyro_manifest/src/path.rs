//! Relative paths that are safe to join onto a download directory.
//!
//! A path in a manifest is attacker-controlled. This module is the only place
//! allowed to turn peer bytes into something path-shaped, and it rejects rather
//! than sanitises: silently rewriting a hostile path tends to produce a
//! different hostile path.
//!
//! The rules are the union of Unix and Windows hazards, applied on every
//! platform, so a manifest that a Linux receiver accepts is exactly the one a
//! Windows receiver accepts.

use crate::error::PathError;
use crate::limits::{MAX_PATH_LEN, MAX_PATH_SEGMENTS, MAX_SEGMENT_LEN};

/// Canonical wire separator. The encoded form never contains a backslash.
pub const SEPARATOR: char = '/';

/// Device names Windows resolves before touching the filesystem.
///
/// They are reserved with or without an extension, so `CON.txt` is checked by
/// its stem.
const WINDOWS_RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// A validated relative path.
///
/// Constructing one is the proof that the path is relative, normalized and free
/// of traversal. Nothing else in the crate builds a path.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelativePath {
    normalized: String,
    segment_count: usize,
}

impl RelativePath {
    /// Validates and normalizes a candidate path.
    ///
    /// # Errors
    ///
    /// Returns the [`PathError`] for the first rule violated.
    pub fn parse(candidate: &str) -> Result<Self, PathError> {
        if candidate.is_empty() {
            return Err(PathError::Empty);
        }
        if candidate.len() > MAX_PATH_LEN {
            return Err(PathError::TooLong {
                length: candidate.len(),
                limit: MAX_PATH_LEN,
            });
        }

        // Checked before anything else: a NUL truncates the path in any C-style
        // API, so a name that looks safe here could become a different name at
        // the syscall boundary.
        if candidate.contains('\0') {
            return Err(PathError::NulByte);
        }
        if candidate.chars().any(|character| character.is_control()) {
            return Err(PathError::ControlCharacter);
        }

        // A backslash is a separator on Windows and a legal filename character
        // on Unix. Accepting it would mean the same manifest describes a
        // different tree on each platform.
        if candidate.contains('\\') {
            return Err(PathError::AmbiguousSeparator);
        }

        if candidate.starts_with("//") {
            return Err(PathError::UncPrefix);
        }
        if candidate.starts_with(SEPARATOR) {
            return Err(PathError::AbsoluteUnix);
        }
        if has_drive_prefix(candidate) {
            return Err(PathError::DrivePrefix);
        }

        let segments: Vec<&str> = candidate.split(SEPARATOR).collect();
        if segments.len() > MAX_PATH_SEGMENTS {
            return Err(PathError::TooManySegments {
                count: segments.len(),
                limit: MAX_PATH_SEGMENTS,
            });
        }

        for segment in &segments {
            validate_segment(segment)?;
        }

        Ok(Self {
            normalized: candidate.to_owned(),
            segment_count: segments.len(),
        })
    }

    /// Validates a candidate supplied as raw bytes.
    ///
    /// # Errors
    ///
    /// Returns [`PathError::InvalidUtf8`] when the bytes are not UTF-8, then the
    /// same rules as [`RelativePath::parse`].
    pub fn parse_bytes(candidate: &[u8]) -> Result<Self, PathError> {
        let text = core::str::from_utf8(candidate).map_err(|_| PathError::InvalidUtf8)?;
        Self::parse(text)
    }

    /// Returns the normalized path, always separated by `/`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.normalized
    }

    /// Returns the path segments.
    #[must_use]
    pub fn segments(&self) -> core::str::Split<'_, char> {
        self.normalized.split(SEPARATOR)
    }

    /// Returns the number of segments.
    #[must_use]
    pub const fn segment_count(&self) -> usize {
        self.segment_count
    }

    /// Returns the final segment, which is the file or directory name.
    #[must_use]
    pub fn file_name(&self) -> &str {
        self.normalized
            .rsplit(SEPARATOR)
            .next()
            .unwrap_or(&self.normalized)
    }

    /// Returns the encoded byte length.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.normalized.len()
    }
}

impl core::fmt::Display for RelativePath {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&self.normalized)
    }
}

fn validate_segment(segment: &str) -> Result<(), PathError> {
    if segment.is_empty() {
        return Err(PathError::EmptySegment);
    }
    if segment == ".." {
        return Err(PathError::ParentSegment);
    }
    if segment == "." {
        return Err(PathError::CurrentSegment);
    }
    if segment.len() > MAX_SEGMENT_LEN {
        return Err(PathError::SegmentTooLong {
            length: segment.len(),
            limit: MAX_SEGMENT_LEN,
        });
    }
    // Windows strips these when creating the file, so `evil.` and `evil` would
    // collide after the receiver believed they were distinct.
    if segment.ends_with('.') || segment.ends_with(' ') {
        return Err(PathError::TrailingDotOrSpace);
    }
    if is_windows_reserved(segment) {
        return Err(PathError::ReservedName);
    }
    Ok(())
}

fn is_windows_reserved(segment: &str) -> bool {
    let stem = segment.split('.').next().unwrap_or(segment);
    WINDOWS_RESERVED
        .iter()
        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
}

fn has_drive_prefix(candidate: &str) -> bool {
    let bytes = candidate.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}
