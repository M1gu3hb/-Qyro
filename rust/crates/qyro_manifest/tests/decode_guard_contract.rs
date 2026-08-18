//! The decode-path checks that survived their own deletion.
//!
//! QYR-0032. Four checks on the path a peer's bytes actually take had no test
//! that failed when they were removed. Each one could be deleted and
//! `cargo test --workspace` stayed green, which means the guarantee they carry
//! was documented but not held.
//!
//! Every manifest here is written byte by byte. That is not stylistic: the
//! construction API enforces the same invariants, so a manifest built through
//! `TransferManifest::new` is rejected before it can be encoded and never
//! reaches the decoder's copy of the check. Bytes are the only way in, and
//! bytes are what a peer sends.
//!
//! | Check | Where |
//! |---|---|
//! | declared total must equal the summed sizes | `model.rs`, `from_sorted` |
//! | items must be in canonical order | `model.rs`, `validate_items` |
//! | a length prefix is bounded before it slices | `codec.rs`, `take_length_prefixed` |
//! | summing sizes uses checked arithmetic | `model.rs`, `validate_items` |

mod common;

use common::{RawItem, RawManifest};
use qyro_manifest::{MAX_MIME_LEN, ManifestError, ManifestField, codec};

#[test]
fn a_declared_total_that_does_not_match_the_items_is_rejected() {
    // The header says 999 bytes are coming; the items account for 2. A receiver
    // that trusted the header would size a progress bar, a disk-space check or
    // a quota against a number no item supports.
    let bytes = RawManifest::new(vec![
        RawItem::file(1, "a.bin", 1),
        RawItem::file(2, "b.bin", 1),
    ])
    .with_total_bytes(999)
    .encode();

    assert_eq!(
        codec::decode(&bytes),
        Err(ManifestError::TotalBytesMismatch {
            declared: 999,
            computed: Some(2),
        }),
        "the declared total must be the summed total"
    );
}

#[test]
fn items_in_descending_order_are_rejected() {
    // Canonical order is what makes one logical manifest have exactly one byte
    // representation, which is what lets the encoded form be authenticated
    // without a normalization pass. The decoder rejects rather than reorders:
    // reordering would change the bytes that were signed.
    let bytes = RawManifest::new(vec![
        RawItem::file(1, "b.bin", 1),
        RawItem::file(2, "a.bin", 1),
    ])
    .encode();

    assert_eq!(
        codec::decode(&bytes),
        Err(ManifestError::UnsortedItems { index: 1 }),
        "items out of canonical order must be refused, not sorted"
    );
}

#[test]
fn an_oversize_length_prefix_is_refused_before_it_can_slice() {
    // The declaration is 200 bytes of MIME type; 35 bytes of item follow. The
    // point is the *order*: the limit is applied to the declared length before
    // that length is used to slice or reserve. A decoder that sliced first and
    // checked afterwards would answer `Truncated`, having already let a
    // peer-chosen number reach an allocation.
    let bytes = RawManifest::new(vec![
        RawItem::file(1, "a.bin", 1).with_declared_mime(200, b""),
    ])
    .encode();

    assert_eq!(
        codec::decode(&bytes),
        Err(ManifestError::FieldTooLong {
            field: ManifestField::MimeType,
            length: 200,
            limit: MAX_MIME_LEN,
        }),
        "the length limit must be applied to the declaration, not to what arrived"
    );
}

#[test]
fn a_hostile_length_prefix_costs_nothing() {
    // Same check, at the size that makes it matter: four billion bytes declared
    // and none supplied.
    let bytes = RawManifest::new(vec![
        RawItem::file(1, "a.bin", 1).with_declared_mime(u32::MAX, b""),
    ])
    .encode();

    assert_eq!(
        codec::decode(&bytes),
        Err(ManifestError::FieldTooLong {
            field: ManifestField::MimeType,
            length: u32::MAX as usize,
            limit: MAX_MIME_LEN,
        }),
        "a four-gigabyte declaration must be an error, not an allocation"
    );
}

#[test]
fn two_sizes_that_wrap_the_total_are_rejected() {
    // 1 and u64::MAX sum to exactly `u64::MAX + 1`, so a plain `+` wraps to
    // zero. The header then declares zero, the sum is zero, and the manifest
    // would be accepted as an empty transfer that is about to write two files.
    //
    // Neither the running total nor the declared total ever exceeds
    // `MAX_TOTAL_BYTES`, so no size limit fires: the checked addition is the
    // only thing standing here. The second size is above the limit on its own,
    // and it has to be — the running total is tested *after* the addition, so
    // the overflow is reached first, which is precisely why this is the control
    // under test and not the limit.
    let bytes = RawManifest::new(vec![
        RawItem::file(1, "a.bin", 1),
        RawItem::file(2, "b.bin", u64::MAX),
    ])
    .with_total_bytes(0)
    .encode();

    assert_eq!(
        codec::decode(&bytes),
        Err(ManifestError::TotalBytesMismatch {
            declared: 0,
            computed: None,
        }),
        "an overflowing sum must be an error, not a small believable total"
    );
}

#[test]
fn a_well_formed_manifest_still_decodes() {
    // Four rejections are only meaningful beside an acceptance. Without this,
    // a decoder that refused everything would pass every test above.
    let bytes = RawManifest::new(vec![
        RawItem::directory(1, "docs"),
        RawItem::file(2, "docs/a.bin", 10).with_declared_mime(10, b"text/plain"),
        RawItem::file(3, "docs/b.bin", 32),
    ])
    .encode();

    let manifest = codec::decode(&bytes).expect("a canonical manifest decodes");
    assert_eq!(manifest.item_count(), 3);
    assert_eq!(manifest.total_bytes(), 42);
    assert_eq!(manifest.items()[1].mime_type(), Some("text/plain"));
}
