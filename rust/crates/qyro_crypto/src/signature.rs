//! Domain-separated signatures.

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
    /// Handshake transcript.
    ///
    /// Reserved by ADR-0020 and unreserved by ADR-0021, which freezes the
    /// transcript this domain signs over.
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
    /// All three are available. `HandshakeTranscript` was reserved while its
    /// transcript format was undefined — signing in a domain whose meaning
    /// nothing has fixed commits to a format by accident — and ADR-0021 fixed
    /// it. The predicate stays because a future version will add domains before
    /// it implements them, and a reserved domain must fail loudly rather than
    /// produce a signature nobody can interpret.
    #[must_use]
    pub const fn is_available(self) -> bool {
        match self {
            Self::TestVector | Self::DeviceClaim | Self::HandshakeTranscript => true,
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
