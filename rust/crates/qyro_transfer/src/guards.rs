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

/// `ItemVerdict` variants that nothing constructs, with the argument.
///
/// `Incomplete` is **unreachable, and provably so**. A verdict is only computed
/// when `Complete` arrives, and `Complete` is refused unless every item has
/// `received >= size`. To reach `Incomplete` an item would need
/// `received >= size` *and* fewer contiguous chunks than `expected_chunks`. With
/// `k` contiguous chunks the receiver has taken at most `k * CHUNK_SIZE` bytes,
/// and `k < ceil(size / CHUNK_SIZE)` bounds that below `size`. The two
/// conditions cannot hold together.
///
/// It is kept rather than deleted because `IntegrityResult` verdict byte `3` is
/// frozen in ADR-0026 §1, and removing a value from a frozen wire format is a
/// format change this sprint has no mandate to make. Registered as QYR-0071 so
/// the decision is owed, not forgotten.
const VERDICTS_WITH_NO_CONSTRUCTION_SITE: [&str; 1] = ["Incomplete"];

#[test]
fn every_transfer_error_has_a_construction_site() {
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "error.rs",
        "TransferError",
        10,
        &[],
    );
}

#[test]
fn every_item_verdict_has_a_construction_site_or_an_argument() {
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "error.rs",
        "ItemVerdict",
        4,
        &VERDICTS_WITH_NO_CONSTRUCTION_SITE,
    );
}
