//! Verifies the committed test vectors and the secret-handling guarantees.
//!
//! The vectors live in `docs/security/test-vectors/identity-v1.json` and are the
//! interoperable source of truth. These tests load that file rather than
//! restating its values, so a divergence between the file and the code is a
//! failure instead of two copies drifting apart.

use qyro_crypto::{
    DeviceIdentity, FINGERPRINT_LEN, IDENTITY_VERSION, IdentityError, IdentityFingerprint,
    IdentitySignature, PUBLIC_KEY_LEN, PublicIdentity, SIGNATURE_LEN, SignatureDomain,
};

/// The RFC 8032 section 7.1 TEST 1 secret key. Public, test-only.
const RFC8032_TEST1_SEED: [u8; 32] = [
    0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c, 0xc4,
    0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae, 0x7f, 0x60,
];

fn vectors() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/security/test-vectors/identity-v1.json"
    ))
    .expect("the committed vector file is readable")
}

/// Minimal extraction: pulls `"key": "value"` without adding a JSON dependency.
fn field(json: &str, key: &str) -> String {
    let needle = format!("\"{key}\": \"");
    let start = json
        .find(&needle)
        .unwrap_or_else(|| panic!("{key} present"))
        + needle.len();
    let rest = &json[start..];
    let end = rest.find('"').expect("closing quote");
    rest[..end].to_owned()
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).expect("hex"))
        .collect()
}

fn test_identity() -> DeviceIdentity {
    DeviceIdentity::from_test_seed(&RFC8032_TEST1_SEED)
}

// -------------------------------------------------------- official vectors

#[test]
fn the_public_key_matches_the_rfc8032_vector() {
    // Checks the Ed25519 implementation itself before anything Qyro-specific.
    let identity = test_identity();
    assert_eq!(
        identity.public_identity().as_bytes().to_vec(),
        unhex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"),
        "RFC 8032 section 7.1 TEST 1 public key"
    );
}

#[test]
fn the_committed_vector_file_matches_the_implementation() {
    let json = vectors();
    let identity = test_identity();
    let public = identity.public_identity();

    assert_eq!(
        unhex(&field(&json, "seed")),
        RFC8032_TEST1_SEED.to_vec(),
        "the file's seed must be the one these tests use"
    );
    assert_eq!(
        unhex(&field(&json, "public_key")),
        public.as_bytes().to_vec()
    );
    assert_eq!(field(&json, "fingerprint"), public.fingerprint().to_hex());
    assert_eq!(
        field(&json, "fingerprint_grouped"),
        public.fingerprint().to_grouped_hex()
    );
}

#[test]
fn every_signature_in_the_vector_file_verifies() {
    let json = vectors();
    let identity = test_identity();
    let public = identity.public_identity();

    let cases = [
        (SignatureDomain::TestVector, "", 1usize),
        (SignatureDomain::TestVector, "7179726f", 2),
        (SignatureDomain::DeviceClaim, "", 3),
        (SignatureDomain::DeviceClaim, "7179726f", 4),
    ];

    // Signatures appear in file order; take them positionally.
    let recorded: Vec<String> = json
        .match_indices("\"signature\": \"")
        .map(|(index, needle)| {
            let rest = &json[index + needle.len()..];
            rest[..rest.find('"').expect("closing quote")].to_owned()
        })
        .collect();
    assert_eq!(recorded.len(), cases.len(), "one signature per case");

    for ((domain, message_hex, position), recorded_hex) in cases.iter().zip(&recorded) {
        let message = unhex(message_hex);
        let signature = IdentitySignature::from_slice(&unhex(recorded_hex)).expect("64 bytes");

        // Ed25519 is deterministic, so signing again must reproduce the file.
        assert_eq!(
            identity.sign(*domain, &message).to_hex(),
            *recorded_hex,
            "case {position} must reproduce the committed signature"
        );
        public
            .verify(*domain, &message, &signature)
            .unwrap_or_else(|error| panic!("case {position} must verify: {error}"));
    }
}

// ------------------------------------------------------------ domain rules

#[test]
fn a_signature_never_verifies_in_another_domain() {
    let identity = test_identity();
    let public = identity.public_identity();
    let message = b"qyro";

    let claim = identity.sign(SignatureDomain::DeviceClaim, message);
    assert_eq!(
        public.verify(SignatureDomain::TestVector, message, &claim),
        Err(IdentityError::SignatureVerificationFailed),
        "domain separation must prevent cross-domain replay"
    );

    let vector = identity.sign(SignatureDomain::TestVector, message);
    assert_eq!(
        public.verify(SignatureDomain::DeviceClaim, message, &vector),
        Err(IdentityError::SignatureVerificationFailed)
    );
}

#[test]
fn the_reserved_handshake_domain_is_refused() {
    let identity = test_identity();
    assert!(!SignatureDomain::HandshakeTranscript.is_available());
    assert_eq!(
        identity.try_sign(SignatureDomain::HandshakeTranscript, b"x"),
        Err(IdentityError::DomainNotAvailable { domain: 3 }),
        "signing a transcript format nothing has frozen must be refused"
    );

    let signature = identity.sign(SignatureDomain::TestVector, b"x");
    assert_eq!(
        identity
            .public_identity()
            .verify(SignatureDomain::HandshakeTranscript, b"x", &signature),
        Err(IdentityError::DomainNotAvailable { domain: 3 })
    );
}

// --------------------------------------------------------- negative cases

#[test]
fn an_altered_message_does_not_verify() {
    let identity = test_identity();
    let public = identity.public_identity();
    let signature = identity.sign(SignatureDomain::DeviceClaim, b"qyro");

    for altered in [
        b"qyrp".as_slice(),
        b"qyr".as_slice(),
        b"qyro ".as_slice(),
        b"".as_slice(),
    ] {
        assert_eq!(
            public.verify(SignatureDomain::DeviceClaim, altered, &signature),
            Err(IdentityError::SignatureVerificationFailed)
        );
    }
}

#[test]
fn an_altered_signature_does_not_verify() {
    let identity = test_identity();
    let public = identity.public_identity();
    let signature = identity.sign(SignatureDomain::DeviceClaim, b"qyro");

    for index in [0usize, 31, 63] {
        let mut bytes = *signature.as_bytes();
        bytes[index] ^= 0x01;
        let altered = IdentitySignature::from_bytes(bytes);
        assert_eq!(
            public.verify(SignatureDomain::DeviceClaim, b"qyro", &altered),
            Err(IdentityError::SignatureVerificationFailed),
            "flipping byte {index} must invalidate the signature"
        );
    }
}

#[test]
fn a_truncated_signature_is_rejected_by_length() {
    for length in [0usize, 63, 65] {
        assert_eq!(
            IdentitySignature::from_slice(&vec![0u8; length]),
            Err(IdentityError::InvalidSignatureLength {
                found: length,
                expected: SIGNATURE_LEN
            })
        );
    }
}

#[test]
fn another_identity_cannot_verify_this_ones_signature() {
    let mine = test_identity();
    let mut other_seed = RFC8032_TEST1_SEED;
    other_seed[0] ^= 0xFF;
    let theirs = DeviceIdentity::from_test_seed(&other_seed);

    let signature = mine.sign(SignatureDomain::DeviceClaim, b"qyro");
    assert_eq!(
        theirs
            .public_identity()
            .verify(SignatureDomain::DeviceClaim, b"qyro", &signature),
        Err(IdentityError::SignatureVerificationFailed)
    );
}

#[test]
fn a_malformed_public_key_is_rejected() {
    for length in [0usize, 31, 33] {
        assert_eq!(
            PublicIdentity::from_bytes(&vec![0u8; length]),
            Err(IdentityError::InvalidPublicKeyLength {
                found: length,
                expected: PUBLIC_KEY_LEN
            })
        );
    }
    // Right length, but the y-coordinate does not decompress to a curve point.
    // Note that many 32-byte patterns *are* valid encodings — [0xFF; 32] and
    // all-zeros both decompress — so the fixture has to be chosen deliberately
    // rather than assumed.
    let mut not_a_point = [0u8; PUBLIC_KEY_LEN];
    not_a_point[0] = 0x02;
    assert_eq!(
        PublicIdentity::from_bytes(&not_a_point),
        Err(IdentityError::MalformedPublicKey)
    );
}

#[test]
fn an_unsupported_identity_version_is_rejected() {
    let identity = test_identity();
    let bytes = identity.public_identity().as_bytes();
    assert_eq!(
        PublicIdentity::from_versioned_bytes(IDENTITY_VERSION + 1, bytes),
        Err(IdentityError::UnsupportedVersion {
            found: IDENTITY_VERSION + 1,
            supported: IDENTITY_VERSION
        })
    );
    assert!(PublicIdentity::from_versioned_bytes(IDENTITY_VERSION, bytes).is_ok());
}

// ------------------------------------------------------------ fingerprints

#[test]
fn a_fingerprint_is_stable_across_reconstruction() {
    let identity = test_identity();
    let direct = *identity.fingerprint();
    let reparsed =
        PublicIdentity::from_bytes(identity.public_identity().as_bytes()).expect("valid key");
    assert_eq!(&direct, reparsed.fingerprint());
    assert_eq!(direct.as_bytes().len(), FINGERPRINT_LEN);
}

#[test]
fn a_different_identity_has_a_different_fingerprint() {
    let mine = test_identity();
    let mut other_seed = RFC8032_TEST1_SEED;
    other_seed[31] ^= 0x01;
    let theirs = DeviceIdentity::from_test_seed(&other_seed);
    assert_ne!(mine.fingerprint(), theirs.fingerprint());
}

#[test]
fn a_fingerprint_round_trips_through_both_representations() {
    let identity = test_identity();
    let original = *identity.fingerprint();

    assert_eq!(
        IdentityFingerprint::parse(&original.to_hex()).expect("plain hex"),
        original
    );
    assert_eq!(
        IdentityFingerprint::parse(&original.to_grouped_hex()).expect("grouped hex"),
        original
    );
    // Never truncated: the canonical value is all 32 bytes.
    assert_eq!(original.to_hex().len(), FINGERPRINT_LEN * 2);
}

#[test]
fn a_malformed_fingerprint_representation_is_rejected() {
    let identity = test_identity();
    let valid = identity.fingerprint().to_hex();

    for bad in [
        String::new(),
        valid[..10].to_owned(),
        format!("{valid}00"),
        valid.to_uppercase(),
        valid.replacen('9', "z", 1),
    ] {
        assert_eq!(
            IdentityFingerprint::parse(&bad),
            Err(IdentityError::MalformedFingerprint),
            "{bad:?} must be rejected"
        );
    }
}

// ------------------------------------------------------- secret handling

#[test]
fn debug_output_never_contains_secret_material() {
    let identity = test_identity();
    let rendered = format!("{identity:?}");

    let seed_hex: String = RFC8032_TEST1_SEED
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert!(
        !rendered.contains(&seed_hex),
        "Debug must not print the seed"
    );
    // Nor any run of it: check every 8-byte window.
    for window in seed_hex.as_bytes().chunks(16) {
        let fragment = String::from_utf8_lossy(window);
        assert!(
            !rendered.contains(fragment.as_ref()),
            "Debug leaked a fragment of the seed"
        );
    }
    assert!(rendered.contains("redacted"), "the secret must be marked");
    // The public fingerprint is fine to show.
    assert!(rendered.contains("fingerprint"));
}

#[test]
fn the_public_half_debug_is_safe() {
    let identity = test_identity();
    let rendered = format!("{:?}", identity.public_identity());
    assert!(rendered.contains("PublicIdentity"));
    assert!(rendered.contains("fingerprint"));
}

#[test]
fn generated_identities_differ() {
    // Uses the real system CSPRNG, so two generations must not collide.
    let first = DeviceIdentity::generate().expect("system entropy is available");
    let second = DeviceIdentity::generate().expect("system entropy is available");
    assert_ne!(first.fingerprint(), second.fingerprint());
    assert_ne!(
        first.public_identity().as_bytes(),
        second.public_identity().as_bytes()
    );
}

#[test]
fn a_generated_identity_signs_and_verifies() {
    let identity = DeviceIdentity::generate().expect("system entropy is available");
    let signature = identity.sign(SignatureDomain::DeviceClaim, b"this device");
    assert!(
        identity
            .public_identity()
            .verify(SignatureDomain::DeviceClaim, b"this device", &signature)
            .is_ok()
    );
}
