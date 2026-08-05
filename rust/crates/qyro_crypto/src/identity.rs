//! Device identity: the signing key and its public half.

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
const SEED_LEN: usize = 32;

/// A device's own identity, including its signing key.
///
/// Deliberately not `Clone`, `Copy` or serializable: a secret that can be copied
/// freely is a secret that ends up somewhere nobody audited. There is **no
/// accessor for the seed or the private key**; persisting an identity is the job
/// of a secure store, which does not exist yet.
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
    /// Crate-private and `cfg(test)`, so it does not exist in any build that is
    /// not the test build. It used to be `pub` behind a non-default
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
        if bytes.len() != PUBLIC_IDENTITY_WIRE_LEN {
            return Err(IdentityError::InvalidPublicKeyLength {
                found: bytes.len(),
                expected: PUBLIC_IDENTITY_WIRE_LEN,
            });
        }
        Self::from_versioned_bytes(bytes[0], &bytes[1..])
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
