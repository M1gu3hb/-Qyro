//! A structural guard over every production file in this crate.
//!
//! Clippy denies `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!` and
//! `unimplemented!` module by module, and the compiler is a far better judge of
//! those than a regular expression. This guard exists for the two things the
//! lint cannot do.
//!
//! *It notices a module nobody added the lint to.* A `#![deny(...)]` protects
//! the module it is written in. A new file with no attribute is unprotected and
//! looks exactly like a protected one. This test enumerates the files instead,
//! so an unlisted module fails here rather than passing silently.
//!
//! *It catches `assert!`.* There is no Clippy lint for it, and `assert!` ends
//! the process exactly as `panic!` does — in a crate that holds keys, in the
//! middle of code that was about to zeroize something.
//!
//! Every input on these paths is chosen by a peer: a hello, a finish message,
//! a public key, a sealed frame. A panic on any of them is a remote denial of
//! service, and an invariant that can fail must return a typed error instead.
//!
//! The analysis is deliberately narrow. It reads only the files below, strips
//! comments, compile-time assertions and test-only items, and refuses to guess:
//! anything it cannot account for makes it fail rather than pass.

use std::fs;

/// Every file that exists in a release build of this crate.
///
/// `fuzzing.rs` is absent because it is `#[cfg(fuzzing)]`: `cargo-fuzz` sets
/// that flag on the command line for one build and no ordinary `cargo build`,
/// `cargo test` or `cargo install` compiles the module at all.
///
/// `schema.rs`, `vectors.rs`, this file and the per-module test files are
/// absent because they are `#[cfg(test)]` in their entirety and may assert
/// freely — a test that cannot assert reports failures worse.
///
/// Adding a production module and forgetting to add it here is the failure this
/// list exists to catch, so [`every_production_file_is_listed`] walks `src/` and
/// compares.
const PRODUCTION_FILES: [&str; 12] = [
    "lib.rs",
    "error.rs",
    "fingerprint.rs",
    "identity.rs",
    "signature.rs",
    "aead/mod.rs",
    "aead/error.rs",
    "aead/replay.rs",
    "handshake/mod.rs",
    "handshake/error.rs",
    "handshake/schedule.rs",
    "handshake/transcript.rs",
];

/// Macros that end the process, and the two that end it only in debug builds.
///
/// `debug_assert!` is here because a release build silently skipping a check is
/// not the property this crate wants either: an invariant worth stating is
/// worth returning an error for.
const PROCESS_ENDING: [&str; 12] = [
    ".unwrap()",
    ".expect(",
    "panic!(",
    "unreachable!(",
    "todo!(",
    "unimplemented!(",
    "assert!(",
    "assert_eq!(",
    "assert_ne!(",
    "debug_assert!(",
    "debug_assert_eq!(",
    "debug_assert_ne!(",
];

/// Reads one production file with everything non-production removed.
pub(crate) fn production_source(relative_path: &str) -> String {
    let path = format!("{}/src/{relative_path}", env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    // Comments first: a doc comment naming `panic!`, or prose containing an
    // unbalanced brace, must not reach either pass below.
    let source = strip_comments(&source);
    let source = strip_const_assertions(&source);
    strip_test_only_items(&source)
}

/// Removes line comments, so prose about `panic!` is not mistaken for one.
fn strip_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Removes `const _: () = ...;` items.
///
/// These are compile-time assertions. `const _: () = assert!(A == B);` is
/// evaluated during const evaluation, so a violation stops the *build*; there
/// is no runtime path through it and nothing it can do to a running process.
/// `handshake/mod.rs` uses two to pin widths that must stay in step with the
/// AEAD, which is exactly the right way to state an invariant that cannot fail
/// at runtime.
///
/// The exemption is deliberately spelled `const _: () =` and nothing wider, so
/// it cannot quietly grow to cover a runtime assertion.
fn strip_const_assertions(source: &str) -> String {
    const MARKER: &str = "const _: () =";
    let mut out = String::with_capacity(source.len());
    let mut rest = source;

    while let Some(position) = rest.find(MARKER) {
        out.push_str(&rest[..position]);
        let after = &rest[position + MARKER.len()..];
        let end = item_end(after)
            .unwrap_or_else(|| panic!("a `const _: () =` item with no terminating `;`"));
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// Removes every item that does not exist in a release build.
///
/// Brace matching rather than "everything after the first marker", because a
/// file may put its test module anywhere and `replay.rs` and `mod.rs` disagree
/// about where.
fn strip_test_only_items(source: &str) -> String {
    // `#[cfg(not(any(test, fuzzing)))]` is production and must not match: the
    // literals below both begin `#[cfg(` followed by the condition itself, so a
    // negated form never matches either of them.
    const MARKERS: [&str; 2] = ["#[cfg(test)]", "#[cfg(any(test, fuzzing))]"];

    let mut out = String::with_capacity(source.len());
    let mut rest = source;

    loop {
        let Some((position, marker)) = MARKERS
            .iter()
            .filter_map(|marker| rest.find(marker).map(|position| (position, *marker)))
            .min_by_key(|(position, _)| *position)
        else {
            break;
        };
        out.push_str(&rest[..position]);
        let after = &rest[position + marker.len()..];
        let end = item_end(after)
            .unwrap_or_else(|| panic!("a test-only item with neither a body nor a `;`"));
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// Finds where one item ends: the first `;` at depth zero, or the close of the
/// first `{` opened at depth zero.
///
/// Depth-aware on purpose. Deciding by "whichever of `;` and `{` comes first"
/// gets `-> &[u8; 32] {` wrong, because the return type carries a semicolon
/// before the body opens, and gets `#[allow(...)] mod corpus;` wrong in the
/// other direction. Both spellings are in this crate.
///
/// String literals are skipped so a brace inside one cannot unbalance the
/// count. Character literals are not: `&'a str` is indistinguishable from an
/// opening quote without parsing Rust, and this crate has lifetimes and no
/// braces in char literals.
fn item_end(source: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut round = 0usize;
    let mut square = 0usize;
    let mut curly = 0usize;
    let mut index = 0usize;
    let mut opened_body = false;

    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'"' {
                    index += if bytes[index] == b'\\' { 2 } else { 1 };
                }
            }
            b'(' => round += 1,
            b')' => round = round.saturating_sub(1),
            b'[' => square += 1,
            b']' => square = square.saturating_sub(1),
            b'{' => {
                if round == 0 && square == 0 && curly == 0 {
                    opened_body = true;
                }
                curly += 1;
            }
            b'}' => {
                curly = curly.saturating_sub(1);
                if opened_body && curly == 0 && round == 0 && square == 0 {
                    return Some(index + 1);
                }
            }
            b';' if round == 0 && square == 0 && curly == 0 => return Some(index + 1),
            _ => {}
        }
        index += 1;
    }
    None
}

#[test]
fn no_production_path_can_panic() {
    for file in PRODUCTION_FILES {
        let source = production_source(file);
        for forbidden in PROCESS_ENDING {
            assert!(
                !source.contains(forbidden),
                "src/{file} uses {forbidden} on the production path. Every input \
                 that reaches this crate is chosen by a peer, so ending the \
                 process is a remote denial of service and an abort in the \
                 middle of code that was about to zeroize something. An \
                 invariant that can fail must return a typed error."
            );
        }
    }
}

#[test]
fn every_handshake_error_has_a_construction_site() {
    // "An error nobody can provoke documents a check that is not there" is the
    // rule `aead/error.rs` already states. `HandshakeError` broke it four times:
    // `UnexpectedRole`, `InvalidEphemeralPublicKey`, `TranscriptMismatch` and
    // `SequenceViolation` were declared, formatted, listed in the ADR and in
    // `docs/security/handshake-state-machine.md`, and constructed by nothing —
    // so a caller could match on a control that did not exist.
    //
    // Declaring a variant is free; the point of this test is that producing one
    // is not.
    let declaration = production_source("handshake/error.rs");
    let body = declaration
        .split("pub enum HandshakeError {")
        .nth(1)
        .expect("HandshakeError is declared in handshake/error.rs");

    let variants: Vec<&str> = body
        .lines()
        .take_while(|line| !line.starts_with('}'))
        // Variants sit at four spaces; the `Display` match arms are deeper, and
        // struct fields inside a variant deeper still.
        .filter(|line| line.starts_with("    ") && !line.starts_with("     "))
        .map(str::trim)
        .filter(|line| line.chars().next().is_some_and(char::is_uppercase))
        .map(|line| {
            line.trim_end_matches(',')
                .split([' ', '{', '('])
                .next()
                .unwrap_or(line)
        })
        .collect();

    assert!(
        variants.len() > 5,
        "the parse found {} variants, which means it stopped reading the enum \
         rather than that the enum shrank",
        variants.len()
    );

    let elsewhere: String = PRODUCTION_FILES
        .iter()
        .filter(|file| **file != "handshake/error.rs")
        .map(|file| production_source(file))
        .collect();

    for variant in variants {
        assert!(
            elsewhere.contains(&format!("HandshakeError::{variant}")),
            "HandshakeError::{variant} is declared but nothing constructs it. A \
             variant a peer can never see is a check that is not there, and a \
             caller matching on it believes otherwise. Either produce it or \
             delete it."
        );
    }
}

#[test]
fn every_production_file_is_listed() {
    // The list above is only a guarantee if it is complete. A new module that
    // nobody added to it would be unguarded and indistinguishable from a
    // guarded one, which is the same shape of defect as a `#![deny(...)]`
    // somebody forgot to write.
    let root = format!("{}/src", env!("CARGO_MANIFEST_DIR"));
    let mut found: Vec<String> = Vec::new();
    collect_sources(&root, "", &mut found);

    // Everything compiled only under `cfg(test)` or `cfg(fuzzing)`, named here
    // so that adding one is a deliberate act.
    const TEST_ONLY: [&str; 10] = [
        "guards.rs",
        "schema.rs",
        "vectors.rs",
        "fuzzing.rs",
        "aead/corpus.rs",
        "aead/guards.rs",
        "aead/tests.rs",
        "aead/vectors.rs",
        "handshake/closure_tests.rs",
        "handshake/tests.rs",
    ];
    // `handshake/vectors.rs` is `cfg(test)` too; kept out of the array above
    // only because two files share the name `vectors.rs` and the check below
    // compares full relative paths.
    for file in found {
        if TEST_ONLY.contains(&file.as_str()) || file == "handshake/vectors.rs" {
            continue;
        }
        assert!(
            PRODUCTION_FILES.contains(&file.as_str()),
            "src/{file} is a production file that no guard covers; add it to \
             PRODUCTION_FILES, or to TEST_ONLY if it is cfg(test)"
        );
    }
}

fn collect_sources(directory: &str, prefix: &str, out: &mut Vec<String>) {
    let entries = fs::read_dir(directory).unwrap_or_else(|error| panic!("{directory}: {error}"));
    for entry in entries {
        let entry = entry.expect("a readable directory entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let kind = entry.file_type().expect("a knowable file type");
        if kind.is_dir() {
            collect_sources(&entry.path().to_string_lossy(), &relative, out);
        } else if name.ends_with(".rs") {
            out.push(relative);
        }
    }
}

#[test]
fn the_stripper_actually_strips() {
    // Without this, every assertion above could be passing because the analysis
    // silently produced an empty string.
    let stripped = strip_test_only_items(
        "fn kept() {}\n#[cfg(test)]\nmod tests {\n    fn inner() { assert!(true); }\n}\nfn also_kept() {}\n",
    );
    assert!(stripped.contains("fn kept"), "production code survives");
    assert!(
        stripped.contains("fn also_kept"),
        "and so does what follows"
    );
    assert!(!stripped.contains("assert!"), "the test body is gone");

    let declaration = strip_test_only_items("#[cfg(test)]\nmod tests;\nfn kept() {}\n");
    assert!(declaration.contains("fn kept"));

    // An attribute between the marker and the declaration, which is how the
    // AEAD module opts its test children out of the lint.
    let attributed = strip_test_only_items(
        "#[cfg(test)]\n#[allow(clippy::unwrap_used)]\nmod corpus;\nfn kept() {}\n",
    );
    assert!(
        attributed.contains("fn kept"),
        "an attribute before the declaration must not swallow what follows"
    );

    // A semicolon inside a return type, before the body opens.
    let typed = strip_test_only_items(
        "#[cfg(test)]\nfn keys(&self) -> &[u8; 32] { self.0.expect(\"x\") }\nfn kept() {}\n",
    );
    assert!(typed.contains("fn kept"), "the body ends at its own brace");
    assert!(!typed.contains(".expect("), "and the body is gone");

    // The deterministic constructors, which are not `cfg(test)` alone.
    let dual = strip_test_only_items(
        "#[cfg(any(test, fuzzing))]\nfn seeded() { panic!(\"x\") }\nfn kept() {}\n",
    );
    assert!(dual.contains("fn kept"));
    assert!(!dual.contains("panic!("));

    // A negated cfg is production and must survive untouched.
    let negated =
        strip_test_only_items("#[cfg(not(any(test, fuzzing)))]\nfn real() { }\nfn kept() {}\n");
    assert!(negated.contains("fn real"), "a negated cfg is production");
    assert!(negated.contains("fn kept"));

    // A brace inside a string literal must not unbalance the count.
    let quoted = strip_test_only_items(
        "#[cfg(test)]\nfn f() { let s = \"}\"; }\nfn kept() {}\nfn tail() { }\n",
    );
    assert!(
        quoted.contains("fn kept"),
        "a brace in a string is not a brace"
    );
    assert!(quoted.contains("fn tail"));

    // Compile-time assertions are exempt, and only in that exact spelling.
    let compile_time = strip_const_assertions("const _: () = assert!(1 == 1);\nfn kept() {}\n");
    assert!(
        !compile_time.contains("assert!("),
        "a const assertion is stripped"
    );
    assert!(compile_time.contains("fn kept"));

    let runtime = strip_const_assertions("fn f() { assert!(x); }\n");
    assert!(
        runtime.contains("assert!("),
        "a runtime assertion is not a const item and must survive to be caught"
    );

    assert_eq!(strip_comments("// panic!\nlet x = 1;"), "let x = 1;");

    for file in PRODUCTION_FILES {
        assert!(
            production_source(file).contains("pub"),
            "{file} still has production code after stripping"
        );
    }
}
