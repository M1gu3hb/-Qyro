//! A structural guard over the harness binary.
//!
//! `qyro_net_smoke` is never shipped, but it is the thing that decides whether
//! Phase 4's claim is true, so a panic in it is a test result nobody can read.
//! It also parses argv and peer-driven state, which is the same shape of input
//! the product handles.
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

/// Every file compiled into this binary.
///
/// One file. The harness is deliberately a single `main.rs`: splitting it would
/// make it look like a library, and it is not one — nothing may ever depend on
/// it.
const PRODUCTION_FILES: [&str; 1] = ["main.rs"];

#[test]
fn no_production_path_can_panic() {
    assert_no_production_path_can_panic(&PRODUCTION_FILES);
}

#[test]
fn every_production_file_is_listed() {
    assert_the_production_list_matches_the_source(&PRODUCTION_FILES);
}

/// The analysis reaches the last line of `main.rs`, and says how far.
#[test]
fn the_analysis_reaches_the_end_of_every_production_file() {
    for file in PRODUCTION_FILES {
        let analysed = production_source(file);
        let raw = production_source_raw(file);
        assert_analysis_reached_the_end(file, &analysed);
        println!(
            "qyro_net_smoke/src/{file}: {} bytes analysed of {} raw",
            analysed.len(),
            raw.len()
        );
        assert!(
            !analysed.is_empty(),
            "src/{file} stripped to nothing, so nothing was analysed"
        );
    }
}
