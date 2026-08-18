//! A structural guard over every production file in this crate.
//!
//! This crate opens paths a peer named. A panic there is a remote denial of
//! service, and it happens with a file handle open.

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
    "error.rs",
    "history.rs",
    "history_types.rs",
    "io.rs",
    "manifest_builder.rs",
    "resume.rs",
    "safe_path.rs",
];

#[test]
fn no_production_path_can_panic() {
    assert_no_production_path_can_panic(&PRODUCTION_FILES);
}

const HISTORY_ERRORS_CONSTRUCTED_ONLY_IN_THEIR_OWN_FILE: [&str; 1] = ["Io"];

#[test]
fn every_history_error_has_a_construction_site() {
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "history_types.rs",
        "HistoryError",
        9,
        &HISTORY_ERRORS_CONSTRUCTED_ONLY_IN_THEIR_OWN_FILE,
    );
}

#[test]
fn every_production_file_is_listed() {
    assert_the_production_list_matches_the_source(&PRODUCTION_FILES);
}

/// `FsError` variants whose only construction site is in the declaring file.
///
/// `Io` is built by the `From<std::io::Error>` conversion that lives beside the
/// enum. The guard deliberately does not read the declaring file, because the
/// `Display` match arms there name every variant and would count as
/// construction — which would make the check pass for every enum, always.
///
/// So this is a genuine blind spot of the analysis and not a hole in the code:
/// `FsError::Io` is reachable from every `?` in this crate. Listed with the
/// argument rather than left silent, which is what the exemption list is for.
const ERRORS_CONSTRUCTED_ONLY_IN_THEIR_OWN_FILE: [&str; 1] = ["Io"];

#[test]
fn every_fs_error_has_a_construction_site() {
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "error.rs",
        "FsError",
        8,
        &ERRORS_CONSTRUCTED_ONLY_IN_THEIR_OWN_FILE,
    );
}
