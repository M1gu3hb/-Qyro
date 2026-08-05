//! Typed handshake failures.
//!
//! As with [`crate::IdentityError`], diagnostics never carry key material. They
//! also never distinguish *which* cryptographic check failed beyond the
//! category: a peer learns that its signature did not verify, not which half of
//! the transcript to keep adjusting.

use core::fmt;

/// Why a handshake step failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum HandshakeError {
    /// The peer speaks a handshake version this build does not implement.
    ///
    /// Refused, never downgraded. Negotiating is how an attacker in the middle
    /// picks the weakest option both sides will accept.
    UnsupportedHandshakeVersion {
        /// Version the peer declared.
        found: u8,
        /// Version this build implements.
        supported: u8,
    },
    /// The peer named a cryptographic suite this build does not implement.
    UnsupportedCryptoSuite {
        /// Suite the peer declared.
        found: u8,
        /// Suite this build implements.
        supported: u8,
    },
    /// A message arrived at the state belonging to the other role.
    UnexpectedRole,
    /// A well-formed message of the wrong kind arrived.
    UnexpectedMessage {
        /// Message type found.
        found: u8,
        /// Message type this step expects.
        expected: u8,
    },
    /// The state machine was driven out of order.
    ///
    /// Mostly unreachable by construction: states are consumed, so the
    /// compiler rejects reuse. The variant exists for the paths types cannot
    /// cover.
    InvalidState,
    /// A message was not the fixed length its kind requires.
    InvalidMessageLength {
        /// Length supplied.
        found: usize,
        /// Length this message kind always has.
        expected: usize,
    },
    /// A public identity inside a message could not be parsed.
    InvalidPublicIdentity,
    /// A public identity inside a message was a low-order point.
    WeakPublicIdentity,
    /// An ephemeral X25519 key was not a usable public key.
    InvalidEphemeralPublicKey,
    /// The X25519 exchange produced a non-contributory shared secret.
    ///
    /// The peer sent a low-order point, so the shared secret is all zeros and
    /// both sides "agree" without either having contributed anything.
    NonContributorySharedSecret,
    /// A signature over the transcript did not verify.
    SignatureVerificationFailed,
    /// A confirmation MAC did not verify.
    ///
    /// Distinct from a signature failure because it means something different:
    /// the peer proved its identity but derived different keys.
    FinishedVerificationFailed,
    /// A recomputed transcript did not match the one in hand.
    TranscriptMismatch,
    /// The system CSPRNG was unavailable.
    EntropyUnavailable,
    /// The key schedule could not produce output.
    KeyDerivationFailed,
    /// Messages arrived in an order this role does not accept.
    SequenceViolation,
    /// A message carried bytes beyond its fixed length.
    TrailingBytes {
        /// How many bytes were left over.
        extra: usize,
    },
}

impl fmt::Display for HandshakeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedHandshakeVersion { found, supported } => write!(
                formatter,
                "handshake version {found} is not supported, this build implements {supported}"
            ),
            Self::UnsupportedCryptoSuite { found, supported } => write!(
                formatter,
                "crypto suite {found} is not supported, this build implements {supported}"
            ),
            Self::UnexpectedRole => formatter.write_str("message arrived at the wrong role"),
            Self::UnexpectedMessage { found, expected } => write!(
                formatter,
                "message type {found} arrived where type {expected} was expected"
            ),
            Self::InvalidState => formatter.write_str("the handshake is not in a state to do this"),
            Self::InvalidMessageLength { found, expected } => write!(
                formatter,
                "message is {found} bytes, this kind is always {expected}"
            ),
            Self::InvalidPublicIdentity => {
                formatter.write_str("the public identity in the message could not be parsed")
            }
            Self::WeakPublicIdentity => {
                formatter.write_str("the public identity in the message has low order")
            }
            Self::InvalidEphemeralPublicKey => {
                formatter.write_str("the ephemeral key is not a usable public key")
            }
            Self::NonContributorySharedSecret => {
                formatter.write_str("the key exchange was non-contributory")
            }
            Self::SignatureVerificationFailed => {
                formatter.write_str("the transcript signature did not verify")
            }
            Self::FinishedVerificationFailed => {
                formatter.write_str("the confirmation MAC did not verify")
            }
            Self::TranscriptMismatch => formatter.write_str("the transcripts do not match"),
            Self::EntropyUnavailable => {
                formatter.write_str("the system random number generator was unavailable")
            }
            Self::KeyDerivationFailed => formatter.write_str("key derivation failed"),
            Self::SequenceViolation => {
                formatter.write_str("handshake messages arrived out of order")
            }
            Self::TrailingBytes { extra } => {
                write!(formatter, "{extra} bytes remained after the message")
            }
        }
    }
}

impl core::error::Error for HandshakeError {}
