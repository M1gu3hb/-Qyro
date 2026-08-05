//! Encrypted envelopes: the wire shape of a frame whose payload is ciphertext.
//!
//! **Untrusted until `qyro_crypto` verifies it.** This crate performs no
//! cryptography, so an envelope proves nothing about authenticity. It carries
//! bytes a peer called a tag, and holding one is not evidence that anybody
//! computed or checked it.
//!
//! The earlier name for this type was `SealedFrame`, which claimed a guarantee
//! the crate cannot provide: its constructor accepted any byte vector as a
//! "tag". Naming and documentation now say only what is true — this is a
//! classified, well-delimited carrier of ciphertext plus trailer.
//!
//! When `qyro_crypto` exists it will own the two types that do make claims:
//!
//! - `SealedFrame`, produced only by `seal`, wrapping an envelope whose tag a
//!   real AEAD computed.
//! - `AuthenticatedFrame`, produced only by `open`, holding plaintext whose tag
//!   was verified.
//!
//! Both will have private constructors. Until then, nothing in this repository
//! may claim authentication.

use crate::error::FrameError;
use crate::frame::Frame;
use crate::header::FrameHeader;
use crate::limits::{HEADER_LEN, MAX_PAYLOAD_LEN, MAX_TRAILER_LEN};
use crate::message::{Flags, MessageType};

/// A frame whose payload is ciphertext, carrying a trailer a peer called a tag.
///
/// **Untrusted until `qyro_crypto` verifies it.** Constructing one proves only
/// that ciphertext and trailer bytes were supplied together and that the header
/// is internally consistent. It does not prove the trailer authenticates
/// anything, because this crate computes no tags.
///
/// The type never exposes plaintext, because it never holds any.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncryptedEnvelope {
    header: FrameHeader,
    ciphertext: Vec<u8>,
    tag: Vec<u8>,
}

impl EncryptedEnvelope {
    /// Wraps ciphertext and a trailer, carrying over the plain frame's metadata.
    ///
    /// Every routing and transport field of `template` is preserved so it stays
    /// inside the associated data a future AEAD will authenticate: message type,
    /// minor version, `END_OF_ITEM`, `END_OF_TRANSFER`, and all four
    /// identifiers plus the sequence number.
    ///
    /// Only three fields are derived rather than carried: `payload_len` from the
    /// ciphertext, `trailer_len` from the trailer, and the `ENCRYPTED` flag.
    ///
    /// The template is a [`Frame`], not a [`FrameHeader`], and that is the whole
    /// point of the parameter type. The earlier signature took a header, and an
    /// envelope hands its own header out, so an envelope could be wrapped a
    /// second time using itself as the template: the resulting `ENCRYPTED` flag
    /// and trailer length described the second wrap while the ciphertext was
    /// still the first. A `Frame` cannot be built around a protected header —
    /// [`Frame::from_parts`] rejects that — so a `&Frame` *is* the proof that
    /// the template is plain.
    ///
    /// This performs no encryption and verifies nothing. See the module docs.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::PayloadTooLarge`] when the ciphertext exceeds
    /// [`MAX_PAYLOAD_LEN`], or [`FrameError::AuthenticationTrailerInvalid`] when
    /// the trailer is empty or longer than [`MAX_TRAILER_LEN`].
    pub fn from_plain_frame(
        template: &Frame,
        ciphertext: Vec<u8>,
        tag: Vec<u8>,
    ) -> Result<Self, FrameError> {
        let template = template.header();
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
        let header = template
            .clone_for_envelope(payload_len)
            .encrypted(tag_len)?;

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

    /// The bytes a future AEAD must authenticate alongside the ciphertext.
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

    /// Rebuilds an envelope from a decoded header and its body.
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
