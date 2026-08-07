//! Device identity: the signing key and its public half.

// Nothing on these paths may end the process. Every byte that reaches this
// module was chosen by a peer — a hello, a finish message, a public key — so a
// panic here is a remote denial of service, and in code that holds keys it is
// also an abort in the middle of something that was about to zeroize.
//
// The compiler enforces it rather than a regular expression, because a regular
// expression cannot tell a `panic!` in a doc comment from one in a match arm.
// `src/guards.rs` covers what the lint cannot: a module nobody added this
// attribute to, and `assert!`, which has no lint at all.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use core::fmt;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use zeroize::Zeroizing;

use crate::error::IdentityError;
use crate::fingerprint::IdentityFingerprint;
use crate::signature::{IdentitySignature, SignatureDomain};

/// Identity format version, folded into the fingerprint.
pub const IDENTITY_VERSION: u8 = 1;

/// Bytes in an Ed25519 public key.
pub const PUBLIC_KEY_LEN: usize = 32;

/// Bytes in the wire encoding of a [`PublicIdentity`]: version plus key.
///
/// Byte 0 is the identity version, bytes 1..33 are the Ed25519 public key. The
/// version travels with the key rather than being agreed out of band, so a peer
/// can never be handed 32 bytes and left to assume which format they are in.
pub const PUBLIC_IDENTITY_WIRE_LEN: usize = 1 + PUBLIC_KEY_LEN;

/// Bytes in the seed a signing key is derived from.
pub const SEED_LEN: usize = 32;

/// The secret half of a device identity, in transit to or from a secure store.
///
/// Exists so that a seed never travels as a bare `[u8; 32]`. It zeroizes on
/// drop, is not `Clone` — two copies is one more than anyone can account for —
/// is not serializable, and its `Debug` is redacted, because the shortest path
/// from a secret to a log file is a `{:?}` in an error message.
pub struct IdentitySecret {
    seed: Zeroizing<[u8; SEED_LEN]>,
}

impl IdentitySecret {
    /// Wraps raw seed bytes coming back out of a store.
    #[must_use]
    pub fn from_bytes(bytes: &[u8; SEED_LEN]) -> Self {
        Self {
            seed: Zeroizing::new(*bytes),
        }
    }

    /// Borrows the seed so a store can wrap it.
    ///
    /// Borrowed rather than returned by value: handing back an owned array
    /// would put a copy outside this type's `Drop`, which is the one thing it
    /// exists to prevent.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; SEED_LEN] {
        &self.seed
    }
}

impl fmt::Debug for IdentitySecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("IdentitySecret(redacted)")
    }
}

/// A device's own identity, including its signing key.
///
/// Deliberately not `Clone`, `Copy` or serializable: a secret that can be copied
/// freely is a secret that ends up somewhere nobody audited.
///
/// Until sprint 4D.1 this paragraph continued "there is **no accessor for the
/// seed or the private key**", and that was the whole protection. Persisting an
/// identity makes it impossible to keep: a secure store cannot store what it
/// cannot read. [`DeviceIdentity::export_secret`] is the one way out and
/// [`DeviceIdentity::from_secret`] the one way back in, both public because the
/// store lives in another crate — ADR-0024 §4 argues why an enumerable public
/// path was preferred to relaxing `forbid(unsafe_code)` in this crate, and does
/// not pretend the choice was free.
///
/// What still contains it is ownership: you must hold a `DeviceIdentity`, and
/// one only comes from [`DeviceIdentity::generate`] or from a store.
///
/// The signing key zeroizes on drop.
pub struct DeviceIdentity {
    signing_key: SigningKey,
    public: PublicIdentity,
}

impl DeviceIdentity {
    /// Generates a new identity from the system CSPRNG.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::EntropyUnavailable`] when the operating system
    /// cannot supply randomness. There is no weaker fallback: an identity from
    /// predictable entropy is worse than no identity.
    pub fn generate() -> Result<Self, IdentityError> {
        // Zeroizing so the seed is wiped even though SigningKey copies it.
        let mut seed = Zeroizing::new([0u8; SEED_LEN]);
        getrandom::fill(seed.as_mut()).map_err(|_| IdentityError::EntropyUnavailable)?;
        Ok(Self::from_seed(&seed))
    }

    /// Builds an identity from a fixed seed. **Test vectors only.**
    ///
    /// Crate-private and `cfg(any(test, fuzzing))`, so it does not exist in an
    /// ordinary build. The documentation said `cfg(test)` alone until sprint
    /// 4C.2, which understated the attribute by one condition (QYR-0031): the
    /// fuzz targets need a deterministic identity, and `--cfg fuzzing` is set on
    /// the command line by `cargo-fuzz` and by nothing else — see
    /// [`crate::fuzzing`]. It used to be `pub` behind a non-default
    /// `test-vectors` feature, which still put a deterministic constructor in
    /// the public API: features are additive, so any crate anywhere in a
    /// dependency graph could switch it on for everybody and no release build
    /// could prove it was off. Production entropy comes from
    /// [`DeviceIdentity::generate`] and nowhere else.
    #[cfg(any(test, fuzzing))]
    pub(crate) fn from_test_seed(seed: &[u8; SEED_LEN]) -> Self {
        Self::from_seed(seed)
    }

    fn from_seed(seed: &[u8; SEED_LEN]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let public = PublicIdentity::from_verifying_key(signing_key.verifying_key());
        Self {
            signing_key,
            public,
        }
    }

    /// Hands out the secret half, for a secure store and nothing else.
    ///
    /// This is one of exactly two public paths in this crate that return key
    /// material; `src/guards.rs` enumerates them by name and fails if a third
    /// appears. See ADR-0024 §4.
    ///
    /// The seed is reconstructed from the signing key rather than kept beside
    /// it, so this type still holds one copy of the secret and not two.
    #[must_use]
    pub fn export_secret(&self) -> IdentitySecret {
        IdentitySecret {
            seed: Zeroizing::new(self.signing_key.to_bytes()),
        }
    }

    /// Rebuilds an identity from a secret a store handed back.
    ///
    /// Infallible by construction: every 32-byte string is a valid Ed25519
    /// seed. That is a property of the algorithm, not an assumption — it is the
    /// same reason `[0xFF; 32]` turned out to be a perfectly valid key when the
    /// sprint 4A guard tried to reject it for looking weak.
    #[must_use]
    pub fn from_secret(secret: &IdentitySecret) -> Self {
        Self::from_seed(secret.as_bytes())
    }

    /// Returns the public half, which is safe to share.
    #[must_use]
    pub const fn public_identity(&self) -> &PublicIdentity {
        &self.public
    }

    /// Returns this device's fingerprint.
    #[must_use]
    pub const fn fingerprint(&self) -> &IdentityFingerprint {
        self.public.fingerprint()
    }

    /// Signs a message within a domain.
    ///
    /// The message is never signed bare; see [`SignatureDomain`].
    ///
    /// This is the only way to sign. There was also an infallible `sign` that
    /// unwrapped this one, on the reasoning that a caller passing a literal
    /// domain knows it is available. That reasoning does not survive contact
    /// with a later version: making an available domain reserved, or adding a
    /// reserved one, silently turns every such call site into a panic. A
    /// library on a security path should not offer the caller a way to crash
    /// instead of deciding.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::DomainNotAvailable`] for a domain reserved for a
    /// future version.
    pub fn try_sign(
        &self,
        domain: SignatureDomain,
        message: &[u8],
    ) -> Result<IdentitySignature, IdentityError> {
        domain.ensure_available()?;
        let input = domain.signing_input(message);
        let signature = self.signing_key.sign(&input);
        Ok(IdentitySignature::from_bytes(signature.to_bytes()))
    }
}

impl fmt::Debug for DeviceIdentity {
    /// Prints the public fingerprint and a fixed marker; never key material.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeviceIdentity")
            .field("fingerprint", &self.public.fingerprint())
            .field("signing_key", &"<redacted>")
            .finish()
    }
}

/// The public half of a device identity.
#[derive(Clone, Eq, PartialEq)]
pub struct PublicIdentity {
    version: u8,
    verifying_key: VerifyingKey,
    fingerprint: IdentityFingerprint,
}

impl PublicIdentity {
    fn from_verifying_key(verifying_key: VerifyingKey) -> Self {
        let fingerprint = IdentityFingerprint::compute(IDENTITY_VERSION, verifying_key.as_bytes());
        Self {
            version: IDENTITY_VERSION,
            verifying_key,
            fingerprint,
        }
    }

    /// Parses a public identity from its canonical 32-byte key encoding.
    ///
    /// Low-order keys are refused. All eight small-order points are valid
    /// Ed25519 encodings, so nothing about the byte pattern gives them away —
    /// `[0u8; 32]` is one of them and was accepted before this check. A
    /// signature under such a key verifies for almost any message, so a peer
    /// presenting one would hold an identity that authenticates nothing while
    /// looking exactly like an identity that does.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::InvalidPublicKeyLength`] for a wrong length,
    /// [`IdentityError::MalformedPublicKey`] when the bytes are not a valid
    /// Ed25519 point, or [`IdentityError::WeakPublicKey`] when the point has
    /// low order.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IdentityError> {
        let array: [u8; PUBLIC_KEY_LEN] =
            bytes
                .try_into()
                .map_err(|_| IdentityError::InvalidPublicKeyLength {
                    found: bytes.len(),
                    expected: PUBLIC_KEY_LEN,
                })?;
        let verifying_key =
            VerifyingKey::from_bytes(&array).map_err(|_| IdentityError::MalformedPublicKey)?;
        if verifying_key.is_weak() {
            return Err(IdentityError::WeakPublicKey);
        }
        Ok(Self::from_verifying_key(verifying_key))
    }

    /// Serializes to the canonical 33-byte wire form: version then key.
    #[must_use]
    pub fn encode(&self) -> [u8; PUBLIC_IDENTITY_WIRE_LEN] {
        let mut out = [0u8; PUBLIC_IDENTITY_WIRE_LEN];
        out[0] = self.version;
        out[1..].copy_from_slice(self.verifying_key.as_bytes());
        out
    }

    /// Parses the canonical 33-byte wire form.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::InvalidPublicKeyLength`] when `bytes` is not
    /// exactly [`PUBLIC_IDENTITY_WIRE_LEN`] long, plus the errors of
    /// [`PublicIdentity::from_versioned_bytes`].
    pub fn decode(bytes: &[u8]) -> Result<Self, IdentityError> {
        let encoded: &[u8; PUBLIC_IDENTITY_WIRE_LEN] =
            bytes
                .try_into()
                .map_err(|_| IdentityError::InvalidPublicKeyLength {
                    found: bytes.len(),
                    expected: PUBLIC_IDENTITY_WIRE_LEN,
                })?;
        // Irrefutable on a fixed-width array: the version byte and the key are
        // split by the type, not by an index that has to stay right.
        let [version, key @ ..] = encoded;
        Self::from_versioned_bytes(*version, key)
    }

    /// Parses a versioned public identity.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::UnsupportedVersion`] when `version` is not
    /// [`IDENTITY_VERSION`], plus the errors of [`PublicIdentity::from_bytes`].
    pub fn from_versioned_bytes(version: u8, bytes: &[u8]) -> Result<Self, IdentityError> {
        if version != IDENTITY_VERSION {
            return Err(IdentityError::UnsupportedVersion {
                found: version,
                supported: IDENTITY_VERSION,
            });
        }
        Self::from_bytes(bytes)
    }

    /// Returns the identity format version.
    #[must_use]
    pub const fn version(&self) -> u8 {
        self.version
    }

    /// Returns the canonical 32-byte public key.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; PUBLIC_KEY_LEN] {
        self.verifying_key.as_bytes()
    }

    /// Returns the fingerprint.
    #[must_use]
    pub const fn fingerprint(&self) -> &IdentityFingerprint {
        &self.fingerprint
    }

    /// Verifies a signature made in `domain` over `message`.
    ///
    /// Uses `verify_strict`, not the permissive `verify`. Strict verification
    /// rejects non-canonical `R` values and signatures with a small torsion
    /// component, which the looser check accepts. That difference matters
    /// wherever a signature is treated as an identifier rather than as a
    /// yes-or-no answer: two distinct signatures that both verify over the same
    /// message let a peer present "the same" statement twice in forms that
    /// compare unequal.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::DomainNotAvailable`] for a reserved domain, or
    /// [`IdentityError::SignatureVerificationFailed`] when the signature does
    /// not verify. The failure is one variant on purpose: telling a caller
    /// whether the key or the message was wrong helps an attacker narrow down
    /// which half to keep changing.
    pub fn verify(
        &self,
        domain: SignatureDomain,
        message: &[u8],
        signature: &IdentitySignature,
    ) -> Result<(), IdentityError> {
        domain.ensure_available()?;
        let input = domain.signing_input(message);
        let parsed = Signature::from_bytes(signature.as_bytes());
        self.verifying_key
            .verify_strict(&input, &parsed)
            .map_err(|_| IdentityError::SignatureVerificationFailed)
    }
}

impl fmt::Debug for PublicIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PublicIdentity")
            .field("version", &self.version)
            .field("fingerprint", &self.fingerprint)
            .finish()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]
mod tests {
    use super::*;

    #[test]
    fn an_exported_secret_rebuilds_the_same_identity() {
        // The property a store depends on. If this ever stopped holding,
        // persisting would silently return a *different* identity, which is
        // worse than failing to persist at all: the fingerprint a peer trusted
        // would change without anything reporting an error.
        let original = DeviceIdentity::from_test_seed(&[7u8; SEED_LEN]);
        let secret = original.export_secret();
        let restored = DeviceIdentity::from_secret(&secret);

        assert_eq!(original.fingerprint(), restored.fingerprint());
        assert_eq!(
            original.public_identity().encode(),
            restored.public_identity().encode()
        );
    }

    #[test]
    fn an_exported_secret_is_the_seed_the_identity_was_built_from() {
        // Checked against the seed itself rather than against another export,
        // so the test cannot pass by agreeing with its own output — the mistake
        // QYR-0025 recorded about verifying vectors from the module that
        // produced them.
        let seed = [0x5Au8; SEED_LEN];
        let identity = DeviceIdentity::from_test_seed(&seed);
        assert_eq!(identity.export_secret().as_bytes(), &seed);
    }

    #[test]
    fn a_secret_does_not_print_itself() {
        // The shortest path from a secret to a log file is a `{:?}` inside an
        // error message somebody added in a hurry.
        let secret = IdentitySecret::from_bytes(&[0xABu8; SEED_LEN]);
        let rendered = format!("{secret:?}");
        assert_eq!(rendered, "IdentitySecret(redacted)");
        assert!(!rendered.contains("ab"), "{rendered}");
        assert!(!rendered.contains("171"), "{rendered}");
    }

    #[test]
    fn every_thirty_two_byte_string_is_a_usable_seed() {
        // `from_secret` is infallible by construction. The sprint 4A guard that
        // tried to reject `[0xFF; 32]` for looking weak is why this is a test
        // and not a comment: a byte pattern is not a weak key.
        for pattern in [0x00u8, 0x01, 0x7F, 0x80, 0xFF] {
            let secret = IdentitySecret::from_bytes(&[pattern; SEED_LEN]);
            let identity = DeviceIdentity::from_secret(&secret);
            assert_eq!(identity.export_secret().as_bytes(), &[pattern; SEED_LEN]);
        }
    }
}
