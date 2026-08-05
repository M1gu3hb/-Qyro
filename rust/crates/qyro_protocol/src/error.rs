//! Typed decoding failures.
//!
//! Callers match on variants, never on message text. Diagnostics carry the
//! offending value so a peer can be told precisely what was wrong without the
//! caller having to re-parse the bytes.

use core::fmt;

/// Why a frame could not be decoded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FrameError {
    /// The first four bytes were not `QYRO`.
    InvalidMagic {
        /// The bytes actually found.
        found: [u8; 4],
    },
    /// The peer speaks a major version whose layout this build cannot assume.
    UnsupportedMajorVersion {
        /// Major version the peer declared.
        found: u8,
        /// Major version this build implements.
        supported: u8,
    },
    /// `header_len` is below the fixed minimum or above [`crate::MAX_HEADER_LEN`].
    InvalidHeaderLength {
        /// Header length the peer declared.
        declared: u16,
        /// Smallest accepted header length.
        minimum: u16,
        /// Largest accepted header length.
        maximum: u16,
    },
    /// `payload_len` exceeds [`crate::MAX_PAYLOAD_LEN`].
    PayloadTooLarge {
        /// Payload length the peer declared.
        declared: u32,
        /// Largest accepted payload length.
        limit: u32,
    },
    /// Header, payload and trailer together exceed [`crate::MAX_FRAME_LEN`].
    FrameTooLarge {
        /// Total frame length implied by the header.
        declared: u64,
        /// Largest accepted frame length.
        limit: u64,
    },
    /// The message type is not part of this protocol version.
    ///
    /// Recoverable: lengths are validated before the type is resolved, so the
    /// receiver knows the exact frame size and can skip it.
    UnknownMessageType {
        /// Wire value that did not map to a known message.
        value: u8,
    },
    /// A reserved flag bit was set, or a reserved header byte was non-zero.
    InvalidFlags {
        /// Bits actually present.
        bits: u8,
        /// Bits required to stay zero.
        reserved_mask: u8,
    },
    /// Fewer bytes than the fixed header require.
    TruncatedHeader {
        /// Bytes available.
        available: usize,
        /// Bytes the header needs.
        required: usize,
    },
    /// The header was complete but the declared body was not.
    TruncatedPayload {
        /// Bytes available.
        available: usize,
        /// Bytes the declared body needs.
        required: usize,
    },
    /// The peer declared a header extension QYRO/1.0 cannot preserve.
    ///
    /// Structural: skipping bytes that are neither stored nor re-serialized
    /// would break byte-exact re-encoding and leave the AEAD unable to
    /// authenticate them. See `docs/adr/ADR-0018-protocol-semantic-errors.md`.
    UnsupportedHeaderExtension {
        /// Header length the peer declared.
        declared: u16,
        /// Header length this version accepts.
        supported: u16,
    },
    /// A flag is defined but its feature does not exist yet.
    UnsupportedFlag {
        /// Bits actually present.
        bits: u8,
        /// The unsupported bits among them.
        unsupported: u8,
    },
    /// `ENCRYPTED` was set without an authentication trailer.
    ///
    /// A frame that claims to be sealed but carries no tag is lying about its
    /// own contents; only the sealing path may set this flag.
    EncryptedWithoutTrailer {
        /// Trailer length the peer declared.
        declared: u8,
    },
    /// An identifier field held a value this protocol version forbids.
    InvalidIdentifier {
        /// Which identifier was rejected.
        field: IdentifierField,
    },
    /// QYRO/1.0 accepts no authentication trailer; a non-zero length appeared.
    AuthenticationTrailerInvalid {
        /// Trailer length the peer declared.
        declared: u8,
        /// Trailer length this version accepts.
        expected: u8,
    },
    /// A header describing a protected frame was used to build a plain one.
    ///
    /// A [`crate::Frame`] holds a payload and nothing else, so it cannot honour
    /// a header that sets `ENCRYPTED` or declares a trailer. Encoding such a
    /// frame emits fewer bytes than its own header promises, which does not
    /// merely fail to decode: it leaves a peer's decoder waiting for a trailer
    /// that will never arrive.
    ProtectedHeaderNotPlain {
        /// Flag bits the header carried.
        flags: u8,
        /// Trailer length the header declared.
        trailer_len: u8,
    },
    /// Buffering more bytes would exceed the decoder's ceiling.
    BufferLimitExceeded {
        /// Buffer size the push would have produced.
        attempted: usize,
        /// Ceiling the decoder enforces.
        limit: usize,
    },
}

/// Identifier fields, so [`FrameError::InvalidIdentifier`] stays typed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum IdentifierField {
    /// `session_id`.
    Session,
    /// `transfer_id`.
    Transfer,
    /// `stream_id`.
    Stream,
    /// `item_id`.
    Item,
}

impl fmt::Display for IdentifierField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Session => "session_id",
            Self::Transfer => "transfer_id",
            Self::Stream => "stream_id",
            Self::Item => "item_id",
        };
        formatter.write_str(name)
    }
}

impl fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic { .. } => formatter.write_str("frame magic is not QYRO"),
            Self::UnsupportedMajorVersion { found, supported } => write!(
                formatter,
                "unsupported major version {found}, this build speaks {supported}"
            ),
            Self::InvalidHeaderLength {
                declared,
                minimum,
                maximum,
            } => write!(
                formatter,
                "header length {declared} outside {minimum}..={maximum}"
            ),
            Self::PayloadTooLarge { declared, limit } => {
                write!(formatter, "payload length {declared} exceeds limit {limit}")
            }
            Self::FrameTooLarge { declared, limit } => {
                write!(formatter, "frame length {declared} exceeds limit {limit}")
            }
            Self::UnknownMessageType { value } => {
                write!(formatter, "unknown message type {value}")
            }
            Self::InvalidFlags {
                bits,
                reserved_mask,
            } => write!(
                formatter,
                "reserved bits set in {bits:#010b} (reserved mask {reserved_mask:#010b})"
            ),
            Self::TruncatedHeader {
                available,
                required,
            } => {
                write!(
                    formatter,
                    "header truncated: {available} of {required} bytes"
                )
            }
            Self::TruncatedPayload {
                available,
                required,
            } => {
                write!(formatter, "body truncated: {available} of {required} bytes")
            }
            Self::UnsupportedHeaderExtension {
                declared,
                supported,
            } => write!(
                formatter,
                "header extension of {declared} bytes is not preserved by this version, which requires {supported}"
            ),
            Self::UnsupportedFlag { bits, unsupported } => write!(
                formatter,
                "flag bits {unsupported:#010b} in {bits:#010b} are defined but unimplemented"
            ),
            Self::EncryptedWithoutTrailer { declared } => write!(
                formatter,
                "ENCRYPTED set with a trailer length of {declared}"
            ),
            Self::InvalidIdentifier { field } => {
                write!(formatter, "invalid identifier in {field}")
            }
            Self::AuthenticationTrailerInvalid { declared, expected } => write!(
                formatter,
                "authentication trailer length {declared}, expected {expected}"
            ),
            Self::ProtectedHeaderNotPlain { flags, trailer_len } => write!(
                formatter,
                "header with flags {flags:#010b} and trailer length {trailer_len} describes a protected frame, not a plain one"
            ),
            Self::BufferLimitExceeded { attempted, limit } => {
                write!(
                    formatter,
                    "buffering {attempted} bytes exceeds limit {limit}"
                )
            }
        }
    }
}

impl core::error::Error for FrameError {}
