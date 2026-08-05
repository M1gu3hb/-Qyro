//! Authenticated encryption for QYRO/1 frames.
//!
//! Format, derivation, nonce rule and replay policy are frozen by
//! `docs/adr/ADR-0022-qyro1-frame-aead.md`. That document is the specification;
//! this module is one implementation of it. Where they disagree, the ADR is
//! right.
//!
//! # What this gives you
//!
//! [`FrameSealer::seal`] turns a plain [`Frame`] into a [`SealedFrame`] whose
//! tag a real ChaCha20-Poly1305 computed over the whole 48-byte header and the
//! ciphertext. [`FrameOpener::open`] turns an [`EncryptedEnvelope`] — which
//! asserts nothing, because it is bytes off a wire — into an
//! [`AuthenticatedFrame`], which asserts that the tag verified under this
//! session's key for this direction and that the sequence had not been seen.
//!
//! The two types with private constructors are the whole point. An envelope can
//! be built by anyone out of anything; a `SealedFrame` and an
//! `AuthenticatedFrame` can only come out of the two functions below.
//!
//! # What this is not
//!
//! No transport. There is no socket, no discovery and no file transfer, so
//! nothing carries these frames anywhere: sealing happens between two values in
//! one process. No rekey and no rotation either — a session uses one key per
//! direction until the sequence is exhausted.
//!
//! # Who decides what
//!
//! The caller chooses the message type, the transport flags, the three routing
//! identifiers and the plaintext. The sealer chooses the session identifier, the
//! sequence, the nonce and the tag, and it overwrites the first two if the
//! caller set them. That split is not a convenience: a caller that could pick a
//! sequence could repeat a nonce, and a repeated nonce on a stream cipher
//! reveals the XOR of the two plaintexts.

mod error;
mod replay;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod vectors;

use chacha20poly1305::{AeadInOut, ChaCha20Poly1305, KeyInit};
use hkdf::Hkdf;
use qyro_protocol::{EncryptedEnvelope, Flags, Frame, FrameHeader, MessageType, SessionId};
use sha2::Sha256;
use zeroize::{Zeroize, Zeroizing};

pub use error::AeadError;
pub use replay::REPLAY_WINDOW;

use replay::ReplayWindow;

/// Bytes in a ChaCha20-Poly1305 key.
pub const AEAD_KEY_LEN: usize = 32;

/// Bytes in a ChaCha20-Poly1305 nonce.
pub const NONCE_LEN: usize = 12;

/// Bytes in a Poly1305 authentication tag.
pub const TAG_LEN: usize = 16;

/// Bytes of the per-direction nonce prefix.
///
/// The remaining eight carry the sequence, so the prefix is whatever a 96-bit
/// nonce has left once a `u64` counter has taken its share.
pub const NONCE_PREFIX_LEN: usize = NONCE_LEN - 8;

/// Bytes of the handshake traffic secret each direction derives from.
pub(crate) const TRAFFIC_SECRET_LEN: usize = 32;

/// Bytes of the authenticated handshake transcript that enters every `info`.
pub(crate) const AUTH_TRANSCRIPT_LEN: usize = 32;

/// Prefix on every derivation label.
const LABEL_PREFIX: &[u8] = b"QYRO-AEAD-V1/";

/// What a derived value is for. One label, one purpose, no reuse.
const PURPOSE_KEY: &[u8] = b"key";
const PURPOSE_NONCE_PREFIX: &[u8] = b"nonce-prefix";

/// Which way traffic flows.
///
/// The direction goes **inside** the derivation label, so the two directions
/// cannot produce the same key even though they start from secrets the same
/// schedule derived. Neither `Clone` nor `Copy`: a direction is chosen once, when
/// a handshake is converted, and nothing has a reason to duplicate one.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Direction {
    /// Written by the initiator, read by the responder.
    InitiatorToResponder,
    /// Written by the responder, read by the initiator.
    ResponderToInitiator,
}

impl Direction {
    /// The label fragment this direction contributes.
    const fn label(&self) -> &'static [u8] {
        match self {
            Self::InitiatorToResponder => b"i2r",
            Self::ResponderToInitiator => b"r2i",
        }
    }
}

/// Builds the HKDF `info` for one direction and purpose.
///
/// `label || 0x00 || auth_transcript || session_id`, exactly as ADR-0022 fixes
/// it. The transcript and the session identifier are in every `info`, so two
/// sessions derive different keys even if some future defect ever repeated a
/// traffic secret.
fn info_for(
    direction: &Direction,
    purpose: &[u8],
    auth_transcript: &[u8; AUTH_TRANSCRIPT_LEN],
    session: SessionId,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(
        LABEL_PREFIX.len() + 4 + purpose.len() + 1 + AUTH_TRANSCRIPT_LEN + session.as_bytes().len(),
    );
    out.extend_from_slice(LABEL_PREFIX);
    out.extend_from_slice(direction.label());
    out.push(b'/');
    out.extend_from_slice(purpose);
    out.push(0x00);
    out.extend_from_slice(auth_transcript);
    out.extend_from_slice(session.as_bytes());
    out
}

/// The key and nonce prefix for one direction of one session.
///
/// The traffic secret the handshake derived is **not** used as an AEAD key. It
/// goes in as an HKDF pseudorandom key — it is already uniform, being the output
/// of the handshake's own expand — and only the expansion phase runs.
struct DirectionalKeys {
    key: Zeroizing<[u8; AEAD_KEY_LEN]>,
    nonce_prefix: [u8; NONCE_PREFIX_LEN],
}

impl DirectionalKeys {
    /// Derives both values for one direction.
    fn derive(
        traffic_secret: &[u8; TRAFFIC_SECRET_LEN],
        direction: &Direction,
        auth_transcript: &[u8; AUTH_TRANSCRIPT_LEN],
        session: SessionId,
    ) -> Result<Self, AeadError> {
        let hkdf = Hkdf::<Sha256>::from_prk(traffic_secret.as_slice())
            .map_err(|_| AeadError::KeyDerivationFailed)?;

        let mut key = Zeroizing::new([0u8; AEAD_KEY_LEN]);
        hkdf.expand(
            &info_for(direction, PURPOSE_KEY, auth_transcript, session),
            key.as_mut_slice(),
        )
        .map_err(|_| AeadError::KeyDerivationFailed)?;

        let mut nonce_prefix = [0u8; NONCE_PREFIX_LEN];
        hkdf.expand(
            &info_for(direction, PURPOSE_NONCE_PREFIX, auth_transcript, session),
            &mut nonce_prefix,
        )
        .map_err(|_| AeadError::KeyDerivationFailed)?;

        Ok(Self { key, nonce_prefix })
    }

    /// Builds the cipher for one operation.
    ///
    /// Rebuilt per frame rather than stored: `ChaCha20Poly1305` is a key in a
    /// wrapper, so this copies 32 bytes and does no key schedule.
    fn cipher(&self) -> ChaCha20Poly1305 {
        <ChaCha20Poly1305 as KeyInit>::new_from_slice(self.key.as_slice())
            .unwrap_or_else(|_| unreachable!("the derived key is exactly AEAD_KEY_LEN bytes"))
    }
}

impl Drop for DirectionalKeys {
    fn drop(&mut self) {
        // The key wipes itself; the prefix is a plain array and would not.
        self.nonce_prefix.zeroize();
    }
}

/// Builds the nonce for one sequence: `prefix || sequence`, big-endian.
///
/// A nonce is not a secret — it is derived rather than sent, and knowing it buys
/// an attacker nothing. What matters is that it never repeats, which is why the
/// sequence is assigned by the sealer and never wraps.
fn nonce_for(prefix: &[u8; NONCE_PREFIX_LEN], sequence: u64) -> [u8; NONCE_LEN] {
    let mut nonce = [0u8; NONCE_LEN];
    nonce[..NONCE_PREFIX_LEN].copy_from_slice(prefix);
    nonce[NONCE_PREFIX_LEN..].copy_from_slice(&sequence.to_be_bytes());
    nonce
}

// ------------------------------------------------------------------- sealing

/// Seals outbound frames for one session and one direction.
///
/// Owns the outbound key, the nonce prefix and the sequence counter, and is the
/// only thing that can produce a [`SealedFrame`]. Not `Clone`: two sealers on one
/// direction would each start at sequence zero and repeat every nonce.
pub struct FrameSealer {
    keys: DirectionalKeys,
    session: SessionId,
    /// The next sequence to use, or `None` once the space is exhausted.
    ///
    /// An `Option` rather than a counter plus a flag, so "exhausted" is a state
    /// the type holds instead of a condition somebody has to remember to check.
    next_sequence: Option<u64>,
}

impl FrameSealer {
    /// Encrypts and authenticates one frame.
    ///
    /// The session identifier and the sequence in `frame` are ignored and
    /// replaced; everything else the caller set — message type, transport flags,
    /// transfer, stream and item identifiers — is preserved and authenticated.
    ///
    /// Returning a frame consumes the sequence. Dropping the result does not
    /// give it back: a nonce that could be reissued is a nonce that can repeat.
    ///
    /// # Errors
    ///
    /// Returns [`AeadError::SequenceExhausted`] once every sequence in this
    /// direction has been used, which is terminal, or
    /// [`AeadError::AuthenticationFailed`] if the AEAD itself refuses the input.
    pub fn seal(&mut self, frame: &Frame) -> Result<SealedFrame, AeadError> {
        let sequence = self.next_sequence.ok_or(AeadError::SequenceExhausted)?;

        let header = frame.header();
        let template = Frame::new(frame.message_type(), Vec::new())
            .and_then(|template| template.with_flags(header.flags()))
            .map(|template| {
                template
                    .with_identifiers(
                        self.session,
                        header.transfer_id(),
                        header.stream_id(),
                        header.item_id(),
                    )
                    .with_sequence(sequence)
            })
            .unwrap_or_else(|_| {
                unreachable!("an empty payload is in range and a Frame's flags are transport-only")
            });

        let mut buffer = frame.payload().to_vec();

        // `EncryptedEnvelope::from_plain_frame` takes `payload_len` from the
        // ciphertext's *length* and nothing from its content, so an envelope over
        // a body of the right size carries exactly the header the sealed one
        // will. Predicted here and compared against the real one below, because
        // a tag computed over a header that is not the header on the wire
        // authenticates nothing.
        let probe = EncryptedEnvelope::from_plain_frame(
            &template,
            vec![0u8; buffer.len()],
            vec![0u8; TAG_LEN],
        )
        .unwrap_or_else(|_| {
            unreachable!("a Frame's payload is within MAX_PAYLOAD_LEN and the tag is sixteen bytes")
        });
        let associated_data = probe.associated_data();
        drop(probe);

        let nonce = nonce_for(&self.keys.nonce_prefix, sequence);
        let tag = self
            .keys
            .cipher()
            .encrypt_inout_detached(
                &nonce.into(),
                &associated_data,
                buffer.as_mut_slice().into(),
            )
            .map_err(|_| AeadError::AuthenticationFailed)?;

        let envelope = EncryptedEnvelope::from_plain_frame(&template, buffer, tag.to_vec())
            .unwrap_or_else(|_| {
                unreachable!(
                    "the ciphertext is the plaintext's length and the tag is sixteen bytes"
                )
            });
        assert_eq!(
            envelope.associated_data(),
            associated_data,
            "the sealed header must be the one the tag was computed over"
        );

        // Last, and only once a frame exists to hand back.
        self.next_sequence = sequence.checked_add(1);

        Ok(SealedFrame { envelope, nonce })
    }

    /// The nonce prefix for this direction.
    ///
    /// `cfg(test)`, never a feature: a feature is additive and any crate in a
    /// dependency graph could switch it on for everybody. The tests and the
    /// committed vectors check that the nonce is `prefix || sequence`; nothing
    /// in production has a reason to ask.
    #[cfg(test)]
    pub(crate) const fn nonce_prefix(&self) -> [u8; NONCE_PREFIX_LEN] {
        self.keys.nonce_prefix
    }

    /// Moves the counter, so a test can reach the far end of the space without
    /// sealing 2^64 frames.
    #[cfg(test)]
    pub(crate) const fn set_sequence_for_test(&mut self, sequence: u64) {
        self.next_sequence = Some(sequence);
    }
}

impl core::fmt::Debug for FrameSealer {
    /// Prints the session and the counter; never the key or the nonce prefix.
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("FrameSealer")
            .field("session", &self.session)
            .field("next_sequence", &self.next_sequence)
            .field("key", &"<redacted>")
            .finish()
    }
}

/// A frame whose tag a real AEAD computed.
///
/// Produced **only** by [`FrameSealer::seal`]. Its fields are private and it has
/// no constructor, so holding one is evidence that this process encrypted and
/// authenticated it under a session key.
pub struct SealedFrame {
    envelope: EncryptedEnvelope,
    nonce: [u8; NONCE_LEN],
}

impl SealedFrame {
    /// The envelope to put on the wire.
    #[must_use]
    pub const fn envelope(&self) -> &EncryptedEnvelope {
        &self.envelope
    }

    /// The nonce this frame was sealed under.
    ///
    /// Not secret, and not transmitted: the peer rebuilds it from its own prefix
    /// and the sequence in the header. Exposed so a transport can trace a frame
    /// and so the committed vectors can state it.
    #[must_use]
    pub const fn nonce(&self) -> [u8; NONCE_LEN] {
        self.nonce
    }

    /// Serializes header, ciphertext and tag.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        self.envelope.encode()
    }
}

impl core::fmt::Debug for SealedFrame {
    /// Prints the framing facts, never the ciphertext or the tag.
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let header = self.envelope.header();
        formatter
            .debug_struct("SealedFrame")
            .field("message_type", &header.message_type())
            .field("session", &header.session_id())
            .field("sequence", &header.sequence())
            .field("ciphertext_len", &header.payload_len())
            .finish()
    }
}

// ------------------------------------------------------------------- opening

/// Opens inbound frames for one session and one direction.
///
/// Owns the inbound key, the nonce prefix and the replay window, and is the only
/// thing that can produce an [`AuthenticatedFrame`]. Not `Clone`: two openers
/// would each hold their own window, and a frame rejected by one would be
/// accepted by the other.
pub struct FrameOpener {
    keys: DirectionalKeys,
    session: SessionId,
    window: ReplayWindow,
}

impl FrameOpener {
    /// Verifies and decrypts one envelope.
    ///
    /// The order of the checks is the security property, not a detail. The
    /// replay window is *consulted* before the AEAD runs and *updated* only
    /// after it passes, so a frame that does not authenticate — and a frame for
    /// somebody else's session — costs this session nothing. Otherwise anyone at
    /// all, holding no key, could send `sequence = u64::MAX - 1` with sixteen
    /// random bytes and leave the session unable to accept anything again.
    ///
    /// # Errors
    ///
    /// Returns [`AeadError::InvalidTagLength`] when the trailer is not a
    /// Poly1305 tag, [`AeadError::WrongSession`] when the header names another
    /// session, [`AeadError::ReplayDetected`] or [`AeadError::SequenceTooOld`]
    /// from the window, or [`AeadError::AuthenticationFailed`] when the tag does
    /// not verify. That last one is deliberately a single variant: telling a
    /// wrong tag apart from an altered header would tell an attacker which half
    /// to keep changing.
    pub fn open(&mut self, envelope: &EncryptedEnvelope) -> Result<AuthenticatedFrame, AeadError> {
        // Framing and the `ENCRYPTED` flag are already settled: an
        // `EncryptedEnvelope` cannot exist without them, whichever of its two
        // constructors produced it.
        let header = *envelope.header();

        let tag: [u8; TAG_LEN] =
            envelope
                .tag()
                .try_into()
                .map_err(|_| AeadError::InvalidTagLength {
                    found: envelope.tag().len(),
                    expected: TAG_LEN,
                })?;

        if header.session_id() != self.session {
            return Err(AeadError::WrongSession);
        }

        let sequence = header.sequence();
        self.window.check(sequence)?;

        let associated_data = envelope.associated_data();
        let nonce = nonce_for(&self.keys.nonce_prefix, sequence);
        let mut buffer = envelope.ciphertext().to_vec();

        // Verify-then-decrypt, by the library: `decrypt_inout_detached` compares
        // the Poly1305 tag in constant time and only touches the buffer if it
        // matched. Nothing here re-implements that order by hand.
        self.keys
            .cipher()
            .decrypt_inout_detached(
                &nonce.into(),
                &associated_data,
                buffer.as_mut_slice().into(),
                &tag.into(),
            )
            .map_err(|_| AeadError::AuthenticationFailed)?;

        // Only now. Everything above could be driven by anyone; this line is
        // reached only by someone holding the key.
        self.window.record(sequence)?;

        Ok(AuthenticatedFrame {
            header,
            payload: buffer,
        })
    }
}

impl core::fmt::Debug for FrameOpener {
    /// Prints the session and how far the window has moved; never the key.
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("FrameOpener")
            .field("session", &self.session)
            .field("window", &self.window)
            .field("key", &"<redacted>")
            .finish()
    }
}

/// A frame whose tag verified.
///
/// Produced **only** by [`FrameOpener::open`]. Its fields are private and it has
/// no constructor, so holding one is evidence that the payload is plaintext this
/// session's peer actually sent, under a header nobody altered.
pub struct AuthenticatedFrame {
    header: FrameHeader,
    payload: Vec<u8>,
}

impl AuthenticatedFrame {
    /// The verified plaintext.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Consumes the frame and returns its verified plaintext.
    #[must_use]
    pub fn into_payload(self) -> Vec<u8> {
        self.payload
    }

    /// The message kind the sender authenticated.
    #[must_use]
    pub const fn message_type(&self) -> MessageType {
        self.header.message_type()
    }

    /// The session this frame belongs to.
    #[must_use]
    pub const fn session_id(&self) -> SessionId {
        self.header.session_id()
    }

    /// The sequence the sender assigned.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.header.sequence()
    }

    /// The transfer this frame belongs to.
    #[must_use]
    pub const fn transfer_id(&self) -> u64 {
        self.header.transfer_id()
    }

    /// The stream this frame belongs to.
    #[must_use]
    pub const fn stream_id(&self) -> u32 {
        self.header.stream_id()
    }

    /// The item this frame belongs to.
    #[must_use]
    pub const fn item_id(&self) -> u32 {
        self.header.item_id()
    }

    /// The transport flags the sender authenticated.
    #[must_use]
    pub const fn flags(&self) -> Flags {
        self.header.flags()
    }
}

impl core::fmt::Debug for AuthenticatedFrame {
    /// Prints the framing facts, never the plaintext.
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("AuthenticatedFrame")
            .field("message_type", &self.header.message_type())
            .field("session", &self.header.session_id())
            .field("sequence", &self.header.sequence())
            .field("payload_len", &self.payload.len())
            .finish()
    }
}

// -------------------------------------------------------------- construction

/// Derives one session's sealer and opener from its traffic secrets.
///
/// Crate-internal, and reached only through `into_frame_crypto` on an
/// established handshake state. That state is consumed, so there is no way to
/// build two sealers for the same direction and start two counters at zero.
///
/// # Errors
///
/// Returns [`AeadError::KeyDerivationFailed`] if HKDF refuses an output length,
/// which cannot happen at 32 and 4 bytes but is not worth an `expect` on a
/// security path.
pub(crate) fn frame_crypto(
    sending_secret: &[u8; TRAFFIC_SECRET_LEN],
    receiving_secret: &[u8; TRAFFIC_SECRET_LEN],
    sending: &Direction,
    receiving: &Direction,
    auth_transcript: &[u8; AUTH_TRANSCRIPT_LEN],
    session: SessionId,
) -> Result<(FrameSealer, FrameOpener), AeadError> {
    let sealer = FrameSealer {
        keys: DirectionalKeys::derive(sending_secret, sending, auth_transcript, session)?,
        session,
        next_sequence: Some(0),
    };
    let opener = FrameOpener {
        keys: DirectionalKeys::derive(receiving_secret, receiving, auth_transcript, session)?,
        session,
        window: ReplayWindow::new(),
    };
    Ok((sealer, opener))
}
