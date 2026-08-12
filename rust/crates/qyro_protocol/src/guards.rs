//! A structural guard over every production file in this crate.
//!
//! `qyro_protocol` is the first thing a peer's bytes reach: the decoder is
//! driven entirely by lengths the peer declared. A panic anywhere on that path
//! is a remote denial of service, and until sprint 4C.3 this crate had neither
//! the Clippy denials `qyro_crypto` carries nor a guard to notice a module that
//! was missing them (QYR-0036).
//!
//! The analysis is shared with the other two crates; the list below is this
//! crate's own. See `rust/guards/source_guard.rs`.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the shared analysis serves three crates, reads files, and must \
              fail loudly when it cannot"
)]

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../guards/source_guard.rs"
));

/// Every file compiled into a release build of this crate.
const PRODUCTION_FILES: [&str; 10] = [
    "lib.rs",
    "decoder.rs",
    "envelope.rs",
    "error.rs",
    "frame.rs",
    "header.rs",
    "limits.rs",
    "message.rs",
    "session.rs",
    "version.rs",
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
fn every_frame_error_has_a_construction_site() {
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "error.rs",
        "FrameError",
        15,
        &[],
    );
}
