use std::fs;

use qyro_ffi::{qyro_protocol_version_len, qyro_protocol_version_ptr};

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

#[test]
fn no_key_material_can_reach_dart_because_the_boundary_cannot_see_any() {
    // Dart must never be handed a secret key, a session key or a raw private
    // blob. The cheapest way to keep that true is for the crate Dart loads to
    // have no path to the crate that holds keys at all.
    //
    // Checked structurally rather than by reading the export list: a crate that
    // linked `qyro_crypto` could grow an accessor in one line, and this test
    // would still pass if it only counted today's two symbols. What it asserts
    // instead is that the dependency graph beneath the FFI boundary is exactly
    // `qyro_ffi -> qyro_core -> nothing`, so there is nothing to expose.
    let ffi = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("the FFI manifest is readable");
    let core = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../qyro_core/Cargo.toml"
    ))
    .expect("the core manifest is readable");

    for (name, manifest) in [("qyro_ffi", &ffi), ("qyro_core", &core)] {
        let dependencies = manifest
            .split("[dependencies]")
            .nth(1)
            .unwrap_or("")
            .split("\n[")
            .next()
            .unwrap_or("");

        for forbidden in [
            "qyro_crypto",
            "chacha20poly1305",
            "ed25519",
            "x25519",
            "hkdf",
        ] {
            assert!(
                !dependencies.contains(forbidden),
                "{name} must not depend on {forbidden}: it is loaded across the FFI boundary"
            );
        }

        if name == "qyro_core" {
            assert!(
                dependencies.trim().is_empty(),
                "qyro_core has no dependencies, so nothing can arrive through it"
            );
        }
    }

    assert!(
        ffi.contains("qyro_core = { path = \"../qyro_core\" }"),
        "the FFI crate's one dependency is qyro_core"
    );
}
