//! The only crate `qyro_ffi` sees.
//!
//! Specification: `docs/adr/ADR-0032-engine-ffi.md`.
//!
//! # Why this crate exists
//!
//! Until phase 01, `qyro_ffi` had no path to `qyro_crypto` at all, and a test
//! asserted its dependency closure was exactly `{qyro_core, qyro_ffi}`. That
//! made the guarantee structural: the code that would leak a key could not be
//! made to compile.
//!
//! Driving a transfer needs the engine, and the engine needs the AEAD, so the
//! cryptographic stack necessarily enters the library Dart loads. **The
//! measurement in ADR-0032 Â§1 is the one that shapes this crate**: once
//! `qyro_crypto` is inside the closure, a closure-shaped test is blind to
//! `qyro_ffi` taking a *direct* edge to it â€” the difference is the empty set.
//!
//! So the boundary moves from reachability to **nameability**, and this crate is
//! what makes that checkable. Rust puts only *direct* dependencies in a crate's
//! extern prelude, so `qyro_ffi` naming exactly `qyro_core` and `qyro_session`
//! is a resolver-verified proof that everything it can reach of the crypto stack
//! is bounded by the public surface below.
//!
//! # The rule this crate lives by
//!
//! **No item in this crate's public API may be, contain, or hand back a type
//! from `qyro_crypto`.** That includes the shapes with no crypto name in them:
//! re-exporting `qyro_net::Session` would expose `into_parts`, which returns a
//! `FrameSealer` and a `FrameOpener` by inference, with `qyro_crypto` appearing
//! nowhere in the signature a reader sees.

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
mod trust;

#[cfg(test)]
mod guards;

pub use error::SessionError;
pub use session::{Progress, ProgressObserver, Session, SessionState};
pub use trust::{PeerTrust, TrustBook, fingerprint_text};
