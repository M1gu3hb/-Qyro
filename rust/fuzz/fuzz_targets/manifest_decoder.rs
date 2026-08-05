#![no_main]
//! Feeds arbitrary bytes through the manifest decoder.
//!
//! Anything the decoder accepts must carry only safe relative paths.

use libfuzzer_sys::fuzz_target;
use qyro_manifest::{MAX_ITEMS, MAX_TOTAL_BYTES, codec};

fuzz_target!(|data: &[u8]| {
    let Ok(manifest) = codec::decode(data) else {
        return;
    };

    assert!(manifest.item_count() <= MAX_ITEMS);
    assert!(manifest.total_bytes() <= MAX_TOTAL_BYTES);

    for item in manifest.items() {
        let path = item.path().as_str();
        assert!(!path.is_empty());
        assert!(!path.starts_with('/'));
        assert!(!path.contains('\\'));
        assert!(!path.contains('\0'));
        // Traversal is a whole-segment property; "notes..txt" is a legal name.
        for segment in item.path().segments() {
            assert!(!segment.is_empty());
            assert_ne!(segment, "..");
            assert_ne!(segment, ".");
        }
    }

    // A manifest that decoded must re-encode to the same bytes it came from.
    let reencoded = codec::encode(&manifest).expect("a decoded manifest re-encodes");
    assert_eq!(reencoded.as_slice(), data);
});
