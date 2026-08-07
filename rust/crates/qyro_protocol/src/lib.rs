//! QYRO/1 binary wire framing.
//!
//! This crate owns the frame layout, its bounds, and the incremental decoder
//! that turns an untrusted byte stream into typed frames. It deliberately knows
//! nothing about sockets, TLS, files, cryptography or storage.
//!
//! # Safety posture
//!
//! The decoder is the first code that touches bytes from a peer, so the crate
//! is built around one rule: **a peer's declared length is validated against a
//! compile-time constant before it can cause an allocation or an unbounded
//! wait.** [`FrameHeader::decode`] checks magic, version and every length
//! before the caller learns how many bytes a frame needs, and
//! [`FrameDecoder::push`] refuses to grow past its ceiling.
//!
//! The crate has no external dependencies, so every parsing path is auditable
//! in this tree.
//!
//! # Example
//!
//! ```
//! use qyro_protocol::{Frame, FrameDecoder, MessageType};
//!
//! let frame = Frame::new(MessageType::Hello, b"qyro".to_vec())?;
//! let bytes = frame.encode();
//!
//! let mut decoder = FrameDecoder::new();
//! // Bytes may arrive in arbitrarily small pieces.
//! for chunk in bytes.chunks(7) {
//!     decoder.push(chunk)?;
//! }
//!
//! let decoded = decoder.next_frame()?.expect("one complete frame");
//! assert_eq!(decoded.message_type(), Some(MessageType::Hello));
//! assert_eq!(decoded.plaintext(), Some(&b"qyro"[..]));
//! # Ok::<(), qyro_protocol::FrameError>(())
//! ```

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod decoder;
mod envelope;
mod error;
mod frame;
#[cfg(test)]
mod guards;
mod header;
mod limits;
mod message;
mod session;
mod version;

pub use decoder::{DecodedFrame, FrameDecoder, UnsupportedFrame};
pub use envelope::EncryptedEnvelope;
pub use error::{FrameError, IdentifierField};
pub use frame::Frame;
pub use header::FrameHeader;
pub use limits::{
    HEADER_LEN, MAX_BUFFER_LEN, MAX_FRAME_LEN, MAX_HEADER_LEN, MAX_PAYLOAD_LEN, MAX_TRAILER_LEN,
    SUPPORTED_TRAILER_LEN,
};
pub use message::{Flags, MessageType};
pub use session::{SESSION_ID_LEN, SessionId};

pub use version::{MAGIC, PROTOCOL_VERSION, VERSION_MAJOR, VERSION_MINOR};
