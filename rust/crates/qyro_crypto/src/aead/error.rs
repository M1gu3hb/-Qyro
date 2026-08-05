//! Typed frame-crypto failures.
//!
//! As everywhere else in this crate, a diagnostic never carries key material,
//! plaintext or ciphertext — only the framing facts a peer could already see.
//!
//! Every variant below is reachable and a test produces it. Three that ADR-0022
//! listed are deliberately absent, because nothing can cause them: an
//! `EncryptedEnvelope` cannot exist without the `ENCRYPTED` flag, whichever of
//! its two constructors built it; a `Frame` cannot hold a payload past
//! `MAX_PAYLOAD_LEN`, so sealing one cannot overflow a frame; and "the nonce
//! state is unusable" was `SequenceExhausted` under a second name. An error
//! nobody can provoke documents a check that is not there.

use core::fmt;

/// Why a frame could not be sealed or opened.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AeadError {
    /// The frame names a session this pair of keys does not belong to.
    ///
    /// Checked before the AEAD runs and before the replay window is touched: a
    /// frame for someone else's session must cost this session nothing.
    WrongSession,
    /// This sequence has already been accepted in this direction.
    ReplayDetected {
        /// The sequence that arrived a second time.
        sequence: u64,
    },
    /// The sequence fell behind the replay window and cannot be judged.
    ///
    /// Not necessarily an attack — a badly delayed frame looks the same — but
    /// once it is out of the window there is no way to tell it from a replay,
    /// and guessing in the accepting direction is the wrong guess to make.
    SequenceTooOld {
        /// The sequence that arrived.
        sequence: u64,
        /// How many sequences the window covers.
        window: u64,
    },
    /// The tag did not verify for this key, nonce, header and ciphertext.
    ///
    /// Deliberately one variant: distinguishing a wrong tag from an altered
    /// header would tell an attacker which half to keep changing.
    AuthenticationFailed,
    /// The trailer was not a ChaCha20-Poly1305 tag.
    InvalidTagLength {
        /// Trailer length found.
        found: usize,
        /// Trailer length this suite requires.
        expected: usize,
    },
    /// Every sequence in this direction has been used.
    ///
    /// Terminal. The counter does not wrap, because a repeated nonce on a
    /// stream cipher reveals the XOR of the two plaintexts.
    SequenceExhausted,
    /// The key schedule could not produce output.
    KeyDerivationFailed,
}

impl fmt::Display for AeadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongSession => formatter.write_str("the frame belongs to another session"),
            Self::ReplayDetected { sequence } => {
                write!(formatter, "sequence {sequence} was already accepted")
            }
            Self::SequenceTooOld { sequence, window } => write!(
                formatter,
                "sequence {sequence} is older than the {window}-frame replay window"
            ),
            Self::AuthenticationFailed => formatter.write_str("the frame did not authenticate"),
            Self::InvalidTagLength { found, expected } => write!(
                formatter,
                "authentication tag is {found} bytes, this suite uses {expected}"
            ),
            Self::SequenceExhausted => {
                formatter.write_str("every sequence in this direction has been used")
            }
            Self::KeyDerivationFailed => formatter.write_str("key derivation failed"),
        }
    }
}

impl core::error::Error for AeadError {}
