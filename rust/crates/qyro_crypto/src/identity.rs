//! Device identity: the signing key and its public half.

use core::fmt;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use zeroize::Zeroizing;

use crate::error::IdentityError;
use crate::fingerprint::IdentityFingerprint;
use crate::signature::{IdentitySignature, SignatureDomain};

/// Identity format version, folded into the fingerprint.
pub const IDENTITY_VERSION: u8 = 1;

/// Bytes in an Ed25519 public key.
pub const PUBLIC_KEY_LEN: usize = 32;

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
    /// Behind the non-default `test-vectors` feature so a release build cannot
    /// reach it by accident. Production entropy comes from
    /// [`DeviceIdentity::generate`] and nowhere else.
    #[cfg(any(test, feature = "test-vectors"))]
    #[must_use]
    pub fn from_test_seed(seed: &[u8; SEED_LEN]) -> Self {
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

    /// Signs in an available domain.
    ///
    /// # Panics
    ///
    /// Panics if `domain` is reserved. Use [`DeviceIdentity::try_sign`] when the
    /// domain is not a compile-time constant.
    #[must_use]
    pub fn sign(&self, domain: SignatureDomain, message: &[u8]) -> IdentitySignature {
        self.try_sign(domain, message)
            .expect("the domain must be available in this version")
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

    /// Parses a public identity from its canonical 32-byte encoding.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::InvalidPublicKeyLength`] for a wrong length, or
    /// [`IdentityError::MalformedPublicKey`] when the bytes are not a valid
    /// Ed25519 point.
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
        Ok(Self::from_verifying_key(verifying_key))
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
            .verify(&input, &parsed)
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
