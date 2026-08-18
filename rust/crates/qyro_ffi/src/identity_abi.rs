//! The device identity, at the C boundary.
//!
//! Specification: `docs/adr/ADR-0040-identity-persistence.md`.
//!
//! Three functions, and between them they close the defect that made every
//! other function on this boundary less useful than it looked: until phase 11
//! the engine minted a fresh keypair per session, so the fingerprint
//! `qyro_session_peer_fingerprint` reported was a fingerprint of nothing lasting
//! and `qyro_session_peer_trust` could only ever answer "new".
//!
//! Same shape as the rest of this surface (ADR-0032 §5): `i32` return, values
//! out through caller-lent buffers, lengths delimited, body inside [`guard`].
//! **Nothing crosses a type.** A path in, text out, two function pointers of
//! scalars and one `usize` of opaque context.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::path::Path;

use qyro_session::{Protection, WrapFn};

use crate::abi::{QYRO_ERR_BAD_ARGUMENT, QYRO_ERR_NULL_OUT, QYRO_OK, guard};
use crate::session_abi::{borrow_text, session_code};
use crate::trust_abi::emit_text;

/// The same signature `qyro_session::WrapFn` already declares.
///
/// Taken as an `Option<...>` and not the bare type: calling a null
/// `extern "C" fn` is undefined behaviour, and checking for it is the
/// boundary's debt rather than the caller's.
pub type QyroWrapFn = WrapFn;

/// Opens the identity stored at `path`, creating one if the store is empty.
///
/// **Must succeed before any session is opened.** If it has not, all three
/// `qyro_session_open_*` functions answer `QYRO_ERR_IDENTITY_UNREADABLE`
/// rather than generating a throwaway identity, which is the behaviour ADR-0040
/// exists to guarantee.
///
/// `protection` is 0 for the platform wrapper (DPAPI on Windows, or whatever
/// [`qyro_identity_set_wrapper`] installed) and 1 for the filesystem sandbox
/// alone. **0 never falls back to 1**: a caller that asks for platform
/// protection on a platform with no wrapper gets `QYRO_ERR_BAD_ARGUMENT`, not
/// less protection than it asked for.
///
/// Blocking: it touches a disk and may draw from the system CSPRNG.
///
/// # Safety
///
/// `path` must address `path_len` readable bytes of UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_identity_open_blocking(
    path: *const u8,
    path_len: usize,
    protection: u32,
) -> i32 {
    guard(|| {
        // SAFETY: the caller promises `path_len` readable bytes at `path`.
        let Some(text) = (unsafe { borrow_text(path, path_len) }) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        if text.is_empty() {
            return QYRO_ERR_BAD_ARGUMENT;
        }
        let Some(protection) = Protection::from_code(protection) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        match qyro_session::open(Path::new(text), protection) {
            Ok(()) => QYRO_OK,
            Err(error) => session_code(error),
        }
    })
}

/// Installs the platform wrapper this process will use for its identity.
///
/// Call before [`qyro_identity_open_blocking`]. Two function pointers and an
/// opaque context: the host implements the pair, and no secret ever travels
/// upward through this boundary as a value the caller can keep.
///
/// Not gated behind `cfg(target_os = "android")` on purpose. A fake wrapper
/// installable on Windows and Linux is what lets CI exercise the bridged path
/// at all — and a bridged path nothing exercised is exactly how the wrap-byte
/// defect of ADR-0040 §5 survived two phases.
///
/// # Safety
///
/// `wrap` and `unwrap` must be valid for the life of the process and callable
/// from any thread, because the engine calls them from whichever thread is
/// driving a session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_identity_set_wrapper(
    wrap: Option<QyroWrapFn>,
    unwrap: Option<QyroWrapFn>,
    context: usize,
) -> i32 {
    guard(|| {
        let (Some(wrap), Some(unwrap)) = (wrap, unwrap) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        match qyro_session::install_wrapper(wrap, unwrap, context) {
            Ok(()) => QYRO_OK,
            Err(error) => session_code(error),
        }
    })
}

/// This device's own fingerprint, grouped for reading aloud.
///
/// Ask with `capacity == 0` and `out == NULL` to learn the length, exactly as
/// `qyro_session_peer_fingerprint` does. **Nothing is written when it does not
/// fit**, because half a fingerprint that matches proves nothing.
///
/// This is what lets the application show its own pairing code. Before
/// ADR-0040 there was no stable identity to build one from, so
/// `ownPairingString()` returned null unconditionally and the manual pairing
/// path — the one that works on every network — could not be used in either
/// direction.
///
/// # Safety
///
/// `out` must address `capacity` writable bytes, or be null when `capacity` is
/// 0. `out_len` must point to one writable `usize`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_identity_fingerprint(
    out: *mut u8,
    capacity: usize,
    out_len: *mut usize,
) -> i32 {
    guard(|| {
        if out_len.is_null() {
            return QYRO_ERR_NULL_OUT;
        }
        match qyro_session::fingerprint() {
            // SAFETY: `out_len` checked non-null above; `out`/`capacity` are the
            // caller's promise, and `emit_text` writes nothing when it does not
            // fit.
            Ok(text) => unsafe { emit_text(&text, out, capacity, out_len) },
            Err(error) => session_code(error),
        }
    })
}

/// The channel advice, as the sentence both faces show.
///
/// ADR-0046 §4 and §5. The engine decides **and formats**: advice that crossed
/// as an enum would become «channel 3» in one face and a paragraph in the other,
/// and those are two products. The same reasoning that put
/// `qyro_identity_fingerprint` here rather than letting each side format a
/// fingerprint its own way.
///
/// The four flags are facts the caller can see and the engine cannot — whether
/// an address exists, whether anybody answered, whether this machine has a
/// serial port, whether the other one has a camera. **Nothing here is a
/// preference**; the ordering and the estimates are the engine's.
///
/// Ask with `capacity == 0` and `out == NULL` to learn the length, exactly as
/// every other text symbol on this surface. **Nothing is written when it does
/// not fit.**
///
/// # Safety
///
/// `out` must address `capacity` writable bytes, or be null when `capacity` is
/// 0. `out_len` must point to one writable `usize`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_advice(
    has_network: i32,
    peer_discovered: i32,
    has_serial_port: i32,
    other_has_camera: i32,
    payload_len: u64,
    out: *mut u8,
    capacity: usize,
    out_len: *mut usize,
) -> i32 {
    guard(|| {
        if out_len.is_null() {
            return QYRO_ERR_NULL_OUT;
        }
        let situation = qyro_session::Situation {
            has_network: has_network != 0,
            peer_discovered: peer_discovered != 0,
            has_serial_port: has_serial_port != 0,
            other_has_camera: other_has_camera != 0,
            payload_len,
        };
        let (text, _channels) = qyro_session::advise(situation);
        // SAFETY: `out_len` checked non-null above; `out`/`capacity` are the
        // caller's promise, and `emit_text` writes nothing when it does not fit.
        unsafe { emit_text(&text, out, capacity, out_len) }
    })
}
