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

/// Every crate reachable from `qyro_ffi` by a normal or build dependency, on
/// any target.
///
/// Asked of Cargo's own resolver rather than of the manifest text. The previous
/// version of this test split `Cargo.toml` on the string `"[dependencies]"` and
/// searched the next section, which meant a
/// `[target.'cfg(target_os = "android")'.dependencies]` table naming
/// `qyro_crypto` sat in a different section and was never looked at — while the
/// test called itself structural.
///
/// Dev-dependencies are excluded: they are not linked into the library Dart
/// loads. Build-dependencies are included, because a build script that could
/// reach the crate holding keys is worth knowing about even though it does not
/// link.
///
/// No `--filter-platform`, deliberately. The question is whether *any* build of
/// `qyro_ffi` can reach the crypto crate, so a dependency that only applies to
/// Android must count.
fn ffi_dependency_closure() -> BTreeSet<String> {
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

    let metadata: Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON");

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

    let root = packages
        .iter()
        .find(|package| package["name"] == "qyro_ffi")
        .and_then(|package| package["id"].as_str())
        .expect("qyro_ffi is a member of this workspace");

    let mut reached = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        let Some(name) = name_of.get(id) else {
            continue;
        };
        if !reached.insert((*name).to_owned()) {
            continue;
        }
        let Some(node) = node_of.get(id) else {
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
                pending.push(dependency);
            }
        }
    }
    reached
}

#[test]
fn the_ffi_dependency_closure_holds_no_crypto() {
    // Dart must never be handed a secret key, a session key or a raw private
    // blob. The cheapest way to keep that true is for the crate Dart loads to
    // have no path to the crate that holds keys at all — then there is nothing
    // to expose, and no export list anybody has to keep reading.
    let closure = ffi_dependency_closure();

    // Named separately from the exactness check below so the failure says what
    // arrived rather than only that something did.
    for forbidden in [
        "qyro_crypto",
        // Sprint 4D.1. The store depends on qyro_crypto and holds the only two
        // public paths that hand out a seed, so reaching it from the FFI
        // boundary would reach key material in two hops. Listed before the
        // platform crate exists so that adding it under any dependency table —
        // including [target.'cfg(windows)'.dependencies], which is exactly the
        // shape that slipped past a textual check in 4C.2 — fails here.
        "qyro_identity_store",
        "ed25519-dalek",
        "x25519-dalek",
        "curve25519-dalek",
        "chacha20poly1305",
        "chacha20",
        "hkdf",
        "hmac",
        "sha2",
        "subtle",
        "zeroize",
        "getrandom",
    ] {
        assert!(
            !closure.contains(forbidden),
            "{forbidden} is reachable from qyro_ffi. The FFI boundary is loaded \
             by Dart, and a crate that links the cryptographic stack can grow an \
             accessor to key material in one line. Closure: {closure:?}"
        );
    }

    // And exactly this, so a crate nobody thought to forbid cannot arrive
    // either. `qyro_core` is deliberately dependency-free, which is what makes
    // the closure two entries long.
    let expected: BTreeSet<String> = ["qyro_core", "qyro_ffi"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        closure, expected,
        "the dependency graph beneath the FFI boundary must stay exactly \
         qyro_ffi -> qyro_core -> nothing"
    );
}
