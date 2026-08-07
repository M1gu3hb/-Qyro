// The shared structural guard, as `qyro_crypto`, `qyro_protocol` and
// `qyro_manifest` use it. See `rust/guards/source_guard.rs`.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the shared analysis reads files and must fail loudly when it cannot"
)]

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../guards/source_guard.rs"
));

/// Every file compiled into a release build of this crate.
const PRODUCTION_FILES: [&str; 3] = ["lib.rs", "blob.rs", "error.rs"];

#[test]
fn no_production_path_can_panic() {
    assert_no_production_path_can_panic(&PRODUCTION_FILES);
}

#[test]
fn every_production_file_is_listed() {
    assert_the_production_list_matches_the_source(&PRODUCTION_FILES);
}

/// Crates allowed to go without `#![forbid(unsafe_code)]`, and why.
///
/// Written before the Windows platform crate exists, deliberately. If this guard
/// arrived afterwards, adding the platform crate to the list would be
/// indistinguishable from a `forbid` that had never been there — and ADR-0024 §1
/// stakes the whole `unsafe` argument on exactly one crate relaxing it.
///
/// Both are here for the same reason: `#[unsafe(no_mangle)]` is an unsafe
/// attribute in edition 2024 and `forbid(unsafe_code)` refuses it. Verified by
/// adding the attribute to `qyro_ffi` and watching the build fail, not assumed.
///
/// Two, not one. `qyro_crypto_smoke` said in a comment that it was the only
/// crate in the repository without the attribute, and that was already untrue
/// when it was written.
/// `qyro_win_dpapi` is the third and, by ADR-0024 §1, the last: it is the crate
/// the whole hand-written-`extern` argument was built around, and it exists so
/// that no other crate in the product needs the exception. Its own guard
/// enumerates the three functions containing an `unsafe` block by name.
const CRATES_THAT_MAY_RELAX_FORBID_UNSAFE: [(&str, &str); 3] = [
    ("qyro_ffi", "rust/crates/qyro_ffi"),
    ("qyro_crypto_smoke", "rust/tools/qyro_crypto_smoke"),
    ("qyro_win_dpapi", "rust/crates/qyro_win_dpapi"),
];

/// Whether a crate root actually *declares* the attribute.
///
/// Line-anchored, not `contains`. A doc comment that mentions
/// `#![forbid(unsafe_code)]` — which the platform crate's does, while explaining
/// why it is the exception — is prose, not a declaration, and the first version
/// of this check could not tell them apart. It is the same shape as the smoke
/// crate whose comment made a `grep` report an attribute that was not there.
fn declares_forbid_unsafe(source: &str) -> bool {
    source
        .lines()
        .map(str::trim)
        .any(|line| line == "#![forbid(unsafe_code)]")
}

#[test]
fn only_the_listed_crates_may_relax_forbid_unsafe() {
    // QYR-0054. STATUS asserted "every crate keeps forbid(unsafe_code),
    // including the new one" and nothing checked it; removing the line from this
    // crate broke nothing. Writing the guard also found the claim was already
    // false: qyro_core and qyro_ffi both lacked it. qyro_core had zero unsafe and
    // simply never had the attribute, so it got one; qyro_ffi genuinely cannot.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .expect("the workspace root is three levels above this crate")
        .to_path_buf();

    let manifest = std::fs::read_to_string(root.join("Cargo.toml"))
        .expect("the workspace manifest is readable");
    let members: Vec<String> = manifest
        .split("members = [")
        .nth(1)
        .expect("the workspace declares members")
        .split(']')
        .next()
        .expect("the members list is closed")
        .lines()
        .filter_map(|line| line.trim().strip_prefix('"'))
        .filter_map(|line| line.split('"').next())
        .map(str::to_owned)
        .collect();

    assert!(
        members.len() >= 6,
        "only {} workspace members parsed; the manifest format changed and this \
         guard would pass by reading nothing: {members:?}",
        members.len()
    );

    let mut missing: Vec<String> = Vec::new();
    for member in &members {
        let name = member.rsplit('/').next().unwrap_or(member);
        if CRATES_THAT_MAY_RELAX_FORBID_UNSAFE
            .iter()
            .any(|(exempt, _)| *exempt == name)
        {
            continue;
        }
        let lib = root.join(member).join("src/lib.rs");
        let Ok(source) = std::fs::read_to_string(&lib) else {
            continue;
        };
        if !declares_forbid_unsafe(&source) {
            missing.push(name.to_owned());
        }
    }
    missing.sort();

    assert!(
        missing.is_empty(),
        "these crates have neither #![forbid(unsafe_code)] nor an entry in \
         CRATES_THAT_MAY_RELAX_FORBID_UNSAFE: {missing:?}\n\
         Relaxing the rule is allowed and must be argued in ADR-0024 §1; \
         relaxing it silently is not."
    );
}

#[test]
fn the_relaxation_list_names_only_crates_that_need_it() {
    // A list that outlives its reason is a list that stops meaning anything.
    // Every exception must actually be missing the attribute — otherwise it has
    // been fixed and the entry should go.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .expect("the workspace root is three levels above this crate")
        .to_path_buf();

    for (name, path) in CRATES_THAT_MAY_RELAX_FORBID_UNSAFE {
        let lib = root.join(path).join("src/lib.rs");
        let source = std::fs::read_to_string(&lib)
            .unwrap_or_else(|_| panic!("{name} is listed as an exception but has no crate root"));
        assert!(
            !declares_forbid_unsafe(&source),
            "{name} is listed as needing to relax forbid(unsafe_code) and now \
             carries it. Remove the exception."
        );
    }
}
