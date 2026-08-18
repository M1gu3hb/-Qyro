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
const PRODUCTION_FILES: [&str; 5] = ["main.rs", "flows.rs", "optical.rs", "serial.rs", "term.rs"];

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

/// The QR decoder must never become a normal dependency.
///
/// **This guard is the other half of an entry in `.cargo/audit.toml`.** Two
/// unsoundness advisories against `lru` 0.12.5 are ignored there, and the whole
/// argument for ignoring them is that `lru` arrives through `rqrr`, which is a
/// **dev**-dependency and reaches no artefact Qyro distributes.
///
/// An argument in a comment is a promise nobody checks. If somebody ever moves
/// `rqrr` into `[dependencies]` — to add a scanner, say — the audit exception
/// becomes false and silently stays. This fails instead.
///
/// It also enforces ADR-0044 §6 in the direction that matters: the CLI draws and
/// the phone reads, so a decoder in the shipped binary would be hundreds of
/// kilobytes of camera plumbing for a machine with no camera.
#[test]
fn the_qr_decoder_never_becomes_a_shipped_dependency() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("this crate's manifest is readable");

    // Comments stripped first, exactly as `production_source` does for Rust:
    // the prose explaining *why* rqrr is dev-only sits above the section header,
    // so a naive split finds the word in the normal half and this guard failed
    // on itself the first time it ran. It was right to -- it was reading a
    // comment as a dependency -- and the fix is to read what the manifest
    // declares rather than what it says.
    let declared: String = manifest
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<&str>>()
        .join(
            "
",
        );
    let normal = declared
        .split("[dev-dependencies]")
        .next()
        .expect("a manifest has a first section")
        .to_owned();

    assert!(
        !normal.contains("rqrr"),
        "rqrr moved into the normal dependencies. Two `lru` advisories are \
         ignored in .cargo/audit.toml on the grounds that it is dev-only, and \
         that argument is now false -- delete the ignores and deal with them, \
         or put rqrr back."
    );
    // And the control: the guard is looking at a manifest that really does
    // mention rqrr somewhere, or it would pass on a file it failed to read.
    assert!(
        declared.contains("rqrr"),
        "this guard stopped finding rqrr at all, so it is passing vacuously"
    );
}
