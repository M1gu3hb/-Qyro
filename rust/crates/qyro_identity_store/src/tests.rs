//! Adversarial cases over the stored format.
//!
//! The wrapper here is a fake, and it is `cfg(test)` for the same reason
//! `from_test_seed` is crate-private in `qyro_crypto`: a working in-memory
//! backend in the public API is one import away from being what somebody ships.
//!
//! What the fake can and cannot show. It reproduces the *shape* of a real
//! wrapper — it refuses when the entropy differs, and it authenticates its own
//! output — so the format's refusals are exercised honestly. It is **not**
//! DPAPI, so nothing here demonstrates that DPAPI behaves this way; that is what
//! the Windows harness in CI is for, and it does not exist yet.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

use super::*;
use crate::blob::{
    ENTROPY_HEADER_LEN, HEADER_LEN, VERSION, WRAP_ANDROID_KEYSTORE, WRAP_BRIDGED, WRAP_DPAPI_USER,
};

/// A stand-in for a platform wrapper.
///
/// Authenticates by prefixing a keyed tag over `entropy || secret`. Not
/// cryptography anybody should ship — it is a test double whose only job is to
/// fail when it should.
struct FakeWrapper;

impl FakeWrapper {
    /// A deliberately trivial checksum. Its job is to detect the mutations this
    /// file makes, not to resist an adversary.
    fn tag(entropy: &[u8], payload: &[u8]) -> [u8; 8] {
        let mut acc: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in entropy.iter().chain(payload) {
            acc ^= u64::from(*byte);
            acc = acc.wrapping_mul(0x0100_0000_01b3);
        }
        acc.to_be_bytes()
    }
}

impl SecretWrapper for FakeWrapper {
    fn wrap(&self, secret: &[u8], entropy: &[u8]) -> Result<Vec<u8>, StoreError> {
        let mut out = Vec::with_capacity(8 + secret.len());
        out.extend_from_slice(&Self::tag(entropy, secret));
        out.extend_from_slice(secret);
        Ok(out)
    }

    fn unwrap(&self, wrapped: &[u8], entropy: &[u8]) -> Result<Zeroizing<Vec<u8>>, StoreError> {
        if wrapped.len() < 8 {
            return Err(StoreError::Unwrap { code: 1 });
        }
        let (tag, payload) = wrapped.split_at(8);
        if tag != Self::tag(entropy, payload) {
            return Err(StoreError::Unwrap { code: 2 });
        }
        Ok(Zeroizing::new(payload.to_vec()))
    }

    fn wrap_id(&self) -> u8 {
        WRAP_DPAPI_USER
    }
}

fn an_identity() -> DeviceIdentity {
    DeviceIdentity::from_secret(&IdentitySecret::from_bytes(&[0x42u8; SEED_LEN]))
}

fn a_blob() -> Vec<u8> {
    seal_identity(&an_identity(), &FakeWrapper).unwrap()
}

#[test]
fn a_sealed_identity_opens_as_the_same_identity() {
    let identity = an_identity();
    let blob = seal_identity(&identity, &FakeWrapper).unwrap();
    let restored = open_identity(&blob, &FakeWrapper).unwrap();
    assert_eq!(identity.fingerprint(), restored.fingerprint());
}

#[test]
fn a_single_flipped_byte_is_a_typed_error() {
    // Every position, one at a time. The three ranges reach an error by three
    // different routes (QYR-0048), and the assertion names which one it expects
    // so that a byte failing for the wrong reason is still a failure.
    let blob = a_blob();
    let identity = an_identity();

    for position in 0..blob.len() {
        for bit in 0..8u8 {
            let mut corrupted = blob.clone();
            corrupted[position] ^= 1 << bit;
            if corrupted == blob {
                continue;
            }

            let outcome = open_identity(&corrupted, &FakeWrapper);
            let Err(error) = outcome else {
                panic!(
                    "byte {position} bit {bit}: corruption produced an identity. \
                     A blob that does not authenticate must never yield one, \
                     neither the same nor a different one."
                );
            };

            let expected_route = if position < ENTROPY_HEADER_LEN {
                "header under entropy, or a field refused outright"
            } else if position < HEADER_LEN {
                "wrapped_len, caught by LengthMismatch"
            } else {
                "wrapper body, caught by the wrapper's own authentication"
            };

            match (position, &error) {
                // 12..16 is wrapped_len. The entropy no longer covers it, so
                // LengthMismatch is the *only* correct route: if this ever
                // arrives as Unwrap, the length check has stopped running.
                (p, e) if (ENTROPY_HEADER_LEN..HEADER_LEN).contains(&p) => assert!(
                    matches!(e, StoreError::LengthMismatch { .. }),
                    "byte {p} bit {bit} is inside wrapped_len and should fail as \
                     LengthMismatch ({expected_route}), got {e:?}"
                ),
                (p, e) if p >= HEADER_LEN => assert!(
                    matches!(e, StoreError::Unwrap { .. }),
                    "byte {p} bit {bit} is inside the wrapper body and should \
                     fail as Unwrap ({expected_route}), got {e:?}"
                ),
                (_, e) => assert!(
                    !matches!(e, StoreError::IdentityAbsent),
                    "a corrupted header must never read as an absent identity \
                     ({expected_route}), got {e:?}"
                ),
            }
        }
    }

    // And the untouched blob still opens, so the loop above was not passing
    // because everything fails.
    assert_eq!(
        open_identity(&blob, &FakeWrapper).unwrap().fingerprint(),
        identity.fingerprint()
    );
}

#[test]
fn a_blob_from_a_future_version_is_refused_by_version() {
    let mut blob = a_blob();
    blob[8] = VERSION + 1;
    assert_eq!(
        open_identity(&blob, &FakeWrapper).unwrap_err(),
        StoreError::UnsupportedVersion { found: VERSION + 1 }
    );
}

#[test]
fn a_blob_from_version_zero_is_refused_by_version() {
    let mut blob = a_blob();
    blob[8] = 0;
    assert_eq!(
        open_identity(&blob, &FakeWrapper).unwrap_err(),
        StoreError::UnsupportedVersion { found: 0 }
    );
}

#[test]
fn an_unknown_wrap_algorithm_is_refused_by_name() {
    let mut blob = a_blob();
    blob[9] = 0x7F;
    assert_eq!(
        open_identity(&blob, &FakeWrapper).unwrap_err(),
        StoreError::UnsupportedWrap { found: 0x7F }
    );
}

#[test]
fn a_blob_with_a_nonzero_reserved_is_refused() {
    for index in 10..12 {
        let mut blob = a_blob();
        blob[index] = 1;
        assert_eq!(
            open_identity(&blob, &FakeWrapper).unwrap_err(),
            StoreError::ReservedNotZero,
            "reserved byte {index}"
        );
    }
}

#[test]
fn a_truncated_blob_is_refused() {
    let blob = a_blob();
    for length in [0, 1, HEADER_LEN - 1, blob.len() / 2] {
        let error = open_identity(&blob[..length], &FakeWrapper).unwrap_err();
        if length < HEADER_LEN {
            assert_eq!(error, StoreError::Truncated { found: length });
        } else {
            // Long enough for a header, so the declared length is what catches
            // it — a different route to the same refusal.
            assert!(
                matches!(error, StoreError::LengthMismatch { .. }),
                "length {length}: {error:?}"
            );
        }
    }
}

#[test]
fn an_exactly_header_sized_blob_reaches_length_validation() {
    let header = crate::blob::BlobHeader {
        version: VERSION,
        wrap: WRAP_DPAPI_USER,
        wrapped_len: 0,
    }
    .encode();

    let (parsed, body) = crate::blob::parse(&header).expect("a complete header is not truncated");
    assert_eq!(parsed.wrapped_len, 0);
    assert!(body.is_empty());
}

#[test]
fn an_empty_blob_is_refused() {
    assert_eq!(
        open_identity(&[], &FakeWrapper).unwrap_err(),
        StoreError::Truncated { found: 0 }
    );
}

#[test]
fn the_right_magic_with_garbage_behind_it_is_refused() {
    let mut blob = vec![0u8; 64];
    blob[..8].copy_from_slice(b"QYRO-IDS");
    let error = open_identity(&blob, &FakeWrapper).unwrap_err();
    // Version zero is what it hits first, and that is the point: the magic
    // alone buys nothing.
    assert_eq!(error, StoreError::UnsupportedVersion { found: 0 });
}

#[test]
fn a_blob_that_is_not_ours_is_refused_before_anything_else() {
    let blob = vec![0xFFu8; 64];
    assert_eq!(
        open_identity(&blob, &FakeWrapper).unwrap_err(),
        StoreError::NotAnIdentityBlob
    );
}

#[test]
fn a_declared_length_that_disagrees_is_refused_both_ways() {
    for delta in [1u32, 2, 1000] {
        let blob = a_blob();
        let actual = u32::try_from(blob.len() - HEADER_LEN).unwrap();

        let mut over = blob.clone();
        over[12..16].copy_from_slice(&(actual + delta).to_be_bytes());
        assert!(
            matches!(
                open_identity(&over, &FakeWrapper),
                Err(StoreError::LengthMismatch { .. })
            ),
            "declared {delta} bytes too many"
        );

        if actual > delta {
            let mut under = blob;
            under[12..16].copy_from_slice(&(actual - delta).to_be_bytes());
            assert!(
                matches!(
                    open_identity(&under, &FakeWrapper),
                    Err(StoreError::LengthMismatch { .. })
                ),
                "declared {delta} bytes too few"
            );
        }
    }
}

#[test]
fn a_blob_read_with_a_different_entropy_constant_is_refused() {
    // The property the entropy exists for: another application running as the
    // same user, calling the platform API without Qyro's constant, gets nothing.
    struct OtherConstant;
    impl SecretWrapper for OtherConstant {
        fn wrap(&self, secret: &[u8], _entropy: &[u8]) -> Result<Vec<u8>, StoreError> {
            FakeWrapper.wrap(secret, b"some other application")
        }
        fn unwrap(
            &self,
            wrapped: &[u8],
            _entropy: &[u8],
        ) -> Result<Zeroizing<Vec<u8>>, StoreError> {
            FakeWrapper.unwrap(wrapped, b"some other application")
        }
        fn wrap_id(&self) -> u8 {
            WRAP_DPAPI_USER
        }
    }

    let ours = a_blob();
    assert!(matches!(
        open_identity(&ours, &OtherConstant),
        Err(StoreError::Unwrap { .. })
    ));
}

#[test]
fn the_entropy_binds_the_header_and_not_only_its_length() {
    // QYR-0052. The previous test here checked the length and then asserted
    // `entropy_for(V, W) == entropy_for(V, W)` — the same pure function with the
    // same arguments, which is true of every function that exists. Replacing the
    // twelve header bytes with twelve zeroes left the whole suite green, so the
    // reason the amendment gives for binding the header was covered by nothing.
    //
    // This is the third time this exact shape has appeared: QYR-0025 verified a
    // transcript by calling the thing that produced it, and the
    // `encrypted_envelope` target carried a tautological assertion.
    let blob = a_blob();
    let (_, body) = crate::blob::parse(&blob).unwrap();
    let ours = entropy_for(VERSION, WRAP_DPAPI_USER);

    for (field, other) in [
        ("wrap", entropy_for(VERSION, WRAP_DPAPI_USER + 1)),
        ("version", entropy_for(VERSION + 1, WRAP_DPAPI_USER)),
    ] {
        // Same length, so a difference cannot be explained by size alone. This
        // is what makes the next assertion mean something.
        assert_eq!(
            other.len(),
            ours.len(),
            "{field}: entropies differ in length, so this test would pass for \
             the wrong reason"
        );
        // The load-bearing one: a header field must change the entropy. Twelve
        // zeroes in place of the header fails right here.
        assert_ne!(
            other, ours,
            "{field}: the entropy does not depend on it, so a wrapper output \
             could be relabelled under a different but valid header"
        );
        assert!(
            matches!(
                FakeWrapper.unwrap(body, &other),
                Err(StoreError::Unwrap { .. })
            ),
            "{field}: a body sealed under our header unwrapped under another"
        );
    }

    // And ours still works, so the loop is not passing because nothing does.
    assert!(FakeWrapper.unwrap(body, &ours).is_ok());
}

#[test]
fn absence_is_not_any_other_error() {
    // The distinction the whole enum exists to hold.
    assert!(StoreError::IdentityAbsent.is_absent());
    for other in [
        StoreError::Truncated { found: 0 },
        StoreError::NotAnIdentityBlob,
        StoreError::UnsupportedVersion { found: 2 },
        StoreError::UnsupportedWrap { found: 2 },
        StoreError::ReservedNotZero,
        StoreError::LengthMismatch {
            declared: 1,
            present: 2,
        },
        StoreError::Unwrap { code: 5 },
        StoreError::MalformedSecret { found: 3 },
        StoreError::AlreadyExists,
        StoreError::Io { code: 13 },
    ] {
        assert!(
            !other.is_absent(),
            "{other:?} must not read as an absent identity: acting on it would \
             replace an identity that is still there"
        );
    }
}

#[test]
fn a_secret_of_the_wrong_length_is_refused() {
    struct ShortSecret;
    impl SecretWrapper for ShortSecret {
        fn wrap(&self, _secret: &[u8], entropy: &[u8]) -> Result<Vec<u8>, StoreError> {
            FakeWrapper.wrap(&[0u8; 8], entropy)
        }
        fn unwrap(&self, wrapped: &[u8], entropy: &[u8]) -> Result<Zeroizing<Vec<u8>>, StoreError> {
            FakeWrapper.unwrap(wrapped, entropy)
        }
        fn wrap_id(&self) -> u8 {
            WRAP_DPAPI_USER
        }
    }
    let blob = seal_identity(&an_identity(), &ShortSecret).unwrap();
    assert_eq!(
        open_identity(&blob, &ShortSecret).unwrap_err(),
        StoreError::MalformedSecret { found: 8 }
    );
}

/// The committed framing vectors, verified against the primitives.
///
/// Built from `MAGIC`, the version and wrap bytes and a big-endian length —
/// **not** by calling `BlobHeader::encode` or `entropy_for`. Checking a vector
/// with the function that produced it proves the function agrees with itself,
/// which is what QYR-0025 recorded about the handshake transcript and what
/// QYR-0052 recorded about the entropy test in this very file.
///
/// What is deliberately absent: a complete sealed blob. The wrapped body is
/// DPAPI output, bound to a user account and a machine by design, so no other
/// machine could reproduce it — a committed one would be a vector nobody but its
/// author could verify.
#[test]
fn the_committed_framing_vectors_match_the_primitives() {
    let raw = include_str!("../../../../docs/security/test-vectors/storage-v1.json");

    let hex_of = |bytes: &[u8]| -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() };
    // Independent of the module: the magic as ASCII, the constant as ASCII.
    let magic = b"QYRO-IDS";
    let constant = b"qyro.identity.store.v1";

    assert!(
        raw.contains(&format!("\"magic\": \"{}\"", hex_of(magic))),
        "the committed magic is not the ASCII bytes QYRO-IDS"
    );
    assert!(
        raw.contains(&format!("\"entropy_constant\": \"{}\"", hex_of(constant))),
        "the committed entropy constant is not QYRO_IDENTITY_ENTROPY_V1"
    );

    // Rebuild each header the way a second implementation would read the ADR,
    // and require the file to contain exactly those bytes.
    for (version, wrap, wrapped_len) in [(1u8, 1u8, 0u32), (1, 1, 256)] {
        let mut header = Vec::new();
        header.extend_from_slice(magic);
        header.extend_from_slice(&[version, wrap, 0, 0]);
        header.extend_from_slice(&wrapped_len.to_be_bytes());
        assert_eq!(header.len(), HEADER_LEN);

        assert!(
            raw.contains(&format!("\"header\": \"{}\"", hex_of(&header))),
            "no committed vector carries the header for version {version}, wrap \
             {wrap}, wrapped_len {wrapped_len}.\n\
             The format changed, or the file did. If the format changed, ADR-0024 \
             is what has to change first — regenerating this file is not the \
             remedy, it is how the disagreement gets hidden."
        );

        let mut entropy = Vec::new();
        entropy.extend_from_slice(constant);
        entropy.extend_from_slice(&header[..ENTROPY_HEADER_LEN]);
        assert!(
            raw.contains(&format!("\"entropy\": \"{}\"", hex_of(&entropy))),
            "the committed entropy for version {version}, wrap {wrap} is not \
             constant || header[0..12] (QYR-0048)"
        );
    }

    // And the production path agrees with the independently built bytes. This
    // assertion is the only one that touches the module, and it comes last: the
    // vector is the reference, not the output.
    let mut expected = Vec::new();
    expected.extend_from_slice(constant);
    expected.extend_from_slice(magic);
    expected.extend_from_slice(&[VERSION, WRAP_DPAPI_USER, 0, 0]);
    assert_eq!(entropy_for(VERSION, WRAP_DPAPI_USER), expected);
}

/// A second wrapper, claiming a different `wrap` byte and using the same
/// trivial tag. It stands in for the Android backend on a host that cannot
/// build it: what matters here is that its `wrap_id` disagrees, not how it
/// wraps.
struct SecondPlatform;

impl SecretWrapper for SecondPlatform {
    fn wrap(&self, secret: &[u8], entropy: &[u8]) -> Result<Vec<u8>, StoreError> {
        FakeWrapper.wrap(secret, entropy)
    }

    fn unwrap(&self, wrapped: &[u8], entropy: &[u8]) -> Result<Zeroizing<Vec<u8>>, StoreError> {
        FakeWrapper.unwrap(wrapped, entropy)
    }

    fn wrap_id(&self) -> u8 {
        WRAP_ANDROID_KEYSTORE
    }
}

#[test]
fn a_blob_from_another_platform_is_refused_by_wrap_and_not_by_luck() {
    // What the `wrap` byte exists to say. A blob sealed on Windows handed to the
    // Android wrapper must fail *because the wrapper is wrong*, with an error
    // that says so — not because some tag happened not to verify.
    //
    // The distinction is the whole point. "The tag did not verify" is what a
    // corrupted blob looks like, and it is what a caller retries or reports as
    // damage. "This blob belongs to another wrapper" is neither: nothing is
    // damaged and retrying will never help.
    //
    // Both wrappers here share a tag function on purpose. If they did not, this
    // test would pass whether or not the wrap byte were ever consulted, because
    // the tag would fail either way — an assertion that holds for the wrong
    // reason, which is the shape this project has now paid for three times.
    let identity = an_identity();
    let windows_blob = seal_identity(&identity, &FakeWrapper).unwrap();

    let error = open_identity(&windows_blob, &SecondPlatform).unwrap_err();
    assert_eq!(
        error,
        StoreError::WrapMismatch {
            blob: WRAP_DPAPI_USER,
            wrapper: WRAP_ANDROID_KEYSTORE,
        },
        "a Windows blob opened with the Android wrapper must name both sides. \
         Got {error:?}"
    );

    // And the other direction, so this is not a property of one order.
    let android_blob = seal_identity(&identity, &SecondPlatform).unwrap();
    assert_eq!(
        open_identity(&android_blob, &FakeWrapper).unwrap_err(),
        StoreError::WrapMismatch {
            blob: WRAP_ANDROID_KEYSTORE,
            wrapper: WRAP_DPAPI_USER,
        }
    );

    // Neither refusal is "everything fails": each blob still opens with its own
    // wrapper, and to the same identity.
    assert_eq!(
        open_identity(&windows_blob, &FakeWrapper)
            .unwrap()
            .fingerprint(),
        identity.fingerprint()
    );
    assert_eq!(
        open_identity(&android_blob, &SecondPlatform)
            .unwrap()
            .fingerprint(),
        identity.fingerprint()
    );
}

#[test]
fn a_wrap_byte_nobody_implements_is_still_refused_at_parse() {
    // Adding 0x02, and later 0x03, must not turn the wrap check into "anything
    // small is fine". The constant has moved twice now -- 0x03 became known as
    // `WRAP_BRIDGED`, then 0x04 as `WRAP_NONE_SANDBOX` -- and each move was a
    // decision, not a build fix.
    //
    // **This test was inverted on purpose in phase 11, and that is worth
    // reading twice.** It used to poke 0x03 in, because 0x03 was unknown. It is
    // known now (`WRAP_BRIDGED`, ADR-0040 §5), so the old body asserted the
    // absence of the very thing being added, and it failed the moment the byte
    // was allowed. The right response to a green test that contradicts a fix is
    // to decide which of the two is wrong — here the test's *constant* was, not
    // its property — and the wrong response is to delete it, which is how a
    // repair erases its own evidence.
    let identity = an_identity();
    let mut blob = seal_identity(&identity, &FakeWrapper).unwrap();
    blob[9] = 0x05;
    assert_eq!(
        open_identity(&blob, &FakeWrapper).unwrap_err(),
        StoreError::UnsupportedWrap { found: 0x05 },
        "an unknown wrap must be refused by name at step 5, before any wrapper \
         is asked to do anything"
    );
}

#[test]
fn a_blob_sealed_by_a_bridged_wrapper_can_be_opened_again() {
    // ADR-0040 §5, and the test that should have existed since ADR-0037.
    //
    // `BridgedWrapper::wrap_id()` has returned 3 since the bridge was designed,
    // and `KNOWN_WRAPS` stopped at 2, so this round trip failed with
    // `UnsupportedWrap { found: 3 }`: a blob the Android bridge sealed could
    // never be reopened. The bridge's own contract exercised it against
    // `entropy_for` and never through `seal_identity`/`open_identity`, so this
    // seam was the one thing nothing crossed.
    //
    // The wrapper here is the fake with the bridge's identifier, not the bridge
    // itself: what is under test is that the **format** admits the byte and
    // that the entropy binding still agrees across a seal and an open. Whether
    // Kotlin's AES-GCM round-trips is `KeystoreIdentityTest`'s job, on a device
    // this project does not have.
    struct BridgedIdWrapper;

    impl SecretWrapper for BridgedIdWrapper {
        fn wrap(&self, plaintext: &[u8], entropy: &[u8]) -> Result<Vec<u8>, StoreError> {
            FakeWrapper.wrap(plaintext, entropy)
        }

        fn unwrap(&self, wrapped: &[u8], entropy: &[u8]) -> Result<Zeroizing<Vec<u8>>, StoreError> {
            FakeWrapper.unwrap(wrapped, entropy)
        }

        fn wrap_id(&self) -> u8 {
            WRAP_BRIDGED
        }
    }

    let identity = an_identity();
    let blob = seal_identity(&identity, &BridgedIdWrapper).expect("sealing under wrap 3");
    assert_eq!(
        blob[9], WRAP_BRIDGED,
        "the header must record the wrapper that sealed it"
    );

    let reopened = open_identity(&blob, &BridgedIdWrapper)
        .expect("a bridged blob must open again; before ADR-0040 this was UnsupportedWrap");
    assert_eq!(
        reopened.fingerprint(),
        identity.fingerprint(),
        "the round trip must return the same identity, not merely succeed"
    );
}
