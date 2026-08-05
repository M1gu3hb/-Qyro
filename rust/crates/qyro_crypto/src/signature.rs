//! Domain-separated signatures.

use core::fmt;

use crate::error::IdentityError;

/// Bytes in an Ed25519 signature.
pub const SIGNATURE_LEN: usize = 64;

/// Prefix that scopes every signature this crate produces.
const SIGNING_PREFIX: &[u8] = b"QYRO-SIGN-V1";

/// What a signature is *for*.
///
/// Nothing signs a bare message. The bytes actually passed to Ed25519 are:
///
/// ```text
/// "QYRO-SIGN-V1" || 0x00 || domain (u8) || len(message) (u64 BE) || message
/// ```
///
/// The `0x00` separator and the explicit length stop two different
/// (domain, message) pairs from producing the same signed bytes. Without them an
/// attacker-chosen message could reproduce another domain's prefix and a
/// signature could be replayed into a context it was never meant for.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum SignatureDomain {
    /// Test vectors only. Never used by product code paths.
    TestVector = 1,
    /// A device asserting something about itself.
    DeviceClaim = 2,
    /// Handshake transcript. **Reserved**: rejected until the handshake exists.
    HandshakeTranscript = 3,
}

impl SignatureDomain {
    /// Returns the stable wire id.
    #[must_use]
    pub const fn to_wire(self) -> u8 {
        self as u8
    }

    /// Whether this build may sign or verify in this domain.
    ///
    /// `HandshakeTranscript` is reserved so its transcript format can be frozen
    /// with the handshake itself. Allowing signatures in it now would mean
    /// committing to a format nothing has validated.
    #[must_use]
    pub const fn is_available(self) -> bool {
        match self {
            Self::TestVector | Self::DeviceClaim => true,
            Self::HandshakeTranscript => false,
        }
    }

    /// Errors when the domain is reserved.
    pub(crate) const fn ensure_available(self) -> Result<(), IdentityError> {
        if self.is_available() {
            Ok(())
        } else {
            Err(IdentityError::DomainNotAvailable {
                domain: self.to_wire(),
            })
        }
    }

    /// Builds the exact bytes handed to Ed25519.
    ///
    /// Exposed within the crate so the test vectors can record them: an
    /// implementation in another language must be able to reproduce this
    /// byte-for-byte.
    pub(crate) fn signing_input(self, message: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(SIGNING_PREFIX.len() + 1 + 1 + 8 + message.len());
        out.extend_from_slice(SIGNING_PREFIX);
        out.push(0x00);
        out.push(self.to_wire());
        out.extend_from_slice(&(message.len() as u64).to_be_bytes());
        out.extend_from_slice(message);
        out
    }
}

/// A 64-byte Ed25519 signature.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct IdentitySignature([u8; SIGNATURE_LEN]);

impl IdentitySignature {
    /// Wraps raw signature bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; SIGNATURE_LEN]) -> Self {
        Self(bytes)
    }

    /// Parses a signature from a slice.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::InvalidSignatureLength`] for any other length.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, IdentityError> {
        let array: [u8; SIGNATURE_LEN] =
            bytes
                .try_into()
                .map_err(|_| IdentityError::InvalidSignatureLength {
                    found: bytes.len(),
                    expected: SIGNATURE_LEN,
                })?;
        Ok(Self(array))
    }

    /// Returns the raw bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; SIGNATURE_LEN] {
        &self.0
    }

    /// Returns the lowercase hex representation.
    #[must_use]
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

impl fmt::Debug for IdentitySignature {
    /// Signatures are public, so printing them leaks nothing.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "IdentitySignature({})", self.to_hex())
    }
}
