//! The structural guards every crate in this workspace carries.

#![allow(
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
const PRODUCTION_FILES: [&str; 5] = ["lib.rs", "error.rs", "lt.rs", "rng.rs", "wire.rs"];

#[test]
fn no_production_path_can_panic() {
    assert_no_production_path_can_panic(&PRODUCTION_FILES);
}

#[test]
fn every_production_file_is_listed() {
    assert_the_production_list_matches_the_source(&PRODUCTION_FILES);
}

/// Every `WireError` must be something the code can actually produce.
///
/// The point of this guard across the workspace: a variant nobody constructs is
/// a failure mode somebody *described* rather than handled, and it reads in a
/// match arm as if it were covered. Here it matters more than usual — every one
/// of these five is a thing a camera really hands over in a room with a screen
/// in it, and a decoder that could not produce one of them would be silently
/// guessing instead of refusing.
#[test]
fn every_wire_error_has_a_construction_site() {
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "error.rs",
        "WireError",
        5,
        &[],
    );
}
