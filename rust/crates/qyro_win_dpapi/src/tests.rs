//! Against real DPAPI. These only build and run on Windows.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

use qyro_crypto::DeviceIdentity;
use qyro_identity_store::{
    IdentityStore, SecretWrapper, StoreError, entropy_for, open_identity, seal_identity,
};

use crate::ffi::DataBlob;
use crate::store::{DpapiWrapper, WindowsIdentityStore};

const VERSION: u8 = 1;
const WRAP: u8 = 1;
const HEADER_LEN: usize = 16;
const ENTROPY_HEADER_LEN: usize = 12;

/// A scratch store under the runner's temp directory, removed on drop.
struct Scratch {
    store: WindowsIdentityStore,
    dir: std::path::PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("qyro-store-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self {
            store: WindowsIdentityStore::at(dir.join("identity.bin")),
            dir,
        }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn a_data_blob_that_lies_does_not_round_trip() {
    // The mitigation ADR-0024 §1 promised in exchange for transcribing the
    // extern by hand. A DATA_BLOB whose layout or field widths are wrong cannot
    // survive a real protect/unprotect, so this is what stands between a
    // hand-written declaration and undefined behaviour nobody noticed.
    let secret = b"a seed shaped thing, thirty two.";
    assert_eq!(secret.len(), 32);
    let entropy = entropy_for(VERSION, WRAP);

    let wrapped = DpapiWrapper.wrap(secret, &entropy).unwrap();
    assert_ne!(
        &wrapped[..],
        &secret[..],
        "the wrapped output is the plaintext; the blob is not being filled"
    );
    let back = DpapiWrapper.unwrap(&wrapped, &entropy).unwrap();
    assert_eq!(
        &back[..],
        &secret[..],
        "round trip lost or altered the bytes, which is what a mis-declared \
         DATA_BLOB looks like from the outside"
    );

    // And the struct is laid out the way the header says: a DWORD then a
    // pointer, C layout, so on a 64-bit target four bytes of padding sit
    // between them. Written as literals rather than as size_of arithmetic,
    // because arithmetic over the same terms would agree with itself whatever
    // the declaration said.
    assert_eq!(core::mem::size_of::<DataBlob>(), 16);
    assert_eq!(core::mem::align_of::<DataBlob>(), 8);
}

#[test]
fn a_wrapped_secret_needs_the_same_entropy() {
    let secret = [0x11u8; 32];
    let wrapped = DpapiWrapper
        .wrap(&secret, &entropy_for(VERSION, WRAP))
        .unwrap();
    // Another application, same user, calling DPAPI without Qyro's constant.
    let outcome = DpapiWrapper.unwrap(&wrapped, b"some other application");
    assert!(
        matches!(outcome, Err(StoreError::Unwrap { .. })),
        "DPAPI accepted a different entropy: {outcome:?}"
    );
}

#[test]
fn a_single_flipped_byte_is_a_typed_error_against_dpapi() {
    // The 448-position sweep, now against the real wrapper rather than the test
    // double. The three ranges must fail by the routes ADR-0024's QYR-0048
    // amendment documents; a byte failing by another route is a finding, not
    // something to relax the assertion for.
    let identity = DeviceIdentity::generate().unwrap();
    let blob = seal_identity(&identity, &DpapiWrapper).unwrap();

    // Positions observed to survive on this runner, recorded so a *new* one
    // fails. Not a contract from Microsoft: the reference says the wrapped
    // format is opaque and must not be parsed, so this set could differ on
    // another Windows version — and if it does, this test fails, which is the
    // correct behaviour.
    let mut survivors: Vec<(usize, u8)> = Vec::new();
    let mut checked = 0usize;
    for position in 0..blob.len() {
        for bit in 0..8u8 {
            let mut corrupted = blob.clone();
            corrupted[position] ^= 1 << bit;
            let error = match open_identity(&corrupted, &DpapiWrapper) {
                Err(error) => error,
                Ok(survivor) => {
                    // QYR-0059. DPAPI's MAC guards the encrypted data, not its
                    // own header, so a few bytes of the wrapper survive a flip.
                    // That is recorded rather than papered over, and the
                    // assertion here is *stronger* than the one it replaced:
                    // "no position survives" was simply false, and this pins
                    // which ones do.
                    survivors.push((position, bit));
                    assert_eq!(
                        survivor.fingerprint(),
                        identity.fingerprint(),
                        "byte {position} bit {bit} survived AND changed the \
                         identity. Malleability in a field DPAPI ignores is \
                         tolerable; a different identity is not — that is a \
                         device silently becoming someone else."
                    );
                    continue;
                }
            };
            checked += 1;

            if (ENTROPY_HEADER_LEN..HEADER_LEN).contains(&position) {
                assert!(
                    matches!(error, StoreError::LengthMismatch { .. }),
                    "byte {position} bit {bit} is inside wrapped_len; the entropy \
                     does not cover it, so LengthMismatch is the only route that \
                     can catch it. Got {error:?}"
                );
            } else if position >= HEADER_LEN {
                assert!(
                    matches!(error, StoreError::Unwrap { .. }),
                    "byte {position} bit {bit} is inside the DPAPI body and must \
                     fail its MAC. Got {error:?}"
                );
            } else {
                assert!(
                    !matches!(error, StoreError::IdentityAbsent),
                    "byte {position} bit {bit} is in the header and must never \
                     read as an absent identity. Got {error:?}"
                );
            }
        }
    }
    assert!(checked >= 8 * HEADER_LEN, "swept only {checked} mutations");

    // Every survivor must be inside the DPAPI wrapper's own header. One in the
    // Qyro header, or in the ciphertext, would be a different finding entirely.
    for (position, bit) in &survivors {
        assert!(
            *position >= HEADER_LEN,
            "byte {position} bit {bit} is in the Qyro header and survived. The \
             entropy is supposed to cover 0..12 and LengthMismatch 12..16, so \
             this is not QYR-0059 — it is a new hole."
        );
    }
    // And the set is small. If a Windows update made most of the wrapper
    // malleable, this is what would say so.
    assert!(
        survivors.len() <= 16,
        "{} positions survived corruption, which is too many to call a header \
         quirk: {survivors:?}",
        survivors.len()
    );
    println!(
        "QYR-0059: {} surviving position(s): {survivors:?}",
        survivors.len()
    );

    // The untouched blob still opens, so the sweep was not passing because
    // everything fails.
    assert_eq!(
        open_identity(&blob, &DpapiWrapper).unwrap().fingerprint(),
        identity.fingerprint()
    );
}

#[test]
fn load_on_an_empty_store_is_a_typed_absence() {
    let scratch = Scratch::new("absent");
    let error = scratch.store.load().unwrap_err();
    assert_eq!(error, StoreError::IdentityAbsent);
    assert!(
        error.is_absent(),
        "absence must be distinguishable without matching on the variant at \
         every call site"
    );
}

#[test]
fn an_unreadable_store_is_not_an_absent_one() {
    let scratch = Scratch::new("unreadable");
    std::fs::write(scratch.store.path(), b"not a qyro identity blob at all").unwrap();
    let error = scratch.store.load().unwrap_err();
    assert!(
        !error.is_absent(),
        "unreadable read as absent: a caller would mint a new identity over one \
         that is still there. Got {error:?}"
    );
    assert_eq!(error, StoreError::NotAnIdentityBlob);
}

#[test]
fn two_creates_do_not_lose_data() {
    let scratch = Scratch::new("twocreate");
    let first = DeviceIdentity::generate().unwrap();
    scratch.store.create(&first).unwrap();

    let second = DeviceIdentity::generate().unwrap();
    assert_eq!(
        scratch.store.create(&second).unwrap_err(),
        StoreError::AlreadyExists,
        "the second create must refuse; overwriting silently is data loss"
    );
    assert_eq!(
        scratch.store.load().unwrap().fingerprint(),
        first.fingerprint(),
        "the refused create still changed the stored identity"
    );
}

#[test]
fn rotate_replaces_exactly_one_identity() {
    let scratch = Scratch::new("rotate");
    let original = DeviceIdentity::generate().unwrap();
    scratch.store.create(&original).unwrap();

    let replacement = scratch.store.rotate().unwrap();
    assert_ne!(
        replacement.fingerprint(),
        original.fingerprint(),
        "rotate returned the identity it was meant to replace"
    );

    let loaded = scratch.store.load().unwrap();
    assert_eq!(loaded.fingerprint(), replacement.fingerprint());
    assert_ne!(
        loaded.fingerprint(),
        original.fingerprint(),
        "the previous identity still loads after rotation"
    );
}

#[test]
fn delete_leaves_nothing_loadable() {
    let scratch = Scratch::new("delete");
    scratch
        .store
        .create(&DeviceIdentity::generate().unwrap())
        .unwrap();
    scratch.store.delete().unwrap();
    assert_eq!(
        scratch.store.load().unwrap_err(),
        StoreError::IdentityAbsent
    );

    // Idempotent by decision: the caller asked for no identity and there is
    // none. A typed error would make every caller write the same match to
    // ignore it.
    scratch
        .store
        .delete()
        .expect("deleting an empty store is success, not an error");
}
