//! A structural guard over every production file in this crate, plus the two
//! this crate needs that no other does.
//!
//! Until phase 01 `qyro_ffi` was the sole entry in
//! `MINIMUM_GUARD_SET_EXCEPTIONS` (QYR-0306). The exemption read "its C ABI has
//! dedicated contract tests", which was true and never sufficient: contract
//! tests check what the ABI *says*, and the shared minimum checks what the
//! source *can do*. This is the one crate in the workspace that crosses to C,
//! and it just went from two functions to eight.
//!
//! A panic here does not fail a test: it unwinds across a C frontier, which is
//! undefined behaviour.
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
const PRODUCTION_FILES: [&str; 4] = ["lib.rs", "abi.rs", "handle.rs", "session_abi.rs"];

#[test]
fn no_production_path_can_panic() {
    assert_no_production_path_can_panic(&PRODUCTION_FILES);
}

#[test]
fn every_production_file_is_listed() {
    assert_the_production_list_matches_the_source(&PRODUCTION_FILES);
}

/// Both variants are exempt, and the argument is the point of the exemption.
///
/// The shared guard asks that a variant be constructed *outside* the file that
/// declares it, which is the right question for an error a peer can provoke
/// across several modules -- `SessionError` is exactly that. `HandleError` is
/// not: the table is the only thing that can fail a lookup or run out of slots,
/// so both variants are produced by the module that declares them, and demanding
/// a construction site elsewhere would only be satisfiable by moving the table's
/// own logic out of the table.
///
/// What the call still buys is the parse floor: if this enum ever stops being
/// found, it reports zero variants and fails, rather than passing silently. The
/// reachability of both variants is held by
/// `handle::tests::an_invalid_handle_is_refused_by_name` and
/// `handle::tests::the_table_refuses_a_fifth_session_instead_of_growing`, which
/// construct them.
#[test]
fn every_handle_error_has_a_construction_site() {
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "handle.rs",
        "HandleError",
        2,
        &["NotLive", "Full"],
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
            "qyro_ffi/src/{file}: {} bytes analysed of {} raw",
            analysed.len(),
            raw.len()
        );
        assert!(!analysed.is_empty(), "src/{file} stripped to nothing");
    }
}

// ------------------------------- the two only this crate needs

/// Every `extern "C"` function opens with the panic guard.
///
/// ADR-0032 §5.5 is the one thing in that document written as "this is not
/// optional and is not decided: it is done". A function that skips `guard` lets
/// a panic unwind into C, and no test elsewhere would notice, because the
/// undefined behaviour is only reachable when something goes wrong.
#[test]
fn every_extern_c_function_sits_behind_the_panic_guard() {
    // The version pair is the documented exception: it returns a pointer to
    // static bytes and their length, running nothing that can panic.
    const WITHOUT_A_BODY_THAT_CAN_PANIC: [&str; 2] =
        ["qyro_protocol_version_ptr", "qyro_protocol_version_len"];

    let mut unguarded = Vec::new();
    let mut checked = 0_usize;
    for file in PRODUCTION_FILES {
        let text = production_source(file);
        // Three states, because a signature spans several lines: most of these
        // take six or seven parameters, so "the line after the name" is a
        // parameter and not the body. An earlier draft checked exactly that and
        // reported three false positives.
        let mut awaiting_body: Option<String> = None;
        let mut awaiting_guard: Option<String> = None;
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.split("extern \"C\" fn ").nth(1) {
                let name = rest.split('(').next().unwrap_or_default().to_owned();
                checked += 1;
                if !WITHOUT_A_BODY_THAT_CAN_PANIC.contains(&name.as_str()) {
                    // A short signature fits on one line, so the name line can
                    // also be the line that opens the body. Both shapes exist
                    // here and both must be read.
                    if trimmed.ends_with('{') {
                        awaiting_guard = Some(name);
                    } else {
                        awaiting_body = Some(name);
                    }
                }
                continue;
            }
            if let Some(name) = awaiting_body.clone() {
                if trimmed.ends_with('{') {
                    awaiting_body = None;
                    awaiting_guard = Some(name);
                }
                continue;
            }
            if let Some(name) = awaiting_guard.clone() {
                if trimmed.is_empty() {
                    continue;
                }
                if !trimmed.starts_with("guard(") {
                    unguarded.push(format!("{file}::{name}"));
                }
                awaiting_guard = None;
            }
        }
        for leftover in [awaiting_body, awaiting_guard].into_iter().flatten() {
            unguarded.push(format!("{file}::{leftover}"));
        }
    }

    assert!(
        unguarded.is_empty(),
        "these extern \"C\" functions do not open with `guard(`: {unguarded:?}"
    );
    // Without this the guard would pass on a crate with no C functions at all,
    // which is precisely the state it must not silently accept.
    assert!(
        checked >= 8,
        "only {checked} extern \"C\" functions found; the surface is eight, so \
         this analysis is not reading what it thinks it is"
    );
}

/// No Cargo profile sets `panic = "abort"`.
///
/// QYR-0305. `catch_unwind` catches nothing under `panic = "abort"`: the process
/// dies and takes the host application with it, which turns every `guard` above
/// into decoration. Nothing sets it today, so this keeps a property that
/// currently holds by accident rather than by contract.
#[test]
fn no_cargo_profile_sets_panic_abort() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("the workspace root is three levels above this crate");
    let manifest = std::fs::read_to_string(root.join("Cargo.toml"))
        .expect("the workspace manifest is readable");
    let stripped: String = manifest
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !stripped.replace(' ', "").contains("panic=\"abort\""),
        "a Cargo profile sets panic = \"abort\", which disables every catch_unwind \
         at the C boundary (QYR-0305)"
    );
    // The positive control: this must be reading a real manifest, not "".
    assert!(
        stripped.contains("[workspace]"),
        "the workspace manifest was not found, so this guard proves nothing"
    );
}
