#![no_main]
//! Feeds arbitrary bytes through the relative path parser.
//!
//! The parser must never panic, never rewrite its input, and never accept a
//! path that could escape the destination directory.

use libfuzzer_sys::fuzz_target;
use qyro_manifest::{MAX_PATH_LEN, MAX_SEGMENT_LEN, RelativePath};

fuzz_target!(|data: &[u8]| {
    let Ok(path) = RelativePath::parse_bytes(data) else {
        return;
    };

    let text = path.as_str();
    assert_eq!(text.as_bytes(), data, "parsing must not rewrite the input");
    assert!(!text.is_empty());
    assert!(text.len() <= MAX_PATH_LEN);
    assert!(!text.starts_with('/'));
    assert!(!text.contains('\\'));
    assert!(!text.contains('\0'));

    for segment in path.segments() {
        assert!(!segment.is_empty());
        assert!(segment.len() <= MAX_SEGMENT_LEN);
        assert_ne!(segment, "..");
        assert_ne!(segment, ".");
        assert!(!segment.ends_with('.'));
        assert!(!segment.ends_with(' '));
    }
});
