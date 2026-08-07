// A structural guard over every production file in this crate.
//
// The analysis is shared with `qyro_protocol` and `qyro_manifest`; the list and
// the extra assertion below are this crate's own. See
// `rust/guards/source_guard.rs`, which also explains why the exemptions are
// derived from the code rather than listed by hand (QYR-0042).

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
///
/// `fuzzing.rs` is absent because it is `#[cfg(fuzzing)]`, and the per-module
/// test files because their `mod` declarations are gated — both derived, not
/// asserted here. A hand-written exemption list used to sit in this file, and
/// deleting a `#[cfg(test)]` would have turned a test file into an unguarded
/// production file with nothing failing.
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

#[test]
fn no_production_path_can_panic() {
    assert_no_production_path_can_panic(&PRODUCTION_FILES);
}

#[test]
fn every_test_only_module_is_actually_gated() {
    // QYR-0042. The exemptions are read from the `#[cfg(test)]` and
    // `#[cfg(fuzzing)]` declarations themselves, so removing one does not
    // quietly exempt a file — it moves that file into the production set, where
    // the list does not name it and this fails.
    assert_the_production_list_matches_the_source(&PRODUCTION_FILES);
}

/// Every public path of this crate that hands out key material.
///
/// Empty until sprint 4D.1, and that emptiness was the property: no caller could
/// obtain a seed or a private key at all, which is what `identity.rs` said in
/// prose. Persisting an identity requires exactly one way in and one way out, so
/// the list stops being empty and starts being *counted* — see ADR-0024 §4.
///
/// Enumerated by name rather than counted. A count lets one path be swapped for
/// another without the number moving, and the swap is the change worth catching.
/// Both are egress. `DeviceIdentity::from_secret` is ingress and is not here:
/// it consumes a secret rather than handing one out, so it does not widen what
/// a dependent crate can *obtain*. ADR-0024 §4 lists it as a new path because
/// it is one; this guard answers a narrower question.
const PUBLIC_KEY_MATERIAL_PATHS: [&str; 2] =
    ["identity.rs::as_bytes", "identity.rs::export_secret"];

/// Type names whose appearance in a return position means key material.
///
/// `[u8; 32]` is deliberately absent: a fingerprint is also thirty-two bytes,
/// and a marker that fires on the public half would get relaxed, and a relaxed
/// guard stops catching the harmful case.
const KEY_MATERIAL_MARKERS: [&str; 5] = [
    "IdentitySecret",
    "SigningKey",
    "SessionKey",
    "StaticSecret",
    "SEED_LEN",
];

#[test]
fn every_public_path_returning_key_material_is_listed() {
    let mut found: Vec<String> = Vec::new();

    for file in PRODUCTION_FILES {
        let source = production_source(file);
        // A signature can wrap, so join from `pub fn` up to the body or the
        // `where`, whichever comes first, before looking at the return type.
        for chunk in source.split("pub fn ").skip(1) {
            let signature: String = chunk
                .chars()
                .take_while(|c| *c != '{')
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let Some(name) = signature.split(['(', '<', ' ']).next() else {
                continue;
            };
            let Some((_, returns)) = signature.split_once("->") else {
                continue;
            };
            if KEY_MATERIAL_MARKERS
                .iter()
                .any(|marker| returns.contains(marker))
            {
                found.push(format!("{file}::{name}"));
            }
        }
    }
    found.sort();

    let expected: Vec<String> = PUBLIC_KEY_MATERIAL_PATHS
        .iter()
        .map(std::string::ToString::to_string)
        .collect();

    assert_eq!(
        found, expected,
        "the set of public paths returning key material changed.\n\
         found:    {found:?}\n\
         expected: {expected:?}\n\
         Adding one is a deliberate widening of what a dependent crate can ask \
         for. Update PUBLIC_KEY_MATERIAL_PATHS and say why in ADR-0024."
    );
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
