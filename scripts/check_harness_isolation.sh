#!/usr/bin/env bash
# Keeps the crypto smoke harness out of the product.
#
# The harness exists so Android, iOS and Windows have real evidence about
# `qyro_crypto`. It links the crate that holds keys, and it exports a symbol
# whose whole job is to run a handshake. Shipping it would mean buying a test
# with an attack surface, which is a bad trade at any price.
#
# Two halves. This script checks what can be checked from the source tree, on
# every platform, in CI and locally. The artifacts themselves — the APK, the
# Runner.app and the Windows ZIP — are searched for the exported symbol inside
# `platform-builds.yml`, where they exist. Neither half is sufficient: a source
# tree can be clean while a build script copies the library in, and an artifact
# check only runs where that artifact is produced.
#
# See docs/adr/ADR-0023-crypto-platform-test-harness.md.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

status=0
symbol="qyro_crypto_smoke_run"
harness="qyro_crypto_smoke"

fail() {
    printf '[FAIL] %s\n' "$1" >&2
    status=1
}

# --- nothing the product builds may name the harness -------------------------

for manifest in \
    rust/crates/qyro_ffi/Cargo.toml \
    rust/crates/qyro_core/Cargo.toml \
    rust/crates/qyro_crypto/Cargo.toml \
    rust/crates/qyro_protocol/Cargo.toml \
    rust/crates/qyro_manifest/Cargo.toml; do
    if grep -q "$harness" "$manifest"; then
        fail "$manifest depends on the test harness"
    fi
done

# The FFI boundary itself, checked here too so this script alone answers "can
# Dart reach a key?" without anyone having to also run the Rust test suite.
# A comment naming the crate is not a dependency on it. This used to grep the
# whole manifest, so writing `qyro_crypto` in a comment failed the check and the
# only way to keep it green was to avoid saying the word — which is the opposite
# of what a manifest comment is for. Comment lines are dropped first
# (sprint 4C.2, QYR-0031). A `[target.'cfg(...)'.dependencies]` entry is still
# caught: it is not a comment.
for manifest in rust/crates/qyro_ffi/Cargo.toml rust/crates/qyro_core/Cargo.toml; do
    if grep -v '^[[:space:]]*#' "$manifest" | grep -q 'qyro_crypto'; then
        fail "$manifest reaches qyro_crypto; the library Dart loads must not"
    fi
done

# --- no application source may reference it ----------------------------------

if [[ -d apps/qyro ]]; then
    if grep -rIl --exclude-dir=build --exclude-dir=.dart_tool \
        -e "$symbol" -e "$harness" apps/qyro 2>/dev/null | grep -q .; then
        fail "the Flutter application references the test harness"
    fi
fi

# --- no staged native library may be the harness -----------------------------
#
# These directories are what the platform build copies into the APK and the iOS
# bundle. A file here goes into the product whatever any manifest says.

for staged in \
    apps/qyro/android/app/src/main/jniLibs \
    apps/qyro/ios/Native; do
    if [[ -d "$staged" ]]; then
        while IFS= read -r library; do
            case "$(basename "$library")" in
                *"$harness"*)
                    fail "$library stages the test harness into a product bundle"
                    ;;
            esac
            if grep -qa "$symbol" "$library" 2>/dev/null; then
                fail "$library contains $symbol"
            fi
        done < <(find "$staged" -type f 2>/dev/null)
    fi
done

# --- the harness must declare itself unshippable -----------------------------

harness_manifest="rust/tools/$harness/Cargo.toml"
if [[ ! -f "$harness_manifest" ]]; then
    fail "the harness manifest is missing"
elif ! grep -q '^publish = false' "$harness_manifest"; then
    fail "$harness_manifest must set publish = false"
fi

# --- and it must never be built in release for distribution ------------------
#
# Not a rule about the `--release` flag: a release *build* of the harness is
# fine on a runner. What must not exist is a workflow step that puts it into
# something a user downloads.

for workflow in .github/workflows/*.yml; do
    # Naming the harness is not the offence — `platform-builds.yml` names it
    # precisely to search the APK, the ZIP and Runner.app for it, which is the
    # opposite of shipping it. What matters is *building* it next to an upload.
    if grep -Eq -- "(--package|-p) $harness" "$workflow"         && grep -q 'upload-artifact' "$workflow"; then
        # crypto-platform.yml builds it and uploads JSON reports, never a
        # binary. Anything else needs a human to look.
        if [[ "$(basename "$workflow")" != "crypto-platform.yml" ]]; then
            fail "$workflow both builds the harness and uploads an artifact"
        fi
    fi
done

if [[ "$status" -ne 0 ]]; then
    printf '[FAIL] Harness isolation: the crypto smoke harness can reach the product\n' >&2
    exit 1
fi

printf '[OK] Harness isolation: the crypto smoke harness cannot reach the product\n'
