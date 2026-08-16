//! The C ABI contract, and the boundary that replaced the old closure guard.
//!
//! Until phase 01 this file asserted that `qyro_ffi`'s dependency closure was
//! exactly `{qyro_core, qyro_ffi}`. That made the guarantee structural: the code
//! that would leak a key could not be made to compile.
//!
//! Driving a transfer needs the engine, the engine needs the AEAD, and so the
//! cryptographic stack necessarily enters the library Dart loads. ADR-0032 §1
//! measures the consequence, and it is the reason this file changed shape:
//!
//! > Once `qyro_crypto` is inside the closure, adding a **direct**
//! > `qyro_ffi -> qyro_crypto` edge changes the closure by **nothing**.
//!
//! That direct edge — one line in a `Cargo.toml` — is exactly what the old
//! guard existed to prevent, and no closure-shaped assertion can see it any
//! more. So the guard is now three pieces, and only the first is load-bearing.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

use std::collections::{BTreeSet, HashMap};
use std::process::Command;

use qyro_ffi::{qyro_protocol_version_len, qyro_protocol_version_ptr};
use serde_json::Value;

#[test]
fn c_abi_exposes_the_protocol_version_without_ownership_transfer() {
    let pointer = qyro_protocol_version_ptr();
    let length = qyro_protocol_version_len();

    assert!(!pointer.is_null());

    // SAFETY: The ABI contract promises a non-null pointer to immutable static
    // bytes and returns their exact length. Ownership remains in the library.
    let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };

    assert_eq!(bytes, b"QYRO/1");
}

// ------------------------------------------------------------------ metadata

fn workspace_metadata() -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON")
}

fn id_of(metadata: &Value, package: &str) -> String {
    metadata["packages"]
        .as_array()
        .expect("metadata lists packages")
        .iter()
        .find(|entry| entry["name"] == package)
        .and_then(|entry| entry["id"].as_str())
        .unwrap_or_else(|| panic!("{package} is a member of this workspace"))
        .to_owned()
}

/// Every crate reachable from `root` by a normal or build dependency.
///
/// Dev-dependencies are excluded: they are not linked into the library Dart
/// loads. No `--filter-platform`, deliberately — a dependency that only applies
/// to Android must still count.
fn dependency_closure(metadata: &Value, root_name: &str) -> BTreeSet<String> {
    let packages = metadata["packages"]
        .as_array()
        .expect("metadata lists packages");
    let name_of: HashMap<&str, &str> = packages
        .iter()
        .filter_map(|package| Some((package["id"].as_str()?, package["name"].as_str()?)))
        .collect();
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .expect("metadata resolves a graph");
    let node_of: HashMap<&str, &Value> = nodes
        .iter()
        .filter_map(|node| Some((node["id"].as_str()?, node)))
        .collect();

    let root = id_of(metadata, root_name);
    let mut reached = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        let Some(name) = name_of.get(id.as_str()) else {
            continue;
        };
        if !reached.insert((*name).to_owned()) {
            continue;
        }
        let Some(node) = node_of.get(id.as_str()) else {
            continue;
        };
        let Some(deps) = node["deps"].as_array() else {
            continue;
        };
        for dep in deps {
            let linked = dep["dep_kinds"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind["kind"] != "dev"));
            if linked && let Some(dependency) = dep["pkg"].as_str() {
                pending.push(dependency.to_owned());
            }
        }
    }
    reached
}

/// The crates `qyro_ffi` itself names — depth one, not the closure.
fn ffi_direct_dependencies(metadata: &Value) -> BTreeSet<String> {
    let ffi = id_of(metadata, "qyro_ffi");
    let name_of: HashMap<&str, &str> = metadata["packages"]
        .as_array()
        .expect("metadata lists packages")
        .iter()
        .filter_map(|package| Some((package["id"].as_str()?, package["name"].as_str()?)))
        .collect();

    metadata["resolve"]["nodes"]
        .as_array()
        .expect("metadata resolves a graph")
        .iter()
        .find(|node| node["id"].as_str() == Some(ffi.as_str()))
        .and_then(|node| node["deps"].as_array())
        .map(|deps| {
            deps.iter()
                .filter(|dep| {
                    dep["dep_kinds"]
                        .as_array()
                        .is_some_and(|kinds| kinds.iter().any(|kind| kind["kind"] != "dev"))
                })
                .filter_map(|dep| name_of.get(dep["pkg"].as_str()?).map(|n| (*n).to_owned()))
                .collect()
        })
        .unwrap_or_default()
}

// ------------------------------------------- guard 1: the load-bearing one

/// The crates `qyro_ffi` can *name*, and there is no list in this assertion.
///
/// This is the piece that actually enforces anything. Rust's name resolution
/// puts only **direct** dependencies in a crate's extern prelude, so
/// `qyro_crypto::DeviceIdentity` cannot be written inside `qyro_ffi` unless
/// `qyro_crypto` appears in `qyro_ffi`'s own manifest. Pinning the direct set to
/// two crates therefore bounds everything reachable of the cryptographic stack
/// by `qyro_session`'s public API — one small, frozen surface — and it does so
/// without any deny-list that a future crate could be missing from.
#[test]
fn the_ffi_names_exactly_two_crates() {
    let direct = ffi_direct_dependencies(&workspace_metadata());
    let expected: BTreeSet<String> = ["qyro_core", "qyro_session"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        direct, expected,
        "qyro_ffi may name qyro_core and qyro_session and nothing else. Anything \
         it names directly is a crate whose entire public API it can reach, and \
         that is the bound this guard exists to keep small."
    );
}

// ------------------------------------------- guard 2: the facade stays a facade

/// Whether a `pub use` line re-exports something of this crate's own.
///
/// A foreign re-export would widen the bound guard 1 establishes: everything
/// `qyro_session` republishes is reachable from `qyro_ffi` by name.
/// The modules `qyro_session/src/lib.rs` declares.
///
/// Read from the source rather than listed, so a new module is local the moment
/// it is declared and not when somebody remembers to update a guard.
fn facade_modules() -> BTreeSet<String> {
    let facade = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../qyro_session/src/lib.rs"
    ))
    .expect("the facade is readable");

    let mut found = BTreeSet::new();
    for line in facade.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed
            .strip_prefix("mod ")
            .or_else(|| trimmed.strip_prefix("pub mod "))
        else {
            continue;
        };
        let name = rest.trim_end_matches(';').trim();
        if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            found.insert(name.to_owned());
        }
    }
    assert!(
        found.contains("session"),
        "the facade's module list came back without `session`, so this guard is \
         reading nothing and every re-export would look local"
    );
    found
}

fn re_export_is_local(line: &str) -> bool {
    let Some(path) = line.trim().strip_prefix("pub use ") else {
        return true;
    };
    let head = path
        .trim_start_matches("::")
        .split([':', ' ', '{', ';'])
        .next()
        .unwrap_or_default();
    // Derived, not listed. This used to enumerate the modules that happened to
    // exist -- `error` and `session` -- so adding a module to the facade made
    // the guard fail for republishing something it owns. A hand-written list of
    // your own modules is a list that goes stale by construction; the `mod`
    // declarations in the facade are the same fact, already written down.
    matches!(head, "crate" | "self" | "super") || facade_modules().contains(head)
}

#[test]
fn qyro_session_re_exports_nothing_it_does_not_own() {
    let facade = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../qyro_session/src/lib.rs"
    ))
    .expect("the facade is readable");

    let offenders: Vec<&str> = facade
        .lines()
        .filter(|line| line.trim_start().starts_with("pub use "))
        .filter(|line| !re_export_is_local(line))
        .collect();

    assert!(
        offenders.is_empty(),
        "qyro_session re-exports something from another crate: {offenders:?}. \
         Every item it republishes becomes nameable from qyro_ffi, which is the \
         surface guard 1 bounds."
    );
    assert!(
        facade.contains("pub use "),
        "the facade re-exports nothing at all, so this guard is passing \
         vacuously rather than checking anything"
    );
}

// --------------------------------- guard 3: a changelog, and it says it is one

/// The closure, frozen so a change is noticed — **not** so it is prevented.
///
/// ADR-0032 §3.3. This assertion prevents nothing: it is byte-identical with and
/// without a direct `qyro_ffi -> qyro_crypto` edge, which
/// `a_direct_crypto_edge_is_invisible_here_and_visible_to_guard_one` measures
/// rather than claims. It is kept because a set that changes without anyone
/// noticing is how a dependency arrives unexamined, and it is documented as a
/// changelog so nobody mistakes it for the guard it replaced.
const CLOSURE: [&str; 52] = [
    "aead",
    "block-buffer",
    "cfg-if",
    "chacha20",
    "chacha20poly1305",
    "cipher",
    "cmov",
    "cpufeatures",
    "crypto-common",
    "ctutils",
    "curve25519-dalek",
    "curve25519-dalek-derive",
    "digest",
    "ed25519",
    "ed25519-dalek",
    "fiat-crypto",
    "getrandom",
    "hkdf",
    "hmac",
    "hybrid-array",
    "inout",
    "libc",
    "poly1305",
    "proc-macro2",
    "quote",
    "qyro_core",
    "qyro_crypto",
    "qyro_ffi",
    "qyro_fs",
    "qyro_identity_store",
    "qyro_manifest",
    "qyro_net",
    "qyro_protocol",
    "qyro_session",
    "qyro_transfer",
    "r-efi",
    "rand_core",
    "rustc_version",
    "semver",
    "sha2",
    "signature",
    "subtle",
    "syn",
    "tinyvec",
    "tinyvec_macros",
    "typenum",
    "unicode-ident",
    "unicode-normalization",
    "universal-hash",
    "x25519-dalek",
    "zeroize",
    "zeroize_derive",
];

#[test]
fn the_dependency_closure_matches_its_changelog() {
    let closure = dependency_closure(&workspace_metadata(), "qyro_ffi");
    let expected: BTreeSet<String> = CLOSURE.into_iter().map(str::to_owned).collect();
    assert_eq!(
        closure, expected,
        "the set of crates linked beneath the FFI changed. This is a changelog, \
         not a guard: update it deliberately, and if a crate arrived that nobody \
         chose, that is the thing to look at."
    );
}

// ------------------------------------------------- the guards, seen to fail

#[test]
fn a_direct_crypto_edge_is_invisible_here_and_visible_to_guard_one() {
    // The one-line accident this whole design is arranged around: somebody adds
    // `qyro_crypto = { path = "../qyro_crypto" }` to qyro_ffi to unblock
    // themselves, and `qyro_crypto::DeviceIdentity::generate()?.export_secret()`
    // compiles inside the library Dart loads.
    //
    // The edge is spliced into real resolver output, so this is the actual graph
    // differing from reality by exactly the one line under test.
    let mut metadata = workspace_metadata();
    let crypto = id_of(&metadata, "qyro_crypto");
    let ffi = id_of(&metadata, "qyro_ffi");

    let before = dependency_closure(&metadata, "qyro_ffi");

    metadata["resolve"]["nodes"]
        .as_array_mut()
        .expect("metadata resolves a graph")
        .iter_mut()
        .find(|node| node["id"].as_str() == Some(ffi.as_str()))
        .expect("the resolver has a node for qyro_ffi")["deps"]
        .as_array_mut()
        .expect("the node lists its deps")
        .push(serde_json::json!({
            "name": "qyro_crypto",
            "pkg": crypto,
            "dep_kinds": [{ "kind": null, "target": null }],
        }));

    // 1. The closure genuinely cannot tell the difference. Measured, not
    //    asserted from the ADR: if this ever stops holding, the argument for
    //    moving the guard to depth one has changed and this says so before
    //    anyone reads the comment above and trusts it.
    let after = dependency_closure(&metadata, "qyro_ffi");
    assert_eq!(
        before, after,
        "the closure is supposed to be blind to this edge; if it can now see \
         it, ADR-0032 §3.3 needs rewriting"
    );

    // 2. And guard 1 is not blind to it, and names it.
    let direct = ffi_direct_dependencies(&metadata);
    assert!(
        direct.contains("qyro_crypto"),
        "the spliced edge did not land where guard 1 reads"
    );
    let expected: BTreeSet<String> = ["qyro_core", "qyro_session"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_ne!(
        direct, expected,
        "a direct qyro_ffi -> qyro_crypto edge must break the depth-one \
         assertion, or that assertion is a comment"
    );
}

#[test]
fn a_foreign_re_export_from_qyro_session_would_be_visible_to_guard_two() {
    // The cheapest accident under this design: "I just need one type from
    // crypto." It passes guard 1 -- qyro_ffi still names only two crates -- and
    // passes the changelog, because no package moved.
    assert!(
        !re_export_is_local("pub use qyro_crypto as crypto;"),
        "a re-export of another crate must be refused"
    );

    // And the shape with no crypto name in it at all: qyro_net::Session carries
    // into_parts() -> (.., FrameSealer, FrameOpener) as an inherent method,
    // reachable by inference without qyro_crypto appearing anywhere a reader
    // looks.
    assert!(
        !re_export_is_local("pub use qyro_net::Session;"),
        "a re-export whose methods hand out crypto types is the same leak with \
         none of the type names in it"
    );

    // The positive control. Without it, a checker that refused everything would
    // pass both assertions above and prove nothing.
    assert!(
        re_export_is_local("pub use error::SessionError;"),
        "a re-export of this crate's own item must be allowed, or the guard is \
         refusing everything"
    );
    assert!(
        re_export_is_local("pub use crate::session::Session;"),
        "and the crate:: form too"
    );
}
