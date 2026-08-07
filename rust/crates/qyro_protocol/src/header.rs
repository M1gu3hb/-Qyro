//! The fixed 48-byte QYRO/1 frame header.
//!
//! Layout and rationale: `docs/adr/ADR-0016-qyro1-wire-framing.md`, amended by
//! `docs/adr/ADR-0018-protocol-semantic-errors.md`.
//!
//! Every field is private. The only ways to build a header are the validated
//! constructors below, so no public API can produce a header that this crate's
//! own decoder would reject.

use crate::error::FrameError;
use crate::limits::{
    HEADER_LEN, MAX_FRAME_LEN, MAX_HEADER_LEN, MAX_PAYLOAD_LEN, MAX_TRAILER_LEN,
    SUPPORTED_TRAILER_LEN,
};
use crate::message::{Flags, MessageType};
use crate::session::SessionId;
use crate::version::{MAGIC, VERSION_MAJOR, VERSION_MINOR};

/// A validated frame header.
///
/// Its existence is the proof that magic, version, flags and every declared
/// length are within QYRO/1.0's rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameHeader {
    version_minor: u8,
    message_type: MessageType,
    flags: Flags,
    trailer_len: u8,
    payload_len: u32,
    session_id: SessionId,
    transfer_id: u64,
    stream_id: u32,
    item_id: u32,
    sequence: u64,
}

/// Copies a fixed-width field out of the fixed-width header.
///
/// Offsets and widths are compile-time constants and the input is an array, so
/// there is no runtime bound to violate. It replaces eight
/// `try_into().expect("slice is eight bytes")` calls, each of which restated in
/// a string a fact the type already carried — and each of which was a panic on
/// a path a peer's bytes reach if the fact ever stopped holding.
fn field<const OFFSET: usize, const WIDTH: usize>(bytes: &[u8; HEADER_LEN]) -> [u8; WIDTH] {
    const {
        assert!(
            OFFSET + WIDTH <= HEADER_LEN,
            "a header field must lie inside the header"
        )
    };
    let mut out = [0u8; WIDTH];
    for (slot, index) in out.iter_mut().zip(OFFSET..OFFSET + WIDTH) {
        if let Some(byte) = bytes.get(index) {
            *slot = *byte;
        }
    }
    out
}

impl FrameHeader {
    /// Builds a plain QYRO/1.0 header: no flags, no trailer.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::PayloadTooLarge`] when `payload_len` exceeds
    /// [`MAX_PAYLOAD_LEN`]. No public constructor may produce a header whose
    /// bytes this crate's own decoder would reject.
    pub const fn new(message_type: MessageType, payload_len: u32) -> Result<Self, FrameError> {
        if payload_len as usize > MAX_PAYLOAD_LEN {
            return Err(FrameError::PayloadTooLarge {
                declared: payload_len,
                limit: MAX_PAYLOAD_LEN as u32,
            });
        }
        Ok(Self::within_limits(message_type, payload_len))
    }

    /// Builds a header from a length the caller already proved is in range.
    const fn within_limits(message_type: MessageType, payload_len: u32) -> Self {
        Self {
            version_minor: VERSION_MINOR,
            message_type,
            flags: Flags::NONE,
            trailer_len: SUPPORTED_TRAILER_LEN as u8,
            payload_len,
            session_id: SessionId::ZERO,
            transfer_id: 0,
            stream_id: 0,
            item_id: 0,
            sequence: 0,
        }
    }

    /// Copies every authenticated metadata field for an encrypted envelope.
    ///
    /// Preserves message type, minor version, transport flags and all
    /// identifiers; only the payload length is replaced, because the ciphertext
    /// has its own. The `ENCRYPTED` flag and trailer come from
    /// [`FrameHeader::encrypted`].
    pub(crate) const fn clone_for_envelope(&self, payload_len: u32) -> Self {
        Self {
            version_minor: self.version_minor,
            message_type: self.message_type,
            flags: self.flags,
            trailer_len: self.trailer_len,
            payload_len,
            session_id: self.session_id,
            transfer_id: self.transfer_id,
            stream_id: self.stream_id,
            item_id: self.item_id,
            sequence: self.sequence,
        }
    }

    /// Minor version declared by the sender.
    #[must_use]
    pub const fn version_minor(&self) -> u8 {
        self.version_minor
    }

    /// Message kind.
    #[must_use]
    pub const fn message_type(&self) -> MessageType {
        self.message_type
    }

    /// Frame flags.
    #[must_use]
    pub const fn flags(&self) -> Flags {
        self.flags
    }

    /// Header length in bytes.
    ///
    /// QYRO/1.0 has exactly one valid value; the accessor exists so callers can
    /// assert it rather than assume it.
    #[must_use]
    pub const fn header_len(&self) -> u16 {
        HEADER_LEN as u16
    }

    /// Authentication trailer length.
    #[must_use]
    pub const fn trailer_len(&self) -> u8 {
        self.trailer_len
    }

    /// Payload length in bytes.
    #[must_use]
    pub const fn payload_len(&self) -> u32 {
        self.payload_len
    }

    /// Opaque session identifier.
    ///
    /// The same eight-byte type the handshake key schedule derives, so no
    /// conversion or truncation happens between establishing a session and
    /// naming it on the wire.
    #[must_use]
    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// Opaque transfer identifier.
    #[must_use]
    pub const fn transfer_id(&self) -> u64 {
        self.transfer_id
    }

    /// Opaque stream identifier.
    #[must_use]
    pub const fn stream_id(&self) -> u32 {
        self.stream_id
    }

    /// Opaque item identifier.
    #[must_use]
    pub const fn item_id(&self) -> u32 {
        self.item_id
    }

    /// Sequence number within the stream.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Rejects any header length other than the one QYRO/1.0 can preserve.
    ///
    /// Exists so the contradiction ADR-0018 records stays visible in the API: a
    /// caller cannot ask for an extended header and get one silently.
    ///
    /// # Errors
    ///
    /// Always returns [`FrameError::UnsupportedHeaderExtension`] for anything
    /// other than [`HEADER_LEN`].
    pub const fn with_header_len(self, header_len: u16) -> Result<Self, FrameError> {
        if header_len as usize == HEADER_LEN {
            return Ok(self);
        }
        Err(FrameError::UnsupportedHeaderExtension {
            declared: header_len,
            supported: HEADER_LEN as u16,
        })
    }

    /// Sets the routing identifiers.
    #[must_use]
    pub const fn with_identifiers(
        mut self,
        session_id: SessionId,
        transfer_id: u64,
        stream_id: u32,
        item_id: u32,
    ) -> Self {
        self.session_id = session_id;
        self.transfer_id = transfer_id;
        self.stream_id = stream_id;
        self.item_id = item_id;
        self
    }

    /// Sets the sequence number.
    #[must_use]
    pub const fn with_sequence(mut self, sequence: u64) -> Self {
        self.sequence = sequence;
        self
    }

    /// Sets transport flags.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::UnsupportedFlag`] for protected flags: `ENCRYPTED`
    /// and `COMPRESSED` assert something about the payload that a caller cannot
    /// make true, so only the sealing path may set them.
    pub const fn with_transport_flags(mut self, flags: Flags) -> Result<Self, FrameError> {
        if !flags.is_transport_only() {
            return Err(FrameError::UnsupportedFlag {
                bits: flags.bits(),
                unsupported: flags.protected_bits(),
            });
        }
        self.flags = flags;
        Ok(self)
    }

    /// Adds the `ENCRYPTED` flag and trailer length to an existing header.
    ///
    /// Crate-internal: only [`crate::EncryptedEnvelope`] reaches it, and only
    /// while a trailer is being attached, so the flag and the trailer length can
    /// never disagree. Existing transport flags are **kept**, not replaced: they
    /// are part of what the AEAD authenticates.
    pub(crate) const fn encrypted(mut self, tag_len: u8) -> Result<Self, FrameError> {
        if tag_len == 0 || tag_len as usize > MAX_TRAILER_LEN {
            return Err(FrameError::AuthenticationTrailerInvalid {
                declared: tag_len,
                expected: 0,
            });
        }
        self.flags = self.flags.union(Flags::ENCRYPTED);
        self.trailer_len = tag_len;
        Ok(self)
    }

    /// Total frame size: header, payload and trailer.
    ///
    /// Uses `u64` so the sum cannot overflow before it is compared against
    /// [`MAX_FRAME_LEN`], even on a 32-bit target.
    #[must_use]
    pub const fn total_len(&self) -> u64 {
        HEADER_LEN as u64 + self.payload_len as u64 + self.trailer_len as u64
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
        out[8..10].copy_from_slice(&(HEADER_LEN as u16).to_be_bytes());
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
    /// The message type is resolved last, so a frame carrying an unknown type is
    /// still fully delimited and can be consumed without desynchronising the
    /// stream. See [`crate::FrameDecoder`] and ADR-0018.
    ///
    /// # Errors
    ///
    /// Returns the [`FrameError`] describing the first violated rule.
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        match Self::parse(bytes)? {
            ParsedHeader::Known(header) => Ok(header),
            ParsedHeader::Unknown(unknown) => Err(FrameError::UnknownMessageType {
                value: unknown.raw_message_type,
            }),
        }
    }

    /// Parses a header into an explicitly Known or Unknown result.
    ///
    /// The decoder needs the Unknown case so a type it does not implement can be
    /// surfaced as a delimited event instead of a framing failure.
    pub(crate) fn parse(bytes: &[u8]) -> Result<ParsedHeader, FrameError> {
        // Narrowed to the fixed width once, here, and read as an array
        // everywhere below. The length was already compared against
        // `HEADER_LEN` and then trusted by forty-odd indexes and eight
        // `expect`s; making it a type means the comparison and the reads cannot
        // drift apart, and there is nothing left for a peer's length to reach.
        let Some(bytes) = bytes
            .get(..HEADER_LEN)
            .and_then(|prefix| <&[u8; HEADER_LEN]>::try_from(prefix).ok())
        else {
            return Err(FrameError::TruncatedHeader {
                available: bytes.len(),
                required: HEADER_LEN,
            });
        };

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
        // A newer minor version may add message types; it may not change the
        // header layout, because 1.0 refuses extensions it cannot preserve.
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
        if header_len != minimum {
            // Skipping bytes that are neither stored nor re-serialized would
            // break byte-exact re-encoding and leave the AEAD unable to
            // authenticate them. Refusing is safer than pretending. (ADR-0018)
            return Err(FrameError::UnsupportedHeaderExtension {
                declared: header_len,
                supported: minimum,
            });
        }

        let flags = Flags::from_bits(bytes[7])?;
        let unimplemented = flags.unimplemented_bits();
        if unimplemented != 0 {
            return Err(FrameError::UnsupportedFlag {
                bits: flags.bits(),
                unsupported: unimplemented,
            });
        }

        let trailer_len = bytes[10];
        let is_encrypted = flags.contains(Flags::ENCRYPTED);
        if is_encrypted {
            // A frame claiming to be sealed must carry a tag; otherwise it is
            // asserting protection it does not have.
            if trailer_len == 0 || usize::from(trailer_len) > MAX_TRAILER_LEN {
                return Err(FrameError::EncryptedWithoutTrailer {
                    declared: trailer_len,
                });
            }
        } else if usize::from(trailer_len) != SUPPORTED_TRAILER_LEN {
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

        // Resolved last: everything above delimits the frame, so an unknown type
        // is recoverable rather than fatal.
        //
        // A header is only built once the type is known. Substituting a real
        // variant for an unknown byte - as this did with MessageType::Hello -
        // creates a value the wire never carried, and a parser that invents a
        // known type is one refactor away from leaking it.
        let raw_message_type = bytes[6];
        let Ok(message_type) = MessageType::from_wire(raw_message_type) else {
            return Ok(ParsedHeader::Unknown(UnknownHeader {
                raw_message_type,
                payload_len,
                trailer_len,
                session_id: u64::from_be_bytes(field::<16, 8>(bytes)),
                transfer_id: u64::from_be_bytes(field::<24, 8>(bytes)),
                sequence: u64::from_be_bytes(field::<40, 8>(bytes)),
            }));
        };

        let header = Self {
            version_minor,
            message_type,
            flags,
            trailer_len,
            payload_len,
            session_id: SessionId::from_be_bytes(field::<16, 8>(bytes)),
            transfer_id: u64::from_be_bytes(field::<24, 8>(bytes)),
            stream_id: u32::from_be_bytes(field::<32, 4>(bytes)),
            item_id: u32::from_be_bytes(field::<36, 4>(bytes)),
            sequence: u64::from_be_bytes(field::<40, 8>(bytes)),
        };

        let total = header.total_len();
        let frame_limit = MAX_FRAME_LEN as u64;
        if total > frame_limit {
            return Err(FrameError::FrameTooLarge {
                declared: total,
                limit: frame_limit,
            });
        }

        Ok(ParsedHeader::Known(header))
    }
}

/// What parsing a well-formed header produced.
///
/// Deliberately not `FrameHeader` plus a raw byte: a [`FrameHeader`] must only
/// ever hold a message type the wire actually carried.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ParsedHeader {
    /// The type is one this version implements.
    Known(FrameHeader),
    /// The framing is valid but the type is not implemented here.
    Unknown(UnknownHeader),
}

/// The delimiting fields of a frame whose type this version does not implement.
///
/// Enough to consume the frame and answer `Error`; no invented message type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct UnknownHeader {
    pub(crate) raw_message_type: u8,
    pub(crate) payload_len: u32,
    pub(crate) trailer_len: u8,
    pub(crate) session_id: u64,
    pub(crate) transfer_id: u64,
    pub(crate) sequence: u64,
}

impl ParsedHeader {
    /// Parses the fixed header from the front of `bytes`.
    pub(crate) fn parse_from(bytes: &[u8]) -> Result<Self, FrameError> {
        FrameHeader::parse(bytes)
    }

    /// Total frame size implied by whichever variant was parsed.
    pub(crate) const fn total_len(&self) -> u64 {
        match self {
            Self::Known(header) => header.total_len(),
            Self::Unknown(unknown) => unknown.total_len(),
        }
    }
}

impl UnknownHeader {
    /// Total frame size implied by the header.
    pub(crate) const fn total_len(&self) -> u64 {
        HEADER_LEN as u64 + self.payload_len as u64 + self.trailer_len as u64
    }
}
