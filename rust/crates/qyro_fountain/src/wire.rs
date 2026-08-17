//! What a frame looks like as bytes, which is what a QR code carries.
//!
//! Specification: ADR-0044 §2 — **raw byte mode, never Base64**. Base64 costs
//! +33 % and BC-UR's bytewords +37.5 %; this project controls both ends, so the
//! byte mode costs 0 %.
//!
//! ```text
//! offset  size  field
//!      0     2  magic, b"QF"
//!      2     1  version, 1
//!      3     8  seed
//!     11     4  payload_len of the ORIGINAL payload
//!     15     2  block_size
//!     17     n  the XOR of the blocks the seed chooses
//! ```
//!
//! Seventeen bytes of header against v27-L's 1 465 B capacity: **1.2 %**. The
//! shape travels in every frame rather than once at the start, because a
//! receiver may point the camera at a stream that is already running, and a
//! header it missed would be a transfer it could not join.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::error::WireError;
use crate::lt::{Frame, Shape};

/// `b"QF"`, so a frame from another protocol is refused rather than misread.
const MAGIC: [u8; 2] = *b"QF";

/// The only version that exists.
const VERSION: u8 = 1;

/// Bytes of header before the payload.
pub const FRAME_HEADER_LEN: usize = 17;

/// Serialises a frame.
#[must_use]
pub fn encode_frame(frame: &Frame) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(FRAME_HEADER_LEN + frame.payload.len());
    bytes.extend_from_slice(&MAGIC);
    bytes.push(VERSION);
    bytes.extend_from_slice(&frame.seed.to_be_bytes());
    bytes.extend_from_slice(&frame.shape.payload_len.to_be_bytes());
    bytes.extend_from_slice(&frame.shape.block_size.to_be_bytes());
    bytes.extend_from_slice(&frame.payload);
    bytes
}

/// Parses a frame, refusing anything it cannot vouch for.
///
/// # Errors
///
/// See [`WireError`]. Every one of them is something a camera really hands over.
pub fn decode_frame(bytes: &[u8]) -> Result<Frame, WireError> {
    let header = bytes.get(..FRAME_HEADER_LEN).ok_or(WireError::TooShort)?;
    if header.get(..2) != Some(&MAGIC) {
        return Err(WireError::NotAFrame);
    }
    let version = *header.get(2).ok_or(WireError::TooShort)?;
    if version != VERSION {
        return Err(WireError::UnknownVersion(version));
    }

    let seed = u64::from_be_bytes(
        header
            .get(3..11)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(WireError::TooShort)?,
    );
    let payload_len = u32::from_be_bytes(
        header
            .get(11..15)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(WireError::TooShort)?,
    );
    let block_size = u16::from_be_bytes(
        header
            .get(15..17)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(WireError::TooShort)?,
    );

    if block_size == 0 || payload_len == 0 {
        return Err(WireError::ImpossibleShape);
    }

    let payload = bytes.get(FRAME_HEADER_LEN..).ok_or(WireError::TooShort)?;
    if payload.len() != block_size as usize {
        // A frame whose payload is not exactly one block is not a frame this
        // decoder can XOR. Refused rather than padded: padding would silently
        // change the bytes and the failure would surface as a hash mismatch
        // with nothing to point at.
        return Err(WireError::BlockSizeMismatch);
    }

    Ok(Frame {
        seed,
        shape: Shape {
            payload_len,
            block_size,
        },
        payload: payload.to_vec(),
    })
}
