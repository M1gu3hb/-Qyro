//! The Qyro transfer session: moves a whole transfer over sealed frames.
//!
//! Specification: `docs/adr/ADR-0026-transfer-session.md`.
//!
//! # What this does and what it does not
//!
//! It moves a transfer between two ends that exchange nothing but `Vec<u8>` of
//! sealed frames. **There is no transport and no filesystem**: the source of
//! bytes is a [`ContentSource`] and the destination is a [`ContentSink`], and
//! this crate does not know or care what is behind either. Sockets are not
//! written yet; files are sprint 5B.
//!
//! Sender and receiver hold no reference to each other. That is what makes it
//! possible for a test to drop, reorder or replay a frame without a transport
//! existing to do it.
//!
//! # What it never does
//!
//! It invents no cryptography. Every frame goes through [`FrameSealer`] and
//! [`FrameOpener`] from `qyro_crypto`, which already authenticate, number and
//! refuse replays. This crate decides who speaks when.
//!
//! [`FrameSealer`]: qyro_crypto::aead::FrameSealer
//! [`FrameOpener`]: qyro_crypto::aead::FrameOpener

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
mod session;
mod wire;

#[cfg(test)]
mod guards;
#[cfg(test)]
mod tests;

pub use error::{ItemVerdict, TransferError};
pub use session::{CHUNK_SIZE, ContentSink, ContentSource, Phase, Receiver, Sender, WINDOW_CHUNKS};
pub use wire::{Accept, Ack, ChunkRef, Complete, Control, Integrity, ItemStart, Offer};
