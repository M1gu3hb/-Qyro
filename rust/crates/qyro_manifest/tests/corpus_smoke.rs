//! Replays the committed fuzz corpus on stable Rust.
//!
//! See the note in `qyro_protocol`'s corpus smoke test: this is a regression
//! guard over known-interesting inputs, not a fuzzing campaign.

use std::fs;
use std::path::PathBuf;

use qyro_manifest::{
    MAX_ITEMS, MAX_PATH_LEN, MAX_SEGMENT_LEN, MAX_TOTAL_BYTES, RelativePath, codec,
};

fn corpus_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fuzz/corpus")
        .join(name)
        .canonicalize()
        .unwrap_or_else(|_| panic!("{name} corpus is committed"))
}

fn corpus_files(name: &str) -> Vec<(PathBuf, Vec<u8>)> {
    fs::read_dir(corpus_dir(name))
        .expect("corpus directory is readable")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if !path.is_file() {
                return None;
            }
            let data = fs::read(&path).ok()?;
            Some((path, data))
        })
        .collect()
}

#[test]
fn manifest_corpus_never_yields_an_unsafe_manifest() {
    let files = corpus_files("manifest_decoder");
    assert!(files.len() >= 15, "expected the committed manifest corpus");

    for (path, data) in files {
        let Ok(manifest) = codec::decode(&data) else {
            continue;
        };

        assert!(manifest.item_count() <= MAX_ITEMS, "{}", path.display());
        assert!(
            manifest.total_bytes() <= MAX_TOTAL_BYTES,
            "{}",
            path.display()
        );

        for item in manifest.items() {
            let text = item.path().as_str();
            assert!(!text.is_empty(), "{}", path.display());
            assert!(
                !text.starts_with('/'),
                "{} accepted an absolute path",
                path.display()
            );
            assert!(
                !text.contains('\\'),
                "{} accepted a backslash",
                path.display()
            );
            assert!(!text.contains('\0'), "{} accepted NUL", path.display());
            for segment in item.path().segments() {
                assert_ne!(segment, "..", "{} accepted traversal", path.display());
                assert_ne!(segment, ".", "{} accepted a dot segment", path.display());
            }
        }

        // Whatever decoded must re-encode to exactly the bytes it came from.
        let reencoded = codec::encode(&manifest).expect("a decoded manifest re-encodes");
        assert_eq!(
            reencoded,
            data,
            "{} did not round-trip canonically",
            path.display()
        );
    }
}

#[test]
fn path_corpus_is_parsed_without_panicking_or_rewriting() {
    let files = corpus_files("relative_path");
    assert!(files.len() >= 15, "expected the committed path corpus");

    for (path, data) in files {
        let Ok(parsed) = RelativePath::parse_bytes(&data) else {
            continue;
        };

        assert_eq!(
            parsed.as_str().as_bytes(),
            data.as_slice(),
            "{} was rewritten instead of rejected",
            path.display()
        );
        assert!(parsed.as_str().len() <= MAX_PATH_LEN);
        assert!(!parsed.as_str().starts_with('/'), "{}", path.display());
        assert!(!parsed.as_str().contains('\\'), "{}", path.display());
        assert!(!parsed.as_str().contains('\0'), "{}", path.display());
        for segment in parsed.segments() {
            assert!(!segment.is_empty(), "{}", path.display());
            assert!(segment.len() <= MAX_SEGMENT_LEN, "{}", path.display());
            assert_ne!(segment, "..", "{}", path.display());
            assert_ne!(segment, ".", "{}", path.display());
            assert!(!segment.ends_with('.'), "{}", path.display());
            assert!(!segment.ends_with(' '), "{}", path.display());
        }
    }
}
