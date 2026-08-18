//! Return codes, and the panic boundary every `extern "C"` function sits behind.
//!
//! Specification: `docs/adr/ADR-0032-engine-ffi.md` §5. Every `extern "C"`
//! function returns `i32`: `0` is success, negatives are errors, and values
//! leave through out-parameters. There is no thread-local `last_error` as the
//! primary channel, because it is read after the fact and Dart does not promise
//! the next call arrives on the thread that failed.
//!
//! # The panic boundary
//!
//! A `panic!` crossing an `extern "C"` frontier is undefined behaviour. ADR-0032
//! §5.5 does not offer this as a choice: every such function wraps its body in
//! [`guard`], which converts a panic into [`QYRO_ERR_PANIC`].
//!
//! [`guard`] is outermost deliberately. Anything running outside it -- resolving
//! a handle, taking the table lock, reading an out-pointer -- would be code whose
//! panic still crosses the boundary, so there is nothing above it to get wrong.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::handle::HandleError;

/// The call succeeded.
pub const QYRO_OK: i32 = 0;

/// The handle does not name a live session.
///
/// One code for every way resolution fails, matching `HandleError::NotLive`:
/// a handle that was never valid and one that stopped being valid are not a
/// difference a caller can act on.
pub const QYRO_ERR_INVALID_HANDLE: i32 = -1;

/// `MAX_ESTABLISHED_SESSIONS` sessions are already open.
pub const QYRO_ERR_TABLE_FULL: i32 = -2;

/// A panic was caught at the boundary. The session, if any, is poisoned.
pub const QYRO_ERR_PANIC: i32 = -3;

/// A required out-parameter was null.
pub const QYRO_ERR_NULL_OUT: i32 = -4;

/// The table lock was poisoned by a panic in an earlier call.
pub const QYRO_ERR_POISONED: i32 = -5;

// One code per `SessionError` variant. They are a flat translation on purpose:
// collapsing two of them here would hide from Dart a distinction qyro_session
// already decided was worth keeping.

/// The address, port or path was not usable.
pub const QYRO_ERR_BAD_ARGUMENT: i32 = -6;
/// The peer could not be reached, or the wire ended.
pub const QYRO_ERR_PEER_UNREACHABLE: i32 = -7;
/// The peer did not authenticate. Never retry this one blindly.
pub const QYRO_ERR_NOT_AUTHENTICATED: i32 = -8;
/// The transfer was refused.
pub const QYRO_ERR_TRANSFER_REFUSED: i32 = -9;
/// The destination refused the content.
pub const QYRO_ERR_STORAGE_REFUSED: i32 = -10;
/// The session was cancelled by this end.
pub const QYRO_ERR_CANCELLED: i32 = -11;

/// A `SessionError` variant this build does not know how to translate.
///
/// `SessionError` is `#[non_exhaustive]`, so the translation must have a
/// catch-all arm or it will not compile from this crate. A catch-all that
/// collapsed an unknown variant onto an existing code would tell Dart something
/// false; this one says "I do not know" instead, and the guard test in
/// `session_abi` reads both source files so a new variant is caught before
/// anybody can construct it.
pub const QYRO_ERR_UNKNOWN: i32 = -12;

/// No usable device identity for this process. ADR-0040.
///
/// Distinct from [`QYRO_ERR_NOT_AUTHENTICATED`] on purpose: that one means the
/// **peer** did not prove who it is, and this one means **this device** does not
/// know who it is. Collapsing them would tell a person to distrust the other end
/// when the problem is at home.
pub const QYRO_ERR_IDENTITY_UNREADABLE: i32 = -13;

/// More files than one transfer can carry (ADR-0047 §3).
///
/// **Its own code and not `BAD_ARGUMENT`**, because «too many» is a number the
/// person can act on — pick fewer, or send in two goes — and a generic argument
/// error is exactly the message that cost this project QYR-0361: a refusal about
/// the call printed as if the network were at fault.
pub const QYRO_ERR_TOO_MANY_FILES: i32 = -14;

impl HandleError {
    /// The code this error crosses the boundary as.
    #[must_use]
    pub const fn code(self) -> i32 {
        match self {
            Self::NotLive => QYRO_ERR_INVALID_HANDLE,
            Self::Full => QYRO_ERR_TABLE_FULL,
        }
    }
}

/// What a guarded body of this type hands back when it panics.
///
/// A trait rather than a `Default` bound, because `Default` for `i32` is `0`, and
/// `0` is [`QYRO_OK`]: a panic would report success. Every return shape that
/// crosses the boundary has to name its own failure value, out loud, once.
pub trait PanicOutcome {
    const ON_PANIC: Self;
}

impl PanicOutcome for i32 {
    const ON_PANIC: Self = QYRO_ERR_PANIC;
}

impl PanicOutcome for *mut u8 {
    /// ADR-0038: a null buffer is how allocation failure already travels, so a
    /// panic during allocation is indistinguishable from running out of memory,
    /// which is exactly what the caller has to handle either way.
    const ON_PANIC: Self = core::ptr::null_mut();
}

impl PanicOutcome for () {
    const ON_PANIC: Self = ();
}

/// Runs `body`, converting a panic into that return type's [`PanicOutcome`].
///
/// `AssertUnwindSafe` is sound here for the reason the name asks for: the bodies
/// this wraps today touch nothing that outlives the unwind. The session
/// operations do touch the handle table, and the assertion stops being free at
/// that point -- ADR-0032 §5 answers it by poisoning the session a panic passed
/// through, which is the invariant written alongside those operations rather
/// than assumed here.
///
/// Generic rather than one function per return shape, because the structural
/// guard in `guards.rs` requires every `extern "C"` body to open with `guard(`
/// literally, and a family of `guard_pointer` / `guard_unit` siblings would have
/// meant weakening it with exceptions.
pub fn guard<T: PanicOutcome, F: FnOnce() -> T>(body: F) -> T {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(value) => value,
        Err(_) => T::ON_PANIC,
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "a test that cannot fail loudly is not a test"
    )]
    use super::{QYRO_ERR_INVALID_HANDLE, QYRO_ERR_PANIC, QYRO_ERR_TABLE_FULL, QYRO_OK, guard};
    use crate::handle::HandleError;

    /// An `extern "C"` function that panics, so the test exercises the real
    /// boundary rather than a Rust function that resembles one.
    ///
    /// Not compiled into the shipped library: it exists only under `cfg(test)`.
    #[cfg(test)]
    extern "C" fn qyro_test_panicking_boundary() -> i32 {
        guard(|| {
            #[expect(
                clippy::panic,
                reason = "the deliberate panic this whole boundary exists to contain"
            )]
            {
                panic!("a deliberate panic, provoked to prove it does not escape");
            }
        })
    }

    #[test]
    fn a_panic_inside_the_c_boundary_becomes_an_error_code() {
        // Silence the panic hook: this test provokes a panic on purpose and its
        // backtrace on stderr reads like a failure to anyone watching CI.
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let code = qyro_test_panicking_boundary();
        std::panic::set_hook(previous);

        assert_eq!(
            code, QYRO_ERR_PANIC,
            "a panic must leave as a code, never as an unwind across the frontier"
        );
    }

    #[test]
    fn the_guard_is_transparent_when_nothing_panics() {
        // Without this, a `guard` that returned QYRO_ERR_PANIC unconditionally
        // would pass the test above.
        assert_eq!(guard(|| QYRO_OK), QYRO_OK);
        assert_eq!(guard(|| -77), -77);
    }

    #[test]
    fn every_code_is_distinct_and_only_success_is_zero() {
        let codes = [
            QYRO_OK,
            QYRO_ERR_INVALID_HANDLE,
            QYRO_ERR_TABLE_FULL,
            QYRO_ERR_PANIC,
            super::QYRO_ERR_NULL_OUT,
            super::QYRO_ERR_POISONED,
            super::QYRO_ERR_BAD_ARGUMENT,
            super::QYRO_ERR_PEER_UNREACHABLE,
            super::QYRO_ERR_NOT_AUTHENTICATED,
            super::QYRO_ERR_TRANSFER_REFUSED,
            super::QYRO_ERR_STORAGE_REFUSED,
            super::QYRO_ERR_CANCELLED,
            super::QYRO_ERR_UNKNOWN,
        ];
        for (index, code) in codes.iter().enumerate() {
            assert!(
                index == 0 || *code < 0,
                "every error code is negative: {code} is not"
            );
            assert_eq!(
                codes.iter().filter(|other| *other == code).count(),
                1,
                "code {code} is used twice"
            );
        }
    }

    #[test]
    fn each_handle_error_crosses_as_its_own_code() {
        assert_eq!(HandleError::NotLive.code(), QYRO_ERR_INVALID_HANDLE);
        assert_eq!(HandleError::Full.code(), QYRO_ERR_TABLE_FULL);
    }
}
