//! The fixed 48-byte QYRO/1 frame header.
//!
//! Layout and rationale: `docs/adr/ADR-0016-qyro1-wire-framing.md`.

use crate::error::FrameError;
use crate::limits::{
    HEADER_LEN, MAX_FRAME_LEN, MAX_HEADER_LEN, MAX_PAYLOAD_LEN, SUPPORTED_TRAILER_LEN,
};
use crate::message::{Flags, MessageType};
use crate::version::{MAGIC, VERSION_MAJOR, VERSION_MINOR};

/// Decoded frame header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameHeader {
    /// Minor version declared by the sender. May exceed [`VERSION_MINOR`].
    pub version_minor: u8,
    /// Message kind.
    pub message_type: MessageType,
    /// Frame flags.
    pub flags: Flags,
    /// Header length in bytes, at least [`HEADER_LEN`].
    pub header_len: u16,
    /// Authentication trailer length. Always zero in QYRO/1.0.
    pub trailer_len: u8,
    /// Payload length in bytes.
    pub payload_len: u32,
    /// Opaque session identifier.
    pub session_id: u64,
    /// Opaque transfer identifier.
    pub transfer_id: u64,
    /// Opaque stream identifier.
    pub stream_id: u32,
    /// Opaque item identifier.
    pub item_id: u32,
    /// Monotonic sequence number within the stream.
    pub sequence: u64,
}

impl FrameHeader {
    /// Builds a QYRO/1.0 header with the current version and no trailer.
    #[must_use]
    pub const fn new(message_type: MessageType, payload_len: u32) -> Self {
        Self {
            version_minor: VERSION_MINOR,
            message_type,
            flags: Flags::NONE,
            header_len: HEADER_LEN as u16,
            trailer_len: SUPPORTED_TRAILER_LEN as u8,
            payload_len,
            session_id: 0,
            transfer_id: 0,
            stream_id: 0,
            item_id: 0,
            sequence: 0,
        }
    }

    /// Total frame size: header, payload and trailer.
    ///
    /// Uses `u64` so the sum cannot overflow before it is compared against
    /// [`MAX_FRAME_LEN`], even on a 32-bit target.
    #[must_use]
    pub const fn total_len(&self) -> u64 {
        self.header_len as u64 + self.payload_len as u64 + self.trailer_len as u64
    }

    /// Serializes the header into exactly [`HEADER_LEN`] big-endian bytes.
    #[must_use]
    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut out = [0u8; HEADER_LEN];
        out[0..4].copy_from_slice(&MAGIC);
        out[4] = VERSION_MAJOR;
        out[5] = self.version_minor;
        out[6] = self.message_type.to_wire();
        out[7] = self.flags.bits();
        out[8..10].copy_from_slice(&self.header_len.to_be_bytes());
        out[10] = self.trailer_len;
        out[11] = 0; // reserved
        out[12..16].copy_from_slice(&self.payload_len.to_be_bytes());
        out[16..24].copy_from_slice(&self.session_id.to_be_bytes());
        out[24..32].copy_from_slice(&self.transfer_id.to_be_bytes());
        out[32..36].copy_from_slice(&self.stream_id.to_be_bytes());
        out[36..40].copy_from_slice(&self.item_id.to_be_bytes());
        out[40..48].copy_from_slice(&self.sequence.to_be_bytes());
        out
    }

    /// Parses a header from the first [`HEADER_LEN`] bytes of `bytes`.
    ///
    /// Validation order is deliberate and is the crate's core safety property:
    /// magic, version and every declared length are checked **before** the
    /// caller learns how many bytes to wait for, and long before anything is
    /// allocated. A hostile `payload_len` of `0xFFFF_FFFF` is rejected here, so
    /// it can never become a reservation.
    ///
    /// # Errors
    ///
    /// Returns the [`FrameError`] describing the first violated rule.
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        if bytes.len() < HEADER_LEN {
            return Err(FrameError::TruncatedHeader {
                available: bytes.len(),
                required: HEADER_LEN,
            });
        }

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);
        if magic != MAGIC {
            return Err(FrameError::InvalidMagic { found: magic });
        }

        let version_major = bytes[4];
        if version_major != VERSION_MAJOR {
            return Err(FrameError::UnsupportedMajorVersion {
                found: version_major,
                supported: VERSION_MAJOR,
            });
        }
        // A newer minor version may only append header fields or add message
        // types, so it is accepted and the extra bytes are skipped.
        let version_minor = bytes[5];

        let header_len = u16::from_be_bytes([bytes[8], bytes[9]]);
        let minimum = HEADER_LEN as u16;
        let maximum = MAX_HEADER_LEN as u16;
        if header_len < minimum || header_len > maximum {
            return Err(FrameError::InvalidHeaderLength {
                declared: header_len,
                minimum,
                maximum,
            });
        }

        let trailer_len = bytes[10];
        if usize::from(trailer_len) != SUPPORTED_TRAILER_LEN {
            return Err(FrameError::AuthenticationTrailerInvalid {
                declared: trailer_len,
                expected: SUPPORTED_TRAILER_LEN as u8,
            });
        }

        let reserved = bytes[11];
        if reserved != 0 {
            return Err(FrameError::InvalidFlags {
                bits: reserved,
                reserved_mask: u8::MAX,
            });
        }

        let payload_len = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let payload_limit = MAX_PAYLOAD_LEN as u32;
        if payload_len > payload_limit {
            return Err(FrameError::PayloadTooLarge {
                declared: payload_len,
                limit: payload_limit,
            });
        }

        let flags = Flags::from_bits(bytes[7])?;
        let message_type = MessageType::from_wire(bytes[6])?;

        let header = Self {
            version_minor,
            message_type,
            flags,
            header_len,
            trailer_len,
            payload_len,
            session_id: u64::from_be_bytes(bytes[16..24].try_into().expect("slice is eight bytes")),
            transfer_id: u64::from_be_bytes(
                bytes[24..32].try_into().expect("slice is eight bytes"),
            ),
            stream_id: u32::from_be_bytes(bytes[32..36].try_into().expect("slice is four bytes")),
            item_id: u32::from_be_bytes(bytes[36..40].try_into().expect("slice is four bytes")),
            sequence: u64::from_be_bytes(bytes[40..48].try_into().expect("slice is eight bytes")),
        };

        let total = header.total_len();
        let frame_limit = MAX_FRAME_LEN as u64;
        if total > frame_limit {
            return Err(FrameError::FrameTooLarge {
                declared: total,
                limit: frame_limit,
            });
        }

        Ok(header)
    }
}
