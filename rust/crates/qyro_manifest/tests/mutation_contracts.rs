//! Focal contracts for peer-controlled manifest limits found by mutation.

use qyro_manifest::{
    HashAlgorithm, HashMetadata, MANIFEST_MAGIC, MANIFEST_VERSION, MAX_ENCODED_LEN, MAX_ITEMS,
    MAX_PATH_SEGMENTS, MAX_TOTAL_BYTES, ManifestError, ManifestItem, RelativePath,
    TransferManifest, codec,
};

const WIRE_HEADER_LEN: usize = 4 + 2 + 8 + 8 + 8 + 4;
const MIN_ITEM_LEN: usize = 4 + 1 + 4 + 8 + 1 + 1 + 1 + 1;

fn wire_header(total_bytes: u64, item_count: u32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(WIRE_HEADER_LEN);
    bytes.extend_from_slice(&MANIFEST_MAGIC);
    bytes.extend_from_slice(&MANIFEST_VERSION.to_be_bytes());
    bytes.extend_from_slice(&1u64.to_be_bytes());
    bytes.extend_from_slice(&2i64.to_be_bytes());
    bytes.extend_from_slice(&total_bytes.to_be_bytes());
    bytes.extend_from_slice(&item_count.to_be_bytes());
    assert_eq!(bytes.len(), WIRE_HEADER_LEN);
    bytes
}

fn hash() -> HashMetadata {
    HashMetadata::new(HashAlgorithm::Sha256, vec![0x5A; 32]).expect("valid SHA-256 fixture")
}

fn file(item_id: u32, path: &str, size: u64) -> ManifestItem {
    ManifestItem::file(
        item_id,
        RelativePath::parse(path).expect("valid path fixture"),
        size,
        hash(),
    )
    .expect("valid file fixture")
}

#[test]
fn encoded_size_distinguishes_the_exact_limit_from_one_byte_more() {
    assert!(matches!(
        codec::decode(&vec![0; MAX_ENCODED_LEN]),
        Err(ManifestError::InvalidMagic { .. })
    ));
    assert_eq!(
        codec::decode(&vec![0; MAX_ENCODED_LEN + 1]),
        Err(ManifestError::EncodedTooLarge {
            length: MAX_ENCODED_LEN + 1,
            limit: MAX_ENCODED_LEN,
        })
    );
}

#[test]
fn declared_total_distinguishes_the_exact_limit_from_one_byte_more() {
    assert_eq!(
        codec::decode(&wire_header(MAX_TOTAL_BYTES, 0)),
        Err(ManifestError::TotalBytesMismatch {
            declared: MAX_TOTAL_BYTES,
            computed: Some(0),
        })
    );
    assert_eq!(
        codec::decode(&wire_header(MAX_TOTAL_BYTES + 1, 0)),
        Err(ManifestError::TotalBytesTooLarge {
            declared: MAX_TOTAL_BYTES + 1,
            limit: MAX_TOTAL_BYTES,
        })
    );
}

#[test]
fn declared_item_count_distinguishes_the_exact_limit_from_one_more() {
    assert_eq!(
        codec::decode(&wire_header(0, MAX_ITEMS as u32)),
        Err(ManifestError::Truncated {
            available: 0,
            required: MAX_ITEMS * MIN_ITEM_LEN,
        })
    );
    assert_eq!(
        codec::decode(&wire_header(0, MAX_ITEMS as u32 + 1)),
        Err(ManifestError::TooManyItems {
            declared: MAX_ITEMS as u64 + 1,
            limit: MAX_ITEMS,
        })
    );
}

#[test]
fn minimum_item_length_is_the_sum_of_every_fixed_field() {
    let mut bytes = wire_header(0, 1);
    bytes.resize(WIRE_HEADER_LEN + MIN_ITEM_LEN - 1, 0);

    assert_eq!(
        codec::decode(&bytes),
        Err(ManifestError::Truncated {
            available: MIN_ITEM_LEN - 1,
            required: MIN_ITEM_LEN,
        })
    );
}

#[test]
fn manifest_item_count_distinguishes_the_exact_limit_from_one_more() {
    let repeated =
        ManifestItem::directory(1, RelativePath::parse("same").expect("valid path fixture"))
            .expect("valid directory fixture");

    assert_eq!(
        TransferManifest::from_sorted(1, 2, vec![repeated.clone(); MAX_ITEMS], 0),
        Err(ManifestError::DuplicatePath { index: 1 })
    );
    assert_eq!(
        TransferManifest::from_sorted(1, 2, vec![repeated; MAX_ITEMS + 1], 0),
        Err(ManifestError::TooManyItems {
            declared: MAX_ITEMS as u64 + 1,
            limit: MAX_ITEMS,
        })
    );
}

#[test]
fn total_content_size_distinguishes_the_exact_limit_from_one_more() {
    let at_limit = TransferManifest::new(1, 2, vec![file(1, "maximum", MAX_TOTAL_BYTES)])
        .expect("the exact content limit is valid");
    assert_eq!(at_limit.total_bytes(), MAX_TOTAL_BYTES);

    assert_eq!(
        TransferManifest::new(1, 2, vec![file(1, "over", MAX_TOTAL_BYTES + 1)]),
        Err(ManifestError::TotalBytesTooLarge {
            declared: MAX_TOTAL_BYTES + 1,
            limit: MAX_TOTAL_BYTES,
        })
    );
}

#[test]
fn portable_collision_reports_the_later_item_and_its_predecessor() {
    assert_eq!(
        TransferManifest::new(1, 2, vec![file(1, "A", 1), file(2, "a", 1)]),
        Err(ManifestError::PortableCollision {
            index: 1,
            collides_with: 0,
        })
    );
}

#[test]
fn path_segments_distinguish_the_exact_limit_from_one_more() {
    let exact = vec!["a"; MAX_PATH_SEGMENTS].join("/");
    RelativePath::parse(&exact).expect("the exact segment limit is valid");

    let over = vec!["a"; MAX_PATH_SEGMENTS + 1].join("/");
    assert_eq!(
        RelativePath::parse(&over),
        Err(qyro_manifest::PathError::TooManySegments {
            count: MAX_PATH_SEGMENTS + 1,
            limit: MAX_PATH_SEGMENTS,
        })
    );
}
