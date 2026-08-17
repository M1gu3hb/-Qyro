//! The shared structural minimum, for the crate that is a binary.
//!
//! See `rust/guards/source_guard.rs`. The meta-guard in `qyro_identity_store`
//! demanded this within a minute of the crate existing, which is the guard
//! doing its job: a new crate with no analysis is a new place for a panic to
//! live unnoticed.
//!
//! # Why a binary needs this as much as a library
//!
//! More, actually. A panic in `qyro_ffi` becomes a return code at the C
//! frontier; a panic here **is the whole program dying in front of somebody
//! who was told to type a command**, on a machine where they cannot install a
//! debugger to find out why.

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
const PRODUCTION_FILES: [&str; 3] = ["main.rs", "flows.rs", "term.rs"];

#[test]
fn no_production_path_can_panic() {
    assert_no_production_path_can_panic(&PRODUCTION_FILES);
}

#[test]
fn every_production_file_is_listed() {
    assert_the_production_list_matches_the_source(&PRODUCTION_FILES);
}

#[test]
fn the_analysis_reaches_the_end_of_every_production_file() {
    for file in PRODUCTION_FILES {
        assert_analysis_reached_the_end(file, &production_source(file));
    }
}

#[test]
fn no_assertion_compares_a_call_to_itself() {
    assert_no_assertion_compares_a_call_to_itself();
}

#[test]
fn no_rust_source_carries_a_raw_nul_byte() {
    for file in PRODUCTION_FILES {
        let path = format!("{}/src/{file}", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
        assert!(
            !bytes.contains(&0),
            "{file} carries a raw NUL byte, which makes grep treat it as binary \
             and skip it entirely (QYR-0327). Write the escape instead."
        );
    }
}
