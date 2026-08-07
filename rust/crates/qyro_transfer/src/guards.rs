//! A structural guard over every production file in this crate.
//!
//! `qyro_transfer` drives peer bytes through a state machine. A panic on that
//! path is a remote denial of service, and it is also an abort in the middle of
//! handling content that is about to be zeroized.
//!
//! The analysis is shared with the other crates; the list below is this one's.
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
const PRODUCTION_FILES: [&str; 4] = ["lib.rs", "error.rs", "session.rs", "wire.rs"];

#[test]
fn no_production_path_can_panic() {
    assert_no_production_path_can_panic(&PRODUCTION_FILES);
}

#[test]
fn every_production_file_is_listed() {
    assert_the_production_list_matches_the_source(&PRODUCTION_FILES);
}
