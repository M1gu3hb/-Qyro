//! DPAPI-backed persistence for a Qyro device identity. **Windows only.**
//!
//! This is the one crate in the product that relaxes `#![forbid(unsafe_code)]`,
//! and ADR-0024 §1 argues why: keeping the `unsafe` in a crate of its own was
//! preferred to putting it in the crate that holds keys, and preferred to
//! pulling eleven crates into an audited graph for two function declarations.
//!
//! The surface, in a sentence: `CryptProtectData`, `CryptUnprotectData`,
//! `LocalFree` and `GetLastError`, plus a `#[repr(C)] DATA_BLOB`. Nothing else,
//! and `src/guards.rs` enumerates every function containing an `unsafe` block by
//! name so it cannot grow quietly.
//!
//! # What this does not protect against
//!
//! An attacker already running code as this user decrypts the blob by calling
//! the same API with the same compiled-in entropy. DPAPI protects against other
//! users and other machines, not against the user being compromised. See
//! `THREAT_MODEL.md`.

// Not `forbid`: this crate is the exception, and the exception is argued in
// ADR-0024 §1 and enforced by `only_the_listed_crates_may_relax_forbid_unsafe`
// in `qyro_identity_store`, which requires it to be named in a list.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
// Gated per module rather than at the crate root. `#![cfg(windows)]` would make
// the whole crate vanish elsewhere — including `guards`, which reads these files
// as text and needs no Windows at all. The point of that guard is that the
// `unsafe` surface cannot grow unnoticed, and a guard that only runs on one
// platform is a guard that is off for most of CI.
#[cfg(windows)]
mod ffi;
#[cfg(windows)]
mod store;

#[cfg(test)]
mod guards;
#[cfg(all(windows, test))]
mod tests;

#[cfg(windows)]
pub use store::{DpapiWrapper, WindowsIdentityStore};
