//! Incremental frame decoder.
//!
//! A socket read is not a frame. It may deliver half a header, several frames,
//! or a frame split at any byte. This decoder buffers whatever arrives and
//! yields frames only once they are complete, under a hard memory ceiling.

use crate::error::FrameError;
use crate::frame::Frame;
use crate::header::FrameHeader;
use crate::limits::{HEADER_LEN, MAX_BUFFER_LEN};

/// Buffers bytes and yields complete frames.
///
/// After a framing error the decoder is poisoned: once the byte stream has
/// desynchronised there is no way to tell payload from header, so it refuses to
/// guess and keeps returning the same error until [`FrameDecoder::reset`].
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

    /// Whether a framing error has poisoned the stream.
    #[must_use]
    pub const fn is_poisoned(&self) -> bool {
        self.poisoned.is_some()
    }

    /// Clears the buffer and the poisoned state.
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
    /// Returns the [`FrameError`] that poisoned the stream.
    pub fn next_frame(&mut self) -> Result<Option<Frame>, FrameError> {
        if let Some(error) = self.poisoned {
            return Err(error);
        }
        if self.buffer.len() < HEADER_LEN {
            return Ok(None);
        }

        let header = match FrameHeader::decode(&self.buffer[..HEADER_LEN]) {
            Ok(header) => header,
            Err(error) => return Err(self.poison(error)),
        };

        // total_len() is u64 and was already bounded by MAX_FRAME_LEN, so this
        // conversion cannot truncate on any supported target.
        let total = match usize::try_from(header.total_len()) {
            Ok(total) => total,
            Err(_) => {
                return Err(self.poison(FrameError::FrameTooLarge {
                    declared: header.total_len(),
                    limit: self.max_buffer_len as u64,
                }));
            }
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

        let header_len = header.header_len as usize;
        let payload_end = header_len + header.payload_len as usize;
        let payload = self.buffer[header_len..payload_end].to_vec();
        self.buffer.drain(..total);

        match Frame::from_parts(header, payload) {
            Ok(frame) => Ok(Some(frame)),
            Err(error) => Err(self.poison(error)),
        }
    }

    fn poison(&mut self, error: FrameError) -> FrameError {
        self.poisoned = Some(error);
        error
    }
}
