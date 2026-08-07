//! Typed handshake failures.
//!
//! As with [`crate::IdentityError`], diagnostics never carry key material. They
//! also never distinguish *which* cryptographic check failed beyond the
//! category: a peer learns that its signature did not verify, not which half of
//! the transcript to keep adjusting.
//!
//! # Every variant below is produced by something
//!
//! Four were not, and are gone: `UnexpectedRole`, `InvalidEphemeralPublicKey`,
//! `TranscriptMismatch` and `SequenceViolation`. Nothing anywhere constructed
//! them, so a caller could match on a check that did not exist and conclude the
//! handshake enforced something it did not.
//!
//! Each had a reason it could not fire. Role confusion and out-of-order
//! messages are impossible by construction — every transition takes `self` by
//! value, so the compiler rejects reuse and reordering. An X25519 public key
//! has no invalid encoding: every 32-byte string is a point, and the hazard
//! that does exist is a low-order one, reported as
//! [`HandshakeError::NonContributorySharedSecret`]. A transcript is never
//! compared, only signed and MACed over, so a disagreement surfaces as
//! [`HandshakeError::SignatureVerificationFailed`] or
//! [`HandshakeError::FinishedVerificationFailed`].
//!
//! `crate::guards` keeps this true: a variant with no construction site
//! anywhere in the crate fails the test run. Recorded as an amendment to
//! ADR-0021.

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
    /// The system CSPRNG was unavailable.
    EntropyUnavailable,
    /// The key schedule could not produce output.
    KeyDerivationFailed,
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
            Self::NonContributorySharedSecret => {
                formatter.write_str("the key exchange was non-contributory")
            }
            Self::SignatureVerificationFailed => {
                formatter.write_str("the transcript signature did not verify")
            }
            Self::FinishedVerificationFailed => {
                formatter.write_str("the confirmation MAC did not verify")
            }
            Self::EntropyUnavailable => {
                formatter.write_str("the system random number generator was unavailable")
            }
            Self::KeyDerivationFailed => formatter.write_str("key derivation failed"),
            Self::TrailingBytes { extra } => {
                write!(formatter, "{extra} bytes remained after the message")
            }
        }
    }
}

impl core::error::Error for HandshakeError {}
