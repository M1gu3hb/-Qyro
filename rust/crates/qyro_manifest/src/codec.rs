//! Canonical binary encoding for manifests.
//!
//! One logical manifest has exactly one byte representation, so the encoded form
//! can be authenticated without a normalization pass. See
//! `docs/adr/ADR-0017-manifest-serialization.md`.

use crate::error::{ManifestError, ManifestField};
use crate::limits::{
    MANIFEST_MAGIC, MANIFEST_VERSION, MAX_ENCODED_LEN, MAX_ITEMS, MAX_MIME_LEN, MAX_PATH_LEN,
    MAX_TOTAL_BYTES,
};
use crate::model::{
    Compression, HashAlgorithm, HashMetadata, ItemKind, ManifestItem, TransferManifest,
};
use crate::path::RelativePath;

/// Computes the exact encoded length without building anything.
///
/// Checked arithmetic throughout, so a manifest engineered to overflow `usize`
/// is an error rather than a small, believable total. Callers can therefore
/// enforce [`MAX_ENCODED_LEN`] *before* a single byte is reserved, instead of
/// discovering it partway through a buffer that already exceeded the limit.
///
/// The result equals `encode(manifest)?.len()` exactly; a test pins that.
///
/// # Errors
///
/// Returns [`ManifestError::EncodedTooLarge`] when the total would exceed
/// [`MAX_ENCODED_LEN`] or overflow.
pub fn encoded_len(manifest: &TransferManifest) -> Result<usize, ManifestError> {
    const HEADER_LEN: usize = 4 + 2 + 8 + 8 + 8 + 4;

    let mut total = HEADER_LEN;
    for item in manifest.items() {
        // item_id + kind + path(len prefix + bytes) + size
        let mut item_len = 4usize
            .checked_add(1)
            .and_then(|n| n.checked_add(4))
            .and_then(|n| n.checked_add(item.path().byte_len()))
            .and_then(|n| n.checked_add(8))
            .ok_or(ManifestError::EncodedTooLarge {
                length: usize::MAX,
                limit: MAX_ENCODED_LEN,
            })?;

        item_len = match item.mime_type() {
            Some(mime) => item_len
                .checked_add(1 + 4)
                .and_then(|n| n.checked_add(mime.len())),
            None => item_len.checked_add(1),
        }
        .ok_or(ManifestError::EncodedTooLarge {
            length: usize::MAX,
            limit: MAX_ENCODED_LEN,
        })?;

        item_len = match item.modified_unix_seconds() {
            Some(_) => item_len.checked_add(1 + 8),
            None => item_len.checked_add(1),
        }
        .ok_or(ManifestError::EncodedTooLarge {
            length: usize::MAX,
            limit: MAX_ENCODED_LEN,
        })?;

        // hash algorithm + digest + compression
        item_len = item_len
            .checked_add(1)
            .and_then(|n| n.checked_add(item.hash().digest().len()))
            .and_then(|n| n.checked_add(1))
            .ok_or(ManifestError::EncodedTooLarge {
                length: usize::MAX,
                limit: MAX_ENCODED_LEN,
            })?;

        total = total
            .checked_add(item_len)
            .ok_or(ManifestError::EncodedTooLarge {
                length: usize::MAX,
                limit: MAX_ENCODED_LEN,
            })?;

        if total > MAX_ENCODED_LEN {
            return Err(ManifestError::EncodedTooLarge {
                length: total,
                limit: MAX_ENCODED_LEN,
            });
        }
    }

    Ok(total)
}

/// Serializes a manifest into its canonical byte form.
///
/// The size is settled by [`encoded_len`] first, so the buffer is reserved once
/// at exactly the right capacity and never grows past the limit mid-encode.
///
/// # Errors
///
/// Returns [`ManifestError::EncodedTooLarge`] when the result would exceed
/// [`MAX_ENCODED_LEN`].
pub fn encode(manifest: &TransferManifest) -> Result<Vec<u8>, ManifestError> {
    let expected = encoded_len(manifest)?;
    let mut out = Vec::with_capacity(expected);
    out.extend_from_slice(&MANIFEST_MAGIC);
    out.extend_from_slice(&MANIFEST_VERSION.to_be_bytes());
    out.extend_from_slice(&manifest.transfer_id().to_be_bytes());
    out.extend_from_slice(&manifest.created_unix_seconds().to_be_bytes());
    out.extend_from_slice(&manifest.total_bytes().to_be_bytes());
    let item_count =
        u32::try_from(manifest.item_count()).map_err(|_| ManifestError::TooManyItems {
            declared: manifest.item_count() as u64,
            limit: MAX_ITEMS,
        })?;
    out.extend_from_slice(&item_count.to_be_bytes());

    for item in manifest.items() {
        encode_item(item, &mut out);
    }

    debug_assert_eq!(
        out.len(),
        expected,
        "encoded_len must match the bytes actually produced"
    );
    Ok(out)
}

fn encode_item(item: &ManifestItem, out: &mut Vec<u8>) {
    out.extend_from_slice(&item.item_id().to_be_bytes());
    out.push(item.kind().to_wire());
    encode_string(item.path().as_str(), out);
    out.extend_from_slice(&item.size().to_be_bytes());
    encode_optional_string(item.mime_type(), out);
    match item.modified_unix_seconds() {
        Some(seconds) => {
            out.push(1);
            out.extend_from_slice(&seconds.to_be_bytes());
        }
        None => out.push(0),
    }
    out.push(item.hash().algorithm().to_wire());
    out.extend_from_slice(item.hash().digest());
    out.push(item.compression().to_wire());
}

fn encode_string(value: &str, out: &mut Vec<u8>) {
    let length = u32::try_from(value.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn encode_optional_string(value: Option<&str>, out: &mut Vec<u8>) {
    match value {
        Some(text) => {
            out.push(1);
            encode_string(text, out);
        }
        None => out.push(0),
    }
}

/// Reads a canonical manifest.
///
/// Every declared count and length is checked against a limit before it can
/// drive an allocation, and trailing bytes are an error so a manifest cannot
/// smuggle data past the parser.
///
/// # Errors
///
/// Returns the [`ManifestError`] for the first rule violated.
pub fn decode(bytes: &[u8]) -> Result<TransferManifest, ManifestError> {
    if bytes.len() > MAX_ENCODED_LEN {
        return Err(ManifestError::EncodedTooLarge {
            length: bytes.len(),
            limit: MAX_ENCODED_LEN,
        });
    }

    let mut reader = Reader::new(bytes);
    let magic = reader.take_array::<4>()?;
    if magic != MANIFEST_MAGIC {
        return Err(ManifestError::InvalidMagic { found: magic });
    }

    let version = u16::from_be_bytes(reader.take_array::<2>()?);
    if version != MANIFEST_VERSION {
        return Err(ManifestError::UnsupportedVersion {
            found: version,
            supported: MANIFEST_VERSION,
        });
    }

    let transfer_id = u64::from_be_bytes(reader.take_array::<8>()?);
    let created_unix_seconds = i64::from_be_bytes(reader.take_array::<8>()?);
    let total_bytes = u64::from_be_bytes(reader.take_array::<8>()?);
    if total_bytes > MAX_TOTAL_BYTES {
        return Err(ManifestError::TotalBytesTooLarge {
            declared: total_bytes,
            limit: MAX_TOTAL_BYTES,
        });
    }

    let declared_items = u32::from_be_bytes(reader.take_array::<4>()?) as usize;
    // Bound the count before it reserves anything. A declared 4 billion items
    // must cost nothing.
    if declared_items > MAX_ITEMS {
        return Err(ManifestError::TooManyItems {
            declared: declared_items as u64,
            limit: MAX_ITEMS,
        });
    }
    // Even within MAX_ITEMS, an item cannot be shorter than its fixed fields, so
    // refuse a count the remaining bytes cannot possibly satisfy before
    // reserving capacity for it.
    const MIN_ITEM_LEN: usize = 4 + 1 + 4 + 8 + 1 + 1 + 1 + 1;
    let remaining = reader.remaining();
    if declared_items.saturating_mul(MIN_ITEM_LEN) > remaining {
        return Err(ManifestError::Truncated {
            available: remaining,
            required: declared_items.saturating_mul(MIN_ITEM_LEN),
        });
    }

    let mut items = Vec::with_capacity(declared_items);
    for index in 0..declared_items {
        items.push(decode_item(&mut reader, index)?);
    }

    if !reader.is_empty() {
        return Err(ManifestError::TrailingBytes {
            count: reader.remaining(),
        });
    }

    if items.len() != declared_items {
        return Err(ManifestError::ItemCountMismatch {
            declared: declared_items,
            present: items.len(),
        });
    }

    TransferManifest::from_sorted(transfer_id, created_unix_seconds, items, total_bytes)
}

fn decode_item(reader: &mut Reader<'_>, index: usize) -> Result<ManifestItem, ManifestError> {
    let item_id = u32::from_be_bytes(reader.take_array::<4>()?);
    let kind = ItemKind::from_wire(reader.take_u8()?)?;

    let path_bytes = reader.take_length_prefixed(MAX_PATH_LEN, ManifestField::Path)?;
    let path = RelativePath::parse_bytes(path_bytes)
        .map_err(|source| ManifestError::InvalidPath { index, source })?;

    let size = u64::from_be_bytes(reader.take_array::<8>()?);

    let mime_type = match reader.take_option_tag()? {
        true => Some(reader.take_string(MAX_MIME_LEN, ManifestField::MimeType)?),
        false => None,
    };

    let modified_unix_seconds = match reader.take_option_tag()? {
        true => Some(i64::from_be_bytes(reader.take_array::<8>()?)),
        false => None,
    };

    let algorithm = HashAlgorithm::from_wire(reader.take_u8()?)?;
    let digest = reader.take_exact(algorithm.digest_len())?.to_vec();
    let hash = HashMetadata::new(algorithm, digest)?;
    let compression = Compression::from_wire(reader.take_u8()?)?;

    ManifestItem::new(
        item_id,
        path,
        kind,
        size,
        mime_type,
        modified_unix_seconds,
        hash,
        compression,
    )
    .map_err(|error| match error {
        ManifestError::InvalidDirectory { .. } => ManifestError::InvalidDirectory { index },
        ManifestError::MissingFileHash { .. } => ManifestError::MissingFileHash { index },
        other => other,
    })
}

/// A bounds-checked cursor. Every read reports how much it needed.
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    const fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    const fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    fn take_exact(&mut self, count: usize) -> Result<&'a [u8], ManifestError> {
        if self.remaining() < count {
            return Err(ManifestError::Truncated {
                available: self.remaining(),
                required: count,
            });
        }
        let slice = &self.bytes[self.offset..self.offset + count];
        self.offset += count;
        Ok(slice)
    }

    fn take_array<const N: usize>(&mut self) -> Result<[u8; N], ManifestError> {
        let slice = self.take_exact(N)?;
        let mut out = [0u8; N];
        out.copy_from_slice(slice);
        Ok(out)
    }

    fn take_u8(&mut self) -> Result<u8, ManifestError> {
        Ok(self.take_exact(1)?[0])
    }

    /// Reads a presence byte. Only `0` and `1` are canonical.
    fn take_option_tag(&mut self) -> Result<bool, ManifestError> {
        match self.take_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(ManifestError::InvalidFieldValue {
                field: ManifestField::OptionTag,
                value: other,
            }),
        }
    }

    /// Reads a `u32`-prefixed byte string, refusing an oversize declaration
    /// before it can be used to slice or allocate.
    fn take_length_prefixed(
        &mut self,
        limit: usize,
        field: ManifestField,
    ) -> Result<&'a [u8], ManifestError> {
        let length = u32::from_be_bytes(self.take_array::<4>()?) as usize;
        if length > limit {
            return Err(ManifestError::FieldTooLong {
                field,
                length,
                limit,
            });
        }
        self.take_exact(length)
    }

    fn take_string(&mut self, limit: usize, field: ManifestField) -> Result<String, ManifestError> {
        let bytes = self.take_length_prefixed(limit, field)?;
        core::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| ManifestError::InvalidUtf8 { field })
    }
}
