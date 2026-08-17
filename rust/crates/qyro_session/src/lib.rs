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

mod bridged_wrapper;
mod discovery;
mod error;
mod identity;
mod link;
mod session;
mod trust;

#[cfg(test)]
mod guards;

pub use bridged_wrapper::{
    BRIDGED_WRAP_ID, BridgedWrapper, DOMAIN_MISMATCH, MAX_WRAPPED_LEN, WrapFn,
};
pub use discovery::{FoundPeer, browse};
pub use error::SessionError;
pub use identity::{Protection, fingerprint, install_wrapper, open};
pub use link::{APIPA_BUDGET, LinkState, is_apipa, is_reachable_by_a_peer, wait_for_link};

/// The port ADR-0041 froze, forwarded so consumers stop keeping their own copy.
///
/// **This was written as `pub use qyro_net::QYRO_PORT` and the guard refused
/// it**, correctly: `qyro_session_re_exports_nothing_it_does_not_own` exists
/// because everything this facade republishes becomes nameable from `qyro_ffi`,
/// and a re-export judged harmless one item at a time is how the first
/// dangerous one arrives.
///
/// A `const` is not the thing that guard forbids, and the difference is not
/// syntax. A re-export makes `qyro_net`'s **item** reachable through this
/// crate; this makes a `u16` reachable and nothing else — no type, no method,
/// no `into_parts`. There is still exactly one definition, in `qyro_net`, so
/// the drift this is meant to end cannot come back through here.
pub const QYRO_PORT: u16 = qyro_net::QYRO_PORT;
pub use session::{Progress, ProgressObserver, RejectReason, Session, SessionState, parse_pairing};
pub use trust::{PeerTrust, TrustBook, fingerprint_text};
