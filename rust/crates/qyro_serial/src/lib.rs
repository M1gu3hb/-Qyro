//! The serial channel: a file into a machine that cannot read a QR.
//!
//! Specification: ADR-0045. The measurements are `R8` §5.
//!
//! This is the literal answer to the scene in `R7` §2: an old desktop whose USB
//! ports are dead, that can *show* a QR but has no camera to read one. What it
//! has, with certainty, is a DB9 — and 115 200 bps 8N1 moves **9–11 KB/s**,
//! which is a megabyte in 1.6 minutes and an order of magnitude past the
//! optical channel.
//!
//! # It talks to `Read + Write` and knows nothing else
//!
//! No port is opened here and no dependency is taken. The protocol is exercised
//! end to end over a pipe, which is the only honest test available without a
//! cable — and the class of that evidence is written down in exactly those
//! words: **not over a physical UART and not over a null-modem cable.**

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

mod arq;
mod base64;
mod bootstrap;
mod crc;
mod error;

#[cfg(test)]
mod guards;

#[cfg(test)]
mod tests;

pub use arq::{
    BLOCK_BYTES, Block, LINE_PREFIX, MAX_ATTEMPTS, Receiver, Reply, Tally, block_of, line_of,
    receive_all, send_all, split,
};
pub use bootstrap::{DEGRADED_WARNING, Target, receiver_for};
pub use crc::crc32;
pub use error::SerialError;
