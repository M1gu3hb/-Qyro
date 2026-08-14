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

use crate::abi::guard;

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

/// Lends the caller `len` writable, zeroed bytes. ADR-0038.
///
/// Rust owns this memory from here until [`qyro_buffer_free`] takes it back.
/// Dart cannot allocate native memory without `package:ffi`, and this phase
/// admits no new pub.dev package, so the buffers Dart fills and passes in are
/// borrowed from here rather than owned there.
///
/// Returns null when `len` is zero, and when the allocation fails. A caller that
/// checks for null handles both without having to tell them apart.
#[unsafe(no_mangle)]
pub extern "C" fn qyro_buffer_alloc(len: usize) -> *mut u8 {
    guard(|| {
        if len == 0 {
            return core::ptr::null_mut();
        }
        let mut buffer: Vec<u8> = Vec::new();
        // `try_reserve_exact` and not `vec![0; len]`: a length that arrived from
        // outside must not be able to abort this process by being large.
        if buffer.try_reserve_exact(len).is_err() {
            return core::ptr::null_mut();
        }
        buffer.resize(len, 0);
        // Capacity is exactly `len` after the reserve above, so this cannot
        // reallocate, and the box carries the length the free side needs.
        let boxed: Box<[u8]> = buffer.into_boxed_slice();
        Box::into_raw(boxed).cast::<u8>()
    })
}

/// Takes back what [`qyro_buffer_alloc`] handed out. ADR-0038.
///
/// A null pointer is a no-op, so the zero-length case needs no branch on the
/// caller's side.
///
/// # Safety
///
/// `ptr` must be a pointer [`qyro_buffer_alloc`] returned and has not already
/// freed, and `len` must be **the same length it was allocated with**. That is
/// the one obligation this boundary cannot check, and the reason the Dart side
/// keeps the length beside the pointer instead of asking for it again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_buffer_free(ptr: *mut u8, len: usize) {
    guard(|| {
        if ptr.is_null() || len == 0 {
            return;
        }
        let slice = core::ptr::slice_from_raw_parts_mut(ptr, len);
        // SAFETY: the caller's contract, stated above: `ptr` came from
        // `qyro_buffer_alloc` with this exact `len`, which is the layout
        // `Box<[u8]>` was created with.
        drop(unsafe { Box::from_raw(slice) });
    });
}

#[cfg(test)]
mod buffer_tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "a test that cannot fail loudly is not a test"
    )]

    use super::{qyro_buffer_alloc, qyro_buffer_free};

    #[test]
    fn a_borrowed_buffer_is_writable_and_zeroed_and_gives_back_what_was_written() {
        const LEN: usize = 4096;
        let pointer = qyro_buffer_alloc(LEN);
        assert!(!pointer.is_null(), "a 4 KiB request came back null");

        // SAFETY: `pointer` addresses LEN writable bytes, per the contract above.
        let slice = unsafe { std::slice::from_raw_parts_mut(pointer, LEN) };
        assert!(
            slice.iter().all(|byte| *byte == 0),
            "the buffer arrived with something in it, so a partly-written path \
             would send whatever was there before"
        );

        for (index, slot) in slice.iter_mut().enumerate() {
            *slot = u8::try_from(index % 251).unwrap();
        }
        // Read back through a fresh borrow, so the assertion is not comparing a
        // slice to itself.
        // SAFETY: same pointer, same length, still owned by this test.
        let reread = unsafe { std::slice::from_raw_parts(pointer.cast_const(), LEN) };
        assert_eq!(reread[0], 0);
        assert_eq!(reread[250], 250);
        assert_eq!(reread[251], 0);
        assert_eq!(reread[LEN - 1], u8::try_from((LEN - 1) % 251).unwrap());

        // SAFETY: allocated here with this exact length, freed once.
        unsafe { qyro_buffer_free(pointer, LEN) };
    }

    #[test]
    fn a_zero_length_request_is_null_and_freeing_null_does_nothing() {
        // The empty case is not a special case on the caller's side: it comes
        // back null, and null frees without effect, so Dart needs no branch.
        let pointer = qyro_buffer_alloc(0);
        assert!(pointer.is_null(), "a zero-length request returned memory");

        // SAFETY: freeing null is defined as a no-op by the contract.
        unsafe { qyro_buffer_free(pointer, 0) };
        unsafe { qyro_buffer_free(core::ptr::null_mut(), 4096) };
    }

    #[test]
    fn two_buffers_do_not_share_memory() {
        // Without this, an allocator that returned one static scratch buffer
        // would satisfy every other assertion in this module.
        const LEN: usize = 64;
        let first = qyro_buffer_alloc(LEN);
        let second = qyro_buffer_alloc(LEN);
        assert!(!first.is_null() && !second.is_null());
        assert_ne!(first, second, "two allocations returned the same address");

        // SAFETY: two distinct live allocations of LEN bytes each.
        unsafe {
            std::slice::from_raw_parts_mut(first, LEN).fill(0xAA);
            std::slice::from_raw_parts_mut(second, LEN).fill(0x55);
            assert_eq!(
                std::slice::from_raw_parts(first.cast_const(), LEN)[0],
                0xAA,
                "writing the second buffer changed the first"
            );
            qyro_buffer_free(first, LEN);
            qyro_buffer_free(second, LEN);
        }
    }

    #[test]
    fn a_length_no_machine_can_satisfy_is_refused_rather_than_aborting() {
        // `vec![0; len]` would abort the process here. `try_reserve_exact`
        // reports it, and a length arriving from outside must never be able to
        // take the process down.
        let pointer = qyro_buffer_alloc(usize::MAX / 2);
        assert!(
            pointer.is_null(),
            "an impossible allocation succeeded, so this test is not exercising \
             the failure path it names"
        );
    }
}
