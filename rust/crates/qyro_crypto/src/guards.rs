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

/// Public paths that hand out secret bytes.
///
/// All egress. `DeviceIdentity::from_secret` is ingress and is not here: it
/// consumes a secret rather than handing one out.
///
/// `into_zeroizing_payload` returns authenticated plaintext rather than a key,
/// and it belongs here anyway: the question this guard answers is what secret
/// bytes a dependent crate can obtain, and decrypted payload is secret. The
/// narrower marker list this replaced never saw it at all (QYR-0053).
///
/// Enumerated by name rather than counted. A count lets one path be swapped for
/// another without the number moving, and the swap is the change worth catching.
const PUBLIC_KEY_MATERIAL_PATHS: [&str; 5] = [
    "aead/mod.rs::into_zeroizing_payload",
    // `VerifiedPayload::as_slice`. The one way to read the verified plaintext,
    // and therefore the one place a copy can still be written -- deliberately,
    // and out loud. See the type's own documentation.
    "aead/mod.rs::as_slice",
    // `AuthenticatedFrame::payload`. Hands out **the same bytes** as
    // `into_zeroizing_payload`, borrowed, and was invisible to this guard until
    // `&[u8]` became a marker on 2026-08-14. The phase-01 report noticed it and
    // filed it nowhere; QYR-0053 is the ficha whose whole subject is a marker
    // list too narrow to see what it guards.
    "aead/mod.rs::payload",
    "identity.rs::as_bytes",
    "identity.rs::export_secret",
];

/// Public paths that return byte-shaped values which are **not** key material,
/// classified deliberately rather than by omission.
///
/// This list is the other half of QYR-0053. The guard used to work off a list of
/// type-name markers, which is an allow-list wearing a deny-list's clothes:
/// adding `pub fn leak_raw(&self) -> Zeroizing<[u8; 32]>` — the seed, in the
/// clear — left it green, because the return type happened to contain none of
/// the five names it knew. `[u8; 32]` had been excluded on the correct
/// observation that a fingerprint is also thirty-two bytes, and on the incorrect
/// conclusion that it could therefore be ignored.
///
/// Now every byte-shaped public return must be in one list or the other, and
/// being in neither fails. A new path forces a decision instead of passing in
/// silence.
const PUBLIC_NON_KEY_BYTE_PATHS: [&str; 0] = [];

/// Return shapes that demand classification.
///
/// Deliberately broad. A marker that is too narrow fails open, which is the
/// defect this replaced; a marker that is too broad costs one line in the
/// not-key-material list.
const BYTE_RETURN_MARKERS: [&str; 10] = [
    "[u8; 32]",
    "[u8; SEED_LEN]",
    "[u8; PUBLIC_KEY_LEN]",
    "Zeroizing",
    "IdentitySecret",
    "SigningKey",
    "SessionKey",
    "StaticSecret",
    // Added 2026-08-14 with QYR-0325. `into_zeroizing_payload` stopped
    // returning a `Zeroizing` and started returning this wrapper, and the guard
    // went blind to it in the same commit -- it refused to keep a list entry it
    // could no longer see, which is the assertion below doing its job.
    "VerifiedPayload",
    // And the marker whose absence is the same defect QYR-0053 names: a bare
    // borrowed slice was never a marker at all, so any `pub fn` handing out
    // `&[u8]` was invisible. `VerifiedPayload::as_slice` would have been.
    "&[u8]",
];

/// Collects every public path whose return type is byte-shaped.
fn public_byte_returning_paths() -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for file in PRODUCTION_FILES {
        let source = production_source(file);
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
            if BYTE_RETURN_MARKERS
                .iter()
                .any(|marker| returns.contains(marker))
            {
                found.push(format!("{file}::{name}"));
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

#[test]
fn every_public_path_returning_key_material_is_listed() {
    let found = public_byte_returning_paths();
    let key: Vec<&str> = PUBLIC_KEY_MATERIAL_PATHS.to_vec();
    let not_key: Vec<&str> = PUBLIC_NON_KEY_BYTE_PATHS.to_vec();

    let unclassified: Vec<&String> = found
        .iter()
        .filter(|path| !key.contains(&path.as_str()) && !not_key.contains(&path.as_str()))
        .collect();
    assert!(
        unclassified.is_empty(),
        "these public paths return byte-shaped values and are in neither \
         list: {unclassified:?}\n\
         Put each in PUBLIC_KEY_MATERIAL_PATHS if it hands out secret bytes, \
         or in PUBLIC_NON_KEY_BYTE_PATHS if it does not, and say which in \
         ADR-0024 §4. Being in neither is the failure: it is how a seed \
         accessor passed this guard unnoticed (QYR-0053). All of them are \
         reported at once so that classifying one does not just reveal the next."
    );

    // The key-material list must not name something that no longer exists, or
    // it becomes a record of what used to be true.
    for path in &key {
        assert!(
            found.iter().any(|f| f == path),
            "{path} is listed as key material but no such public path was found"
        );
    }
}

#[test]
fn every_plain_secret_array_is_zeroized_on_drop() {
    let source = format!(
        "{}\n{}",
        production_source("aead/mod.rs"),
        production_source("handshake/schedule.rs")
    );
    let compact: String = source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();

    for (secret_type, drop_body) in [
        (
            "DirectionalKeys",
            "implDropforDirectionalKeys{fndrop(&mutself){self.nonce_prefix.zeroize();}}",
        ),
        (
            "SessionKey",
            "implDropforSessionKey{fndrop(&mutself){self.0.zeroize();}}",
        ),
    ] {
        assert!(
            compact.contains(drop_body),
            "{secret_type} must keep an explicit Drop implementation that zeroizes its plain secret array"
        );
    }
}

#[test]
fn every_identity_error_has_a_construction_site() {
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "error.rs",
        "IdentityError",
        9,
        &[],
    );
}

#[test]
fn every_aead_error_has_a_construction_site() {
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "aead/error.rs",
        "AeadError",
        12,
        &[],
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
    // is not. Use the shared parser so the meta-guard can prove this check did
    // not stay behind as a crypto-only implementation again.
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "handshake/error.rs",
        "HandshakeError",
        13,
        &[],
    );
}
