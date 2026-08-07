//! Typed manifest failures.

use core::fmt;

/// Why a relative path was refused.
///
/// Every variant describes a way a peer-supplied path could escape the download
/// directory or produce a surprising file on some platform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PathError {
    /// The path had no segments.
    Empty,
    /// The path exceeded the total length limit.
    TooLong {
        /// Length found.
        length: usize,
        /// Largest accepted length.
        limit: usize,
    },
    /// A single segment exceeded the segment length limit.
    SegmentTooLong {
        /// Length found.
        length: usize,
        /// Largest accepted length.
        limit: usize,
    },
    /// The path had more segments than allowed.
    TooManySegments {
        /// Count found.
        count: usize,
        /// Largest accepted count.
        limit: usize,
    },
    /// A `..` segment, which would climb out of the destination directory.
    ParentSegment,
    /// A `.` segment, which has two spellings for the same location.
    CurrentSegment,
    /// An empty segment, produced by a leading, trailing or doubled separator.
    EmptySegment,
    /// A Unix absolute path.
    AbsoluteUnix,
    /// A Windows drive-qualified path such as `C:\Windows`.
    DrivePrefix,
    /// A UNC path such as `\\server\share`.
    UncPrefix,
    /// A NUL byte, which truncates the path in C-style APIs.
    NulByte,
    /// A backslash, which is a separator on Windows and a legal name elsewhere.
    AmbiguousSeparator,
    /// A Windows reserved device name such as `CON` or `COM1`.
    ReservedName,
    /// A segment ending in a dot or space, which Windows silently strips.
    TrailingDotOrSpace,
    /// A control character.
    ControlCharacter,
    /// A Unicode format character: general category `Cf`.
    ///
    /// Its own variant rather than a second spelling of
    /// [`PathError::ControlCharacter`], because it is a different hazard and the
    /// message has to be able to say so. `Cc` characters break a path at the
    /// syscall boundary; `Cf` characters are invisible and change how the *rest*
    /// of the name renders, which is how `invoice<RLO>fdp.exe` is displayed as
    /// `invoiceexe.pdf`.
    FormatCharacter {
        /// The offending code point.
        found: char,
    },
    /// The path was not valid UTF-8.
    InvalidUtf8,
    /// A character no Windows filesystem accepts in a name.
    NonPortableCharacter {
        /// The offending character.
        found: char,
    },
}

impl fmt::Display for PathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("relative path is empty"),
            Self::TooLong { length, limit } => {
                write!(formatter, "path length {length} exceeds limit {limit}")
            }
            Self::SegmentTooLong { length, limit } => {
                write!(formatter, "segment length {length} exceeds limit {limit}")
            }
            Self::TooManySegments { count, limit } => {
                write!(formatter, "segment count {count} exceeds limit {limit}")
            }
            Self::ParentSegment => formatter.write_str("path contains a .. segment"),
            Self::CurrentSegment => formatter.write_str("path contains a . segment"),
            Self::EmptySegment => formatter.write_str("path contains an empty segment"),
            Self::AbsoluteUnix => formatter.write_str("path is absolute"),
            Self::DrivePrefix => formatter.write_str("path carries a drive prefix"),
            Self::UncPrefix => formatter.write_str("path is a UNC path"),
            Self::NulByte => formatter.write_str("path contains a NUL byte"),
            Self::AmbiguousSeparator => formatter.write_str("path contains a backslash"),
            Self::ReservedName => formatter.write_str("path uses a reserved device name"),
            Self::TrailingDotOrSpace => formatter.write_str("segment ends in a dot or space"),
            Self::ControlCharacter => formatter.write_str("path contains a control character"),
            // Printed as a code point, never as the character itself: rendering
            // it would reproduce, in the diagnostic, the display attack the
            // rejection exists to stop.
            Self::FormatCharacter { found } => write!(
                formatter,
                "path contains the Unicode format character U+{:04X}",
                u32::from(*found)
            ),
            Self::InvalidUtf8 => formatter.write_str("path is not valid UTF-8"),
            Self::NonPortableCharacter { found } => {
                write!(
                    formatter,
                    "path contains the non-portable character {found:?}"
                )
            }
        }
    }
}

impl core::error::Error for PathError {}

/// Why a manifest was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ManifestError {
    /// A path failed validation.
    InvalidPath {
        /// Index of the offending item, when decoding a manifest.
        index: usize,
        /// The path rule that was violated.
        source: PathError,
    },
    /// The serialized manifest did not start with the expected magic.
    InvalidMagic {
        /// Bytes actually found.
        found: [u8; 4],
    },
    /// The manifest declares a format version this build cannot read.
    UnsupportedVersion {
        /// Version declared.
        found: u16,
        /// Version this build implements.
        supported: u16,
    },
    /// The declared item count exceeds [`crate::MAX_ITEMS`].
    TooManyItems {
        /// Count declared.
        declared: u64,
        /// Largest accepted count.
        limit: usize,
    },
    /// The declared total size exceeds [`crate::MAX_TOTAL_BYTES`].
    TotalBytesTooLarge {
        /// Total declared.
        declared: u64,
        /// Largest accepted total.
        limit: u64,
    },
    /// Item sizes did not sum to the declared total, or the sum overflowed.
    TotalBytesMismatch {
        /// Total the manifest declared.
        declared: u64,
        /// Total obtained by summing the items, when it did not overflow.
        computed: Option<u64>,
    },
    /// The number of items present did not match the declared count.
    ItemCountMismatch {
        /// Count declared.
        declared: usize,
        /// Items actually present.
        present: usize,
    },
    /// Two items shared the same normalized path.
    DuplicatePath {
        /// Index of the second occurrence.
        index: usize,
    },
    /// Two items shared the same identifier.
    DuplicateItemId {
        /// Index of the second occurrence.
        index: usize,
    },
    /// Items were not in canonical order.
    UnsortedItems {
        /// Index where ordering broke.
        index: usize,
    },
    /// A directory declared a non-zero size or a content hash.
    InvalidDirectory {
        /// Index of the offending item.
        index: usize,
    },
    /// A file carried no final digest.
    ///
    /// Every file needs one, including an empty one: the digest is what proves
    /// the received bytes are the sent bytes.
    MissingFileHash {
        /// Index of the offending item.
        index: usize,
    },
    /// Two items would land on the same file on a real filesystem.
    ///
    /// Case folding or Unicode composition made distinct-looking paths collide;
    /// accepting both would silently overwrite one with the other.
    PortableCollision {
        /// Index of the second occurrence.
        index: usize,
        /// Index of the item it collides with.
        collides_with: usize,
    },
    /// A string field exceeded its limit.
    FieldTooLong {
        /// Which field.
        field: ManifestField,
        /// Length found.
        length: usize,
        /// Largest accepted length.
        limit: usize,
    },
    /// A hash length did not match its algorithm.
    InvalidHashLength {
        /// Length found.
        length: usize,
        /// Length the algorithm requires.
        expected: usize,
    },
    /// A field held a value outside its allowed set.
    InvalidFieldValue {
        /// Which field.
        field: ManifestField,
        /// Raw value found.
        value: u8,
    },
    /// A string field was not valid UTF-8.
    InvalidUtf8 {
        /// Which field.
        field: ManifestField,
    },
    /// The encoded manifest ended before the declared structure did.
    Truncated {
        /// Bytes available.
        available: usize,
        /// Bytes required.
        required: usize,
    },
    /// Bytes remained after the manifest was fully decoded.
    TrailingBytes {
        /// Number of unexpected bytes.
        count: usize,
    },
    /// The serialized manifest exceeded [`crate::MAX_ENCODED_LEN`].
    EncodedTooLarge {
        /// Length found.
        length: usize,
        /// Largest accepted length.
        limit: usize,
    },
}

/// Manifest fields, so errors stay typed instead of stringly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ManifestField {
    /// Item relative path.
    Path,
    /// Item MIME type.
    MimeType,
    /// Item hash digest.
    Hash,
    /// Hash algorithm discriminant.
    HashAlgorithm,
    /// Compression discriminant.
    Compression,
    /// Item kind discriminant.
    ItemKind,
    /// Optional-field presence byte.
    OptionTag,
}

impl fmt::Display for ManifestField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Path => "path",
            Self::MimeType => "mime_type",
            Self::Hash => "hash",
            Self::HashAlgorithm => "hash_algorithm",
            Self::Compression => "compression",
            Self::ItemKind => "item_kind",
            Self::OptionTag => "option_tag",
        };
        formatter.write_str(name)
    }
}

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath { index, source } => {
                write!(formatter, "item {index} has an invalid path: {source}")
            }
            Self::InvalidMagic { .. } => formatter.write_str("manifest magic is not QYRM"),
            Self::UnsupportedVersion { found, supported } => write!(
                formatter,
                "unsupported manifest version {found}, this build reads {supported}"
            ),
            Self::TooManyItems { declared, limit } => {
                write!(formatter, "item count {declared} exceeds limit {limit}")
            }
            Self::TotalBytesTooLarge { declared, limit } => {
                write!(formatter, "total bytes {declared} exceeds limit {limit}")
            }
            Self::TotalBytesMismatch { declared, computed } => match computed {
                Some(computed) => write!(
                    formatter,
                    "declared total {declared} does not match computed {computed}"
                ),
                None => write!(formatter, "summing item sizes overflowed u64"),
            },
            Self::ItemCountMismatch { declared, present } => write!(
                formatter,
                "manifest declares {declared} items but carries {present}"
            ),
            Self::DuplicatePath { index } => {
                write!(formatter, "item {index} repeats an earlier path")
            }
            Self::DuplicateItemId { index } => {
                write!(formatter, "item {index} repeats an earlier item id")
            }
            Self::UnsortedItems { index } => {
                write!(formatter, "item {index} breaks canonical ordering")
            }
            Self::InvalidDirectory { index } => {
                write!(formatter, "item {index} is a directory with file metadata")
            }
            Self::MissingFileHash { index } => {
                write!(formatter, "item {index} is a file without a final digest")
            }
            Self::PortableCollision {
                index,
                collides_with,
            } => write!(
                formatter,
                "item {index} collides with item {collides_with} on a case-insensitive or normalizing filesystem"
            ),
            Self::FieldTooLong {
                field,
                length,
                limit,
            } => write!(formatter, "{field} length {length} exceeds limit {limit}"),
            Self::InvalidHashLength { length, expected } => write!(
                formatter,
                "hash length {length} does not match the algorithm's {expected}"
            ),
            Self::InvalidFieldValue { field, value } => {
                write!(formatter, "invalid value {value} for {field}")
            }
            Self::InvalidUtf8 { field } => write!(formatter, "{field} is not valid UTF-8"),
            Self::Truncated {
                available,
                required,
            } => write!(
                formatter,
                "manifest truncated: {available} of {required} bytes"
            ),
            Self::TrailingBytes { count } => {
                write!(formatter, "{count} unexpected bytes after the manifest")
            }
            Self::EncodedTooLarge { length, limit } => {
                write!(formatter, "encoded manifest {length} exceeds limit {limit}")
            }
        }
    }
}

impl core::error::Error for ManifestError {}

impl From<PathError> for ManifestError {
    fn from(source: PathError) -> Self {
        Self::InvalidPath { index: 0, source }
    }
}
