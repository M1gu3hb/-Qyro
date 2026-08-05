//! Structural guards over the AEAD module's own production source.
//!
//! Three properties here cannot be expressed as ordinary tests.
//!
//! *No `assert!` on the production path.* Clippy denies `unwrap`, `expect`,
//! `panic!`, `unreachable!` and indexing at the top of `mod.rs`, and the
//! compiler is a far better judge of those than a regular expression. But there
//! is no Clippy lint for `assert!`, and `assert!` ends the process exactly as
//! `panic!` does — in a module that holds keys, in the middle of code that was
//! about to zeroize something.
//!
//! *Plaintext lives in a zeroizing container.* A test can check that a byte
//! slice compares equal; it cannot check that the allocation behind it is wiped
//! when it drops, because reading freed memory is undefined behaviour and an
//! allocator may reuse or unmap the page. What can be checked is the type, and
//! the type is what carries the guarantee.
//!
//! *No accessor hands the plaintext out unprotected.* `into_payload(self) ->
//! Vec<u8>` would silently drop the guarantee at the one call that matters.
//!
//! The analysis is deliberately narrow. It reads only this module's production
//! files, strips comments and `#[cfg(test)]` items by brace matching, and never
//! guesses: an expression it cannot account for makes it fail rather than pass.

use std::fs;

/// The production source files of this module.
///
/// `tests.rs`, `vectors.rs` and `corpus.rs` are absent on purpose: they are
/// `cfg(test)` in their entirety and may assert freely.
const PRODUCTION_FILES: [&str; 3] = ["mod.rs", "error.rs", "replay.rs"];

fn production_source(file: &str) -> String {
    let path = format!("{}/src/aead/{file}", env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    strip_comments(&strip_cfg_test_items(&source))
}

/// Removes every `#[cfg(test)]` item, including inline modules with a body.
///
/// Brace matching rather than "everything after the first `#[cfg(test)]`",
/// because `replay.rs` puts its test module last today and nothing guarantees
/// the next file will.
fn strip_cfg_test_items(source: &str) -> String {
    const MARKER: &str = "#[cfg(test)]";
    let mut out = String::with_capacity(source.len());
    let mut rest = source;

    while let Some(position) = rest.find(MARKER) {
        out.push_str(&rest[..position]);
        let after = &rest[position + MARKER.len()..];

        // Either `mod name;` (nothing to strip beyond the declaration) or
        // `mod name { ... }` / `fn ... { ... }` (strip the whole body).
        let semicolon = after.find(';');
        let brace = after.find('{');
        rest = match (semicolon, brace) {
            (Some(end), None) => &after[end + 1..],
            (Some(end), Some(open)) if end < open => &after[end + 1..],
            (_, Some(open)) => {
                let mut depth = 0usize;
                let mut close = None;
                for (index, byte) in after.bytes().enumerate().skip(open) {
                    match byte {
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                close = Some(index);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                match close {
                    Some(index) => &after[index + 1..],
                    None => panic!("unbalanced braces after a #[cfg(test)] item"),
                }
            }
            (None, None) => panic!("a #[cfg(test)] item with neither a body nor a semicolon"),
        };
    }
    out.push_str(rest);
    out
}

/// Removes line comments, so prose about `panic!` is not mistaken for one.
fn strip_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_stripper_actually_strips() {
    // Without this, every assertion below could be passing because the analysis
    // silently produced an empty string.
    let stripped = strip_cfg_test_items(
        "fn kept() {}\n#[cfg(test)]\nmod tests {\n    fn inner() { assert!(true); }\n}\nfn also_kept() {}\n",
    );
    assert!(stripped.contains("fn kept"), "production code survives");
    assert!(stripped.contains("fn also_kept"), "and so does what follows");
    assert!(!stripped.contains("assert!"), "the test body is gone");

    let declaration = strip_cfg_test_items("#[cfg(test)]\nmod tests;\nfn kept() {}\n");
    assert!(declaration.contains("fn kept"));

    assert_eq!(strip_comments("// panic!\nlet x = 1;"), "let x = 1;");

    for file in PRODUCTION_FILES {
        assert!(
            production_source(file).contains("pub"),
            "{file} still has production code after stripping"
        );
    }
}

#[test]
fn the_production_aead_path_contains_no_assertions() {
    // `assert!` is `panic!` under another name, and Clippy has no lint for it.
    // A frame the peer controls must never be able to end this process.
    for file in PRODUCTION_FILES {
        let source = production_source(file);
        for forbidden in [
            "assert!(",
            "assert_eq!(",
            "assert_ne!(",
            "debug_assert!(",
            "debug_assert_eq!(",
            "debug_assert_ne!(",
        ] {
            assert!(
                !source.contains(forbidden),
                "src/aead/{file} uses {forbidden} on the production path; \
                 an invariant that can fail must return a typed error"
            );
        }
    }
}

#[test]
fn verified_plaintext_lives_in_a_zeroizing_container() {
    let source = production_source("mod.rs");

    assert!(
        source.contains("payload: Zeroizing<Vec<u8>>"),
        "AuthenticatedFrame must store its plaintext in a zeroizing container; \
         a plain Vec leaves verified plaintext in freed memory"
    );
    assert!(
        !source.contains("fn into_payload(self) -> Vec<u8>"),
        "into_payload would drop the zeroization guarantee at the one call that \
         matters; hand out the zeroizing container instead"
    );
    assert!(
        source.contains("fn into_zeroizing_payload"),
        "there must be a way to take ownership that keeps the guarantee"
    );
}

#[test]
fn the_temporary_buffers_are_zeroizing_too() {
    // The buffer inside `seal` starts as plaintext, and the buffer inside
    // `open` becomes plaintext the moment the tag verifies. Both outlive their
    // usefulness by one drop, and both are the obvious place for plaintext to
    // survive an early return.
    let source = production_source("mod.rs");
    let occurrences = source.matches("Zeroizing::new(").count();
    assert!(
        occurrences >= 3,
        "expected zeroizing temporaries in seal and open plus the derived key, \
         found {occurrences} constructions"
    );
}
