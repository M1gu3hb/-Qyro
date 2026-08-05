//! Contracts for portable collision detection.
//!
//! The first sprint-4A audit finding: the hand-written folding table stripped
//! diacritics, so `ano.txt` and `año.txt` folded to the same key and a manifest
//! carrying both legitimate files was rejected as a collision. Over-folding is
//! as much a defect as under-folding — it makes valid transfers impossible.

use qyro_manifest::{
    HashAlgorithm, HashMetadata, ManifestError, ManifestItem, PortableCollisionKey, RelativePath,
    TransferManifest,
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

fn key(path: &str) -> PortableCollisionKey {
    PortableCollisionKey::of(&RelativePath::parse(path).expect("valid path"))
}

fn both_accepted(left: &str, right: &str) {
    let result = TransferManifest::new(1, 0, vec![file(1, left), file(2, right)]);
    assert!(
        result.is_ok(),
        "{left:?} and {right:?} are different files and must both be accepted, got {result:?}"
    );
    assert_ne!(key(left), key(right), "{left:?} and {right:?} must differ");
}

fn must_collide(left: &str, right: &str) {
    assert_eq!(
        key(left),
        key(right),
        "{left:?} and {right:?} must share a portable key"
    );
    let result = TransferManifest::new(1, 0, vec![file(1, left), file(2, right)]);
    assert!(
        matches!(result, Err(ManifestError::PortableCollision { .. })),
        "{left:?} and {right:?} would be one file on a real filesystem, got {result:?}"
    );
}

// --------------------------------------------- diacritics must be preserved

#[test]
fn an_accent_is_a_different_file_not_a_collision() {
    both_accepted("ano.txt", "año.txt");
}

#[test]
fn accented_words_stay_distinct_from_their_unaccented_spellings() {
    both_accepted("resume.pdf", "résumé.pdf");
    both_accepted("pinguino.jpg", "pingüino.jpg");
    both_accepted("cote.txt", "côte.txt");
}

#[test]
fn diacritics_outside_latin1_are_preserved_too() {
    // Czech, Polish, Turkish, Vietnamese and Greek all carry marks the old
    // Latin-1 table never covered, so they were silently left alone while
    // Spanish and French names were mangled.
    both_accepted("rada.txt", "řada.txt");
    both_accepted("zdzblo.txt", "źdźbło.txt");
    both_accepted("is.txt", "iş.txt");
    both_accepted("viet.txt", "việt.txt");
    both_accepted("sigma.txt", "σίγμα.txt");
}

#[test]
fn different_scripts_never_collide() {
    both_accepted("日本.txt", "中国.txt");
    both_accepted("alpha.txt", "αlpha.txt");
}

// ------------------------------------------------- real collisions must hold

#[test]
fn case_only_differences_collide() {
    must_collide("Foto.jpg", "foto.jpg");
    must_collide("README.md", "readme.md");
}

#[test]
fn case_folding_applies_per_segment() {
    must_collide("A/B.txt", "a/b.TXT");
}

#[test]
fn composed_and_decomposed_spellings_collide() {
    // Same word, two byte sequences: precomposed U+00F1 versus n + U+0303.
    let composed = "ma\u{00F1}ana.txt";
    let decomposed = "man\u{0303}ana.txt";
    assert_ne!(composed, decomposed, "fixtures must differ in bytes");
    must_collide(composed, decomposed);
}

#[test]
fn composed_and_decomposed_collide_for_accented_latin() {
    let composed = "caf\u{00E9}.txt";
    let decomposed = "cafe\u{0301}.txt";
    assert_ne!(composed, decomposed, "fixtures must differ in bytes");
    must_collide(composed, decomposed);
}

#[test]
fn composed_and_decomposed_collide_outside_latin1() {
    // Czech ř: U+0159 versus r + U+030C.
    let composed = "\u{0159}ada.txt";
    let decomposed = "r\u{030C}ada.txt";
    assert_ne!(composed, decomposed, "fixtures must differ in bytes");
    must_collide(composed, decomposed);
}

#[test]
fn case_and_normalization_combine() {
    must_collide("Ma\u{00F1}ana.txt", "man\u{0303}ana.txt");
}

// ----------------------------------------------------------- key properties

#[test]
fn the_key_never_alters_the_stored_path() {
    let original = "Año/Café.TXT";
    let path = RelativePath::parse(original).expect("valid");
    let _ = PortableCollisionKey::of(&path);
    assert_eq!(
        path.as_str(),
        original,
        "deriving a key must not rewrite the path"
    );
}

#[test]
fn the_key_is_deterministic() {
    let first = key("Año/Café.txt");
    let second = key("Año/Café.txt");
    assert_eq!(first, second);
}

#[test]
fn segments_stay_separated_in_the_key() {
    // "a/bc" and "ab/c" must not fold into the same flat string.
    both_accepted("a/bc.txt", "ab/c.txt");
}

// ---------------------------------------------------- traversal by segment

#[test]
fn consecutive_dots_inside_a_name_are_legal() {
    // Only a whole segment of "." or ".." is traversal. A substring check would
    // reject these perfectly ordinary names.
    for candidate in [
        "notes..txt",
        "archive...tar",
        "version..1",
        "a/b..c/d.txt",
        "..hidden",
        "trailing..name",
    ] {
        assert!(
            RelativePath::parse(candidate).is_ok(),
            "{candidate:?} is a legal name and must be accepted"
        );
    }
}

#[test]
fn only_whole_dot_segments_are_traversal() {
    for candidate in ["..", ".", "a/../b", "a/./b", "../x", "x/.."] {
        assert!(
            RelativePath::parse(candidate).is_err(),
            "{candidate:?} must be rejected"
        );
    }
}
