//! Manifest data model.

use crate::error::{ManifestError, ManifestField};
use crate::limits::{MAX_HASH_LEN, MAX_ITEMS, MAX_MIME_LEN, MAX_TOTAL_BYTES};
use crate::path::{PortableCollisionKey, RelativePath};

/// What a manifest item represents.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum ItemKind {
    /// A regular file.
    File = 1,
    /// A directory, carried so empty directories survive the transfer.
    Directory = 2,
}

impl ItemKind {
    /// Returns the stable wire value.
    #[must_use]
    pub const fn to_wire(self) -> u8 {
        self as u8
    }

    /// Resolves a wire value.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::InvalidFieldValue`] for anything else.
    pub const fn from_wire(value: u8) -> Result<Self, ManifestError> {
        match value {
            1 => Ok(Self::File),
            2 => Ok(Self::Directory),
            other => Err(ManifestError::InvalidFieldValue {
                field: ManifestField::ItemKind,
                value: other,
            }),
        }
    }
}

/// Digest algorithm used for an item.
///
/// Manifest v2 has exactly one final digest: SHA-256. BLAKE3 was accepted as an
/// alternative, which meant two peers could disagree about what "verified"
/// means and a file could arrive with no SHA-256 anywhere. A fast digest for
/// chunk-level checks is useful, but it belongs in a separate optional field
/// added by a later version, not as a substitute for the final verdict.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum HashAlgorithm {
    /// No digest recorded. Only valid for directories.
    None = 0,
    /// SHA-256, 32 bytes. The only final digest for a file.
    Sha256 = 1,
}

impl HashAlgorithm {
    /// Digest length this algorithm produces, in bytes.
    #[must_use]
    pub const fn digest_len(self) -> usize {
        match self {
            Self::None => 0,
            Self::Sha256 => 32,
        }
    }

    /// Returns the stable wire value.
    #[must_use]
    pub const fn to_wire(self) -> u8 {
        self as u8
    }

    /// Resolves a wire value.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::InvalidFieldValue`] for anything else.
    pub const fn from_wire(value: u8) -> Result<Self, ManifestError> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Sha256),
            // Value 2 was BLAKE3 in an unreleased draft. It is rejected rather
            // than silently accepted as a final digest.
            other => Err(ManifestError::InvalidFieldValue {
                field: ManifestField::HashAlgorithm,
                value: other,
            }),
        }
    }
}

/// Compression applied to an item's content.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Compression {
    /// Content is stored as-is.
    #[default]
    None = 0,
}

impl Compression {
    /// Returns the stable wire value.
    #[must_use]
    pub const fn to_wire(self) -> u8 {
        self as u8
    }

    /// Resolves a wire value.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::InvalidFieldValue`] for anything else.
    pub const fn from_wire(value: u8) -> Result<Self, ManifestError> {
        match value {
            0 => Ok(Self::None),
            other => Err(ManifestError::InvalidFieldValue {
                field: ManifestField::Compression,
                value: other,
            }),
        }
    }
}

/// Digest metadata for an item.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HashMetadata {
    algorithm: HashAlgorithm,
    digest: Vec<u8>,
}

impl HashMetadata {
    /// The absence of a digest.
    #[must_use]
    pub fn none() -> Self {
        Self {
            algorithm: HashAlgorithm::None,
            digest: Vec::new(),
        }
    }

    /// Builds digest metadata, checking the length matches the algorithm.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::InvalidHashLength`] when the digest length does
    /// not match, or [`ManifestError::FieldTooLong`] beyond [`MAX_HASH_LEN`].
    pub fn new(algorithm: HashAlgorithm, digest: Vec<u8>) -> Result<Self, ManifestError> {
        if digest.len() > MAX_HASH_LEN {
            return Err(ManifestError::FieldTooLong {
                field: ManifestField::Hash,
                length: digest.len(),
                limit: MAX_HASH_LEN,
            });
        }
        if digest.len() != algorithm.digest_len() {
            return Err(ManifestError::InvalidHashLength {
                length: digest.len(),
                expected: algorithm.digest_len(),
            });
        }
        Ok(Self { algorithm, digest })
    }

    /// Returns the algorithm.
    #[must_use]
    pub const fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }

    /// Returns the digest bytes.
    #[must_use]
    pub fn digest(&self) -> &[u8] {
        &self.digest
    }

    /// Whether a digest is present.
    #[must_use]
    pub fn is_present(&self) -> bool {
        self.algorithm != HashAlgorithm::None
    }
}

/// One entry in a transfer manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestItem {
    item_id: u32,
    path: RelativePath,
    kind: ItemKind,
    size: u64,
    mime_type: Option<String>,
    modified_unix_seconds: Option<i64>,
    hash: HashMetadata,
    compression: Compression,
}

impl ManifestItem {
    /// Builds a file item.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::MissingFileHash`] when `hash` carries no digest.
    /// Every file needs a final digest, including an empty one: the digest is
    /// what proves the received bytes are the sent bytes.
    pub fn file(
        item_id: u32,
        path: RelativePath,
        size: u64,
        hash: HashMetadata,
    ) -> Result<Self, ManifestError> {
        Self::new(
            item_id,
            path,
            ItemKind::File,
            size,
            None,
            None,
            hash,
            Compression::None,
        )
    }

    /// Builds a directory item.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError`] when a field exceeds its limit.
    pub fn directory(item_id: u32, path: RelativePath) -> Result<Self, ManifestError> {
        Self::new(
            item_id,
            path,
            ItemKind::Directory,
            0,
            None,
            None,
            HashMetadata::none(),
            Compression::None,
        )
    }

    /// Builds an item with every field specified.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::FieldTooLong`] when a string exceeds its limit,
    /// or [`ManifestError::InvalidDirectory`] when a directory carries file
    /// metadata.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        item_id: u32,
        path: RelativePath,
        kind: ItemKind,
        size: u64,
        mime_type: Option<String>,
        modified_unix_seconds: Option<i64>,
        hash: HashMetadata,
        compression: Compression,
    ) -> Result<Self, ManifestError> {
        if let Some(mime) = &mime_type
            && mime.len() > MAX_MIME_LEN
        {
            return Err(ManifestError::FieldTooLong {
                field: ManifestField::MimeType,
                length: mime.len(),
                limit: MAX_MIME_LEN,
            });
        }
        match kind {
            ItemKind::Directory => {
                if size != 0 || hash.is_present() {
                    return Err(ManifestError::InvalidDirectory { index: 0 });
                }
            }
            ItemKind::File => {
                // SHA-256 exactly: the final verdict must be one algorithm both
                // peers compute, and a 32-byte digest even for an empty file.
                if hash.algorithm() != HashAlgorithm::Sha256 {
                    return Err(ManifestError::MissingFileHash { index: 0 });
                }
            }
        }

        Ok(Self {
            item_id,
            path,
            kind,
            size,
            mime_type,
            modified_unix_seconds,
            hash,
            compression,
        })
    }

    /// Returns the item identifier.
    #[must_use]
    pub const fn item_id(&self) -> u32 {
        self.item_id
    }

    /// Returns the validated relative path.
    #[must_use]
    pub const fn path(&self) -> &RelativePath {
        &self.path
    }

    /// Returns the name to show, always the path's final segment.
    ///
    /// Derived rather than stored: a separately supplied name could disagree
    /// with where the bytes land, letting `invoice.pdf.exe` be presented as
    /// `invoice.pdf`. See ADR-0019.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.path.file_name()
    }

    /// Returns the item kind.
    #[must_use]
    pub const fn kind(&self) -> ItemKind {
        self.kind
    }

    /// Returns the declared size in bytes.
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// Returns the MIME type, when declared.
    #[must_use]
    pub fn mime_type(&self) -> Option<&str> {
        self.mime_type.as_deref()
    }

    /// Returns the modification time as Unix seconds, when declared.
    #[must_use]
    pub const fn modified_unix_seconds(&self) -> Option<i64> {
        self.modified_unix_seconds
    }

    /// Returns the digest metadata.
    #[must_use]
    pub const fn hash(&self) -> &HashMetadata {
        &self.hash
    }

    /// Returns the compression metadata.
    #[must_use]
    pub const fn compression(&self) -> Compression {
        self.compression
    }

    /// Sets the MIME type.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::FieldTooLong`] beyond [`MAX_MIME_LEN`].
    pub fn with_mime_type(mut self, mime_type: &str) -> Result<Self, ManifestError> {
        if mime_type.len() > MAX_MIME_LEN {
            return Err(ManifestError::FieldTooLong {
                field: ManifestField::MimeType,
                length: mime_type.len(),
                limit: MAX_MIME_LEN,
            });
        }
        self.mime_type = Some(mime_type.to_owned());
        Ok(self)
    }

    /// Sets the modification time.
    #[must_use]
    pub const fn with_modified_unix_seconds(mut self, seconds: i64) -> Self {
        self.modified_unix_seconds = Some(seconds);
        self
    }
}

/// A complete transfer manifest.
///
/// Building one enforces the collection-level invariants: canonical ordering,
/// unique paths and identifiers, and a declared total that matches the items.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferManifest {
    transfer_id: u64,
    created_unix_seconds: i64,
    total_bytes: u64,
    items: Vec<ManifestItem>,
}

impl TransferManifest {
    /// Builds a manifest from items, validating every collection invariant.
    ///
    /// Items are sorted by path here; the decoder, by contrast, rejects
    /// unsorted input rather than reordering it, because reordering would
    /// change the bytes that were authenticated.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError`] when a limit, uniqueness or totals rule fails.
    pub fn new(
        transfer_id: u64,
        created_unix_seconds: i64,
        mut items: Vec<ManifestItem>,
    ) -> Result<Self, ManifestError> {
        if items.len() > MAX_ITEMS {
            return Err(ManifestError::TooManyItems {
                declared: items.len() as u64,
                limit: MAX_ITEMS,
            });
        }
        items.sort_by(|left, right| left.path.cmp(&right.path));

        let total_bytes = validate_items(&items)?;
        Ok(Self {
            transfer_id,
            created_unix_seconds,
            total_bytes,
            items,
        })
    }

    /// Builds a manifest from items already in canonical order.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::UnsortedItems`] when ordering is wrong, plus the
    /// same rules as [`TransferManifest::new`].
    pub fn from_sorted(
        transfer_id: u64,
        created_unix_seconds: i64,
        items: Vec<ManifestItem>,
        declared_total_bytes: u64,
    ) -> Result<Self, ManifestError> {
        if items.len() > MAX_ITEMS {
            return Err(ManifestError::TooManyItems {
                declared: items.len() as u64,
                limit: MAX_ITEMS,
            });
        }
        let computed = validate_items(&items)?;
        if computed != declared_total_bytes {
            return Err(ManifestError::TotalBytesMismatch {
                declared: declared_total_bytes,
                computed: Some(computed),
            });
        }
        Ok(Self {
            transfer_id,
            created_unix_seconds,
            total_bytes: declared_total_bytes,
            items,
        })
    }

    /// Returns the transfer identifier.
    #[must_use]
    pub const fn transfer_id(&self) -> u64 {
        self.transfer_id
    }

    /// Returns the creation time as Unix seconds.
    #[must_use]
    pub const fn created_unix_seconds(&self) -> i64 {
        self.created_unix_seconds
    }

    /// Returns the number of items.
    #[must_use]
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Returns the declared total size in bytes.
    #[must_use]
    pub const fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Returns the items in canonical order.
    #[must_use]
    pub fn items(&self) -> &[ManifestItem] {
        &self.items
    }
}

/// Checks ordering, uniqueness and totals, returning the summed size.
///
/// The sum uses checked arithmetic so a set of items engineered to wrap `u64`
/// produces an error rather than a small, believable total.
fn validate_items(items: &[ManifestItem]) -> Result<u64, ManifestError> {
    let mut total: u64 = 0;
    // The predecessor is carried rather than indexed. `items[index - 1]` is
    // correct and unprovable; a value threaded through the loop is neither.
    let mut previous: Option<&ManifestItem> = None;
    for (index, item) in items.iter().enumerate() {
        if let Some(previous) = previous {
            match item.path.cmp(&previous.path) {
                core::cmp::Ordering::Less => {
                    return Err(ManifestError::UnsortedItems { index });
                }
                core::cmp::Ordering::Equal => {
                    return Err(ManifestError::DuplicatePath { index });
                }
                core::cmp::Ordering::Greater => {}
            }
        }
        previous = Some(item);
        if item.kind == ItemKind::Directory && (item.size != 0 || item.hash.is_present()) {
            return Err(ManifestError::InvalidDirectory { index });
        }
        total = total
            .checked_add(item.size)
            .ok_or(ManifestError::TotalBytesMismatch {
                declared: 0,
                computed: None,
            })?;
        if total > MAX_TOTAL_BYTES {
            return Err(ManifestError::TotalBytesTooLarge {
                declared: total,
                limit: MAX_TOTAL_BYTES,
            });
        }
    }

    // Two paths that a real filesystem would treat as one file must not both be
    // present: the receiver would silently overwrite one with the other after
    // accepting the transfer. Sorted keys keep this linearithmic.
    let mut keys: Vec<(PortableCollisionKey, usize)> = items
        .iter()
        .enumerate()
        .map(|(index, item)| (PortableCollisionKey::of(&item.path), index))
        .collect();
    keys.sort_unstable();
    // Pairwise by iterator rather than by `windows(2)` and two indexes: `zip`
    // yields the pair as a pair, so there is no length to assume.
    for (left, right) in keys.iter().zip(keys.iter().skip(1)) {
        if left.0 == right.0 {
            let (first, second) = if left.1 < right.1 {
                (left.1, right.1)
            } else {
                (right.1, left.1)
            };
            return Err(ManifestError::PortableCollision {
                index: second,
                collides_with: first,
            });
        }

        // Equality is only half of it. `a` and `a/b` fold to `"a"` and
        // `"a\0b"`, which are different strings and were therefore accepted —
        // even though a receiver would have to create `a` as a file and `a` as
        // a directory, and whichever it did second would lose the other.
        //
        // The rule is a prefix rule. A key that is a proper prefix of the next
        // one, *at a NUL boundary*, is that key's ancestor. The NUL matters:
        // `"report"` is a prefix of `"reports\0..."` as a string and an
        // ancestor of nothing, so a raw prefix test would refuse two unrelated
        // files.
        //
        // Sorting is enough to make adjacency sufficient. NUL is the lowest
        // byte no path may contain, so every descendant of `a` sorts
        // immediately after `a` and before any other key beginning with `a`.
        //
        // Only a *file* ancestor is a conflict. A directory item with children
        // is the ordinary shape of a tree.
        if let Some(rest) = right.0.as_str().strip_prefix(left.0.as_str())
            && rest.starts_with('\u{0}')
            && items
                .get(left.1)
                .is_some_and(|item| item.kind == ItemKind::File)
        {
            return Err(ManifestError::FileIsAlsoADirectory {
                index: right.1,
                ancestor: left.1,
            });
        }
    }

    // Identifier uniqueness is quadratic-free via a sorted copy: manifests are
    // bounded by MAX_ITEMS, and sorting keeps this linearithmic.
    let mut identifiers: Vec<(u32, usize)> = items
        .iter()
        .enumerate()
        .map(|(index, item)| (item.item_id, index))
        .collect();
    identifiers.sort_unstable();
    for (left, right) in identifiers.iter().zip(identifiers.iter().skip(1)) {
        if left.0 == right.0 {
            return Err(ManifestError::DuplicateItemId { index: right.1 });
        }
    }

    Ok(total)
}
