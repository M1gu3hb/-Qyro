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
const PRODUCTION_FILES: [&str; 5] = [
    "lib.rs",
    "blob.rs",
    "error.rs",
    "known_peer_types.rs",
    "known_peers.rs",
];

#[test]
fn no_production_path_can_panic() {
    assert_no_production_path_can_panic(&PRODUCTION_FILES);
}

#[test]
fn every_production_file_is_listed() {
    assert_the_production_list_matches_the_source(&PRODUCTION_FILES);
}

/// Variants constructed by a platform backend or its `PlatformWrapper`, not by
/// this platform-neutral crate. Each remains externally reachable by design:
/// absence, duplicate-create and I/O belong to the store implementation, while
/// `Unwrap` carries a wrapper's native refusal code.
const STORE_ERRORS_CONSTRUCTED_BY_PLATFORM_IMPLEMENTATIONS: [&str; 4] =
    ["IdentityAbsent", "Unwrap", "AlreadyExists", "Io"];

#[test]
fn every_store_error_has_a_construction_site_or_a_platform_argument() {
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "error.rs",
        "StoreError",
        13,
        &STORE_ERRORS_CONSTRUCTED_BY_PLATFORM_IMPLEMENTATIONS,
    );
}

#[test]
fn every_known_peer_store_error_has_a_construction_site() {
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "known_peer_types.rs",
        "KnownPeerStoreError",
        22,
        &[],
    );
}

#[test]
fn every_trust_verdict_has_a_construction_site() {
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "known_peer_types.rs",
        "TrustVerdict",
        3,
        &[],
    );
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

/// Workspace members that do not yet carry the shared minimum, and why.
///
/// The two absent network crates are named because they arrive from the
/// coordinated `claude/qyro-net-6a` branch with their own guards. Their entries
/// expire as soon as those members exist here: a merge must inspect their real
/// guard set rather than inheriting an exemption written before the files did.
const MINIMUM_GUARD_SET_EXCEPTIONS: [(&str, &str); 3] = [
    (
        "qyro_ffi",
        "reserved to the claude/qyro-net-6a branch; its C ABI has dedicated contract tests",
    ),
    (
        "qyro_net",
        "absent here; arrives with guards from the claude/qyro-net-6a branch",
    ),
    (
        "qyro_net_smoke",
        "absent here; arrives with guards from the claude/qyro-net-6a branch",
    ),
];

#[test]
fn every_workspace_crate_has_the_minimum_structural_guards_or_an_exact_exception() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .expect("the workspace root is three levels above this crate");
    let manifest = std::fs::read_to_string(root.join("Cargo.toml"))
        .expect("the workspace manifest is readable");
    let members: Vec<&str> = manifest
        .split("members = [")
        .nth(1)
        .expect("the workspace declares members")
        .split(']')
        .next()
        .expect("the members list is closed")
        .lines()
        .filter_map(|line| line.trim().strip_prefix('"'))
        .filter_map(|line| line.split('"').next())
        .collect();

    let mut missing = Vec::new();
    for member in &members {
        let name = member.rsplit('/').next().unwrap_or(member);
        let exception = MINIMUM_GUARD_SET_EXCEPTIONS
            .iter()
            .find(|(excepted, _)| *excepted == name);
        let guard_path = root.join(member).join("src/guards.rs");
        let guard = std::fs::read_to_string(&guard_path).unwrap_or_default();
        let source_root = root.join(member).join("src");
        let guard_module_is_active = ["lib.rs", "main.rs"].iter().any(|crate_root| {
            std::fs::read_to_string(source_root.join(crate_root))
                .is_ok_and(|source| source.lines().any(|line| line.trim() == "mod guards;"))
        });
        let has_minimum = guard_module_is_active
            && guard.contains("/../../guards/source_guard.rs")
            && guard.contains("assert_no_production_path_can_panic(&PRODUCTION_FILES)")
            && guard.contains("assert_the_production_list_matches_the_source(&PRODUCTION_FILES)");

        if let Some((_, reason)) = exception {
            assert!(
                !reason.trim().is_empty(),
                "{name} has an unargued guard exception"
            );
            if has_minimum {
                panic!("{name} now has the minimum guard set; remove its stale exception");
            }
        } else if !has_minimum {
            missing.push(name.to_owned());
        }

        if has_minimum {
            let listed = guard
                .split("const PRODUCTION_FILES")
                .nth(1)
                .and_then(|rest| rest.split("= [").nth(1))
                .and_then(|rest| rest.split("];").next())
                .unwrap_or_else(|| panic!("{name} has no parseable PRODUCTION_FILES list"));
            let files: Vec<&str> = listed
                .split('"')
                .enumerate()
                .filter_map(|(index, part)| (index % 2 == 1).then_some(part))
                .collect();
            assert!(!files.is_empty(), "{name} has an empty production list");

            let mut error_like_enums = Vec::new();
            for file in files {
                let source = std::fs::read_to_string(root.join(member).join("src").join(file))
                    .unwrap_or_else(|error| panic!("{member}/src/{file}: {error}"));
                for rest in source.split("pub enum ").skip(1) {
                    let enum_name: String = rest
                        .chars()
                        .take_while(|character| character.is_alphanumeric() || *character == '_')
                        .collect();
                    if enum_name.ends_with("Error") || enum_name.ends_with("Verdict") {
                        error_like_enums.push(enum_name);
                    }
                }
            }
            error_like_enums.sort();
            error_like_enums.dedup();
            for enum_name in error_like_enums {
                assert!(
                    guard.contains("assert_every_variant_has_a_construction_site(")
                        && guard.contains(&format!("\"{enum_name}\"")),
                    "{name} declares {enum_name} but its structural guards do not check every variant for a construction site"
                );
            }
        }
    }

    for (name, reason) in MINIMUM_GUARD_SET_EXCEPTIONS {
        if !members
            .iter()
            .any(|member| member.rsplit('/').next() == Some(name))
        {
            assert!(
                matches!(name, "qyro_net" | "qyro_net_smoke")
                    && reason.contains("claude/qyro-net-6a"),
                "{name} is absent from the workspace; only the two coordinated network crates may have a pre-merge exception"
            );
        }
    }

    assert!(
        missing.is_empty(),
        "workspace crates missing the shared minimum (production list, no-panic analysis, end-of-analysis check and anti-tautology guard): {missing:?}"
    );

    let crypto_guards = std::fs::read_to_string(root.join("rust/crates/qyro_crypto/src/guards.rs"))
        .expect("qyro_crypto guards are readable");
    assert!(
        crypto_guards.contains("every_public_path_returning_key_material_is_listed"),
        "qyro_crypto lost the public key-material egress guard"
    );
    let own_guards =
        std::fs::read_to_string(root.join("rust/crates/qyro_identity_store/src/guards.rs"))
            .expect("workspace policy guards are readable");
    assert!(
        own_guards.contains("only_the_listed_crates_may_relax_forbid_unsafe"),
        "the workspace lost the exact unsafe-relaxation guard"
    );
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
