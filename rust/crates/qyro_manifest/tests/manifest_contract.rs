//! Security and canonicity contracts for transfer manifests.

use qyro_manifest::{
    Compression, HashAlgorithm, HashMetadata, ItemKind, MANIFEST_MAGIC, MAX_ITEMS, MAX_PATH_LEN,
    MAX_SEGMENT_LEN, ManifestError, ManifestField, ManifestItem, PathError, RelativePath,
    TransferManifest, codec,
};

/// Every file needs a final digest, so fixtures carry a deterministic one.
fn digest_for(item_id: u32) -> HashMetadata {
    HashMetadata::new(
        HashAlgorithm::Sha256,
        vec![u8::try_from(item_id % 251).unwrap_or(0); 32],
    )
    .expect("valid fixture digest")
}

fn file(item_id: u32, path: &str, size: u64) -> ManifestItem {
    ManifestItem::file(
        item_id,
        RelativePath::parse(path).expect("valid fixture path"),
        size,
        digest_for(item_id),
    )
    .expect("valid fixture item")
}

fn manifest(items: Vec<ManifestItem>) -> TransferManifest {
    TransferManifest::new(42, 1_760_000_000, items).expect("valid fixture manifest")
}

// ---------------------------------------------------------------- valid paths

#[test]
fn ordinary_paths_are_accepted() {
    for candidate in [
        "file.txt",
        "photos/beach.jpg",
        "a/b/c/d/e/deep.bin",
        "documento con espacios.pdf",
        "año/mañana.txt",
        "日本語/ファイル.txt",
        "emoji/🎉-party.png",
        "dotfile/.gitignore",
        "trailing.dots.in.name.tar.gz",
        "CONsole.txt",
        "COM.txt",
        "COM10.txt",
    ] {
        let parsed = RelativePath::parse(candidate)
            .unwrap_or_else(|error| panic!("{candidate:?} should be valid, got {error:?}"));
        assert_eq!(parsed.as_str(), candidate);
    }
}

#[test]
fn a_maximum_length_segment_is_accepted() {
    let segment = "n".repeat(MAX_SEGMENT_LEN);
    let parsed = RelativePath::parse(&segment).expect("exactly at the limit");
    assert_eq!(parsed.byte_len(), MAX_SEGMENT_LEN);

    let oversize = "n".repeat(MAX_SEGMENT_LEN + 1);
    assert!(matches!(
        RelativePath::parse(&oversize),
        Err(PathError::SegmentTooLong { .. })
    ));
}

#[test]
fn file_name_returns_the_final_segment() {
    let path = RelativePath::parse("a/b/c.txt").expect("valid");
    assert_eq!(path.file_name(), "c.txt");
    assert_eq!(path.segment_count(), 3);
}

// -------------------------------------------------------------- hostile paths

#[test]
fn traversal_is_rejected() {
    for candidate in [
        "../evil",
        "../../etc/passwd",
        "a/../../../etc/passwd",
        "a/b/..",
        "..",
    ] {
        assert_eq!(
            RelativePath::parse(candidate),
            Err(PathError::ParentSegment),
            "{candidate:?} must be rejected"
        );
    }
}

#[test]
fn current_directory_segments_are_rejected() {
    // Two spellings of one location would break canonical ordering.
    for candidate in [".", "./file", "a/./b"] {
        assert_eq!(
            RelativePath::parse(candidate),
            Err(PathError::CurrentSegment),
            "{candidate:?} must be rejected"
        );
    }
}

#[test]
fn unix_absolute_paths_are_rejected() {
    for candidate in ["/etc/passwd", "/", "/tmp/evil"] {
        assert_eq!(
            RelativePath::parse(candidate),
            Err(PathError::AbsoluteUnix),
            "{candidate:?} must be rejected"
        );
    }
}

#[test]
fn windows_drive_prefixes_are_rejected() {
    for candidate in ["C:/Windows", "c:/windows/system32", "Z:/data"] {
        assert_eq!(
            RelativePath::parse(candidate),
            Err(PathError::DrivePrefix),
            "{candidate:?} must be rejected"
        );
    }
}

#[test]
fn backslash_paths_are_rejected_as_ambiguous() {
    // A backslash separates on Windows and is a legal name character on Unix,
    // so the same manifest would describe two different trees.
    for candidate in [r"C:\Windows", r"a\b", r"..\..\evil", r"\\server\share"] {
        assert_eq!(
            RelativePath::parse(candidate),
            Err(PathError::AmbiguousSeparator),
            "{candidate:?} must be rejected"
        );
    }
}

#[test]
fn unc_style_double_slash_is_rejected() {
    assert_eq!(
        RelativePath::parse("//server/share"),
        Err(PathError::UncPrefix)
    );
}

#[test]
fn nul_bytes_are_rejected() {
    assert_eq!(RelativePath::parse("evil\0.txt"), Err(PathError::NulByte));
    assert_eq!(
        RelativePath::parse("a/b\0c/d"),
        Err(PathError::NulByte),
        "a NUL anywhere truncates the path in C-style APIs"
    );
}

#[test]
fn control_characters_are_rejected() {
    assert_eq!(
        RelativePath::parse("bell\u{7}.txt"),
        Err(PathError::ControlCharacter)
    );
    assert_eq!(
        RelativePath::parse("newline\n.txt"),
        Err(PathError::ControlCharacter)
    );
}

#[test]
fn windows_reserved_device_names_are_rejected() {
    for candidate in [
        "CON",
        "NUL",
        "COM1",
        "LPT1",
        "PRN",
        "AUX",
        "con",
        "Nul",
        "CON.txt",
        "com1.log",
        "sub/CON",
        "sub/NUL.dat",
    ] {
        assert_eq!(
            RelativePath::parse(candidate),
            Err(PathError::ReservedName),
            "{candidate:?} must be rejected"
        );
    }
}

#[test]
fn trailing_dot_or_space_is_rejected() {
    // Windows strips these on creation, so `evil.` and `evil` would collide
    // after the receiver believed they were distinct entries.
    for candidate in ["evil.", "evil ", "a/b./c", "name "] {
        assert_eq!(
            RelativePath::parse(candidate),
            Err(PathError::TrailingDotOrSpace),
            "{candidate:?} must be rejected"
        );
    }
}

#[test]
fn empty_and_doubled_separators_are_rejected() {
    assert_eq!(RelativePath::parse(""), Err(PathError::Empty));
    assert_eq!(RelativePath::parse("a//b"), Err(PathError::EmptySegment));
    assert_eq!(RelativePath::parse("a/"), Err(PathError::EmptySegment));
}

#[test]
fn oversize_paths_are_rejected() {
    let long = format!("{}/x", "a".repeat(MAX_PATH_LEN));
    assert!(matches!(
        RelativePath::parse(&long),
        Err(PathError::TooLong { .. })
    ));
}

#[test]
fn excessive_nesting_is_rejected() {
    let deep = vec!["a"; 65].join("/");
    assert!(matches!(
        RelativePath::parse(&deep),
        Err(PathError::TooManySegments { .. })
    ));
}

#[test]
fn invalid_utf8_paths_are_rejected() {
    assert_eq!(
        RelativePath::parse_bytes(&[0xFF, 0xFE, 0x00]),
        Err(PathError::InvalidUtf8)
    );
}

// ------------------------------------------------------------ manifest models

#[test]
fn empty_manifest_round_trips() {
    let empty = manifest(Vec::new());
    assert_eq!(empty.item_count(), 0);
    assert_eq!(empty.total_bytes(), 0);
    let bytes = codec::encode(&empty).expect("encodes");
    assert_eq!(codec::decode(&bytes).expect("decodes"), empty);
}

#[test]
fn zero_byte_and_one_byte_files_round_trip() {
    let subject = manifest(vec![file(1, "empty.bin", 0), file(2, "one.bin", 1)]);
    assert_eq!(subject.total_bytes(), 1);
    let bytes = codec::encode(&subject).expect("encodes");
    assert_eq!(codec::decode(&bytes).expect("decodes"), subject);
}

#[test]
fn directories_and_nested_folders_round_trip() {
    let items = vec![
        ManifestItem::directory(1, RelativePath::parse("photos").expect("valid"))
            .expect("valid directory"),
        ManifestItem::directory(2, RelativePath::parse("photos/summer").expect("valid"))
            .expect("valid directory"),
        file(3, "photos/summer/beach.jpg", 4096),
    ];
    let subject = manifest(items);
    assert_eq!(subject.item_count(), 3);
    assert_eq!(subject.total_bytes(), 4096);
    assert_eq!(subject.items()[0].kind(), ItemKind::Directory);

    let bytes = codec::encode(&subject).expect("encodes");
    assert_eq!(codec::decode(&bytes).expect("decodes"), subject);
}

#[test]
fn a_directory_may_not_carry_a_size_or_a_hash() {
    let path = RelativePath::parse("folder").expect("valid");
    assert!(matches!(
        ManifestItem::new(
            1,
            path.clone(),
            ItemKind::Directory,
            512,
            None,
            None,
            HashMetadata::none(),
            Compression::None,
        ),
        Err(ManifestError::InvalidDirectory { .. })
    ));

    let hash = HashMetadata::new(HashAlgorithm::Sha256, vec![7; 32]).expect("valid digest");
    assert!(matches!(
        ManifestItem::new(
            1,
            path,
            ItemKind::Directory,
            0,
            None,
            None,
            hash,
            Compression::None,
        ),
        Err(ManifestError::InvalidDirectory { .. })
    ));
}

#[test]
fn unicode_and_emoji_metadata_round_trips() {
    let item = ManifestItem::file(
        1,
        RelativePath::parse("año/mañana-🎉.txt").expect("valid"),
        12,
        HashMetadata::new(HashAlgorithm::Blake3, vec![3; 32]).expect("valid digest"),
    )
    .expect("valid item")
    .with_mime_type("text/plain; charset=utf-8")
    .expect("valid mime")
    .with_modified_unix_seconds(-1);

    let subject = manifest(vec![item]);
    let bytes = codec::encode(&subject).expect("encodes");
    let decoded = codec::decode(&bytes).expect("decodes");
    assert_eq!(decoded, subject);
    assert_eq!(decoded.items()[0].modified_unix_seconds(), Some(-1));
    assert_eq!(
        decoded.items()[0].mime_type(),
        Some("text/plain; charset=utf-8")
    );
}

#[test]
fn hash_length_must_match_its_algorithm() {
    assert!(matches!(
        HashMetadata::new(HashAlgorithm::Sha256, vec![0; 31]),
        Err(ManifestError::InvalidHashLength {
            length: 31,
            expected: 32
        })
    ));
    assert!(matches!(
        HashMetadata::new(HashAlgorithm::None, vec![0; 32]),
        Err(ManifestError::InvalidHashLength { .. })
    ));
}

#[test]
fn oversize_mime_type_is_rejected() {
    let item = file(1, "a.bin", 1);
    assert!(matches!(
        item.with_mime_type(&"x".repeat(129)),
        Err(ManifestError::FieldTooLong {
            field: ManifestField::MimeType,
            ..
        })
    ));
}

#[test]
fn duplicate_paths_are_rejected() {
    let result =
        TransferManifest::new(1, 0, vec![file(1, "same.txt", 10), file(2, "same.txt", 20)]);
    assert!(matches!(result, Err(ManifestError::DuplicatePath { .. })));
}

#[test]
fn duplicate_item_ids_are_rejected() {
    let result = TransferManifest::new(1, 0, vec![file(9, "a.txt", 1), file(9, "b.txt", 2)]);
    assert!(matches!(result, Err(ManifestError::DuplicateItemId { .. })));
}

#[test]
fn summing_item_sizes_cannot_wrap_into_a_believable_total() {
    // Engineered so a wrapping add would report a tiny total.
    let result = TransferManifest::new(
        1,
        0,
        vec![
            file(1, "a.bin", u64::MAX),
            file(2, "b.bin", u64::MAX),
            file(3, "c.bin", 4),
        ],
    );
    assert!(
        matches!(
            result,
            Err(ManifestError::TotalBytesMismatch { computed: None, .. })
                | Err(ManifestError::TotalBytesTooLarge { .. })
        ),
        "overflow must be an error, got {result:?}"
    );
}

#[test]
fn declared_total_beyond_the_limit_is_rejected() {
    let result = TransferManifest::new(1, 0, vec![file(1, "huge.bin", u64::MAX)]);
    assert!(matches!(
        result,
        Err(ManifestError::TotalBytesTooLarge { .. })
    ));
}

#[test]
fn items_are_stored_in_canonical_order() {
    let subject = manifest(vec![
        file(1, "zebra.txt", 1),
        file(2, "alpha.txt", 2),
        file(3, "middle.txt", 3),
    ]);
    let paths: Vec<&str> = subject
        .items()
        .iter()
        .map(|item| item.path().as_str())
        .collect();
    assert_eq!(paths, ["alpha.txt", "middle.txt", "zebra.txt"]);
}

// --------------------------------------------------------------- codec limits

#[test]
fn encoding_is_canonical_and_stable() {
    let subject = manifest(vec![file(1, "b.txt", 2), file(2, "a.txt", 1)]);
    let first = codec::encode(&subject).expect("encodes");
    let second = codec::encode(&subject).expect("encodes");
    assert_eq!(first, second, "encoding must be deterministic");
    assert_eq!(&first[0..4], &MANIFEST_MAGIC);

    // The same logical manifest built in another order encodes identically.
    let reordered = manifest(vec![file(2, "a.txt", 1), file(1, "b.txt", 2)]);
    assert_eq!(codec::encode(&reordered).expect("encodes"), first);
}

#[test]
fn corrupt_magic_is_rejected() {
    let subject = manifest(vec![file(1, "a.txt", 1)]);
    let mut bytes = codec::encode(&subject).expect("encodes");
    bytes[0] = b'X';
    assert!(matches!(
        codec::decode(&bytes),
        Err(ManifestError::InvalidMagic { .. })
    ));
}

#[test]
fn unsupported_manifest_version_is_rejected() {
    let subject = manifest(vec![file(1, "a.txt", 1)]);
    let mut bytes = codec::encode(&subject).expect("encodes");
    bytes[4..6].copy_from_slice(&99u16.to_be_bytes());
    assert!(matches!(
        codec::decode(&bytes),
        Err(ManifestError::UnsupportedVersion { found: 99, .. })
    ));
}

#[test]
fn a_hostile_item_count_is_rejected_without_reserving_for_it() {
    let subject = manifest(vec![file(1, "a.txt", 1)]);
    let mut bytes = codec::encode(&subject).expect("encodes");
    // Item count lives at offset 4+2+8+8+8 = 30.
    bytes[30..34].copy_from_slice(&u32::MAX.to_be_bytes());

    let error = codec::decode(&bytes).expect_err("must refuse");
    assert!(
        matches!(
            error,
            ManifestError::TooManyItems {
                declared: 4_294_967_295,
                limit: MAX_ITEMS
            }
        ),
        "expected TooManyItems, got {error:?}"
    );
}

#[test]
fn an_item_count_larger_than_the_remaining_bytes_is_rejected_early() {
    let subject = manifest(vec![file(1, "a.txt", 1)]);
    let mut bytes = codec::encode(&subject).expect("encodes");
    // Within MAX_ITEMS but impossible for the bytes that follow.
    bytes[30..34].copy_from_slice(&50_000u32.to_be_bytes());
    assert!(matches!(
        codec::decode(&bytes),
        Err(ManifestError::Truncated { .. })
    ));
}

#[test]
fn truncation_at_every_byte_is_an_error_not_a_partial_manifest() {
    let subject = manifest(vec![file(1, "photos/a.jpg", 100), file(2, "b.txt", 5)]);
    let bytes = codec::encode(&subject).expect("encodes");
    for cut in 0..bytes.len() {
        assert!(
            codec::decode(&bytes[..cut]).is_err(),
            "a {cut}-byte prefix must not decode"
        );
    }
    assert!(codec::decode(&bytes).is_ok());
}

#[test]
fn trailing_bytes_are_rejected() {
    let subject = manifest(vec![file(1, "a.txt", 1)]);
    let mut bytes = codec::encode(&subject).expect("encodes");
    bytes.push(0);
    assert!(matches!(
        codec::decode(&bytes),
        Err(ManifestError::TrailingBytes { count: 1 })
    ));
}

#[test]
fn a_non_canonical_option_tag_is_rejected() {
    let subject = manifest(vec![file(1, "a.txt", 1)]);
    let bytes = codec::encode(&subject).expect("encodes");
    // Presence bytes may only be 0 or 1; anything else is a second spelling of
    // the same value and would break canonicity.
    let mut found_rejection = false;
    for index in 34..bytes.len() {
        if bytes[index] != 0 {
            continue;
        }
        let mut mutated = bytes.clone();
        mutated[index] = 2;
        if matches!(
            codec::decode(&mutated),
            Err(ManifestError::InvalidFieldValue {
                field: ManifestField::OptionTag,
                value: 2
            })
        ) {
            found_rejection = true;
            break;
        }
    }
    assert!(found_rejection, "a mutated presence byte must be rejected");
}

#[test]
fn a_decoded_manifest_never_yields_an_absolute_or_escaping_path() {
    let subject = manifest(vec![
        file(1, "a/b/c.txt", 1),
        file(2, "z.bin", 2),
        file(3, "deep/nested/leaf.dat", 3),
    ]);
    let bytes = codec::encode(&subject).expect("encodes");
    let decoded = codec::decode(&bytes).expect("decodes");

    for item in decoded.items() {
        let path = item.path().as_str();
        assert!(!path.starts_with('/'), "{path} must not be absolute");
        assert!(!path.contains(".."), "{path} must not traverse");
        assert!(!path.contains('\\'), "{path} must not contain a backslash");
        assert!(!path.contains('\0'), "{path} must not contain NUL");
        assert!(!path.is_empty());
    }
}

#[test]
fn a_manifest_carrying_a_hostile_path_cannot_be_decoded() {
    // Hand-build the bytes a malicious peer would send, bypassing the builders.
    let subject = manifest(vec![file(1, "safe.txt", 1)]);
    let bytes = codec::encode(&subject).expect("encodes");
    let original = b"safe.txt";
    let hostile = b"../evil!";
    assert_eq!(original.len(), hostile.len(), "same length keeps offsets");

    let position = bytes
        .windows(original.len())
        .position(|window| window == original)
        .expect("path is present in the encoding");

    let mut mutated = bytes.clone();
    mutated[position..position + hostile.len()].copy_from_slice(hostile);

    assert!(matches!(
        codec::decode(&mutated),
        Err(ManifestError::InvalidPath {
            source: PathError::ParentSegment,
            ..
        })
    ));
}

#[test]
fn arbitrary_bytes_never_panic() {
    for seed in 0u16..=512 {
        let byte = u8::try_from(seed & 0xFF).expect("masked");
        let noise = vec![byte; usize::from(byte) + 8];
        let _ = codec::decode(&noise);
    }
    let mut prefixed = MANIFEST_MAGIC.to_vec();
    prefixed.extend_from_slice(&[0xFF; 64]);
    let _ = codec::decode(&prefixed);
}

// ------------------------------------------------- sprint 3 hardening (P0)

#[test]
fn the_visible_name_always_comes_from_the_path() {
    // ADR-0019: a separately supplied name could disagree with where the bytes
    // land. Deriving it removes the whole class of mismatch.
    let item = file(1, "docs/reports/q3.pdf", 10);
    assert_eq!(item.display_name(), "q3.pdf");
    assert_eq!(item.display_name(), item.path().file_name());
}

#[test]
fn an_executable_cannot_be_presented_as_a_document() {
    let item = file(1, "invoice.pdf.exe", 10);
    assert_eq!(
        item.display_name(),
        "invoice.pdf.exe",
        "the visible name must reveal the real extension"
    );
    assert!(item.display_name().ends_with(".exe"));
}

#[test]
fn every_public_constructor_derives_the_same_name() {
    let path = RelativePath::parse("a/b/real.bin").expect("valid");
    let full = ManifestItem::new(
        7,
        path.clone(),
        ItemKind::File,
        1,
        None,
        None,
        HashMetadata::new(HashAlgorithm::Sha256, vec![1; 32]).expect("digest"),
        Compression::None,
    )
    .expect("valid item");
    assert_eq!(full.display_name(), "real.bin");
    assert_eq!(full.display_name(), path.file_name());
}

#[test]
fn every_file_needs_a_final_digest_including_an_empty_one() {
    let path = RelativePath::parse("empty.bin").expect("valid");
    assert!(matches!(
        ManifestItem::file(1, path.clone(), 0, HashMetadata::none()),
        Err(ManifestError::MissingFileHash { .. })
    ));

    // A zero-byte file still has a digest: it is what proves the received bytes
    // are the sent bytes.
    let hashed = ManifestItem::file(
        1,
        path,
        0,
        HashMetadata::new(HashAlgorithm::Sha256, vec![0xE3; 32]).expect("digest"),
    )
    .expect("empty files are hashed too");
    assert_eq!(hashed.size(), 0);
    assert!(hashed.hash().is_present());
}

#[test]
fn a_directory_still_may_not_carry_a_digest() {
    let path = RelativePath::parse("folder").expect("valid");
    assert!(matches!(
        ManifestItem::new(
            1,
            path,
            ItemKind::Directory,
            0,
            None,
            None,
            HashMetadata::new(HashAlgorithm::Sha256, vec![2; 32]).expect("digest"),
            Compression::None,
        ),
        Err(ManifestError::InvalidDirectory { .. })
    ));
}

#[test]
fn windows_illegal_characters_are_rejected_on_every_platform() {
    for candidate in [
        "a<b.txt",
        "a>b.txt",
        "a:b.txt",
        "a\"b.txt",
        "a|b.txt",
        "a?b.txt",
        "a*b.txt",
        "dir/na<me",
        "col:on",
    ] {
        assert!(
            matches!(
                RelativePath::parse(candidate),
                Err(PathError::NonPortableCharacter { .. } | PathError::DrivePrefix)
            ),
            "{candidate:?} must be rejected"
        );
    }
}

#[test]
fn the_delete_character_is_rejected() {
    assert_eq!(
        RelativePath::parse("a\u{7F}b"),
        Err(PathError::ControlCharacter)
    );
}

#[test]
fn case_only_differences_collide_portably() {
    let result = TransferManifest::new(1, 0, vec![file(1, "Foto.jpg", 1), file(2, "foto.jpg", 2)]);
    assert!(
        matches!(result, Err(ManifestError::PortableCollision { .. })),
        "Windows and macOS would fold these onto one file, got {result:?}"
    );
}

#[test]
fn case_only_differences_collide_across_segments() {
    let result = TransferManifest::new(1, 0, vec![file(1, "A/B.txt", 1), file(2, "a/b.TXT", 2)]);
    assert!(matches!(
        result,
        Err(ManifestError::PortableCollision { .. })
    ));
}

#[test]
fn composed_and_decomposed_unicode_collide() {
    // "mañana" precomposed (U+00F1) versus decomposed (n + U+0303). Distinct
    // bytes, one file on most filesystems.
    let composed = "ma\u{00F1}ana.txt";
    let decomposed = "man\u{0303}ana.txt";
    assert_ne!(composed, decomposed, "the fixtures must differ in bytes");

    let result = TransferManifest::new(1, 0, vec![file(1, composed, 1), file(2, decomposed, 2)]);
    assert!(
        matches!(result, Err(ManifestError::PortableCollision { .. })),
        "NFC and NFD spellings must not both be accepted, got {result:?}"
    );
}

#[test]
fn a_folder_and_a_file_sharing_a_key_collide() {
    let folder = ManifestItem::directory(1, RelativePath::parse("Data").expect("valid"))
        .expect("valid directory");
    let result = TransferManifest::new(1, 0, vec![folder, file(2, "data", 5)]);
    assert!(matches!(
        result,
        Err(ManifestError::PortableCollision { .. })
    ));
}

#[test]
fn genuinely_different_unicode_stays_distinct() {
    // Folding must not over-merge: these are different letters, not spellings.
    let subject = manifest(vec![
        file(1, "日本.txt", 1),
        file(2, "中国.txt", 2),
        file(3, "🎉.txt", 3),
        file(4, "alpha.txt", 4),
        file(5, "beta.txt", 5),
    ]);
    assert_eq!(subject.item_count(), 5);
}

#[test]
fn encoded_len_matches_the_bytes_actually_produced() {
    let cases = vec![
        manifest(Vec::new()),
        manifest(vec![file(1, "a.txt", 1)]),
        manifest(vec![
            file(1, "photos/one.jpg", 100),
            ManifestItem::directory(2, RelativePath::parse("photos").expect("valid"))
                .expect("valid"),
            file(3, "año/🎉.bin", 7)
                .with_mime_type("application/octet-stream")
                .expect("valid mime")
                .with_modified_unix_seconds(-12345),
        ]),
    ];

    for subject in cases {
        let predicted = codec::encoded_len(&subject).expect("preflight succeeds");
        let actual = codec::encode(&subject).expect("encodes").len();
        assert_eq!(
            predicted,
            actual,
            "preflight must equal the real length for {} items",
            subject.item_count()
        );
    }
}

#[test]
fn manifest_version_two_rejects_version_one_bytes() {
    let subject = manifest(vec![file(1, "a.txt", 1)]);
    let mut bytes = codec::encode(&subject).expect("encodes");
    bytes[4..6].copy_from_slice(&1u16.to_be_bytes());
    assert!(matches!(
        codec::decode(&bytes),
        Err(ManifestError::UnsupportedVersion {
            found: 1,
            supported: 2
        })
    ));
}
