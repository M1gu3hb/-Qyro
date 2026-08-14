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

/// This crate closes no descriptor by hand, so `Drop` is the only closer.
///
/// The half of "exactly once" that cannot be observed from inside the process,
/// stated structurally instead. `session_abi::the_descriptor_is_closed_exactly_once`
/// proves the descriptor was **released**; it cannot prove the release happened
/// once rather than twice, because a second close of a number that has already
/// been reused is indistinguishable from a first close of whatever now holds it.
///
/// So the other half rests on ownership: `File::from_raw_fd` makes the `File`
/// the sole owner and its `Drop` runs once. A second close could only come from
/// code that calls one, there is none, and this fails if one appears.
///
/// The needles are assembled at run time rather than written whole, because a
/// guard that searches for a string it also contains reports itself.
#[test]
fn the_crate_closes_no_descriptor_by_hand() {
    let spellings = [
        format!("libc::{}", "close"),
        format!("{}(fd", "close"),
        format!("{}Handle", "Close"),
    ];

    let mut found = Vec::new();
    for file in PRODUCTION_FILES {
        let source = production_source(file);
        for spelling in &spellings {
            if source.contains(spelling.as_str()) {
                found.push(format!("src/{file}: {spelling}"));
            }
        }
    }
    assert!(
        found.is_empty(),
        "this crate closes a descriptor by hand: {found:?}. `File`'s Drop is the \
         single close ADR-0034 §2 depends on; a second one can close a descriptor \
         that was reassigned in between, and the victim is whatever took the \
         number — the transfer socket, for instance"
    );

    // Two positive controls, because the assertion above is an absence and an
    // absence passes for free when the analysis reads nothing.
    let boundary = production_source("session_abi.rs");
    assert!(
        boundary.contains("from_raw_fd"),
        "nothing takes ownership of a descriptor any more, so this guard asserts \
         the absence of a closer for something that never opens"
    );
    assert!(
        boundary.len() > 8_000,
        "the analysed source is {} bytes, which is not the C boundary; a search \
         over a truncated file finds nothing and says so cheerfully",
        boundary.len()
    );
}

/// Whether `bytes` carries a raw NUL, which is what makes a file "binary".
///
/// Split out of the assertion so the measurement can be shown to work on a
/// string that has one. A scan that cannot see a NUL is not evidence that there
/// is none, and this whole check exists because a NUL hid for a commit.
fn carries_a_raw_nul(bytes: &[u8]) -> bool {
    bytes.contains(&0)
}

/// No Rust source in this repository carries a raw NUL byte.
///
/// QYR-0327. `session_abi.rs` shipped with a literal NUL inside `split('\0')`
/// — the separator written as the byte instead of as the escape. It compiled,
/// it behaved identically, and it made **ripgrep and `grep` skip the entire
/// file as binary**: a repository-wide search for
/// `qyro_session_open_sender_fd_blocking` returned nothing while the function
/// was right there. That is not a cosmetic defect. Half the verification in
/// this project is textual — the guards, the reviews, the searches — and a file
/// no text tool will read is a file none of it covers.
///
/// Repository-wide rather than crate-local, for the same reason the panic guard
/// is: a check that lives in one crate protects one crate, and the file this
/// happened in is not special.
#[test]
fn no_rust_source_carries_a_raw_nul_byte() {
    let rust_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("rust/ is two levels above this crate")
        .to_path_buf();

    let mut offenders = Vec::new();
    let mut scanned = 0_usize;
    let mut pending = vec![rust_root.clone()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // `target` is build output and is genuinely full of binaries.
                if path.file_name().is_some_and(|name| name == "target") {
                    continue;
                }
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let Ok(bytes) = std::fs::read(&path) else {
                    continue;
                };
                scanned += 1;
                if carries_a_raw_nul(&bytes) {
                    offenders.push(path.display().to_string());
                }
            }
        }
    }

    // The positive control. Without it a walk that found nothing — a moved
    // directory, a changed layout — would report success.
    assert!(
        scanned >= 100,
        "only {scanned} Rust files were read under {}; the walk is not seeing \
         the workspace, so this guard proves nothing",
        rust_root.display()
    );
    // And the measurement can see what it is for (R2 §1.7).
    assert!(
        carries_a_raw_nul(b"split('\0')"),
        "the scan cannot detect a raw NUL, so a clean result means nothing"
    );
    assert!(
        !carries_a_raw_nul(br"split('\0')"),
        "the scan flags the escape as well as the byte, so it cannot tell the \
         fix from the defect"
    );

    assert!(
        offenders.is_empty(),
        "these Rust sources carry a raw NUL byte, which makes grep and ripgrep \
         treat them as binary and skip them entirely: {offenders:?}. Write the \
         byte as the escape `\\0`."
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
