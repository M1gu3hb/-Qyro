//! Incremental frame decoder.
//!
//! A socket read is not a frame. It may deliver half a header, several frames,
//! or a frame split at any byte. This decoder buffers whatever arrives and
//! yields frames only once they are complete, under a hard memory ceiling.
//!
//! It distinguishes two kinds of failure, per
//! `docs/adr/ADR-0018-protocol-semantic-errors.md`:
//!
//! - **Structural** failures mean framing itself is no longer trustworthy. The
//!   decoder is poisoned and only an explicit [`FrameDecoder::reset`] recovers.
//! - **Delimited semantic events**, today just an unknown message type, keep the
//!   stream synchronised: the frame is consumed whole and reported as
//!   [`DecodedFrame::Unsupported`].

use crate::error::FrameError;
use crate::frame::Frame;
use crate::header::FrameHeader;
use crate::limits::{HEADER_LEN, MAX_BUFFER_LEN};
use crate::message::{Flags, MessageType};
use crate::sealed::SealedFrame;

/// A frame the peer sent whose type this version does not implement.
///
/// Carries enough to answer `Error` without re-parsing bytes, and deliberately
/// does **not** expose the payload: bytes whose meaning is unknown must not
/// become something the application can process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnsupportedFrame {
    message_type_value: u8,
    payload_len: u32,
    total_len: usize,
    session_id: u64,
    transfer_id: u64,
    sequence: u64,
}

impl UnsupportedFrame {
    /// The wire value that did not map to a known message.
    #[must_use]
    pub const fn message_type_value(&self) -> u8 {
        self.message_type_value
    }

    /// Payload length the frame declared.
    #[must_use]
    pub const fn payload_len(&self) -> u32 {
        self.payload_len
    }

    /// Total bytes consumed from the stream.
    #[must_use]
    pub const fn total_len(&self) -> usize {
        self.total_len
    }

    /// Session the frame belonged to.
    #[must_use]
    pub const fn session_id(&self) -> u64 {
        self.session_id
    }

    /// Transfer the frame belonged to.
    #[must_use]
    pub const fn transfer_id(&self) -> u64 {
        self.transfer_id
    }

    /// Sequence number the frame carried.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
}

/// What the decoder produced for one complete frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodedFrame {
    /// A frame this version understands.
    Message(Frame),
    /// A frame whose payload is ciphertext awaiting authentication.
    ///
    /// Its plaintext is not available here and must not be: only `qyro_crypto`
    /// can verify the tag, and nothing may treat unauthenticated bytes as data.
    Sealed(SealedFrame),
    /// A well-formed frame whose type this version does not implement.
    Unsupported(UnsupportedFrame),
}

impl DecodedFrame {
    /// Returns the message type, or `None` for an unsupported frame.
    #[must_use]
    pub const fn message_type(&self) -> Option<MessageType> {
        match self {
            Self::Message(frame) => Some(frame.message_type()),
            Self::Sealed(frame) => Some(frame.message_type()),
            Self::Unsupported(_) => None,
        }
    }

    /// Returns the payload, or an empty slice for an unsupported frame.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        match self {
            Self::Message(frame) => frame.payload(),
            // Never the ciphertext: unauthenticated bytes are not a payload.
            Self::Sealed(_) | Self::Unsupported(_) => &[],
        }
    }

    /// Re-serializes a known message.
    ///
    /// # Panics
    ///
    /// Panics on an unsupported frame, whose bytes were deliberately not kept.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Message(frame) => frame.encode(),
            Self::Sealed(frame) => frame.encode(),
            Self::Unsupported(_) => {
                panic!("an unsupported frame is not retained for re-serialization")
            }
        }
    }

    /// Returns the known message, if this is one.
    #[must_use]
    pub const fn as_message(&self) -> Option<&Frame> {
        match self {
            Self::Message(frame) => Some(frame),
            Self::Sealed(_) | Self::Unsupported(_) => None,
        }
    }

    /// Returns the sealed frame, if this is one.
    #[must_use]
    pub const fn as_sealed(&self) -> Option<&SealedFrame> {
        match self {
            Self::Sealed(frame) => Some(frame),
            Self::Message(_) | Self::Unsupported(_) => None,
        }
    }
}

/// Buffers bytes and yields complete frames.
#[derive(Debug)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
    max_buffer_len: usize,
    poisoned: Option<FrameError>,
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameDecoder {
    /// Creates a decoder bounded by [`MAX_BUFFER_LEN`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: Vec::new(),
            max_buffer_len: MAX_BUFFER_LEN,
            poisoned: None,
        }
    }

    /// Creates a decoder with a custom buffer ceiling.
    ///
    /// The ceiling is clamped to [`MAX_BUFFER_LEN`] so a caller cannot widen the
    /// bound past what the protocol guarantees.
    #[must_use]
    pub const fn with_max_buffer_len(max_buffer_len: usize) -> Self {
        let bounded = if max_buffer_len > MAX_BUFFER_LEN {
            MAX_BUFFER_LEN
        } else {
            max_buffer_len
        };
        Self {
            buffer: Vec::new(),
            max_buffer_len: bounded,
            poisoned: None,
        }
    }

    /// Bytes currently buffered.
    #[must_use]
    pub fn buffered_len(&self) -> usize {
        self.buffer.len()
    }

    /// Buffer capacity, exposed so tests can assert no hostile reservation.
    #[must_use]
    pub fn buffer_capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Whether a structural failure has poisoned the stream.
    #[must_use]
    pub const fn is_poisoned(&self) -> bool {
        self.poisoned.is_some()
    }

    /// Clears the buffer and the poisoned state.
    ///
    /// The only way out of a structural failure, and deliberately explicit:
    /// pushing more bytes must never look like recovery.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.poisoned = None;
    }

    /// Appends received bytes.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::BufferLimitExceeded`] when the bytes would push the
    /// buffer past its ceiling. The buffer is left untouched, so the caller can
    /// drain frames and retry rather than losing the connection state.
    pub fn push(&mut self, bytes: &[u8]) -> Result<(), FrameError> {
        if let Some(error) = self.poisoned {
            return Err(error);
        }
        let attempted =
            self.buffer
                .len()
                .checked_add(bytes.len())
                .ok_or(FrameError::BufferLimitExceeded {
                    attempted: usize::MAX,
                    limit: self.max_buffer_len,
                })?;
        if attempted > self.max_buffer_len {
            return Err(FrameError::BufferLimitExceeded {
                attempted,
                limit: self.max_buffer_len,
            });
        }
        self.buffer.extend_from_slice(bytes);
        Ok(())
    }

    /// Returns the next complete frame, or `None` when more bytes are needed.
    ///
    /// The declared lengths are validated by [`FrameHeader::decode`] before this
    /// method computes how many bytes to wait for, so a hostile length becomes
    /// an error instead of a reservation or an unbounded wait.
    ///
    /// # Errors
    ///
    /// Returns the structural [`FrameError`] that poisoned the stream. An
    /// unknown message type is **not** an error here; it arrives as
    /// [`DecodedFrame::Unsupported`].
    pub fn next_frame(&mut self) -> Result<Option<DecodedFrame>, FrameError> {
        if let Some(error) = self.poisoned {
            return Err(error);
        }
        if self.buffer.len() < HEADER_LEN {
            return Ok(None);
        }

        let (header, raw_message_type) = match FrameHeader::decode_parts(&self.buffer[..HEADER_LEN])
        {
            Ok(parts) => parts,
            Err(error) => return Err(self.poison(error)),
        };

        // total_len() is u64 and was already bounded by MAX_FRAME_LEN, so this
        // conversion cannot truncate on any supported target.
        let Ok(total) = usize::try_from(header.total_len()) else {
            return Err(self.poison(FrameError::FrameTooLarge {
                declared: header.total_len(),
                limit: self.max_buffer_len as u64,
            }));
        };

        if total > self.max_buffer_len {
            return Err(self.poison(FrameError::BufferLimitExceeded {
                attempted: total,
                limit: self.max_buffer_len,
            }));
        }

        if self.buffer.len() < total {
            return Ok(None);
        }

        // The frame is fully delimited from here on, so an unknown type can be
        // consumed cleanly and the stream stays synchronised. (ADR-0018)
        if MessageType::from_wire(raw_message_type).is_err() {
            self.buffer.drain(..total);
            return Ok(Some(DecodedFrame::Unsupported(UnsupportedFrame {
                message_type_value: raw_message_type,
                payload_len: header.payload_len(),
                total_len: total,
                session_id: header.session_id(),
                transfer_id: header.transfer_id(),
                sequence: header.sequence(),
            })));
        }

        // A sealed frame keeps its ciphertext and tag together and never
        // becomes a plain payload here.
        if header.flags().contains(Flags::ENCRYPTED) {
            let body = self.buffer[HEADER_LEN..total].to_vec();
            self.buffer.drain(..total);
            return match SealedFrame::from_parts(header, &body) {
                Ok(frame) => Ok(Some(DecodedFrame::Sealed(frame))),
                Err(error) => Err(self.poison(error)),
            };
        }

        let payload_end = HEADER_LEN + header.payload_len() as usize;
        let payload = self.buffer[HEADER_LEN..payload_end].to_vec();
        self.buffer.drain(..total);

        match Frame::from_parts(header, payload) {
            Ok(frame) => Ok(Some(DecodedFrame::Message(frame))),
            Err(error) => Err(self.poison(error)),
        }
    }

    fn poison(&mut self, error: FrameError) -> FrameError {
        self.poisoned = Some(error);
        error
    }
}
