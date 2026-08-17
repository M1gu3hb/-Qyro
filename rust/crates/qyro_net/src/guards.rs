//! A structural guard over every production file in this crate.
//!
//! `qyro_net` reads bytes a stranger controls, before that stranger has proved
//! anything. A panic on that path is a remote denial of service that needs no
//! authentication to trigger: an unauthenticated peer reaches
//! `FrameStream::read_frame` on its first packet.
//!
//! The analysis is shared with the other crates; the list below is this one's.
//! See `rust/guards/source_guard.rs`.
//!
//! # Why the byte counts matter here
//!
//! QYR-0071: this same analysis once read 13 401 of a file's 30 861 bytes and
//! reported success, because an item shape it could not parse made it swallow
//! the rest. Every assertion built on it was measuring less than half of what it
//! claimed. `assert_analysis_reached_the_end` is the check that closes that, and
//! `the_analysis_reaches_the_end_of_every_production_file` below runs it over
//! each file in this crate and reports the sizes, so a future regression shows
//! up as a number that moved rather than as a silent pass.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the shared analysis serves several crates, reads files, and must \
              fail loudly when it cannot"
)]

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../guards/source_guard.rs"
));

/// Every file compiled into a release build of this crate.
const PRODUCTION_FILES: [&str; 8] = [
    "lib.rs",
    "discovery.rs",
    "error.rs",
    "handshake.rs",
    "pairing.rs",
    "limits.rs",
    "listener.rs",
    "stream.rs",
];

#[test]
fn no_production_path_can_panic() {
    assert_no_production_path_can_panic(&PRODUCTION_FILES);
}

#[test]
fn every_production_file_is_listed() {
    assert_the_production_list_matches_the_source(&PRODUCTION_FILES);
}

#[test]
fn every_net_error_has_a_construction_site() {
    // Fourteen variants at the time of writing. The minimum is the guard's
    // defence against the enum being silently truncated: if a future edit leaves
    // fewer, the guard says the parse stopped early rather than that the enum
    // shrank.
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "error.rs",
        "NetError",
        12,
        &[],
    );
}

/// Every refusal a pairing string can produce is produced somewhere.
///
/// All seven are exempt from the *cross-file* requirement and the exemption is
/// the point of writing it down: `PairingError` is declared and constructed in
/// the same file because the only thing that can refuse a pairing string is the
/// parser of pairing strings. Demanding a construction site elsewhere would be
/// satisfiable only by scattering the parse.
///
/// What the call still buys is the parse floor: if this enum ever stops being
/// found, it reports zero variants and fails rather than passing silently. The
/// reachability of all seven is held by
/// `tests/pairing_contract.rs::every_way_a_pairing_string_can_be_wrong_is_its_own_refusal`
/// and its two neighbours, which construct each one by parsing a string that
/// deserves it.
#[test]
fn every_pairing_error_has_a_construction_site() {
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "pairing.rs",
        "PairingError",
        7,
        &[
            "NotAPairingString",
            "WrongFieldCount",
            "UnreadableAddress",
            "UnspecifiedAddress",
            "ZeroPort",
            "FingerprintWrongLength",
            "FingerprintNotLowercaseHex",
        ],
    );
}

#[test]
fn every_socket_op_has_a_construction_site() {
    // `SocketOp` is not an `Error` by name, so the workspace meta-guard does not
    // demand this one. It is here anyway: an operation label nothing ever
    // constructs is a label that will be wrong the first time someone reads it
    // in a bug report.
    assert_every_variant_has_a_construction_site(&PRODUCTION_FILES, "error.rs", "SocketOp", 8, &[]);
}

/// The analysis reaches the last line of every production file, and says how far.
///
/// The sizes are printed rather than merely asserted because the failure this
/// guards against is quiet: an analysis that stops early still returns
/// well-formed source and still passes every `contains` check built on it.
#[test]
fn the_analysis_reaches_the_end_of_every_production_file() {
    for file in PRODUCTION_FILES {
        let analysed = production_source(file);
        let raw = production_source_raw(file);
        assert_analysis_reached_the_end(file, &analysed);
        println!(
            "qyro_net/src/{file}: {} bytes analysed of {} raw",
            analysed.len(),
            raw.len()
        );
        assert!(
            !analysed.is_empty(),
            "src/{file} stripped to nothing, so nothing was analysed"
        );
    }
}
