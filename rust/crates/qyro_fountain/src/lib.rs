//! Fountain coding for the optical channel.
//!
//! Specification: ADR-0044 §4.
//!
//! One payload goes in and an **endless stream of frames** comes out. The
//! receiver collects frames until it has enough — any frames, in any order,
//! with any of them missing — and rebuilds the payload. That is the whole
//! contract, and it exists because a screen does not rewind: a camera that
//! misses a frame cannot ask for it again, and every fixed-piece scheme turns
//! one missed frame into starting over.
//!
//! # What is here and what is not
//!
//! This crate does the coding and nothing else. It does not know what a QR code
//! is, does not draw anything, and has no opinion about how frames travel — the
//! optical channel is one carrier and the same frames survive any other. Keeping
//! it that way is what makes it testable without a screen or a camera.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

mod error;
mod lt;
mod rng;
mod wire;

#[cfg(test)]
mod guards;

#[cfg(test)]
mod tests;

pub use error::WireError;
pub use lt::{Decoder, Frame, Shape, encode, neighbours, split};
pub use rng::Rng;
pub use wire::{FRAME_HEADER_LEN, decode_frame, encode_frame};
