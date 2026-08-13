//! Stable native boundary for Qyro.
//!
//! Specification: `docs/adr/ADR-0032-engine-ffi.md`.
//!
//! The returned protocol-version memory is static, immutable, and always owned
//! by this library. Callers must never free or mutate it.
//!
//! # What this crate may name
//!
//! Exactly `qyro_core` and `qyro_session`, and that is enforced rather than
//! documented: `tests/c_abi_contract.rs` asks the resolver for this crate's
//! *direct* dependencies and pins the set. Rust puts only direct dependencies in
//! a crate's extern prelude, so the cryptographic stack -- which is linked
//! underneath this library, since driving a transfer needs it -- cannot be named
//! here at all. That is the whole boundary, and ADR-0032 §1 explains why it sits
//! at depth one rather than over the dependency closure.

// Public so that the C entry points in step 4 are a thin layer over parts that
// can be tested on their own, and so that neither is dead code in the meantime.
// Nothing here can name anything cryptographic; see the module note above.
pub mod abi;
pub mod handle;
mod session_abi;

#[cfg(test)]
mod guards;

fn protocol_version_bytes() -> &'static [u8] {
    qyro_core::protocol_version().as_bytes()
}

/// Returns a borrowed pointer to the UTF-8 protocol-version bytes.
///
/// Read exactly the number of bytes returned by qyro_protocol_version_len.
/// The pointer remains valid for the process lifetime; ownership is not moved.
#[unsafe(no_mangle)]
pub extern "C" fn qyro_protocol_version_ptr() -> *const u8 {
    protocol_version_bytes().as_ptr()
}

/// Returns the byte length of the value from qyro_protocol_version_ptr.
#[unsafe(no_mangle)]
pub extern "C" fn qyro_protocol_version_len() -> usize {
    protocol_version_bytes().len()
}
