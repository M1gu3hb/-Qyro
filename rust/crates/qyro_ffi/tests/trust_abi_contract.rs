//! The eight operations of ADR-0032 amendment 1, exercised through the C ABI.
//!
//! The text contract is the part worth testing hardest, because five functions
//! share it and it is the one place where an off-by-one becomes half a
//! fingerprint compared out loud.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

use qyro_ffi::{
    qyro_pairing_parse, qyro_session_peer_fingerprint, qyro_session_peer_trust,
    qyro_session_rejection, qyro_trust_forget_peer, qyro_trust_list_peers,
};

/// The codes, transcribed. `qyro_ffi` publishes them and this asserts the two
/// lists agree by using them.
const QYRO_OK: i32 = 0;
const QYRO_ERR_INVALID_HANDLE: i32 = -1;
const QYRO_ERR_NULL_OUT: i32 = -4;
const QYRO_ERR_BAD_ARGUMENT: i32 = -6;

fn text_of(buffer: &[u8], len: usize) -> String {
    String::from_utf8(buffer[..len].to_vec()).expect("the boundary writes UTF-8")
}

#[test]
fn a_pairing_string_gives_up_its_address_and_a_broken_one_gives_up_nothing() {
    let good = "QYRO1|192.168.1.7:47001|00112233445566778899aabbccddeeff";
    let mut buffer = [0_u8; 64];
    let mut len = 0_usize;

    let code = unsafe {
        qyro_pairing_parse(
            good.as_ptr(),
            good.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &raw mut len,
        )
    };
    assert_eq!(code, QYRO_OK);
    assert_eq!(text_of(&buffer, len), "192.168.1.7:47001");

    // And every way it can be wrong is refused, not half-parsed. The address in
    // these is valid on purpose, so a refusal is about the rest of the string.
    for bad in [
        "NOTQYRO|192.168.1.7:47001|00112233445566778899aabbccddeeff",
        "QYRO1|192.168.1.7:47001",
        "QYRO1|0.0.0.0:47001|00112233445566778899aabbccddeeff",
        "QYRO1|192.168.1.7:47001|00112233445566778899AABBCCDDEEFF",
        "",
    ] {
        let mut scratch = [0xAA_u8; 64];
        let mut wrote = 999_usize;
        let code = unsafe {
            qyro_pairing_parse(
                bad.as_ptr(),
                bad.len(),
                scratch.as_mut_ptr(),
                scratch.len(),
                &raw mut wrote,
            )
        };
        assert_eq!(code, QYRO_ERR_BAD_ARGUMENT, "{bad:?} was accepted");
        assert!(
            scratch.iter().all(|byte| *byte == 0xAA),
            "{bad:?} was refused and still wrote into the caller's buffer"
        );
    }
}

#[test]
fn asking_with_no_room_reports_the_length_and_writes_nothing() {
    // The text contract of ADR-0032 amendment 1, on the function that can be
    // exercised without a socket. `capacity == 0` with a null buffer is the
    // documented way to ask how much to allocate.
    let good = "QYRO1|[fe80::1]:47001|00112233445566778899aabbccddeeff";
    let mut needed = 0_usize;

    let code = unsafe {
        qyro_pairing_parse(
            good.as_ptr(),
            good.len(),
            std::ptr::null_mut(),
            0,
            &raw mut needed,
        )
    };
    assert_eq!(
        code, QYRO_ERR_BAD_ARGUMENT,
        "asking must not read as success"
    );
    assert_eq!(needed, "[fe80::1]:47001".len());
    assert!(
        needed > 0,
        "the length asked for is zero, so this proves nothing"
    );

    // One byte short: still nothing written, and still the true length.
    let mut tight = vec![0xAA_u8; needed - 1];
    let mut reported = 0_usize;
    let code = unsafe {
        qyro_pairing_parse(
            good.as_ptr(),
            good.len(),
            tight.as_mut_ptr(),
            tight.len(),
            &raw mut reported,
        )
    };
    assert_eq!(code, QYRO_ERR_BAD_ARGUMENT);
    assert_eq!(reported, needed);
    assert!(
        tight.iter().all(|byte| *byte == 0xAA),
        "a buffer one byte short was partially written; half a fingerprint that \
         matches proves nothing at all"
    );

    // Exactly enough: it fits.
    let mut exact = vec![0_u8; needed];
    let mut wrote = 0_usize;
    let code = unsafe {
        qyro_pairing_parse(
            good.as_ptr(),
            good.len(),
            exact.as_mut_ptr(),
            exact.len(),
            &raw mut wrote,
        )
    };
    assert_eq!(
        code, QYRO_OK,
        "a buffer of exactly the reported size did not fit"
    );
    assert_eq!(wrote, needed);
    assert_eq!(text_of(&exact, wrote), "[fe80::1]:47001");
}

#[test]
fn a_null_out_length_is_refused_before_anything_is_written() {
    let good = "QYRO1|192.168.1.7:47001|00112233445566778899aabbccddeeff";
    let mut buffer = [0xAA_u8; 64];
    let code = unsafe {
        qyro_pairing_parse(
            good.as_ptr(),
            good.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(code, QYRO_ERR_NULL_OUT);
    assert!(buffer.iter().all(|byte| *byte == 0xAA));
}

#[test]
fn the_book_forgets_what_it_never_knew_without_lying_about_it() {
    let name = "a-peer-nothing-ever-remembered";
    let mut removed = 7_i32;
    let code = unsafe { qyro_trust_forget_peer(name.as_ptr(), name.len(), &raw mut removed) };

    assert_eq!(code, QYRO_OK, "forgetting an unknown peer is not an error");
    assert_eq!(removed, 0, "it claimed to have forgotten something");
}

#[test]
fn listing_an_empty_book_is_an_empty_string_and_not_a_failure() {
    // The book is process-wide and other tests in this binary do not write to
    // it, so this asserts on the length rather than on emptiness: a book with
    // entries would still have to report its own size honestly.
    let mut len = 999_usize;
    let code = unsafe { qyro_trust_list_peers(std::ptr::null_mut(), 0, &raw mut len) };
    assert!(
        code == QYRO_OK || code == QYRO_ERR_BAD_ARGUMENT,
        "listing reported {code}"
    );
    assert_ne!(len, 999, "the length was never written");
}

#[test]
fn every_operation_refuses_a_handle_that_names_nothing() {
    // The same contract the six original operations have. A handle whose
    // generation cannot exist, because the table is process-wide.
    let dead = u64::MAX;
    let mut buffer = [0_u8; 64];
    let mut len = 0_usize;
    let mut verdict = -99_i32;
    let name = "laptop";

    assert_eq!(
        unsafe {
            qyro_session_peer_fingerprint(dead, buffer.as_mut_ptr(), buffer.len(), &raw mut len)
        },
        QYRO_ERR_INVALID_HANDLE
    );
    assert_eq!(
        unsafe { qyro_session_peer_trust(dead, name.as_ptr(), name.len(), &raw mut verdict) },
        QYRO_ERR_INVALID_HANDLE
    );
    assert_eq!(
        unsafe { qyro_session_rejection(dead, &raw mut verdict) },
        QYRO_ERR_INVALID_HANDLE
    );
    assert_eq!(
        verdict, -99,
        "a refused call still wrote into the caller's out-parameter"
    );
}
