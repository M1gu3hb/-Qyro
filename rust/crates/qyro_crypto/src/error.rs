//! Typed identity failures.
//!
//! Diagnostics never contain key material, not even truncated. A caller that
//! logs an error must not thereby log a secret.

use core::fmt;

/// Why an identity operation failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum IdentityError {
    /// The system CSPRNG was unavailable.
    ///
    /// Generation fails rather than falling back to weaker entropy.
    EntropyUnavailable,
    /// A public key was not the expected length.
    InvalidPublicKeyLength {
        /// Length supplied.
        found: usize,
        /// Length required.
        expected: usize,
    },
    /// The bytes were the right length but not a valid Ed25519 point.
    MalformedPublicKey,
    /// The key is a valid point of low order.
    ///
    /// One of the eight small-order points. A signature made under such a key
    /// verifies for almost any message, so accepting one would let a peer
    /// present an identity that authenticates nothing.
    WeakPublicKey,
    /// A signature was not the expected length.
    InvalidSignatureLength {
        /// Length supplied.
        found: usize,
        /// Length required.
        expected: usize,
    },
    /// The signature did not verify for this key, domain and message.
    ///
    /// Deliberately one variant: distinguishing "wrong key" from "tampered
    /// message" would tell an attacker which half to keep changing.
    SignatureVerificationFailed,
    /// A domain reserved for a future version was used.
    DomainNotAvailable {
        /// Wire id of the reserved domain.
        domain: u8,
    },
    /// An identity version this build does not implement.
    UnsupportedVersion {
        /// Version found.
        found: u8,
        /// Version implemented.
        supported: u8,
    },
    /// A fingerprint string was not in the canonical representation.
    MalformedFingerprint,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntropyUnavailable => {
                formatter.write_str("the system random number generator was unavailable")
            }
            Self::InvalidPublicKeyLength { found, expected } => {
                write!(
                    formatter,
                    "public key is {found} bytes, expected {expected}"
                )
            }
            Self::MalformedPublicKey => formatter.write_str("public key is not a valid point"),
            Self::WeakPublicKey => {
                formatter.write_str("public key has low order and authenticates nothing")
            }
            Self::InvalidSignatureLength { found, expected } => {
                write!(formatter, "signature is {found} bytes, expected {expected}")
            }
            Self::SignatureVerificationFailed => formatter.write_str("signature did not verify"),
            Self::DomainNotAvailable { domain } => {
                write!(
                    formatter,
                    "signature domain {domain} is reserved for a future version"
                )
            }
            Self::UnsupportedVersion { found, supported } => write!(
                formatter,
                "identity version {found} is not supported, this build implements {supported}"
            ),
            Self::MalformedFingerprint => {
                formatter.write_str("fingerprint is not in the canonical representation")
            }
        }
    }
}

impl core::error::Error for IdentityError {}
