//! A complete frame: header plus payload.

use crate::error::FrameError;
use crate::header::FrameHeader;
use crate::limits::MAX_PAYLOAD_LEN;
use crate::message::{Flags, MessageType};

/// An owned QYRO/1 frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    header: FrameHeader,
    payload: Vec<u8>,
}

impl Frame {
    /// Builds a frame, deriving `payload_len` from the payload itself.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::PayloadTooLarge`] when the payload exceeds
    /// [`MAX_PAYLOAD_LEN`].
    pub fn new(message_type: MessageType, payload: Vec<u8>) -> Result<Self, FrameError> {
        if payload.len() > MAX_PAYLOAD_LEN {
            return Err(FrameError::PayloadTooLarge {
                declared: u32::try_from(payload.len()).unwrap_or(u32::MAX),
                limit: MAX_PAYLOAD_LEN as u32,
            });
        }
        let payload_len = u32::try_from(payload.len()).expect("payload fits in u32 after check");
        Ok(Self {
            header: FrameHeader::new(message_type, payload_len)?,
            payload,
        })
    }

    /// Builds a frame from an already validated header and its payload.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::TruncatedPayload`] when the payload length does not
    /// match what the header declares.
    pub fn from_parts(header: FrameHeader, payload: Vec<u8>) -> Result<Self, FrameError> {
        if payload.len() != header.payload_len() as usize {
            return Err(FrameError::TruncatedPayload {
                available: payload.len(),
                required: header.payload_len() as usize,
            });
        }
        Ok(Self { header, payload })
    }

    /// Returns the header.
    #[must_use]
    pub const fn header(&self) -> &FrameHeader {
        &self.header
    }

    /// Returns the payload.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Consumes the frame and returns its payload.
    #[must_use]
    pub fn into_payload(self) -> Vec<u8> {
        self.payload
    }

    /// Returns the message type.
    #[must_use]
    pub const fn message_type(&self) -> MessageType {
        self.header.message_type()
    }

    /// Sets the routing identifiers.
    #[must_use]
    pub const fn with_identifiers(
        mut self,
        session_id: u64,
        transfer_id: u64,
        stream_id: u32,
        item_id: u32,
    ) -> Self {
        self.header = self
            .header
            .with_identifiers(session_id, transfer_id, stream_id, item_id);
        self
    }

    /// Sets the sequence number.
    #[must_use]
    pub const fn with_sequence(mut self, sequence: u64) -> Self {
        self.header = self.header.with_sequence(sequence);
        self
    }

    /// Sets transport flags.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::UnsupportedFlag`] for `ENCRYPTED` or `COMPRESSED`.
    /// Those assert something about the payload a caller cannot make true, so
    /// only the sealing path in `qyro_crypto` may set them.
    pub fn with_flags(mut self, flags: Flags) -> Result<Self, FrameError> {
        self.header = self.header.with_transport_flags(flags)?;
        Ok(self)
    }

    /// Serializes the frame to bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let header = self.header.encode();
        let mut out = Vec::with_capacity(header.len() + self.payload.len());
        out.extend_from_slice(&header);
        out.extend_from_slice(&self.payload);
        out
    }

    /// Appends the encoded frame to `out`, avoiding an intermediate allocation.
    pub fn encode_into(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.header.encode());
        out.extend_from_slice(&self.payload);
    }
}
