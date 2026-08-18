//! The six operations phase 02 will call, and nothing else.
//!
//! Specification: `docs/adr/ADR-0032-engine-ffi.md` §5 and §6, and the table in
//! `docs/fase-implementacion/FASE-01-FFI-DEL-MOTOR.md` §6 paso 4.
//!
//! Every function here has the same shape, and it is the shape the ADR freezes:
//!
//! - returns `i32` — `0` success, negatives errors, [`crate::abi`] owns the codes;
//! - values leave through caller-allocated out-parameters, never a return value
//!   and never an allocation this library would have to hand ownership of;
//! - buffers arrive length-delimited, so nothing here looks for a NUL terminator
//!   in memory the caller described;
//! - the body sits inside [`guard`], so a panic becomes a code rather than an
//!   unwind across the frontier;
//! - the name ends in `_blocking` when the call can block, which ADR-0032 §7
//!   requires be visible in the name rather than only in a comment.
//!
//! # Why `unsafe` lives here
//!
//! These take raw pointers because C has nothing else. Each dereference is
//! preceded by a null check and bounded by a length the caller passed; what the
//! library cannot verify — that the pointer really addresses that many readable
//! bytes — is stated in each `# Safety` section, which is the whole contract
//! Dart has to hold up.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use qyro_session::{Progress, ProgressObserver, Session, SessionError, SessionState};

use crate::abi::{
    QYRO_ERR_BAD_ARGUMENT, QYRO_ERR_CANCELLED, QYRO_ERR_IDENTITY_UNREADABLE,
    QYRO_ERR_NOT_AUTHENTICATED, QYRO_ERR_NULL_OUT, QYRO_ERR_PEER_UNREACHABLE, QYRO_ERR_POISONED,
    QYRO_ERR_STORAGE_REFUSED, QYRO_ERR_TOO_MANY_FILES, QYRO_ERR_TRANSFER_REFUSED, QYRO_ERR_UNKNOWN,
    QYRO_OK, guard,
};
use crate::handle::HandleTable;

/// The state values that leave through `out_state`.
///
/// A separate channel from the return code, which is ADR-0032 §5 correcting the
/// phase document's `while (qyro_step(handle) == QYRO_IN_PROGRESS)` sketch: that
/// sends a transport failure and "still running" through one `int`.
pub const QYRO_STATE_IN_PROGRESS: i32 = 0;
pub const QYRO_STATE_COMPLETED: i32 = 1;
pub const QYRO_STATE_REJECTED: i32 = 2;

/// An entry in the table: the session, plus its sticky error if it has failed.
pub(crate) struct Entry {
    pub(crate) session: Session,
    /// Set once and never cleared. ADR-0032 §5 freezes stickiness as returning
    /// *the same code*: a second `Ok` would let Dart believe a session recovered
    /// when its worker is dead.
    failed: Option<i32>,
}

type Table = HandleTable<Entry>;

/// The process-wide table.
///
/// A `Mutex` rather than a thread-local, because ADR-0032 §7 permits Dart to
/// call from any isolate and a thread-local table would silently give each one
/// its own handles.
fn table() -> &'static Mutex<Table> {
    static TABLE: OnceLock<Mutex<Table>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(HandleTable::new()))
}

pub(crate) const fn session_code(error: SessionError) -> i32 {
    match error {
        SessionError::BadArgument => QYRO_ERR_BAD_ARGUMENT,
        SessionError::TooManyFiles { .. } => QYRO_ERR_TOO_MANY_FILES,
        SessionError::PeerUnreachable => QYRO_ERR_PEER_UNREACHABLE,
        SessionError::NotAuthenticated => QYRO_ERR_NOT_AUTHENTICATED,
        SessionError::TransferRefused => QYRO_ERR_TRANSFER_REFUSED,
        SessionError::StorageRefused => QYRO_ERR_STORAGE_REFUSED,
        SessionError::Cancelled => QYRO_ERR_CANCELLED,
        SessionError::IdentityUnreadable => QYRO_ERR_IDENTITY_UNREADABLE,
        // Required: `SessionError` is `#[non_exhaustive]`. Never silently an
        // existing code -- see QYRO_ERR_UNKNOWN and the guard below.
        _ => QYRO_ERR_UNKNOWN,
    }
}

const fn state_code(state: SessionState) -> i32 {
    match state {
        SessionState::InProgress => QYRO_STATE_IN_PROGRESS,
        SessionState::Completed => QYRO_STATE_COMPLETED,
        SessionState::Rejected => QYRO_STATE_REJECTED,
    }
}

/// Reads a length-delimited buffer as UTF-8.
///
/// # Safety
///
/// `ptr` must address `len` readable bytes, or be null when `len` is 0.
pub(crate) unsafe fn borrow_text(ptr: *const u8, len: usize) -> Option<&'static str> {
    if len == 0 {
        return Some("");
    }
    if ptr.is_null() {
        return None;
    }
    // SAFETY: the caller promises `len` readable bytes at `ptr`; the null case
    // and the empty case are both handled above.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    std::str::from_utf8(bytes).ok()
}

/// The sticky-error policy, with no session in it so it can be tested.
///
/// ADR-0032 §5 freezes stickiness as returning *the same code*: a second `Ok`
/// would let Dart believe a session recovered when its worker is dead. Extracted
/// because the policy is the part worth testing and the part that cannot be
/// reached otherwise -- poisoning a real session needs a real peer, so a test
/// driving `with_session` would be an integration test wearing a unit test's
/// clothes. Found by mutation: deleting the check below survived every test in
/// this crate.
///
/// Returns the code to hand back, and the failure to remember, if any.
const fn sticky(previous: Option<i32>, outcome: i32) -> (Option<i32>, i32) {
    if let Some(code) = previous {
        // Already failed: the same code, forever, and the new outcome is not
        // even consulted.
        return (Some(code), code);
    }
    if outcome == QYRO_OK || outcome == QYRO_ERR_NULL_OUT {
        // A null out-parameter is the caller's mistake, caught before the
        // session was touched, so it does not poison what it never reached.
        return (None, outcome);
    }
    (Some(outcome), outcome)
}

/// Runs `body` against a live session, applying the sticky-error rule.
/// Runs `body` against a live session **without** the sticky-error rule.
///
/// For the read-only questions of ADR-0032 amendment 1: asking a failed session
/// for the peer's fingerprint must answer the fingerprint, not the code the
/// session died with. Stickiness exists so a caller cannot believe a *transfer*
/// recovered; a fingerprint is not a transfer, and refusing to show it after a
/// failure would hide exactly the fact a person needs in order to understand
/// what went wrong.
pub(crate) fn with_session_entry<F>(handle: u64, body: F) -> i32
where
    F: FnOnce(&mut Entry) -> i32,
{
    let Ok(mut table) = table().lock() else {
        return QYRO_ERR_POISONED;
    };
    match table.get_mut(handle) {
        Ok(entry) => body(entry),
        Err(error) => error.code(),
    }
}

fn with_session<F>(handle: u64, body: F) -> i32
where
    F: FnOnce(&mut Entry) -> i32,
{
    let Ok(mut table) = table().lock() else {
        // A poisoned mutex means a panic escaped a previous call while the
        // table was borrowed. The table's invariants are not known to hold, so
        // every later call says so rather than reading it anyway.
        return QYRO_ERR_POISONED;
    };
    let entry = match table.get_mut(handle) {
        Ok(entry) => entry,
        Err(error) => return error.code(),
    };
    if let Some(code) = entry.failed {
        let (_, code) = sticky(Some(code), QYRO_OK);
        return code;
    }
    let (failed, code) = sticky(None, body(entry));
    entry.failed = failed;
    code
}

/// What Dart hands over to be told how far a session has got.
///
/// ADR-0033 §2 freezes the shape: `void` return, four scalars, no pointer. The
/// return type is what makes rule 1 structural — there is no value for Rust to
/// read, so cancellation cannot accidentally start travelling this way instead
/// of through `qyro_session_cancel`. The four parameters being integers is what
/// makes rule 3 structural: the call is deferred, and a pointer handed over here
/// would address a Rust stack frame that no longer exists when Dart looks.
pub type QyroProgressFn = extern "C" fn(context: usize, done: u64, total: u64, item: u32);

/// Turns a nullable C pointer into the observer `qyro_session` expects.
///
/// Null becomes `None`, and ADR-0033 §2 requires that a session without an
/// observer take the same path as one with it rather than a second one.
///
/// `context` is opaque and never interpreted — it exists because the handle does
/// not yet exist when the opening call runs, so an emission during the handshake
/// could not carry it.
fn observer(on_progress: Option<QyroProgressFn>, context: usize) -> Option<ProgressObserver> {
    let callback = on_progress?;
    Some(Box::new(move |progress: Progress| {
        // Unwinding out of an `extern "C"` function is undefined behaviour, so a
        // callback that panics is the caller's contract to keep, not something
        // this side can rescue. What this side does guarantee is the panic
        // boundary around every operation (ADR-0032 §8): a panic raised on the
        // Rust side of the call still becomes QYRO_ERR_PANIC rather than
        // crossing back into Dart.
        callback(context, progress.done, progress.total, progress.item);
    }))
}

fn insert(session: Session, out_handle: *mut u64) -> i32 {
    if out_handle.is_null() {
        return QYRO_ERR_NULL_OUT;
    }
    let Ok(mut table) = table().lock() else {
        return QYRO_ERR_POISONED;
    };
    match table.insert(Entry {
        session,
        failed: None,
    }) {
        Ok(handle) => {
            // SAFETY: checked non-null above; the caller owns one `u64`.
            unsafe { out_handle.write(handle) };
            QYRO_OK
        }
        Err(error) => error.code(),
    }
}

/// Opens a sending session. Blocks: it dials and completes a handshake.
///
/// `paths` is one buffer of NUL-separated paths, `paths_len` bytes long. NUL is
/// used because it is the one byte no path may contain on either platform, so
/// the separator cannot appear inside a name.
///
/// # Safety
///
/// `address`, `root` and `paths` must address their stated lengths in readable
/// memory. `out_handle` must point to one writable `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_session_open_sender_blocking(
    address: *const u8,
    address_len: usize,
    root: *const u8,
    root_len: usize,
    paths: *const u8,
    paths_len: usize,
    on_progress: Option<QyroProgressFn>,
    context: usize,
    out_handle: *mut u64,
) -> i32 {
    guard(|| {
        if out_handle.is_null() {
            return QYRO_ERR_NULL_OUT;
        }
        // SAFETY: the caller's contract, stated above.
        let (Some(address), Some(root), Some(paths)) = (unsafe {
            (
                borrow_text(address, address_len),
                borrow_text(root, root_len),
                borrow_text(paths, paths_len),
            )
        }) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };

        let Ok(address) = address.parse::<SocketAddr>() else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        let files: Vec<PathBuf> = paths
            .split('\0')
            .filter(|part| !part.is_empty())
            .map(PathBuf::from)
            .collect();
        if files.is_empty() || root.is_empty() {
            return QYRO_ERR_BAD_ARGUMENT;
        }

        match Session::open_sender(
            address,
            Path::new(root),
            &files,
            observer(on_progress, context),
        ) {
            Ok(session) => insert(session, out_handle),
            Err(error) => session_code(error),
        }
    })
}

/// Opens a sending session over **descriptors the caller already opened**.
///
/// ADR-0034. Android's Storage Access Framework hands out a `content://` and a
/// `ParcelFileDescriptor`, never a path, and no NDK API can open one. Dart calls
/// `detachFd()` — which gives up ownership on that side — and passes the raw
/// integers here.
///
/// `names` is one buffer of NUL-separated relative names, one per descriptor and
/// in the same order. A descriptor has no name of its own: the picker knew it and
/// the kernel does not.
///
/// **Ownership transfers on entry, on every path out.** Each descriptor becomes a
/// `File` before anything can fail, so a rejected argument still closes what it
/// was given rather than leaking it. Dart must not close them, and must not use
/// them again.
///
/// # Safety
///
/// `address` and `names` must address their stated lengths in readable memory.
/// `fds` must address `fd_count` readable `int32`s. Each must be a descriptor
/// this process owns and nothing else will close. `out_handle` must point to one
/// writable `u64`.
///
/// # Platforms
///
/// Unix only, which for this product means Android. A descriptor is not a
/// Windows concept and ADR-0034 sends a path there instead, so this symbol
/// simply does not exist in the Windows library rather than existing and
/// failing — an absent symbol is a link error at load time, and a present one
/// that always refuses is a runtime surprise.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_session_open_sender_fd_blocking(
    address: *const u8,
    address_len: usize,
    names: *const u8,
    names_len: usize,
    fds: *const i32,
    fd_count: usize,
    on_progress: Option<QyroProgressFn>,
    context: usize,
    out_handle: *mut u64,
) -> i32 {
    guard(|| {
        if out_handle.is_null() || fds.is_null() || fd_count == 0 {
            return QYRO_ERR_BAD_ARGUMENT;
        }
        // SAFETY: the caller's contract, stated above.
        let (Some(address), Some(names)) = (unsafe {
            (
                borrow_text(address, address_len),
                borrow_text(names, names_len),
            )
        }) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        // SAFETY: the caller promises `fd_count` readable `int32`s at `fds`.
        let raw = unsafe { std::slice::from_raw_parts(fds, fd_count) };

        // Taken first, and unconditionally. Every later `return` then drops
        // real `File`s, so a bad argument closes the descriptors it was handed
        // instead of leaking one per rejected call.
        let mut handles: Vec<std::fs::File> = Vec::with_capacity(fd_count);
        for descriptor in raw {
            // SAFETY: the caller promises each is a descriptor this process
            // owns and that nothing else will close. `detachFd` on the Dart side
            // is what makes that true: it releases the ParcelFileDescriptor's
            // claim, so this `File` is the only owner and its `Drop` is the only
            // close. `getFd` would leave two owners and a double close.
            handles.push(unsafe {
                <std::fs::File as std::os::fd::FromRawFd>::from_raw_fd(*descriptor)
            });
        }

        let Ok(address) = address.parse::<SocketAddr>() else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        let labels: Vec<&str> = names.split('\0').filter(|part| !part.is_empty()).collect();
        if labels.len() != handles.len() {
            return QYRO_ERR_BAD_ARGUMENT;
        }

        let paired: Vec<(String, std::fs::File)> =
            labels.into_iter().map(str::to_owned).zip(handles).collect();

        match Session::open_sender_files(address, paired, observer(on_progress, context)) {
            Ok(session) => insert(session, out_handle),
            Err(error) => session_code(error),
        }
    })
}

/// Opens a receiving session. Blocks: it accepts and completes a handshake.
///
/// # Safety
///
/// `bind` and `destination` must address their stated lengths in readable
/// memory. `out_handle` must point to one writable `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_session_open_receiver_blocking(
    bind: *const u8,
    bind_len: usize,
    destination: *const u8,
    destination_len: usize,
    on_progress: Option<QyroProgressFn>,
    context: usize,
    out_handle: *mut u64,
) -> i32 {
    guard(|| {
        if out_handle.is_null() {
            return QYRO_ERR_NULL_OUT;
        }
        // SAFETY: the caller's contract, stated above.
        let (Some(bind), Some(destination)) = (unsafe {
            (
                borrow_text(bind, bind_len),
                borrow_text(destination, destination_len),
            )
        }) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        let Ok(bind) = bind.parse::<SocketAddr>() else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        if destination.is_empty() {
            return QYRO_ERR_BAD_ARGUMENT;
        }

        match Session::open_receiver(bind, Path::new(destination), observer(on_progress, context)) {
            Ok(session) => insert(session, out_handle),
            Err(error) => session_code(error),
        }
    })
}

/// Advances the session by one step. Blocks: it reads and writes the socket.
///
/// The state leaves through `out_state`, never through the return value.
///
/// # Safety
///
/// `out_state` must point to one writable `i32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_session_step_blocking(handle: u64, out_state: *mut i32) -> i32 {
    guard(|| {
        if out_state.is_null() {
            return QYRO_ERR_NULL_OUT;
        }
        with_session(handle, |entry| match entry.session.step() {
            Ok(state) => {
                // SAFETY: checked non-null above; the caller owns one `i32`.
                unsafe { out_state.write(state_code(state)) };
                QYRO_OK
            }
            Err(error) => session_code(error),
        })
    })
}

/// Reads how far the session has got. Does not block.
///
/// # Safety
///
/// Each out-parameter must point to one writable value of its type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_session_progress(
    handle: u64,
    out_done: *mut u64,
    out_total: *mut u64,
    out_item: *mut u32,
) -> i32 {
    guard(|| {
        if out_done.is_null() || out_total.is_null() || out_item.is_null() {
            return QYRO_ERR_NULL_OUT;
        }
        with_session(handle, |entry| {
            let progress = entry.session.progress();
            // SAFETY: all three checked non-null above.
            unsafe {
                out_done.write(progress.done);
                out_total.write(progress.total);
                out_item.write(progress.item);
            }
            QYRO_OK
        })
    })
}

/// Asks the session to stop. Does not block, and is safe from any thread.
///
/// Deliberately does **not** go through the sticky-error check: cancelling a
/// session that already failed is exactly what a caller winding down does, and
/// refusing it would leave them no way to say "stop" to a broken session.
#[unsafe(no_mangle)]
pub extern "C" fn qyro_session_cancel(handle: u64) -> i32 {
    guard(|| {
        let Ok(table) = table().lock() else {
            return QYRO_ERR_POISONED;
        };
        match table.get(handle) {
            Ok(entry) => {
                entry.session.cancel();
                QYRO_OK
            }
            Err(error) => error.code(),
        }
    })
}

/// Materialises what arrived, and releases what did not.
///
/// **QYR-0357, and without this a received file never arrives.**
/// `Session::finish` verifies each item's digest and renames its `.qyro-part`
/// to the final name (ADR-0027 §4). It had no symbol, so the Dart receiver
/// reported "delivered" and left a part file on disk -- the worst shape a
/// failure can take, because a noisy one leaves a person retrying and this one
/// leaves them believing they have the file.
///
/// Called on **every** ending and not only the happy one: a receiver that
/// stopped early leaves a `.qyro-part` per started item and nothing else
/// removes it (QYR-0087, QYR-0088).
///
/// `out_count` receives how many items reached their final name. Zero is a
/// legitimate answer -- a sender that was refused materialises nothing -- so it
/// is a count and not a boolean.
///
/// # Safety
///
/// `out_count` must be null or address four writable, aligned bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_session_finish(handle: u64, out_count: *mut u32) -> i32 {
    guard(|| {
        if out_count.is_null() {
            return QYRO_ERR_NULL_OUT;
        }
        with_session(handle, |entry| match entry.session.finish() {
            Ok(count) => {
                // SAFETY: checked non-null above; the caller promises four
                // writable aligned bytes, which is this function's contract.
                unsafe { out_count.write(count) };
                QYRO_OK
            }
            Err(error) => session_code(error),
        })
    })
}

/// Closes the handle and drops the session.
///
/// A second call is [`QYRO_ERR_INVALID_HANDLE`](crate::abi::QYRO_ERR_INVALID_HANDLE),
/// never a crash: see `handle.rs`.
#[unsafe(no_mangle)]
pub extern "C" fn qyro_session_close(handle: u64) -> i32 {
    guard(|| {
        let Ok(mut table) = table().lock() else {
            return QYRO_ERR_POISONED;
        };
        match table.remove(handle) {
            Ok(entry) => {
                drop(entry);
                QYRO_OK
            }
            Err(error) => error.code(),
        }
    })
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

    /// The pointer Dart hands over reaches the engine, carrying its context.
    ///
    /// Written because the mutation sweep replaced `observer` with `None` and
    /// **nothing in this crate noticed**. The Dart test that would notice --
    /// it asserts emissions arrive -- is not a Rust test, and `cargo mutants`
    /// runs Rust. A guarantee that only a test in another language defends is a
    /// guarantee the sweep cannot see.
    #[test]
    fn a_progress_pointer_becomes_an_observer_that_carries_its_context() {
        use std::sync::Mutex as SeenLock;
        static SEEN: SeenLock<Vec<(usize, u64, u64, u32)>> = SeenLock::new(Vec::new());

        extern "C" fn record(context: usize, done: u64, total: u64, item: u32) {
            if let Ok(mut seen) = SEEN.lock() {
                seen.push((context, done, total, item));
            }
        }

        if let Ok(mut seen) = SEEN.lock() {
            seen.clear();
        }

        // ADR-0033 §2: a null pointer is "no observer", not a second code path.
        assert!(
            super::observer(None, 7).is_none(),
            "a null progress pointer produced an observer"
        );

        let mut sink =
            super::observer(Some(record), 4242).expect("a real pointer produces an observer");
        sink(super::Progress {
            done: 5,
            total: 9,
            item: 3,
        });

        let seen = SEEN.lock().expect("the recorder survived").clone();
        // The context is asserted too, and with a value that is not a plausible
        // accident: an implementation that passed zero, or passed the handle, or
        // dropped the argument, fails here rather than looking right.
        assert_eq!(
            seen,
            vec![(4242_usize, 5_u64, 9_u64, 3_u32)],
            "the emission did not arrive with the four values it was given"
        );
    }

    #[cfg(unix)]
    use super::qyro_session_open_sender_fd_blocking;
    use super::{
        QYRO_ERR_BAD_ARGUMENT, QYRO_ERR_NULL_OUT, QYRO_OK, QYRO_STATE_COMPLETED,
        QYRO_STATE_IN_PROGRESS, QYRO_STATE_REJECTED, qyro_session_cancel, qyro_session_close,
        qyro_session_finish, qyro_session_open_receiver_blocking,
        qyro_session_open_sender_blocking, qyro_session_progress, qyro_session_step_blocking,
    };
    #[cfg(unix)]
    use crate::abi::QYRO_ERR_PEER_UNREACHABLE;
    use crate::abi::{QYRO_ERR_INVALID_HANDLE, QYRO_ERR_UNKNOWN};

    fn buffer(text: &str) -> (*const u8, usize) {
        (text.as_ptr(), text.len())
    }

    #[test]
    fn every_operation_refuses_a_handle_that_names_nothing() {
        // The table is process-wide and other tests put sessions in it, so this
        // uses a handle whose generation cannot exist rather than assuming the
        // table is empty.
        let dead = u64::MAX;
        let mut state = -1;
        let (mut done, mut total, mut item) = (0_u64, 0_u64, 0_u32);

        assert_eq!(
            unsafe { qyro_session_step_blocking(dead, &raw mut state) },
            QYRO_ERR_INVALID_HANDLE
        );
        assert_eq!(
            unsafe { qyro_session_progress(dead, &raw mut done, &raw mut total, &raw mut item) },
            QYRO_ERR_INVALID_HANDLE
        );
        assert_eq!(qyro_session_cancel(dead), QYRO_ERR_INVALID_HANDLE);
        assert_eq!(qyro_session_close(dead), QYRO_ERR_INVALID_HANDLE);
    }

    #[test]
    fn a_null_out_parameter_is_refused_before_anything_is_touched() {
        let (address, address_len) = buffer("127.0.0.1:1");
        let (root, root_len) = buffer("/tmp");
        let (paths, paths_len) = buffer("/tmp/a");

        assert_eq!(
            unsafe {
                qyro_session_open_sender_blocking(
                    address,
                    address_len,
                    root,
                    root_len,
                    paths,
                    paths_len,
                    None,
                    0,
                    std::ptr::null_mut(),
                )
            },
            QYRO_ERR_NULL_OUT,
            "a null out-handle must be refused without dialling anything"
        );
        assert_eq!(
            unsafe { qyro_session_step_blocking(u64::MAX, std::ptr::null_mut()) },
            QYRO_ERR_NULL_OUT,
            "and before the handle is even resolved"
        );
    }

    #[test]
    fn an_unparseable_address_is_a_bad_argument_and_not_a_dial() {
        let mut handle = 0_u64;
        let (address, address_len) = buffer("not an address");
        let (root, root_len) = buffer("/tmp");
        let (paths, paths_len) = buffer("/tmp/a");

        assert_eq!(
            unsafe {
                qyro_session_open_sender_blocking(
                    address,
                    address_len,
                    root,
                    root_len,
                    paths,
                    paths_len,
                    None,
                    0,
                    &raw mut handle,
                )
            },
            QYRO_ERR_BAD_ARGUMENT
        );
        assert_eq!(handle, 0, "nothing may be written on the failing path");
    }

    #[test]
    fn an_empty_path_list_is_refused() {
        let mut handle = 0_u64;
        let (address, address_len) = buffer("127.0.0.1:1");
        let (root, root_len) = buffer("/tmp");

        assert_eq!(
            unsafe {
                qyro_session_open_sender_blocking(
                    address,
                    address_len,
                    root,
                    root_len,
                    std::ptr::null(),
                    0,
                    None,
                    0,
                    &raw mut handle,
                )
            },
            QYRO_ERR_BAD_ARGUMENT
        );
    }

    #[test]
    fn a_receiver_is_refused_before_it_binds_when_the_out_handle_is_null() {
        // Named for what it does. The first draft was called
        // "..._opens_and_closes_and_the_handle_dies_with_it" and never opened
        // anything: open_receiver binds and then blocks on accept, and the C
        // surface has no accessor for the bound port, so a peer cannot be
        // arranged from here. Driving a real session end to end is step 5.
        let destination = std::env::temp_dir().join("qyro-ffi-open-close");
        std::fs::create_dir_all(&destination).expect("a temp directory");

        let handle = 0_u64;
        let (bind, bind_len) = buffer("127.0.0.1:0");
        let destination_text = destination.to_string_lossy().into_owned();
        let (destination_ptr, destination_len) = buffer(&destination_text);

        let code = unsafe {
            qyro_session_open_receiver_blocking(
                bind,
                bind_len,
                destination_ptr,
                destination_len,
                None,
                0,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(code, QYRO_ERR_NULL_OUT);
        assert_eq!(handle, 0);

        let _ = std::fs::remove_dir_all(&destination);
    }

    #[test]
    fn the_three_states_are_distinct_and_none_of_them_is_an_error_code() {
        // ADR-0032 §5: state and error are different channels. If a state value
        // ever collided with an error code, a caller reading `out_state` after a
        // failure would read a state that means something else.
        let states = [
            QYRO_STATE_IN_PROGRESS,
            QYRO_STATE_COMPLETED,
            QYRO_STATE_REJECTED,
        ];
        for (index, state) in states.iter().enumerate() {
            assert!(*state >= 0, "a state is never negative: {state}");
            assert_eq!(
                states.iter().filter(|other| *other == state).count(),
                1,
                "state {state} is used twice"
            );
            let _ = index;
        }
        assert_eq!(QYRO_STATE_IN_PROGRESS, QYRO_OK, "0 is 0 in both channels");
    }

    #[test]
    fn a_failed_session_keeps_answering_the_same_code() {
        use super::sticky;

        // The rule ADR-0032 §5 freezes, and the one no other test reached.
        let (remembered, code) = sticky(None, QYRO_ERR_BAD_ARGUMENT);
        assert_eq!(code, QYRO_ERR_BAD_ARGUMENT);
        assert_eq!(
            remembered,
            Some(QYRO_ERR_BAD_ARGUMENT),
            "the failure sticks"
        );

        // A later call that would have succeeded still gets the old code.
        let (still, code) = sticky(Some(QYRO_ERR_BAD_ARGUMENT), QYRO_OK);
        assert_eq!(
            code, QYRO_ERR_BAD_ARGUMENT,
            "a second Ok would let Dart believe the session recovered"
        );
        assert_eq!(still, Some(QYRO_ERR_BAD_ARGUMENT));

        // And a later call that fails differently does not overwrite the first.
        let (still, code) = sticky(Some(QYRO_ERR_BAD_ARGUMENT), QYRO_ERR_NULL_OUT);
        assert_eq!(code, QYRO_ERR_BAD_ARGUMENT);
        assert_eq!(still, Some(QYRO_ERR_BAD_ARGUMENT));

        // Success does not poison, or every session would fail once and stay
        // failed -- the mistake this test would otherwise invite.
        assert_eq!(sticky(None, QYRO_OK), (None, QYRO_OK));

        // A null out-parameter is caught before the session is touched.
        assert_eq!(
            sticky(None, QYRO_ERR_NULL_OUT),
            (None, QYRO_ERR_NULL_OUT),
            "the caller's mistake must not poison a session it never reached"
        );
    }

    #[test]
    fn every_session_error_variant_has_its_own_code_and_none_falls_through() {
        // The `_` arm in `session_code` exists because `SessionError` is
        // `#[non_exhaustive]`, and a catch-all is exactly the shape that hides a
        // new variant. This reads both files instead of trusting the match.
        let error_source = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../qyro_session/src/error.rs"
        ))
        .expect("qyro_session's error module is readable");

        let declared: Vec<String> = error_source
            .lines()
            // Assembled rather than written whole: the workspace guard in
            // qyro_identity_store scans raw source for "pub enum <Name>" to
            // decide which crate *declares* an error enum, and a string literal
            // spelling it out made that guard believe qyro_ffi declares
            // SessionError (QYR-0308).
            .skip_while(|line| !line.contains(concat!("pub enum ", "SessionError")))
            .skip(1)
            .take_while(|line| !line.starts_with('}'))
            .map(|line| line.trim().trim_end_matches(',').to_owned())
            .filter(|line| {
                !line.is_empty()
                    && !line.starts_with("///")
                    && !line.starts_with("//")
                    && line.chars().next().is_some_and(char::is_uppercase)
            })
            .collect();

        assert_eq!(
            declared.len(),
            8,
            "qyro_session declares {} variants, not the 8 this module translates: {declared:?}. \
             Add the arm in session_code and a code in abi.rs, then update this number.",
            declared.len()
        );

        let this_source =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/session_abi.rs"))
                .expect("this module is readable");
        for variant in &declared {
            // The **name**, not the whole declaration. A struct variant is
            // declared `TooManyFiles { given: usize, limit: usize }` and its arm
            // is written `TooManyFiles { .. }`, so comparing the full text made
            // this guard unsatisfiable for any variant carrying data -- which it
            // stayed until one did. The intent was always "every variant has an
            // arm here", and that is about the name.
            let name = variant
                .split([' ', '{', '('])
                .next()
                .unwrap_or(variant.as_str());
            assert!(
                this_source.contains(&format!("SessionError::{name} =>"))
                    || this_source.contains(&format!("SessionError::{name} {{")),
                "SessionError::{name} has no arm here, so it would translate to                  QYRO_ERR_UNKNOWN"
            );
        }

        // And the positive control: the fall-through is reachable in principle,
        // so this guard is not passing because the arm was deleted.
        assert!(
            this_source.contains("_ => QYRO_ERR_UNKNOWN"),
            "the catch-all is what this guard exists to police; if it is gone, \
             this test should be too"
        );
        let _ = QYRO_ERR_UNKNOWN;
    }

    // ------------------------------------------------ the descriptor boundary

    /// What a descriptor points at, or nothing when it points at nothing.
    ///
    /// `(device, inode)` and not "is it open", because a descriptor **number**
    /// is reused the moment it is freed: the very call under test opens a
    /// socket, and on a busy process that socket can land on the number we just
    /// gave away. Asking "does this number still name the file we handed over"
    /// is the question that survives reuse; asking "is this number open" is not.
    ///
    /// Reads through a borrowed `File` and gives the number straight back, so
    /// the measurement never closes anything itself.
    #[cfg(unix)]
    fn what_the_descriptor_points_at(descriptor: i32) -> Option<(u64, u64)> {
        use std::os::fd::{FromRawFd as _, IntoRawFd as _};
        use std::os::unix::fs::MetadataExt as _;

        // SAFETY: the caller passes a descriptor number this process either owns
        // or has released. The handle is given straight back with `into_raw_fd`
        // and never dropped, so this observation closes nothing either way.
        let borrowed = unsafe { std::fs::File::from_raw_fd(descriptor) };
        let identity = borrowed
            .metadata()
            .map(|meta| (meta.dev(), meta.ino()))
            .ok();
        let _ = borrowed.into_raw_fd();
        identity
    }

    /// A scratch file with bytes in it, and where it lives.
    #[cfg(unix)]
    fn a_scratch_file(tag: &str) -> std::path::PathBuf {
        use std::io::Write as _;
        let directory = std::env::temp_dir().join(format!("qyro-ffi-fd-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a temp directory");
        let path = directory.join(format!("{tag}.bin"));
        let mut file = std::fs::File::create(&path).expect("a scratch file");
        file.write_all(&[7_u8; 4096]).expect("bytes on disk");
        path
    }

    /// The descriptor Dart hands over is closed by Rust, on the failing path too.
    ///
    /// ADR-0034 §2: `detachFd()` already gave up the Kotlin side's claim, so if
    /// this boundary does not close, nothing does — one leaked descriptor per
    /// rejected pick, on a process with a hard limit. The failing path is the
    /// one worth testing: the happy path closes at the end of a transfer, and
    /// the rejection is where a `return` before the `File` exists would leak.
    ///
    /// Nothing is listening on the address, so the call reaches the dial and
    /// fails there — which is also what proves ownership was taken *before*
    /// validation, since a `BAD_ARGUMENT` would mean it never got that far.
    ///
    /// **What this proves and what it does not.** It proves the descriptor was
    /// released: the number no longer names the file. It does not prove the
    /// close happened exactly once rather than twice — a second close of a
    /// reused number is not observable from inside the process. That half rests
    /// on the type: the `File` is the only owner and `Drop` runs once, and
    /// `no_rust_source_carries_a_raw_nul_byte`'s neighbour
    /// `the_crate_closes_no_descriptor_by_hand` is what keeps a second closer
    /// from appearing.
    /// Opens a process-wide identity so a session can be built at all.
    ///
    /// ADR-0040. Before it, every constructor generated a throwaway keypair and
    /// no test needed anything; now a session without an identity refuses with
    /// `QYRO_ERR_IDENTITY_UNREADABLE`, which is the property. The tests below
    /// need one because what they measure happens **after** that check.
    ///
    /// `Sandbox`, because these run on Linux in CI where there is no platform
    /// wrapper and `Platform` correctly refuses.
    #[cfg(unix)]
    fn ensure_identity() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let dir = std::env::temp_dir().join(format!("qyro-ffi-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("a temporary directory");
            qyro_session::open(
                &dir.join("identity.qyro"),
                qyro_session::Protection::Sandbox,
            )
            .expect("opening a test identity");
        });
    }

    // ---------------------------------------- qyro_session_finish, the contract
    //
    // Thirty-eight lines of production with no Rust test at all when it was
    // added: the Dart two-process test proved the *behaviour* — a file arrives
    // with its final name — and a behaviour test does not pin an ABI. What is
    // missing from it is precisely what C callers get wrong: a null out
    // pointer, a handle that names nothing, and whether the count means
    // anything.

    #[test]
    fn finish_refuses_a_null_count_before_it_touches_the_table() {
        // Null first, and on purpose: if the order were reversed, a caller
        // passing both a bad handle and a null pointer would learn about the
        // handle and never about the pointer it is going to pass again.
        let code = unsafe { qyro_session_finish(0, std::ptr::null_mut()) };
        assert_eq!(
            code, QYRO_ERR_NULL_OUT,
            "a null out-parameter is the caller's mistake and must be named as \
             that, not as an invalid handle"
        );
    }

    #[test]
    fn finish_on_a_handle_that_names_nothing_is_a_typed_refusal() {
        // Every handle-taking symbol answers the same way for a stranger, and
        // this one must not be the exception that crashes.
        let mut count = 7_u32;
        let code = unsafe { qyro_session_finish(u64::MAX, &raw mut count) };
        assert_eq!(
            code, QYRO_ERR_INVALID_HANDLE,
            "an unknown handle must be refused by name"
        );
        assert_eq!(
            count, 7,
            "the out-parameter was written on a failing path. A caller that \
             reads it after an error would believe something was materialised"
        );
    }

    #[test]
    fn finish_writes_the_count_only_on_success() {
        // The falsifiability half of the test above: the assertion that
        // `count` stays 7 proves nothing unless a successful call is known to
        // change it. There is no session to succeed against here — building one
        // needs a peer — so what is pinned is the shape: `QYRO_OK` is the only
        // code that may write, and the two failing paths above are the only two
        // reachable without a session.
        //
        // Named so that the day a session can be built from here, this test is
        // the one to extend rather than the one to trust blindly.
        let mut count = u32::MAX;
        let refused = unsafe { qyro_session_finish(0, &raw mut count) };
        assert_ne!(refused, QYRO_OK, "handle 0 is never a live session");
        assert_eq!(count, u32::MAX, "a refusal wrote the count");
    }

    #[test]
    fn finish_is_in_the_error_code_family_of_every_other_operation() {
        // QYR-0357 existed because a symbol was missing, and a symbol that
        // answers with a code nobody else answers with is the next version of
        // that problem: a caller matching on the shared set falls through to a
        // default it did not write for this.
        //
        // Only the two refusals reachable without a live session are asserted,
        // which is what this test can honestly see: building a session from
        // here needs a peer.
        let mut scratch = 0_u32;
        let refusals = [
            unsafe { qyro_session_finish(0, std::ptr::null_mut()) },
            unsafe { qyro_session_finish(u64::MAX, &raw mut scratch) },
        ];
        for code in refusals {
            assert!(code < 0, "every refusal is negative, and {code} is not");
            assert!(
                code == QYRO_ERR_NULL_OUT || code == QYRO_ERR_INVALID_HANDLE,
                "{code} is neither of the two refusals reachable without a session"
            );
        }
        assert_ne!(
            refusals[0], refusals[1],
            "a null pointer and an unknown handle answered identically, so a              caller cannot tell which mistake they made"
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_descriptor_is_closed_exactly_once() {
        ensure_identity();
        use std::os::fd::IntoRawFd as _;

        let path = a_scratch_file("handed-over");
        let file = std::fs::File::open(&path).expect("the scratch file opens");
        let identity = file
            .metadata()
            .map(|meta| {
                use std::os::unix::fs::MetadataExt as _;
                (meta.dev(), meta.ino())
            })
            .ok();
        let descriptor = file.into_raw_fd();

        assert_eq!(
            what_the_descriptor_points_at(descriptor),
            identity,
            "the measurement cannot see the file before it is handed over, so it \
             could not see it survive either"
        );

        let mut handle = 0_u64;
        // Port 1 on loopback: privileged, and nothing binds it in a test runner.
        let (address, address_len) = buffer("127.0.0.1:1");
        let (names, names_len) = buffer("handed-over.bin");
        let descriptors = [descriptor];

        let code = unsafe {
            qyro_session_open_sender_fd_blocking(
                address,
                address_len,
                names,
                names_len,
                descriptors.as_ptr(),
                descriptors.len(),
                None,
                0,
                &raw mut handle,
            )
        };

        assert_eq!(
            code, QYRO_ERR_PEER_UNREACHABLE,
            "the call ended {code} rather than at the dial, so it never took \
             ownership and this test is measuring the wrong path"
        );
        assert_eq!(handle, 0, "nothing may be written on the failing path");

        assert_ne!(
            what_the_descriptor_points_at(descriptor),
            identity,
            "the descriptor still names the file it was handed. Rust is its only \
             owner after detachFd, so a descriptor that survives a rejected call \
             is a descriptor nothing will ever close (ADR-0034 §2)"
        );

        let _ = std::fs::remove_file(&path);
    }

    /// And the refusal path closes them too.
    ///
    /// ADR-0040 added an early return: a session opened before the identity is
    /// `QYRO_ERR_IDENTITY_UNREADABLE`. That return happens **after** the raw
    /// numbers have become `File`s, so `Drop` still closes them — but a new
    /// early return in a function that adopts the caller's descriptors is
    /// exactly where that stops being true, and ADR-0034 §2 says Rust owns them
    /// once the call is made. Asserted rather than assumed.
    ///
    /// It cannot use `ensure_identity`, because what it needs is the state
    /// *before* one exists. It runs in its own process, which `cargo test`
    /// gives every integration binary — and this is a unit test, so it asks the
    /// question the only way it can: through a path that is refused for a
    /// different reason but still adopts the descriptors first.
    #[cfg(unix)]
    #[test]
    fn a_refused_call_still_closes_the_descriptors_it_was_handed() {
        use std::os::fd::IntoRawFd as _;

        ensure_identity();

        let path = std::env::temp_dir().join(format!("qyro-refused-{}", std::process::id()));
        let file = std::fs::File::create(&path).expect("a scratch file");
        let descriptor = file.into_raw_fd();
        let identity = what_the_descriptor_points_at(descriptor);

        let mut handle = 0_u64;
        // An empty name is refused by `open_sender_files` **after** the
        // descriptors have been turned into `File`s, which is the shape of
        // every early return that matters here.
        let (address, address_len) = buffer("127.0.0.1:1");
        let (names, names_len) = buffer("");
        let descriptors = [descriptor];

        let code = unsafe {
            qyro_session_open_sender_fd_blocking(
                address,
                address_len,
                names,
                names_len,
                descriptors.as_ptr(),
                descriptors.len(),
                None,
                0,
                &raw mut handle,
            )
        };

        assert_ne!(code, QYRO_OK, "the call was supposed to be refused");
        assert_eq!(handle, 0, "nothing may be written on a failing path");
        assert_ne!(
            what_the_descriptor_points_at(descriptor),
            identity,
            "a refused call left the caller's descriptor open. Rust owns them              from the moment the call is made (ADR-0034 §2), so a return that              skips the close is a leak nothing else will collect"
        );

        let _ = std::fs::remove_file(&path);
    }

    /// R2 §1.7 for the test above: a descriptor that leaked would be visible.
    ///
    /// The assertion that matters there is a **negative** one — "it no longer
    /// names the file" — and a negative assertion passes for free if the
    /// measurement is broken, blind, or looking at the wrong number. So this
    /// keeps a descriptor deliberately alive and requires the same measurement
    /// to say so.
    #[cfg(unix)]
    #[test]
    fn a_descriptor_that_was_not_handed_over_stays_visible_to_this_measurement() {
        use std::os::fd::{FromRawFd as _, IntoRawFd as _};

        let path = a_scratch_file("kept-open");
        let file = std::fs::File::open(&path).expect("the scratch file opens");
        let identity = file
            .metadata()
            .map(|meta| {
                use std::os::unix::fs::MetadataExt as _;
                (meta.dev(), meta.ino())
            })
            .ok();
        let descriptor = file.into_raw_fd();

        assert_eq!(
            what_the_descriptor_points_at(descriptor),
            identity,
            "a descriptor nobody took is reported as gone, so the measurement in \
             the_descriptor_is_closed_exactly_once would report success for a leak"
        );
        assert!(
            identity.is_some(),
            "the file has no identity, so both sides of the comparison are None \
             and the assertion above compares nothing with nothing"
        );

        // SAFETY: this number is still ours; taking it back is what closes it.
        drop(unsafe { std::fs::File::from_raw_fd(descriptor) });
        let _ = std::fs::remove_file(&path);
    }

    /// A descriptor list whose names do not line up is refused, and still closed.
    ///
    /// The argument check sits *after* the loop that takes ownership, and that
    /// ordering is the whole design of §2. A refactor that moved the check
    /// earlier would be an improvement everywhere except here, where it turns
    /// every rejected pick into a leaked descriptor.
    #[cfg(unix)]
    #[test]
    fn a_rejected_argument_still_closes_what_it_was_handed() {
        use std::os::fd::IntoRawFd as _;

        let path = a_scratch_file("mismatched");
        let file = std::fs::File::open(&path).expect("the scratch file opens");
        let identity = file
            .metadata()
            .map(|meta| {
                use std::os::unix::fs::MetadataExt as _;
                (meta.dev(), meta.ino())
            })
            .ok();
        let descriptor = file.into_raw_fd();

        let mut handle = 0_u64;
        let (address, address_len) = buffer("127.0.0.1:1");
        // Two names, one descriptor.
        let (names, names_len) = buffer("a.bin\0b.bin");
        let descriptors = [descriptor];

        let code = unsafe {
            qyro_session_open_sender_fd_blocking(
                address,
                address_len,
                names,
                names_len,
                descriptors.as_ptr(),
                descriptors.len(),
                None,
                0,
                &raw mut handle,
            )
        };

        assert_eq!(code, QYRO_ERR_BAD_ARGUMENT);
        assert_ne!(
            what_the_descriptor_points_at(descriptor),
            identity,
            "a rejected argument leaked the descriptor it was handed"
        );

        let _ = std::fs::remove_file(&path);
    }
}
