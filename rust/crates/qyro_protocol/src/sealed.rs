//! Sealed frames: the only representation that may carry `ENCRYPTED`.
//!
//! The invariant this module exists to enforce is structural rather than
//! documented: a [`SealedFrame`] cannot be constructed without supplying an
//! authentication tag, and the `ENCRYPTED` flag is set by that constructor
//! alone. There is no path — from the UI, a transport, or a test — that produces
//! a frame claiming protection it does not have.
//!
//! This crate does no cryptography. It defines the shape and hands the caller
//! the exact bytes that must be authenticated; `qyro_crypto` supplies the AEAD.

use crate::error::FrameError;
use crate::header::FrameHeader;
use crate::limits::{HEADER_LEN, MAX_PAYLOAD_LEN, MAX_TRAILER_LEN};
use crate::message::{Flags, MessageType};

/// A frame whose payload is ciphertext, carrying its authentication tag.
///
/// Constructing one is the proof that a tag exists. Opening it is
/// `qyro_crypto`'s job; this type never exposes plaintext because it never has
/// any.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealedFrame {
    header: FrameHeader,
    ciphertext: Vec<u8>,
    tag: Vec<u8>,
}

impl SealedFrame {
    /// Builds a sealed frame from ciphertext and its tag.
    ///
    /// The header is derived from `template`, then marked encrypted with the
    /// exact tag length. Because ciphertext and tag arrive together, the flag
    /// and the trailer can never disagree with reality.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::PayloadTooLarge`] when the ciphertext exceeds
    /// [`MAX_PAYLOAD_LEN`], or [`FrameError::AuthenticationTrailerInvalid`] when
    /// the tag is empty or longer than [`MAX_TRAILER_LEN`].
    pub fn new(
        template: FrameHeader,
        ciphertext: Vec<u8>,
        tag: Vec<u8>,
    ) -> Result<Self, FrameError> {
        if ciphertext.len() > MAX_PAYLOAD_LEN {
            return Err(FrameError::PayloadTooLarge {
                declared: u32::try_from(ciphertext.len()).unwrap_or(u32::MAX),
                limit: MAX_PAYLOAD_LEN as u32,
            });
        }
        let tag_len =
            u8::try_from(tag.len()).map_err(|_| FrameError::AuthenticationTrailerInvalid {
                declared: u8::MAX,
                expected: MAX_TRAILER_LEN as u8,
            })?;

        let payload_len =
            u32::try_from(ciphertext.len()).expect("ciphertext fits in u32 after the check");
        let header = FrameHeader::new(template.message_type(), payload_len)
            .with_identifiers(
                template.session_id(),
                template.transfer_id(),
                template.stream_id(),
                template.item_id(),
            )
            .with_sequence(template.sequence())
            .sealed(tag_len)?;

        Ok(Self {
            header,
            ciphertext,
            tag,
        })
    }

    /// Returns the header.
    #[must_use]
    pub const fn header(&self) -> &FrameHeader {
        &self.header
    }

    /// Returns the message type.
    #[must_use]
    pub const fn message_type(&self) -> MessageType {
        self.header.message_type()
    }

    /// The bytes that must be authenticated alongside the ciphertext.
    ///
    /// The whole header is the associated data, so any tampering with a length,
    /// a flag, a sequence number or an identifier invalidates the tag. This is
    /// only sound because re-encoding a decoded header is byte-exact — see
    /// `docs/adr/ADR-0018-protocol-semantic-errors.md`.
    #[must_use]
    pub fn associated_data(&self) -> [u8; HEADER_LEN] {
        self.header.encode()
    }

    /// Returns the ciphertext.
    #[must_use]
    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }

    /// Returns the authentication tag.
    #[must_use]
    pub fn tag(&self) -> &[u8] {
        &self.tag
    }

    /// Serializes header, ciphertext and tag.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let header = self.header.encode();
        let mut out = Vec::with_capacity(header.len() + self.ciphertext.len() + self.tag.len());
        out.extend_from_slice(&header);
        out.extend_from_slice(&self.ciphertext);
        out.extend_from_slice(&self.tag);
        out
    }

    /// Rebuilds a sealed frame from a decoded header and its body.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::EncryptedWithoutTrailer`] when the header is not
    /// marked sealed, or [`FrameError::TruncatedPayload`] when the body length
    /// disagrees with the header.
    pub(crate) fn from_parts(header: FrameHeader, body: &[u8]) -> Result<Self, FrameError> {
        if !header.flags().contains(Flags::ENCRYPTED) || header.trailer_len() == 0 {
            return Err(FrameError::EncryptedWithoutTrailer {
                declared: header.trailer_len(),
            });
        }
        let payload_len = header.payload_len() as usize;
        let tag_len = usize::from(header.trailer_len());
        if body.len() != payload_len + tag_len {
            return Err(FrameError::TruncatedPayload {
                available: body.len(),
                required: payload_len + tag_len,
            });
        }
        Ok(Self {
            header,
            ciphertext: body[..payload_len].to_vec(),
            tag: body[payload_len..].to_vec(),
        })
    }
}
