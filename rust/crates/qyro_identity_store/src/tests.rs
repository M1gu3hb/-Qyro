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
use crate::blob::{ENTROPY_HEADER_LEN, HEADER_LEN, VERSION, WRAP_DPAPI_USER};

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
fn the_entropy_covers_twelve_header_bytes_and_not_the_length() {
    // QYR-0048 stated as an executable fact rather than a comment. If somebody
    // "restores" the original rule, the entropy would depend on wrapped_len and
    // this length would change.
    assert_eq!(entropy_for(VERSION, WRAP_DPAPI_USER).len(), {
        QYRO_IDENTITY_ENTROPY_V1.len() + ENTROPY_HEADER_LEN
    });
    // And it must not vary with the size of what gets wrapped.
    assert_eq!(
        entropy_for(VERSION, WRAP_DPAPI_USER),
        entropy_for(VERSION, WRAP_DPAPI_USER)
    );
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
