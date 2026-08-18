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
//!
//! The four invariant variants added in sprint 4C.1 are the opposite case. They
//! replace `unreachable!`, `assert_eq!` and array indexing on the production
//! path — places where the code was certain of something and ended the process
//! if it turned out to be wrong. They are reachable only through the crate's own
//! fault injection, which is `cfg(test)`, and that is the point: an invariant
//! nobody can violate still must not be enforced with a crash, because `open` is
//! driven by bytes a peer chose.

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
    /// The sealer could not build the plain template it authenticates over.
    ///
    /// The framing layer refused a frame this module constructed from a frame
    /// it had already accepted, which means the two layers disagree about what
    /// is valid. Nothing is sealed and the sequence is not consumed.
    FrameTemplateRejected,
    /// The envelope layer refused the ciphertext and tag.
    ///
    /// Reached only after the AEAD has run, so the nonce is spent: the sealer is
    /// poisoned rather than allowed to try again on the same sequence.
    EnvelopeConstructionFailed,
    /// The header that went on the wire is not the header the tag covered.
    ///
    /// A tag computed over a header that is not the header a peer receives
    /// authenticates nothing. Checked rather than assumed, and terminal.
    AssociatedDataMismatch,
    /// The replay window was asked about a slot outside itself.
    ///
    /// Every caller proves the offset is in range before asking. This exists so
    /// that a wrong proof is an error rather than an index panic on a path a
    /// peer's sequence number reaches.
    ReplayStateCorrupt,
    /// The sealer stopped after an internal invariant failed.
    ///
    /// Terminal and deliberate. Once a nonce may or may not have been used, no
    /// answer to "may I seal again" is safe except no.
    SealerPoisoned,
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
            Self::FrameTemplateRejected => {
                formatter.write_str("the framing layer refused the sealer's own template")
            }
            Self::EnvelopeConstructionFailed => {
                formatter.write_str("the envelope layer refused the sealed frame")
            }
            Self::AssociatedDataMismatch => {
                formatter.write_str("the sealed header is not the header the tag covered")
            }
            Self::ReplayStateCorrupt => {
                formatter.write_str("the replay window was asked about a slot outside itself")
            }
            Self::SealerPoisoned => {
                formatter.write_str("this sealer stopped after an internal failure")
            }
        }
    }
}

impl core::error::Error for AeadError {}
