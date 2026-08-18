//! A structural guard over every production file in this crate.
//!
//! `qyro_session` is the crate that stands between the C boundary and the
//! cryptographic stack (ADR-0032 §2). A panic here is a panic one frame below a
//! `catch_unwind` that has not been written yet, on input a peer controls.
//!
//! See `rust/guards/source_guard.rs`.

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
const PRODUCTION_FILES: [&str; 11] = [
    "scanner.rs",
    "scan_state.rs",
    "advisor.rs",
    "lib.rs",
    "bridged_wrapper.rs",
    "discovery.rs",
    "error.rs",
    "link.rs",
    // ADR-0040. Both guards demanded it the moment the file existed: the
    // production-list guard because it compiles into a release, and the
    // construction-site guard because `SessionError::IdentityUnreadable` is
    // produced here and nowhere else, so without this line the variant looked
    // like one nothing can reach.
    "identity.rs",
    "session.rs",
    "trust.rs",
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
fn every_session_error_has_a_construction_site() {
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "error.rs",
        "SessionError",
        7,
        &[],
    );
}

/// The analysis reaches the last line of every production file, and says how far.
#[test]
fn the_analysis_reaches_the_end_of_every_production_file() {
    for file in PRODUCTION_FILES {
        let analysed = production_source(file);
        let raw = production_source_raw(file);
        assert_analysis_reached_the_end(file, &analysed);
        println!(
            "qyro_session/src/{file}: {} bytes analysed of {} raw",
            analysed.len(),
            raw.len()
        );
        assert!(!analysed.is_empty(), "src/{file} stripped to nothing");
    }
}
