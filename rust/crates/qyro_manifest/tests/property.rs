//! Property tests for manifest encoding and path validation.
//!
//! Uses the same seeded generator approach as `qyro_protocol`; the `proptest`
//! evaluation is recorded in `TESTING.md`.

use qyro_manifest::{
    HashAlgorithm, HashMetadata, MAX_ITEMS, MAX_PATH_LEN, MAX_SEGMENT_LEN, MAX_TOTAL_BYTES,
    ManifestItem, RelativePath, TransferManifest, codec,
};

/// xorshift64*, deterministic and dependency-free.
struct Rng(u64);

impl Rng {
    const fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut state = self.0;
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        self.0 = state;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        usize::try_from(self.next_u64() % bound as u64).unwrap_or(0)
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| u8::try_from(self.next_u64() & 0xFF).unwrap_or(0))
            .collect()
    }
}

const SAFE_SEGMENTS: [&str; 12] = [
    "alpha",
    "beta",
    "gamma",
    "photos",
    "docs",
    "año",
    "日本",
    "file.txt",
    "a",
    "z-9",
    "with space",
    "🎉",
];

fn arbitrary_path(rng: &mut Rng) -> String {
    let depth = 1 + rng.below(4);
    (0..depth)
        .map(|index| {
            let base = SAFE_SEGMENTS[rng.below(SAFE_SEGMENTS.len())];
            // Keep segments unique enough that duplicates stay rare.
            format!("{base}{index}-{}", rng.below(10_000))
        })
        .collect::<Vec<String>>()
        .join("/")
}

fn arbitrary_manifest(rng: &mut Rng) -> TransferManifest {
    let count = rng.below(12);
    let mut items = Vec::new();
    for index in 0..count {
        let path = match RelativePath::parse(&arbitrary_path(rng)) {
            Ok(path) => path,
            Err(_) => continue,
        };
        let item_id = u32::try_from(index).unwrap_or(0);
        let item = if rng.below(4) == 0 {
            ManifestItem::directory(item_id, path).expect("valid directory")
        } else {
            // Every file needs a final digest now, so there is no hashless branch.
            let hash =
                HashMetadata::new(HashAlgorithm::Sha256, rng.bytes(32)).expect("valid digest");
            let size = rng.next_u64() % 1_000_000;
            ManifestItem::file(item_id, path, size, hash).expect("valid file")
        };
        items.push(item);
    }

    // Duplicate paths are possible but rare; drop the manifest and retry rather
    // than silently deduplicating, which would weaken the property.
    match TransferManifest::new(rng.next_u64(), 1_760_000_000, items) {
        Ok(manifest) => manifest,
        Err(_) => TransferManifest::new(1, 0, Vec::new()).expect("empty manifest is valid"),
    }
}

#[test]
fn decoding_what_was_encoded_preserves_the_manifest() {
    let mut rng = Rng::new(0x5159_524D_0001);
    for case in 0..2_000 {
        let manifest = arbitrary_manifest(&mut rng);
        let bytes = codec::encode(&manifest).expect("encodes");
        let decoded = codec::decode(&bytes)
            .unwrap_or_else(|error| panic!("case {case} failed to decode: {error}"));
        assert_eq!(decoded, manifest, "case {case} did not round-trip");
        // Canonical: re-encoding the decoded value reproduces the bytes.
        assert_eq!(codec::encode(&decoded).expect("encodes"), bytes);
    }
}

#[test]
fn a_valid_manifest_never_yields_an_unsafe_path() {
    let mut rng = Rng::new(0x5159_524D_0002);
    for _ in 0..2_000 {
        let manifest = arbitrary_manifest(&mut rng);
        let bytes = codec::encode(&manifest).expect("encodes");
        let decoded = codec::decode(&bytes).expect("decodes");

        for item in decoded.items() {
            let path = item.path().as_str();
            assert!(!path.is_empty());
            assert!(!path.starts_with('/'), "{path} escaped as absolute");
            assert!(!path.contains('\\'), "{path} carried a backslash");
            assert!(!path.contains('\0'), "{path} carried NUL");
            assert!(path.byte_len_ok(), "{path} exceeded the length limit");
            for segment in item.path().segments() {
                assert_ne!(segment, "..", "{path} traversed");
                assert_ne!(segment, ".", "{path} was not normalized");
                assert!(!segment.is_empty(), "{path} had an empty segment");
                assert!(segment.len() <= MAX_SEGMENT_LEN);
            }
        }
    }
}

/// Small helper so the assertion above reads as a property, not arithmetic.
trait PathLengthCheck {
    fn byte_len_ok(&self) -> bool;
}

impl PathLengthCheck for str {
    fn byte_len_ok(&self) -> bool {
        self.len() <= MAX_PATH_LEN
    }
}

#[test]
fn limits_hold_for_every_accepted_manifest() {
    let mut rng = Rng::new(0x5159_524D_0003);
    for _ in 0..1_000 {
        let manifest = arbitrary_manifest(&mut rng);
        assert!(manifest.item_count() <= MAX_ITEMS);
        assert!(manifest.total_bytes() <= MAX_TOTAL_BYTES);

        let summed: u64 = manifest
            .items()
            .iter()
            .map(qyro_manifest::ManifestItem::size)
            .sum();
        assert_eq!(
            summed,
            manifest.total_bytes(),
            "declared total must equal the sum of item sizes"
        );
    }
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut rng = Rng::new(0x5159_524D_0004);
    let template = codec::encode(
        &TransferManifest::new(
            1,
            0,
            vec![
                ManifestItem::file(
                    1,
                    RelativePath::parse("seed.bin").expect("valid"),
                    10,
                    HashMetadata::new(HashAlgorithm::Sha256, vec![0x5A; 32]).expect("valid"),
                )
                .expect("valid"),
            ],
        )
        .expect("valid"),
    )
    .expect("encodes");

    for _ in 0..5_000 {
        let mut input = if rng.below(2) == 0 {
            {
                let noise_len = rng.below(256);
                rng.bytes(noise_len)
            }
        } else {
            // Start from a real manifest and corrupt it, so the generator gets
            // past the magic and exercises the deeper validation.
            let mut copy = template.clone();
            let flips = 1 + rng.below(4);
            for _ in 0..flips {
                let index = rng.below(copy.len());
                copy[index] ^= u8::try_from(1 + rng.below(255)).unwrap_or(1);
            }
            copy
        };
        if rng.below(4) == 0 {
            input.truncate(rng.below(input.len().max(1)));
        }

        // The only contract is: no panic, and anything accepted is safe.
        // Traversal is a whole-segment property: "notes..txt" is a legal name,
        // so a substring check would reject safe paths and hide real ones.
        if let Ok(manifest) = codec::decode(&input) {
            for item in manifest.items() {
                assert!(!item.path().as_str().starts_with('/'));
                for segment in item.path().segments() {
                    assert_ne!(segment, "..");
                    assert_ne!(segment, ".");
                }
            }
        }
    }
}

#[test]
fn arbitrary_strings_never_panic_the_path_parser() {
    let mut rng = Rng::new(0x5159_524D_0005);
    let alphabet: [char; 16] = [
        'a', 'b', '/', '.', '\\', ':', ' ', '\0', 'C', 'O', 'N', '1', 'ñ', '🎉', '\n', '-',
    ];
    for _ in 0..10_000 {
        let length = rng.below(24);
        let candidate: String = (0..length)
            .map(|_| alphabet[rng.below(alphabet.len())])
            .collect();

        if let Ok(path) = RelativePath::parse(&candidate) {
            // Whatever survives must satisfy every safety invariant.
            assert!(!path.as_str().is_empty());
            assert!(!path.as_str().starts_with('/'));
            assert!(!path.as_str().contains('\\'));
            assert!(!path.as_str().contains('\0'));
            assert_eq!(path.as_str(), candidate, "parsing must not rewrite");
            for segment in path.segments() {
                assert!(!segment.is_empty());
                assert_ne!(segment, "..");
                assert_ne!(segment, ".");
                assert!(!segment.ends_with('.'));
                assert!(!segment.ends_with(' '));
            }
        }
    }
}
