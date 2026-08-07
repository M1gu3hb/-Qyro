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

use unicode_normalization::UnicodeNormalization;

use crate::error::PathError;
use crate::limits::{MAX_PATH_LEN, MAX_PATH_SEGMENTS, MAX_SEGMENT_LEN};

/// Canonical wire separator. The encoded form never contains a backslash.
pub const SEPARATOR: char = '/';

/// Characters Windows forbids in a filename.
///
/// Rejected on every platform: a manifest must describe a tree that can be
/// materialised identically everywhere, so a name Linux would accept but Windows
/// would refuse is a portability failure, not a receiver's problem.
const WINDOWS_ILLEGAL: [char; 7] = ['<', '>', ':', '"', '|', '?', '*'];

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
        // `is_control()` is category `Cc` and stops there. The invisible half of
        // the problem is category `Cf`, and it is the dangerous half: see
        // [`is_unicode_format`].
        if let Some(found) = candidate
            .chars()
            .find(|character| is_unicode_format(*character))
        {
            return Err(PathError::FormatCharacter { found });
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
    if let Some(found) = segment.chars().find(|c| WINDOWS_ILLEGAL.contains(c)) {
        return Err(PathError::NonPortableCharacter { found });
    }
    // U+007F used to be checked again here. It is `Cc`, so `is_control()` in
    // `parse` already refused it before any segment was examined; the second
    // check could never fire and suggested the first one had a gap it does not
    // have. The real gap was category `Cf`, which is now closed in `parse`.
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

/// Every code point in Unicode general category `Cf`, as inclusive ranges.
///
/// # Source
///
/// Transcribed from the Unicode Character Database,
/// `extracted/DerivedGeneralCategory.txt` of Unicode 16.0.0 (2024-04-30), by
/// taking every line whose category is `Cf` in file order. The table is sorted
/// and disjoint, which [`is_unicode_format`] relies on and a test checks.
///
/// A hand-written table rather than a new dependency, deliberately. This code
/// runs on peer-supplied bytes before anything reaches the filesystem, so
/// twenty-one reviewable lines with a citation are worth more here than a crate
/// whose contents nobody in this repository has read. The cost is that a future
/// Unicode version can add a range: a code point added to `Cf` after 16.0.0
/// would be accepted until this table is updated. That is a bounded, visible
/// staleness, and it is the trade ADR-0019 records.
///
/// # Why the whole category
///
/// Unicode UTR #36 §2.5.1 (*Security Considerations*) says never to allow the
/// bidirectional override characters in identifiers, and treats the invisible
/// `Cf` characters as a display-spoofing hazard generally. A filename is
/// exactly the string a human reads before deciding to accept a transfer, so
/// the display is the security property.
///
/// `U+200C ZERO WIDTH NON-JOINER` and `U+200D ZERO WIDTH JOINER` are the two
/// UTR #36 carves out, because Indic and Persian orthography needs them next to
/// a virama and a rule that drops them changes the word. They are **rejected
/// here anyway**, and that is a decision rather than an oversight: a filename is
/// not a linguistic identifier, the crate's declared posture is to reject rather
/// than sanitise, and accepting a character that renders as nothing would put
/// two visually identical names in one manifest. A sender that needs one in a
/// name gets a clear error and can rename; a receiver that cannot tell two
/// entries apart has no such recourse. See ADR-0019.
const UNICODE_FORMAT_RANGES: [(char, char); 21] = [
    ('\u{00AD}', '\u{00AD}'),   // SOFT HYPHEN
    ('\u{0600}', '\u{0605}'),   // ARABIC NUMBER SIGN..ARABIC NUMBER MARK ABOVE
    ('\u{061C}', '\u{061C}'),   // ARABIC LETTER MARK
    ('\u{06DD}', '\u{06DD}'),   // ARABIC END OF AYAH
    ('\u{070F}', '\u{070F}'),   // SYRIAC ABBREVIATION MARK
    ('\u{0890}', '\u{0891}'),   // ARABIC POUND MARK ABOVE..ARABIC PIASTRE MARK ABOVE
    ('\u{08E2}', '\u{08E2}'),   // ARABIC DISPUTED END OF AYAH
    ('\u{180E}', '\u{180E}'),   // MONGOLIAN VOWEL SEPARATOR
    ('\u{200B}', '\u{200F}'),   // ZERO WIDTH SPACE..RIGHT-TO-LEFT MARK
    ('\u{202A}', '\u{202E}'),   // LEFT-TO-RIGHT EMBEDDING..RIGHT-TO-LEFT OVERRIDE
    ('\u{2060}', '\u{2064}'),   // WORD JOINER..INVISIBLE PLUS
    ('\u{2066}', '\u{206F}'),   // LEFT-TO-RIGHT ISOLATE..NOMINAL DIGIT SHAPES
    ('\u{FEFF}', '\u{FEFF}'),   // ZERO WIDTH NO-BREAK SPACE
    ('\u{FFF9}', '\u{FFFB}'),   // INTERLINEAR ANNOTATION ANCHOR..TERMINATOR
    ('\u{110BD}', '\u{110BD}'), // KAITHI NUMBER SIGN
    ('\u{110CD}', '\u{110CD}'), // KAITHI NUMBER SIGN ABOVE
    ('\u{13430}', '\u{1343F}'), // EGYPTIAN HIEROGLYPH VERTICAL JOINER..END WALLED ENCLOSURE
    ('\u{1BCA0}', '\u{1BCA3}'), // SHORTHAND FORMAT LETTER OVERLAP..UP STEP
    ('\u{1D173}', '\u{1D17A}'), // MUSICAL SYMBOL BEGIN BEAM..END PHRASE
    ('\u{E0001}', '\u{E0001}'), // LANGUAGE TAG
    ('\u{E0020}', '\u{E007F}'), // TAG SPACE..CANCEL TAG
];

/// Whether a character is in Unicode general category `Cf`.
///
/// Binary search over [`UNICODE_FORMAT_RANGES`], which is sorted and disjoint.
fn is_unicode_format(character: char) -> bool {
    UNICODE_FORMAT_RANGES
        .binary_search_by(|(first, last)| {
            if character < *first {
                core::cmp::Ordering::Greater
            } else if character > *last {
                core::cmp::Ordering::Less
            } else {
                core::cmp::Ordering::Equal
            }
        })
        .is_ok()
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

/// A key two paths share when a real filesystem would treat them as one file.
///
/// Linux keeps `Foto.jpg` and `foto.jpg` apart; Windows and macOS do not. Unicode
/// adds a second axis: `ñ` composed (U+00F1) and decomposed (`n` + U+0303) are
/// different byte sequences that most filesystems consider the same name.
///
/// A manifest carrying both would overwrite one item with the other on the
/// receiver, silently, after the transfer was accepted. The key exists to reject
/// that pair. The original spelling of each path is never altered.
///
/// # Policy
///
/// Per segment, in order:
///
/// 1. **Canonical composition (NFC)** via `unicode-normalization`. This is real
///    Unicode canonical equivalence, so it folds NFC against NFD for every
///    script, not just the ones somebody remembered to tabulate.
/// 2. **Simple lowercase** via `str::to_lowercase`, which is full-Unicode in
///    std and handles the locale-independent special cases.
///
/// Segments are joined with a NUL, which no valid path may contain, so `a/bc`
/// and `ab/c` cannot fold together.
///
/// **Diacritics are preserved.** An earlier hand-written table mapped `ñ` to
/// `n`, `é` to `e` and dropped combining marks outright, which made `ano.txt`
/// collide with `año.txt` and `resume.pdf` with `résumé.pdf` — legitimate,
/// different files that the manifest then refused. Over-folding is as much a
/// defect as under-folding, and a partial table was wrong in both directions:
/// it never covered `ř`, `ż` or `ệ` at all.
///
/// This is case-insensitive matching, not case-insensitive storage: two names
/// differing only by case are treated as one file, which is what Windows and
/// the default macOS filesystem do.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PortableCollisionKey(String);

impl PortableCollisionKey {
    /// Derives the key for a validated path.
    #[must_use]
    pub fn of(path: &RelativePath) -> Self {
        let folded: Vec<String> = path.segments().map(fold_segment).collect();
        Self(folded.join("\u{0}"))
    }

    /// Returns the folded representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Canonically composes a segment, then lowercases it.
fn fold_segment(segment: &str) -> String {
    segment.nfc().collect::<String>().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{UNICODE_FORMAT_RANGES, is_unicode_format};

    #[test]
    fn the_format_table_is_sorted_and_disjoint() {
        // `is_unicode_format` binary-searches this table. A table that is out of
        // order, or whose ranges touch, makes the search silently miss a code
        // point rather than fail loudly — the exact shape of defect QYR-0021
        // was.
        for window in UNICODE_FORMAT_RANGES.windows(2) {
            let (first_start, first_end) = window[0];
            let (second_start, _) = window[1];
            assert!(
                first_start <= first_end,
                "range {first_start:?}..={first_end:?} is inverted"
            );
            assert!(
                first_end < second_start,
                "range ending {first_end:?} overlaps the one starting {second_start:?}"
            );
        }
    }

    #[test]
    fn the_search_finds_every_boundary_of_every_range() {
        // Both endpoints of every range, and the code points immediately
        // outside them, so an off-by-one in the comparator cannot pass.
        for (first, last) in UNICODE_FORMAT_RANGES {
            assert!(
                is_unicode_format(first),
                "{first:?} is the start of a range"
            );
            assert!(is_unicode_format(last), "{last:?} is the end of a range");

            if let Some(before) = char::from_u32(u32::from(first) - 1) {
                assert!(
                    !is_unicode_format(before),
                    "{before:?} is below a range and must not match"
                );
            }
            if let Some(after) = char::from_u32(u32::from(last) + 1) {
                assert!(
                    !is_unicode_format(after),
                    "{after:?} is above a range and must not match"
                );
            }
        }
    }

    #[test]
    fn ordinary_characters_are_not_format_characters() {
        for character in ['a', 'Z', '0', '.', '/', 'ñ', '日', '🎉', '\u{0301}'] {
            assert!(!is_unicode_format(character), "{character:?}");
        }
    }
}
