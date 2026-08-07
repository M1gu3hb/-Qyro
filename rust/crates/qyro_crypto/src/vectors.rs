//! Conformance tests: committed vectors, the RFC 8032 KAT, and the invariants
//! the public API is supposed to enforce.
//!
//! These live inside the crate rather than in `tests/` because the deterministic
//! seed constructor is `cfg(test)` and crate-private. An integration test is a
//! separate crate and would only reach it through a public constructor, which is
//! exactly what this crate must not have.
//!
//! Both vector files are the interoperable source of truth and are parsed as
//! JSON, not scraped for `"key": "value"` substrings. The previous string search
//! read whichever field happened to appear first, so a renamed or reordered key
//! would have silently changed which value was being checked while the test kept
//! passing.

use serde_json::Value;

use crate::error::IdentityError;
use crate::fingerprint::{FINGERPRINT_LEN, IdentityFingerprint};
use crate::identity::{
    DeviceIdentity, IDENTITY_VERSION, PUBLIC_IDENTITY_WIRE_LEN, PUBLIC_KEY_LEN, PublicIdentity,
};
use crate::signature::{IdentitySignature, SIGNATURE_LEN, SignatureDomain};

/// The RFC 8032 section 7.1 TEST 1 secret key. Public, test-only.
const RFC8032_TEST1_SEED: [u8; 32] = [
    0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c, 0xc4,
    0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae, 0x7f, 0x60,
];

/// Qyro's own vectors: domain separation, fingerprints, grouped form.
const IDENTITY_VECTORS: &str =
    include_str!("../../../../docs/security/test-vectors/identity-v1.json");

/// RFC 8032 section 7.1, verbatim. Verifies the dependency, not Qyro.
const RFC8032_VECTORS: &str =
    include_str!("../../../../docs/security/test-vectors/rfc8032-ed25519.json");

fn unhex(text: &str) -> Vec<u8> {
    assert!(text.len().is_multiple_of(2), "hex must be byte-aligned");
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).expect("hex digit"))
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn test_identity() -> DeviceIdentity {
    DeviceIdentity::from_test_seed(&RFC8032_TEST1_SEED)
}

fn sign(identity: &DeviceIdentity, domain: SignatureDomain, message: &[u8]) -> IdentitySignature {
    identity
        .try_sign(domain, message)
        .expect("the domain is available in this version")
}

// ----------------------------------------------------------- RFC 8032 known answers

#[test]
fn the_ed25519_implementation_matches_every_rfc8032_vector() {
    // A known-answer test against the standard itself, covering all five of
    // section 7.1 including the 1023-byte message. It signs the message
    // directly, with no Qyro domain separation, because what is under test here
    // is the dependency's conformance to RFC 8032 rather than anything of ours.
    let parsed: Value = serde_json::from_str(RFC8032_VECTORS).expect("the KAT file is valid JSON");
    let cases = parsed["vectors"].as_array().expect("vectors is an array");
    assert_eq!(cases.len(), 5, "RFC 8032 section 7.1 defines five vectors");

    for case in cases {
        let name = case["name"].as_str().expect("name");
        let secret = unhex(case["secret_key"].as_str().expect("secret_key"));
        let public = unhex(case["public_key"].as_str().expect("public_key"));
        let message = unhex(case["message"].as_str().expect("message"));
        let signature = unhex(case["signature"].as_str().expect("signature"));

        let declared = case["message_len"].as_u64().expect("message_len");
        assert_eq!(
            message.len() as u64,
            declared,
            "{name}: the file's own declared message length must match its bytes"
        );

        let seed: [u8; 32] = secret.as_slice().try_into().expect("32-byte secret");
        let signing = ed25519_dalek::SigningKey::from_bytes(&seed);

        assert_eq!(
            hex(signing.verifying_key().as_bytes()),
            hex(&public),
            "{name}: derived public key must match the RFC"
        );

        // Ed25519 is deterministic, so the signature is reproducible exactly.
        let produced = ed25519_dalek::Signer::sign(&signing, &message);
        assert_eq!(
            hex(&produced.to_bytes()),
            hex(&signature),
            "{name}: signature must match the RFC byte for byte"
        );

        let verifying =
            ed25519_dalek::VerifyingKey::from_bytes(&signing.verifying_key().to_bytes())
                .expect("valid key");
        let recorded = ed25519_dalek::Signature::from_slice(&signature).expect("64-byte signature");
        verifying
            .verify_strict(&message, &recorded)
            .unwrap_or_else(|error| panic!("{name}: the RFC's own signature must verify: {error}"));
    }
}

// ------------------------------------------------------------ committed Qyro vectors

#[test]
fn the_public_key_matches_the_rfc8032_vector() {
    let identity = test_identity();
    assert_eq!(
        hex(identity.public_identity().as_bytes()),
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        "RFC 8032 section 7.1 TEST 1 public key"
    );
}

#[test]
fn the_committed_vector_file_matches_the_implementation() {
    let parsed: Value =
        serde_json::from_str(IDENTITY_VECTORS).expect("the vector file is valid JSON");
    let identity = test_identity();
    let public = identity.public_identity();

    assert_eq!(
        parsed["identity_version"].as_u64(),
        Some(u64::from(IDENTITY_VERSION))
    );
    assert_eq!(
        unhex(parsed["identity"]["seed"].as_str().expect("seed")),
        RFC8032_TEST1_SEED.to_vec(),
        "the file's seed must be the one these tests use"
    );
    assert_eq!(
        parsed["identity"]["public_key"].as_str(),
        Some(hex(public.as_bytes()).as_str())
    );
    assert_eq!(
        parsed["identity"]["fingerprint"].as_str(),
        Some(public.fingerprint().to_hex().as_str())
    );
    assert_eq!(
        parsed["identity"]["fingerprint_grouped"].as_str(),
        Some(public.fingerprint().to_grouped_hex().as_str())
    );
}

#[test]
fn every_signature_in_the_vector_file_verifies() {
    let parsed: Value =
        serde_json::from_str(IDENTITY_VECTORS).expect("the vector file is valid JSON");
    let identity = test_identity();
    let public = identity.public_identity();

    let recorded = parsed["signatures"]
        .as_array()
        .expect("signatures is an array");
    assert_eq!(recorded.len(), 4, "two domains times two messages");

    for case in recorded {
        let domain_id = case["domain_id"].as_u64().expect("domain_id");
        let domain = match domain_id {
            1 => SignatureDomain::TestVector,
            2 => SignatureDomain::DeviceClaim,
            other => panic!("the file names an unexpected domain id {other}"),
        };
        // The name and the id must agree; a file that disagrees with itself is
        // not a source of truth.
        assert_eq!(
            case["domain"].as_str(),
            Some(match domain {
                SignatureDomain::TestVector => "TestVector",
                SignatureDomain::DeviceClaim => "DeviceClaim",
                SignatureDomain::HandshakeTranscript => unreachable!(),
            })
        );
        assert_eq!(domain.to_wire(), u8::try_from(domain_id).expect("small id"));

        let message = unhex(case["message_hex"].as_str().expect("message_hex"));
        let expected = case["signature"].as_str().expect("signature");

        // Ed25519 is deterministic, so signing again must reproduce the file.
        assert_eq!(
            sign(&identity, domain, &message).to_hex(),
            expected,
            "domain {domain_id} over {message:02x?} must reproduce the committed signature"
        );

        let signature = IdentitySignature::from_slice(&unhex(expected)).expect("64 bytes");
        public
            .verify(domain, &message, &signature)
            .unwrap_or_else(|error| panic!("domain {domain_id} must verify: {error}"));
    }
}

// ------------------------------------------------------------------- weak public keys

/// The eight small-order point encodings published in the Ed25519 literature.
///
/// Not asserted to be weak because this file says so: each is checked against
/// `ed25519_dalek`'s own `is_weak` below, so the test states what the library
/// concludes rather than what the author believed. Every one of them is a valid
/// point encoding, which is the point — nothing about the bytes marks them out.
const SMALL_ORDER_ENCODINGS: [&str; 8] = [
    "0100000000000000000000000000000000000000000000000000000000000000",
    "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000080",
    "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05",
    "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a",
    "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa",
    "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85",
];

#[test]
fn every_low_order_key_is_refused() {
    for encoding in SMALL_ORDER_ENCODINGS {
        let bytes = unhex(encoding);
        let raw: [u8; PUBLIC_KEY_LEN] = bytes.as_slice().try_into().expect("32 bytes");

        // The premise: the library parses it, and the library calls it weak.
        // If a future version of the dependency disagrees, this fails here
        // rather than silently weakening the assertion below.
        let parsed = ed25519_dalek::VerifyingKey::from_bytes(&raw)
            .unwrap_or_else(|_| panic!("{encoding} is a valid encoding"));
        assert!(parsed.is_weak(), "{encoding} must be a low-order point");

        assert_eq!(
            PublicIdentity::from_bytes(&bytes),
            Err(IdentityError::WeakPublicKey),
            "{encoding} authenticates almost any message and must not become an identity"
        );
        assert_eq!(
            PublicIdentity::decode(&[&[IDENTITY_VERSION][..], &bytes].concat()),
            Err(IdentityError::WeakPublicKey),
            "the wire decoder must refuse it too, not only from_bytes"
        );
    }
}

#[test]
fn an_all_zero_key_is_refused_as_weak_not_as_malformed() {
    // Worth stating separately: all-zeros is the shape a caller is most likely
    // to produce by accident, from an uninitialised buffer or a failed read. It
    // is a *valid* Ed25519 encoding, so before the low-order check it became a
    // perfectly ordinary-looking identity.
    assert_eq!(
        PublicIdentity::from_bytes(&[0u8; PUBLIC_KEY_LEN]),
        Err(IdentityError::WeakPublicKey)
    );
}

#[test]
fn a_real_key_is_not_weak() {
    let identity = test_identity();
    assert!(PublicIdentity::from_bytes(identity.public_identity().as_bytes()).is_ok());
}

// ------------------------------------------------------------- the 33-byte wire form

#[test]
fn a_public_identity_round_trips_through_its_wire_form() {
    let identity = test_identity();
    let public = identity.public_identity();
    let encoded = public.encode();

    assert_eq!(encoded.len(), PUBLIC_IDENTITY_WIRE_LEN);
    assert_eq!(encoded[0], IDENTITY_VERSION, "byte 0 is the version");
    assert_eq!(&encoded[1..], public.as_bytes(), "bytes 1..33 are the key");

    let decoded = PublicIdentity::decode(&encoded).expect("its own encoding round-trips");
    assert_eq!(&decoded, public);
    assert_eq!(decoded.fingerprint(), public.fingerprint());
    assert_eq!(decoded.encode(), encoded);
}

#[test]
fn a_wire_form_of_the_wrong_length_is_rejected() {
    let identity = test_identity();
    let encoded = identity.public_identity().encode();

    for length in [0usize, 1, 32, 34] {
        let mut truncated = encoded.to_vec();
        truncated.resize(length, 0);
        assert_eq!(
            PublicIdentity::decode(&truncated),
            Err(IdentityError::InvalidPublicKeyLength {
                found: length,
                expected: PUBLIC_IDENTITY_WIRE_LEN
            }),
            "{length} bytes is not a public identity"
        );
    }
}

#[test]
fn a_wire_form_declaring_another_version_is_rejected() {
    let identity = test_identity();
    let mut encoded = identity.public_identity().encode();
    encoded[0] = IDENTITY_VERSION + 1;

    assert_eq!(
        PublicIdentity::decode(&encoded),
        Err(IdentityError::UnsupportedVersion {
            found: IDENTITY_VERSION + 1,
            supported: IDENTITY_VERSION
        }),
        "the version travels with the key precisely so this is detectable"
    );
}

// -------------------------------------------------------------- canonical fingerprints

#[test]
fn a_fingerprint_round_trips_through_both_canonical_forms() {
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
    assert_eq!(original.to_hex().len(), FINGERPRINT_LEN * 2);
    assert_eq!(original.to_grouped_hex().matches('-').count(), 7);
}

#[test]
fn only_the_two_canonical_spellings_parse() {
    let identity = test_identity();
    let plain = identity.fingerprint().to_hex();
    let grouped = identity.fingerprint().to_grouped_hex();

    // Every one of these used to parse: the old implementation stripped all
    // hyphens before looking at anything, so a fingerprint had a whole family
    // of spellings and comparing strings stopped implying equal identities.
    let rejected = [
        format!("-{plain}"),
        format!("{plain}-"),
        format!("-{grouped}"),
        format!("{grouped}-"),
        grouped.replacen('-', "--", 1),
        // Right number of hyphens, wrong positions.
        plain
            .chars()
            .enumerate()
            .flat_map(|(index, character)| {
                if index > 0 && index < 8 && index.is_multiple_of(1) && index <= 7 {
                    vec!['-', character]
                } else {
                    vec![character]
                }
            })
            .collect::<String>(),
        // Right shape, uppercase.
        grouped.to_uppercase(),
        plain.to_uppercase(),
        // Separators that are not hyphens.
        grouped.replace('-', ":"),
        grouped.replace('-', " "),
        format!(" {plain}"),
        format!("{plain} "),
        // Lengths.
        String::new(),
        plain[..10].to_owned(),
        format!("{plain}00"),
        plain.replacen('9', "z", 1),
    ];

    for spelling in rejected {
        assert_eq!(
            IdentityFingerprint::parse(&spelling),
            Err(IdentityError::MalformedFingerprint),
            "{spelling:?} must not name an identity"
        );
    }

    // And the two that must work still do.
    assert!(IdentityFingerprint::parse(&plain).is_ok());
    assert!(IdentityFingerprint::parse(&grouped).is_ok());
}

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

// ------------------------------------------------------------------------ domain rules

#[test]
fn a_signature_never_verifies_in_another_domain() {
    let identity = test_identity();
    let public = identity.public_identity();
    let message = b"qyro";

    let claim = sign(&identity, SignatureDomain::DeviceClaim, message);
    assert_eq!(
        public.verify(SignatureDomain::TestVector, message, &claim),
        Err(IdentityError::SignatureVerificationFailed),
        "domain separation must prevent cross-domain replay"
    );

    let vector = sign(&identity, SignatureDomain::TestVector, message);
    assert_eq!(
        public.verify(SignatureDomain::DeviceClaim, message, &vector),
        Err(IdentityError::SignatureVerificationFailed)
    );
}

#[test]
fn the_handshake_domain_is_available_now_that_its_transcript_is_frozen() {
    // ADR-0020 reserved this domain and this test asserted it was refused:
    // signing in a domain whose meaning nothing has fixed commits to a format
    // by accident. ADR-0021 freezes the transcript, so the domain opens.
    let identity = test_identity();
    assert!(SignatureDomain::HandshakeTranscript.is_available());

    let signature = sign(&identity, SignatureDomain::HandshakeTranscript, b"x");
    assert!(
        identity
            .public_identity()
            .verify(SignatureDomain::HandshakeTranscript, b"x", &signature)
            .is_ok()
    );

    // Still domain-separated from the others.
    assert_eq!(
        identity
            .public_identity()
            .verify(SignatureDomain::DeviceClaim, b"x", &signature),
        Err(IdentityError::SignatureVerificationFailed)
    );
}

#[test]
fn the_vector_file_agrees_with_the_code_about_which_domains_are_available() {
    // The file records an `available` flag per domain. Nothing checked it, so
    // when ADR-0021 opened the handshake domain the file would have gone on
    // claiming it was reserved.
    let parsed: Value =
        serde_json::from_str(IDENTITY_VECTORS).expect("the vector file is valid JSON");
    let recorded = parsed["domains"].as_array().expect("domains is an array");

    let all = [
        SignatureDomain::TestVector,
        SignatureDomain::DeviceClaim,
        SignatureDomain::HandshakeTranscript,
    ];
    assert_eq!(recorded.len(), all.len(), "every domain must be recorded");

    for (entry, domain) in recorded.iter().zip(all) {
        assert_eq!(
            entry["id"].as_u64(),
            Some(u64::from(domain.to_wire())),
            "the file's ids must be the wire values"
        );
        assert_eq!(
            entry["available"].as_bool(),
            Some(domain.is_available()),
            "the file disagrees with the code about domain {}",
            domain.to_wire()
        );
    }
}

// --------------------------------------------------------------------- negative cases

#[test]
fn an_altered_message_does_not_verify() {
    let identity = test_identity();
    let public = identity.public_identity();
    let signature = sign(&identity, SignatureDomain::DeviceClaim, b"qyro");

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
    let signature = sign(&identity, SignatureDomain::DeviceClaim, b"qyro");

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

    let signature = sign(&mine, SignatureDomain::DeviceClaim, b"qyro");
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
    // Note that many 32-byte patterns *are* valid encodings, so the fixture has
    // to be chosen deliberately rather than assumed. All-zeros, for one, is
    // valid — it is rejected as weak, not as malformed, and has its own test.
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

// ------------------------------------------------------------------- secret handling

#[test]
fn debug_output_never_contains_secret_material() {
    let identity = test_identity();
    let rendered = format!("{identity:?}");

    let seed_hex = hex(&RFC8032_TEST1_SEED);
    assert!(
        !rendered.contains(&seed_hex),
        "Debug must not print the seed"
    );
    for window in seed_hex.as_bytes().chunks(16) {
        let fragment = String::from_utf8_lossy(window);
        assert!(
            !rendered.contains(fragment.as_ref()),
            "Debug leaked a fragment of the seed"
        );
    }
    assert!(rendered.contains("redacted"), "the secret must be marked");
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
    let signature = sign(&identity, SignatureDomain::DeviceClaim, b"this device");
    assert!(
        identity
            .public_identity()
            .verify(SignatureDomain::DeviceClaim, b"this device", &signature)
            .is_ok()
    );
}

// ---------------------------------------------------- sprint 4C.2 (QYR-0023)

/// A signature the permissive verifier accepts and the strict one refuses.
///
/// # What makes it different
///
/// `ed25519_dalek::VerifyingKey::verify` and `verify_strict` differ in exactly
/// one way in the pinned 3.0.0: `verify_strict` decompresses `R`, refuses if
/// `R` or `A` has small order, and only then compares the recomputed `R`. So a
/// signature that separates them must have a small-order `R` that still
/// satisfies `[s]B = R + [k]A`. Since `[s]B - [k]A` always lies in the
/// prime-order subgroup, the only small-order point it can equal is the
/// identity — which is why `R` here is the identity point, `0x01` followed by
/// thirty-one zero bytes, the first entry of [`SMALL_ORDER_ENCODINGS`] above.
///
/// This is the degenerate small-order-`R` case catalogued by *Taming the Many
/// EdDSAs* (Chalkias, Garillot and Nikolaenko, 2020) and excluded by ZIP-215;
/// it is the class of signature `verify_strict` exists to refuse.
///
/// # Provenance, and why the bytes are derived rather than quoted
///
/// The key is the RFC 8032 §7.1 TEST 1 key this file already uses, and the
/// signed bytes are this crate's own domain-separated signing input. That is
/// forced: `PublicIdentity::verify` hashes
/// `"QYRO-SIGN-V1" || 0x00 || domain || len || message`, so no external vector's
/// `(A, M)` pair can ever be presented to it — a published triple would test
/// `ed25519_dalek` rather than this crate's use of it.
///
/// So the signature is constructed, and reproducibly. With
/// `a = clamp(SHA-512(seed)[0..32])`, `R = identity`,
/// `k = SHA-512(R || A || M) mod L` and `s = k · a mod L`, the verification
/// equation `[s]B = [k·a]B = [k]A = R + [k]A` holds with `R` the identity, so
/// the permissive check passes and the strict one refuses the small-order `R`.
/// `M` is the signing input for `SignatureDomain::TestVector` over `b"qyro"`.
const SMALL_ORDER_R_SIGNATURE: &str = concat!(
    "0100000000000000000000000000000000000000000000000000000000000000",
    "23b9427fe725a5209f4f0f876f2dacaa4a3972f40f255a037a584d151450c50a",
);

#[test]
fn a_non_strict_signature_is_refused() {
    // QYR-0023. `identity.rs` calls `verify_strict`, and swapping it for
    // `verify` left the suite at 124 passed, 0 failed: every other signature
    // test uses signatures this crate produced, which both verifiers accept.
    // Strictness is only observable on a signature built to exploit the
    // difference.
    let identity = test_identity();
    let public = identity.public_identity();

    let raw = unhex(SMALL_ORDER_R_SIGNATURE);
    assert_eq!(
        hex(&raw[..32]),
        SMALL_ORDER_ENCODINGS[0],
        "the premise of this vector is that R is the identity point"
    );

    let signature = IdentitySignature::from_slice(&raw).expect("64 bytes");
    assert_eq!(
        public.verify(SignatureDomain::TestVector, b"qyro", &signature),
        Err(IdentityError::SignatureVerificationFailed),
        "a signature whose R has small order must be refused; accepting it \
         means this crate is calling the permissive verifier, and two distinct \
         signatures over one message would both be valid"
    );

    // The control: an ordinary signature over the same message under the same
    // key still verifies, so the rejection above is about this signature and
    // not about the message, the domain or the key.
    let honest = sign(&identity, SignatureDomain::TestVector, b"qyro");
    assert!(
        public
            .verify(SignatureDomain::TestVector, b"qyro", &honest)
            .is_ok(),
        "the strict verifier must still accept what this crate signs"
    );
}
