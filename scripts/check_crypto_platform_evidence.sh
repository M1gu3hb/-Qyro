#!/usr/bin/env bash
# Rejects a workflow set that claims platform coverage it does not have.
#
# The four workflows that existed before this check all build and run
# `qyro_ffi`. None of them built `qyro_crypto` for Android, iOS or Windows, and
# `qyro_ffi` deliberately does not depend on `qyro_crypto` — so a green Platform
# builds run was evidence about the minimal ABI and nothing at all about the
# handshake or the AEAD on those targets.
#
# Building qyro_ffi for a target is not building qyro_crypto for it. This script
# exists because that sentence is easy to agree with and easy to forget.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

workflows=".github/workflows"
crypto_workflow="$workflows/crypto-platform.yml"
status=0

fail() {
    printf '[FAIL] %s\n' "$1" >&2
    status=1
}

require_workflow_pattern() {
    local file="$1"
    local pattern="$2"
    local description="$3"

    if [[ ! -f "$file" ]]; then
        fail "$description: $file does not exist"
        return
    fi
    if ! grep -Eq -- "$pattern" "$file"; then
        fail "$description: $file does not match /$pattern/"
    fi
}

# --- the dedicated workflow must exist at all --------------------------------

if [[ ! -f "$crypto_workflow" ]]; then
    fail "no workflow builds or runs qyro_crypto on a target platform: $crypto_workflow is missing"
    printf '[FAIL] Crypto platform evidence: %s\n' \
        "the existing workflows cover qyro_ffi only" >&2
    exit 1
fi

# --- one job per platform ----------------------------------------------------

for job in linux-crypto windows-crypto android-crypto ios-crypto; do
    require_workflow_pattern "$crypto_workflow" "^  $job:" "job $job"
done

# --- qyro_crypto itself must be built for every target -----------------------
#
# The package name is checked together with the target, so building qyro_ffi for
# the same triple cannot satisfy the rule.

require_qyro_crypto_target() {
    local target="$1"
    if ! grep -Eq -- "--package qyro_crypto( .*)? --target $target" "$crypto_workflow"; then
        fail "qyro_crypto is not built for $target"
    fi
}

require_qyro_crypto_target "x86_64-linux-android"
require_qyro_crypto_target "aarch64-linux-android"
require_qyro_crypto_target "aarch64-apple-ios"
require_qyro_crypto_target "aarch64-apple-ios-sim"

# --- and its tests must actually run where a host can run them ---------------

require_workflow_pattern "$crypto_workflow" \
    "cargo test( .*)? --package qyro_crypto" \
    "qyro_crypto unit tests"

require_workflow_pattern "$crypto_workflow" "runs-on: windows" \
    "a Windows runner"
require_workflow_pattern "$crypto_workflow" "runs-on: macos" \
    "a macOS runner"

# --- the smoke has to run, not just compile ----------------------------------
#
# Compiling proves the toolchain accepts the code. It does not prove X25519,
# Ed25519, HKDF, ChaCha20-Poly1305 and the replay window behave on that
# platform's word size, endianness and CPU features.

for platform in linux windows android ios; do
    if ! grep -Eq -- "qyro_crypto_smoke" "$crypto_workflow"; then
        fail "the crypto smoke harness is never executed ($platform)"
        break
    fi
done

require_workflow_pattern "$crypto_workflow" "adb" \
    "an Android emulator execution path"
require_workflow_pattern "$crypto_workflow" "xcodebuild test" \
    "an iOS simulator execution path"

# --- the harness must exist and stay out of the product ----------------------

if [[ ! -d rust/tools/qyro_crypto_smoke ]]; then
    fail "the isolated crypto smoke harness does not exist"
elif ! grep -q '^publish = false' rust/tools/qyro_crypto_smoke/Cargo.toml; then
    fail "the crypto smoke harness must set publish = false"
fi

# A comment naming the crate is not a dependency on it. This used to grep the
# whole manifest, so writing `qyro_crypto` in a comment failed the check and the
# only way to keep it green was to avoid saying the word — which is the opposite
# of what a manifest comment is for. Comment lines are dropped first
# (sprint 4C.2, QYR-0031). A `[target.'cfg(...)'.dependencies]` entry is still
# caught: it is not a comment.
for manifest in rust/crates/qyro_ffi/Cargo.toml rust/crates/qyro_core/Cargo.toml; do
    if grep -v '^[[:space:]]*#' "$manifest" | grep -q 'qyro_crypto'; then
        fail "$manifest reaches qyro_crypto; the FFI boundary must stay crypto-free"
    fi
done

if [[ "$status" -ne 0 ]]; then
    printf '[FAIL] Crypto platform evidence: qyro_crypto is not proven on the product platforms\n' >&2
    exit 1
fi

printf '[OK] Crypto platform evidence: qyro_crypto is built and executed on every product platform\n'
