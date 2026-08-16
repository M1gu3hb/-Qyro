//! Trust, fingerprints, refusals and pairing strings, at the C boundary.
//!
//! Specification: `docs/adr/ADR-0032-engine-ffi.md` amendment 1, and
//! `docs/adr/ADR-0035-discovery-and-pairing.md` for what the values mean.
//!
//! Every function here has the shape ADR-0032 froze: `i32` return, values out
//! through caller-allocated parameters, buffers length-delimited, body inside
//! [`guard`]. **Nothing crosses a type.** Integers and UTF-8 text, and the text
//! goes into a buffer the caller lent us (ADR-0038).

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::sync::{Mutex, OnceLock};

use qyro_session::{PeerTrust, RejectReason, TrustBook};

use crate::abi::{QYRO_ERR_BAD_ARGUMENT, QYRO_ERR_NULL_OUT, QYRO_ERR_POISONED, QYRO_OK, guard};
use crate::session_abi::{borrow_text, with_session_entry};

/// The one book this process has.
///
/// ADR-0032 amendment 1: an application has one, so a handle table for it would
/// be a table that can only be used wrongly. The table that does exist is for
/// sessions, which are several.
fn book() -> &'static Mutex<TrustBook> {
    static BOOK: OnceLock<Mutex<TrustBook>> = OnceLock::new();
    BOOK.get_or_init(|| Mutex::new(TrustBook::new()))
}

/// Writes `text` into the caller's buffer, or says how much it needed.
///
/// The whole text contract of ADR-0032 amendment 1 in one place, so five
/// functions cannot drift into five slightly different contracts.
///
/// **Nothing is written when it does not fit.** A partially written buffer
/// returned alongside an error code is how half a fingerprint gets compared out
/// loud, and half a fingerprint that matches proves nothing at all.
///
/// # Safety
///
/// `out` must address `capacity` writable bytes, or be null when `capacity` is
/// 0. `out_len` must point to one writable `usize`.
unsafe fn emit_text(text: &str, out: *mut u8, capacity: usize, out_len: *mut usize) -> i32 {
    if out_len.is_null() {
        return QYRO_ERR_NULL_OUT;
    }
    let bytes = text.as_bytes();
    // The length always, whether it fits or not: asking with `capacity == 0` is
    // the documented way to find out how much to allocate.
    // SAFETY: checked non-null immediately above.
    unsafe { out_len.write(bytes.len()) };

    if bytes.len() > capacity {
        return QYRO_ERR_BAD_ARGUMENT;
    }
    if bytes.is_empty() {
        return QYRO_OK;
    }
    if out.is_null() {
        return QYRO_ERR_NULL_OUT;
    }
    // SAFETY: `capacity` writable bytes were promised, and `bytes.len()` is not
    // greater than `capacity` by the check above.
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len()) };
    QYRO_OK
}

/// The peer's fingerprint, formatted by the core, ready to show.
///
/// ADR-0035 §4: the interface never invents a format. Two devices rendering the
/// same fingerprint differently makes comparing it out loud worthless, and
/// comparing it out loud is the only thing a fingerprint is for.
///
/// # Safety
///
/// `out` must address `capacity` writable bytes or be null when `capacity` is 0;
/// `out_len` must point to one writable `usize`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_session_peer_fingerprint(
    handle: u64,
    out: *mut u8,
    capacity: usize,
    out_len: *mut usize,
) -> i32 {
    guard(|| {
        with_session_entry(handle, |entry| {
            let text = entry.session.peer_fingerprint();
            // SAFETY: the caller's contract, stated above.
            unsafe { emit_text(&text, out, capacity, out_len) }
        })
    })
}

/// The address this end bound, so it can be put in a pairing string.
///
/// # Safety
///
/// As [`qyro_session_peer_fingerprint`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_session_local_address(
    handle: u64,
    out: *mut u8,
    capacity: usize,
    out_len: *mut usize,
) -> i32 {
    guard(|| {
        with_session_entry(handle, |entry| {
            let Ok(address) = entry.session.local_addr() else {
                return QYRO_ERR_BAD_ARGUMENT;
            };
            // SAFETY: the caller's contract.
            unsafe { emit_text(&address.to_string(), out, capacity, out_len) }
        })
    })
}

/// What the book says about the peer **this handshake authenticated**.
///
/// `0` known · `1` **changed** · `2` new. ADR-0035 §3: the identity handed to
/// the decision is the authenticated one, never a fingerprint that arrived in a
/// pairing string.
///
/// # Safety
///
/// `name` must address `name_len` readable bytes; `out_verdict` must point to
/// one writable `i32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_session_peer_trust(
    handle: u64,
    name: *const u8,
    name_len: usize,
    out_verdict: *mut i32,
) -> i32 {
    guard(|| {
        if out_verdict.is_null() {
            return QYRO_ERR_NULL_OUT;
        }
        // SAFETY: the caller's contract.
        let Some(name) = (unsafe { borrow_text(name, name_len) }) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        let Ok(book) = book().lock() else {
            return QYRO_ERR_POISONED;
        };
        with_session_entry(handle, |entry| {
            match entry.session.peer_trust(&book, name) {
                Ok(verdict) => {
                    // SAFETY: checked non-null above.
                    unsafe { out_verdict.write(trust_code(verdict)) };
                    QYRO_OK
                }
                Err(_) => QYRO_ERR_BAD_ARGUMENT,
            }
        })
    })
}

/// Records this peer under `name`. **Only a person may cause this.**
///
/// ADR-0035 §4: a peer never enters the book because a transfer succeeded.
///
/// # Safety
///
/// `name` must address `name_len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_session_remember_peer(
    handle: u64,
    name: *const u8,
    name_len: usize,
) -> i32 {
    guard(|| {
        // SAFETY: the caller's contract.
        let Some(name) = (unsafe { borrow_text(name, name_len) }) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        let Ok(mut book) = book().lock() else {
            return QYRO_ERR_POISONED;
        };
        with_session_entry(handle, |entry| {
            match entry.session.remember_peer(&mut book, name) {
                Ok(()) => QYRO_OK,
                Err(_) => QYRO_ERR_BAD_ARGUMENT,
            }
        })
    })
}

/// Forgets `name`. `out_removed` gets 1 if there was something, 0 if not.
///
/// The only way back from a changed key, and deliberately a separate act.
///
/// # Safety
///
/// `name` must address `name_len` readable bytes; `out_removed` must point to
/// one writable `i32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_trust_forget_peer(
    name: *const u8,
    name_len: usize,
    out_removed: *mut i32,
) -> i32 {
    guard(|| {
        if out_removed.is_null() {
            return QYRO_ERR_NULL_OUT;
        }
        // SAFETY: the caller's contract.
        let Some(name) = (unsafe { borrow_text(name, name_len) }) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        let Ok(mut book) = book().lock() else {
            return QYRO_ERR_POISONED;
        };
        let removed = i32::from(book.forget(name));
        // SAFETY: checked non-null above.
        unsafe { out_removed.write(removed) };
        QYRO_OK
    })
}

/// Every remembered name, separated by NUL.
///
/// NUL because it is the one byte a name cannot contain — the peer store refuses
/// control characters — so splitting on it is exact and no name needs escaping.
/// The same reasoning as the path list of `qyro_session_open_sender_blocking`.
///
/// # Safety
///
/// As [`qyro_session_peer_fingerprint`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_trust_list_peers(
    out: *mut u8,
    capacity: usize,
    out_len: *mut usize,
) -> i32 {
    guard(|| {
        let Ok(book) = book().lock() else {
            return QYRO_ERR_POISONED;
        };
        let joined = book.names().join("\0");
        // SAFETY: the caller's contract.
        unsafe { emit_text(&joined, out, capacity, out_len) }
    })
}

/// Refuses the offered transfer, with a reason the sender will see.
///
/// `reason`: 0 declined · 1 no room · 2 unacceptable manifest · anything else
/// unspecified. QYR-0089: until this existed the only refusal a receiver could
/// express was a cancel, which says something different.
#[unsafe(no_mangle)]
pub extern "C" fn qyro_session_reject(handle: u64, reason: i32) -> i32 {
    guard(|| {
        with_session_entry(handle, |entry| {
            match entry.session.reject(reason_of(reason)) {
                Ok(()) => QYRO_OK,
                Err(_) => QYRO_ERR_BAD_ARGUMENT,
            }
        })
    })
}

/// Why the receiver refused, or `-1` if it did not.
///
/// `SessionState::Rejected` says the transfer did not happen; this says why, and
/// that is the difference between «could not send it» and «they said no» on a
/// screen.
///
/// # Safety
///
/// `out_reason` must point to one writable `i32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_session_rejection(handle: u64, out_reason: *mut i32) -> i32 {
    guard(|| {
        if out_reason.is_null() {
            return QYRO_ERR_NULL_OUT;
        }
        with_session_entry(handle, |entry| {
            let code = entry.session.rejection().map_or(-1, reason_code);
            // SAFETY: checked non-null above.
            unsafe { out_reason.write(code) };
            QYRO_OK
        })
    })
}

/// Validates a pairing string and hands back its address half.
///
/// The fingerprint half is **not** returned here on purpose: it is an
/// expectation to compare against the authenticated fingerprint, not a value the
/// interface should display as if it were established (ADR-0035 §2.1). What the
/// caller does with a valid string is dial the address and then compare
/// [`qyro_session_peer_fingerprint`] against what it scanned.
///
/// # Safety
///
/// `text` must address `text_len` readable bytes; `out`/`out_len` as in
/// [`qyro_session_peer_fingerprint`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_pairing_parse(
    text: *const u8,
    text_len: usize,
    out: *mut u8,
    capacity: usize,
    out_len: *mut usize,
) -> i32 {
    guard(|| {
        // SAFETY: the caller's contract.
        let Some(text) = (unsafe { borrow_text(text, text_len) }) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        let Ok(endpoint) = qyro_session::parse_pairing(text) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        // SAFETY: the caller's contract.
        unsafe { emit_text(&endpoint, out, capacity, out_len) }
    })
}

/// The integer a verdict travels as. Written out, not derived from the ordering.
const fn trust_code(verdict: PeerTrust) -> i32 {
    verdict.code()
}

/// The integer a reason travels as.
const fn reason_code(reason: RejectReason) -> i32 {
    reason.code()
}

/// The reason an integer names. Total: an unknown value is «unspecified»
/// rather than a refusal, because a caller from a later version asking for a
/// reason this build has not heard of still meant «no».
const fn reason_of(code: i32) -> RejectReason {
    match code {
        0 => RejectReason::Declined,
        1 => RejectReason::NoRoom,
        2 => RejectReason::UnacceptableManifest,
        _ => RejectReason::Unspecified,
    }
}
