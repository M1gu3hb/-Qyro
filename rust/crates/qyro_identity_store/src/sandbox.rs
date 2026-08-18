//! The wrapper that does not wrap.
//!
//! ADR-0040 amendment 1. Android's stage A stores the identity seed in
//! `getNoBackupFilesDir()` with **the per-UID filesystem sandbox as its only
//! protection**, because the mechanism ADR-0037 specified for reaching Keystore
//! cannot be built — and the one that can needs a JNI shim in C that this
//! project cannot execute even once.
//!
//! # Why this exists as a named type rather than as an `if`
//!
//! Three reasons, and each one is a defect this project has already paid for.
//!
//! **It has to be asked for.** `qyro_identity_open_blocking` takes a protection
//! argument and `PLATFORM` never falls back to this. Nobody gets less protection
//! than they asked for because a wrapper happened to be missing.
//!
//! **The file records it.** `wrap_id` is [`WRAP_NONE_SANDBOX`], so a blob says
//! what protected it. A stage B build that meets one can refuse it, migrate it,
//! or warn — three options that exist only because the fact is written down.
//!
//! **It is not hidden.** A wrapper somebody has to go looking for is a wrapper
//! somebody reimplements worse, inline, without the header byte.
//!
//! # What it is not
//!
//! It is not encryption and its name does not pretend otherwise. `wrap` returns
//! its input. On Android, stage A, the honest sentence is: *with Keystore an
//! attacker with root would still need the TEE; with the sandbox, root is
//! enough.* That sentence is in `THREAT_MODEL.md`, not in a footnote.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use zeroize::Zeroizing;

use crate::SecretWrapper;
use crate::blob::WRAP_NONE_SANDBOX;
use crate::error::StoreError;

/// Stores the secret as it is, protected by the filesystem and nothing else.
#[derive(Debug, Clone, Copy, Default)]
pub struct SandboxWrapper;

impl SecretWrapper for SandboxWrapper {
    /// Returns the input.
    ///
    /// The `entropy` argument is ignored, and that is worth stating rather than
    /// leaving to be noticed: for every other wrapper the entropy binds the blob
    /// to its header, and here nothing binds anything. Tampering with a sandbox
    /// blob is caught by the format's length and reserved-byte checks and by
    /// `DeviceIdentity::from_secret` refusing a wrong-sized seed — not by any
    /// authentication, because there is none.
    fn wrap(&self, secret: &[u8], _entropy: &[u8]) -> Result<Vec<u8>, StoreError> {
        Ok(secret.to_vec())
    }

    /// Returns the stored bytes, in a `Zeroizing` so the caller's contract holds.
    fn unwrap(&self, wrapped: &[u8], _entropy: &[u8]) -> Result<Zeroizing<Vec<u8>>, StoreError> {
        Ok(Zeroizing::new(wrapped.to_vec()))
    }

    fn wrap_id(&self) -> u8 {
        WRAP_NONE_SANDBOX
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "a test that cannot fail loudly is not a test"
    )]

    use super::SandboxWrapper;
    use crate::{DeviceIdentity, IdentitySecret, SEED_LEN, open_identity, seal_identity};

    fn an_identity() -> DeviceIdentity {
        DeviceIdentity::from_secret(&IdentitySecret::from_bytes(&[0x5au8; SEED_LEN]))
    }

    #[test]
    fn a_sandbox_blob_round_trips_and_says_so_in_its_header() {
        let identity = an_identity();
        let blob = seal_identity(&identity, &SandboxWrapper).expect("sealing under the sandbox");

        assert_eq!(
            blob[9], 4,
            "the header must record that nothing wrapped this"
        );
        assert_eq!(
            open_identity(&blob, &SandboxWrapper)
                .expect("a sandbox blob opens")
                .fingerprint(),
            identity.fingerprint()
        );
    }

    #[test]
    fn the_seed_really_is_in_the_file_and_the_test_says_so_out_loud() {
        // Not a curiosity: this is the security property, asserted, so that
        // nobody reads `SandboxWrapper` as "some lighter encryption". The seed
        // is present verbatim. If this test ever fails because the wrapper
        // started transforming something, the threat model changed and the
        // sentence about root in THREAT_MODEL.md has to change with it.
        let secret = [0x5au8; SEED_LEN];
        let blob = seal_identity(&an_identity(), &SandboxWrapper).unwrap();
        assert!(
            blob.windows(SEED_LEN).any(|window| window == secret),
            "the sandbox wrapper stores the seed verbatim, by design"
        );
    }

    #[test]
    fn a_dpapi_blob_is_not_opened_by_the_sandbox_wrapper() {
        // The `wrap` byte mismatch must bite before any unwrap runs, or a build
        // that lost its platform wrapper would silently read a protected blob
        // as an unprotected one.
        let mut blob = seal_identity(&an_identity(), &SandboxWrapper).unwrap();
        blob[9] = 1;
        assert!(
            open_identity(&blob, &SandboxWrapper).is_err(),
            "a blob labelled DPAPI must not open through the sandbox wrapper"
        );
    }
}
