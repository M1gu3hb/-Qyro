//! An item that is a file and also the parent directory of another item.
//!
//! QYR-0028. [`PortableCollisionKey`] folds a whole path into one string whose
//! segments are joined by NUL, and `validate_items` compared those strings for
//! *equality*. Equality catches `a/b` against `A/B`. It cannot see that `a` and
//! `a/b` are the same name at different depths, because `"a"` and `"a\0b"` are
//! simply two different strings.
//!
//! A receiver materialising such a manifest has to create `a` as a file and `a`
//! as a directory. Whichever it does second either fails, or replaces the first
//! — after the transfer was accepted, with no way to report which entry was
//! lost.
//!
//! The rule is a prefix rule, not an equality rule: after canonical ordering, a
//! key that is a NUL-delimited prefix of the next key is that key's ancestor.
//! See `docs/adr/ADR-0017-manifest-serialization.md`.

mod common;

use common::{RawItem, RawManifest};
use qyro_manifest::{
    HashAlgorithm, HashMetadata, ManifestError, ManifestItem, RelativePath, TransferManifest, codec,
};

fn digest(item_id: u32) -> HashMetadata {
    HashMetadata::new(
        HashAlgorithm::Sha256,
        vec![u8::try_from(item_id % 251).unwrap_or(0); 32],
    )
    .expect("valid digest")
}

fn file(item_id: u32, path: &str) -> ManifestItem {
    ManifestItem::file(
        item_id,
        RelativePath::parse(path).expect("valid fixture path"),
        1,
        digest(item_id),
    )
    .expect("valid fixture item")
}

fn directory(item_id: u32, path: &str) -> ManifestItem {
    ManifestItem::directory(
        item_id,
        RelativePath::parse(path).expect("valid fixture path"),
    )
    .expect("valid fixture item")
}

fn build(items: Vec<ManifestItem>) -> Result<TransferManifest, ManifestError> {
    TransferManifest::new(1, 0, items)
}

#[test]
fn a_file_cannot_also_be_a_directory() {
    let result = build(vec![file(1, "a"), file(2, "a/b")]);
    assert!(
        matches!(result, Err(ManifestError::FileIsAlsoADirectory { .. })),
        "`a` is a file and the parent of `a/b`; one of them cannot be written, \
         got {result:?}"
    );

    // The same pair after case folding: a Windows or macOS receiver resolves
    // `A/b` inside the file it just wrote as `a`.
    let folded = build(vec![file(1, "a"), file(2, "A/b")]);
    assert!(
        matches!(folded, Err(ManifestError::FileIsAlsoADirectory { .. })),
        "`a` and `A/b` collide on a case-insensitive filesystem, got {folded:?}"
    );
}

#[test]
fn the_collision_is_found_at_any_depth() {
    for (ancestor, descendant) in [
        ("a/b", "a/b/c"),
        ("docs/report", "docs/report/page1.txt"),
        ("x/y/z", "x/y/z/w/v"),
        // Composition and case at the same time.
        ("a\u{00F1}o", "A\u{006E}\u{0303}O/inner.txt"),
    ] {
        let result = build(vec![file(1, ancestor), file(2, descendant)]);
        assert!(
            matches!(result, Err(ManifestError::FileIsAlsoADirectory { .. })),
            "{ancestor:?} is a file and an ancestor of {descendant:?}, got {result:?}"
        );
    }
}

#[test]
fn a_directory_may_still_contain_files() {
    // The rule is about a *file* that is also a directory. Rejecting a directory
    // item that has children would refuse every ordinary tree, which is the
    // over-rejection failure mode this crate has shipped once already.
    let result = build(vec![
        directory(1, "a"),
        file(2, "a/b"),
        directory(3, "a/sub"),
        file(4, "a/sub/c"),
    ]);
    assert!(
        result.is_ok(),
        "a directory and the files inside it are the normal case, got {result:?}"
    );
}

#[test]
fn a_shared_prefix_that_is_not_a_path_boundary_is_not_a_collision() {
    // `a` is a prefix of `ab` as a string, and of nothing as a path. Folding on
    // raw string prefixes instead of NUL-delimited ones would refuse these.
    for (left, right) in [
        ("a", "ab"),
        ("a", "ab/c"),
        ("report", "reports/2026.txt"),
        ("x/y", "x/yz"),
    ] {
        let result = build(vec![file(1, left), file(2, right)]);
        assert!(
            result.is_ok(),
            "{left:?} and {right:?} are unrelated paths and must both be \
             accepted, got {result:?}"
        );
    }
}

#[test]
fn the_decoder_refuses_an_ancestor_collision_too() {
    // Canonical order by raw path, which is what the decoder requires: `a` then
    // `a/b`.
    let bytes =
        RawManifest::new(vec![RawItem::file(1, "a", 1), RawItem::file(2, "a/b", 1)]).encode();

    assert!(
        matches!(
            codec::decode(&bytes),
            Err(ManifestError::FileIsAlsoADirectory { .. })
        ),
        "the decoder must refuse what the constructor refuses"
    );
}
