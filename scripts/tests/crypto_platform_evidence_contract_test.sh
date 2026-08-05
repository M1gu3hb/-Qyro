#!/usr/bin/env bash
# Contract for the checker that refuses "qyro_ffi built for Android" as evidence
# that qyro_crypto works on Android.
#
# Run against fixtures rather than the repository alone, so the checker is shown
# to reject each specific omission instead of passing for an unrelated reason.
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
checker="$repo_root/scripts/check_crypto_platform_evidence.sh"
if [[ ! -f "$checker" ]]; then echo "Expected $checker to exist." >&2; exit 1; fi

failures=0

# A fixture repository that satisfies every rule, so each assertion below can
# remove exactly one thing and show the checker noticing that one thing.
make_fixture() {
  local root="$1"
  mkdir -p "$root/.github/workflows"
  mkdir -p "$root/rust/tools/qyro_crypto_smoke"
  mkdir -p "$root/rust/crates/qyro_ffi" "$root/rust/crates/qyro_core"
  mkdir -p "$root/scripts"

  cp "$checker" "$root/scripts/check_crypto_platform_evidence.sh"

  printf 'publish = false\n' > "$root/rust/tools/qyro_crypto_smoke/Cargo.toml"
  printf '[dependencies]\nqyro_core = { path = "../qyro_core" }\n' \
    > "$root/rust/crates/qyro_ffi/Cargo.toml"
  printf '[dependencies]\n' > "$root/rust/crates/qyro_core/Cargo.toml"

  cat > "$root/.github/workflows/crypto-platform.yml" <<'WORKFLOW'
name: Crypto platform
jobs:
  linux-crypto:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test --locked --package qyro_crypto
      - run: cargo run --package qyro_crypto_smoke
  windows-crypto:
    runs-on: windows-latest
    steps:
      - run: cargo test --locked --package qyro_crypto
      - run: cargo run --package qyro_crypto_smoke
  android-crypto:
    runs-on: ubuntu-latest
    steps:
      - run: cargo build --locked --package qyro_crypto --target x86_64-linux-android
      - run: cargo build --locked --package qyro_crypto --target aarch64-linux-android
      - run: adb push qyro_crypto_smoke /data/local/tmp/
  ios-crypto:
    runs-on: macos-latest
    steps:
      - run: cargo build --locked --package qyro_crypto --target aarch64-apple-ios
      - run: cargo build --locked --package qyro_crypto --target aarch64-apple-ios-sim
      - run: xcodebuild test -scheme CryptoSmoke
WORKFLOW
}

run_checker() {
  local root="$1"
  set +e
  local output status
  output="$(bash "$root/scripts/check_crypto_platform_evidence.sh" 2>&1)"
  status=$?
  set -e
  printf '%s\n' "$output"
  return "$status"
}

assert_rejects() {
  local description="$1"
  shift
  local root
  root="$(mktemp -d)"
  make_fixture "$root"
  # The mutation runs with the fixture root as $1.
  "$@" "$root"

  set +e
  local output status
  output="$(run_checker "$root")"
  status=$?
  set -e

  if [[ "$status" -eq 0 ]]; then
    echo "FAIL: $description must be rejected, but the checker passed" >&2
    failures=$((failures + 1))
  else
    echo "ok: rejects $description"
  fi
  rm -rf "$root"
}

assert_accepts_fixture() {
  local root
  root="$(mktemp -d)"
  make_fixture "$root"

  set +e
  local output status
  output="$(run_checker "$root")"
  status=$?
  set -e

  if [[ "$status" -ne 0 ]]; then
    echo "FAIL: the complete fixture must be accepted, or every rejection above proves nothing" >&2
    echo "$output" >&2
    failures=$((failures + 1))
  else
    echo "ok: accepts a complete fixture"
  fi
  rm -rf "$root"
}

# The fixture is valid before any mutation. Without this, a checker that
# rejected everything would pass every assertion below.
assert_accepts_fixture

drop_workflow()      { rm -f "$1/.github/workflows/crypto-platform.yml"; }
drop_job()           { sed -i '/^  windows-crypto:/d' "$1/.github/workflows/crypto-platform.yml"; }
drop_android_arm64() { sed -i '/aarch64-linux-android/d' "$1/.github/workflows/crypto-platform.yml"; }
drop_ios_sim()       { sed -i '/aarch64-apple-ios-sim/d' "$1/.github/workflows/crypto-platform.yml"; }
drop_smoke_run()     { sed -i '/qyro_crypto_smoke/d' "$1/.github/workflows/crypto-platform.yml"; }
drop_emulator()      { sed -i '/adb push/d' "$1/.github/workflows/crypto-platform.yml"; }
drop_simulator()     { sed -i '/xcodebuild test/d' "$1/.github/workflows/crypto-platform.yml"; }
drop_harness()       { rm -rf "$1/rust/tools/qyro_crypto_smoke"; }
publish_harness()    { printf 'publish = true\n' > "$1/rust/tools/qyro_crypto_smoke/Cargo.toml"; }
leak_crypto_to_ffi() {
  printf '[dependencies]\nqyro_crypto = { path = "../qyro_crypto" }\n' \
    > "$1/rust/crates/qyro_ffi/Cargo.toml"
}
# The exact substitution this checker exists to refuse: building qyro_ffi for a
# target and calling it crypto evidence.
substitute_ffi_for_crypto() {
  sed -i 's/--package qyro_crypto --target/--package qyro_ffi --target/' \
    "$1/.github/workflows/crypto-platform.yml"
}

assert_rejects "a missing crypto workflow"                drop_workflow
assert_rejects "a missing per-platform job"               drop_job
assert_rejects "Android arm64 never being built"          drop_android_arm64
assert_rejects "the iOS simulator target never being built" drop_ios_sim
assert_rejects "the smoke never being executed"           drop_smoke_run
assert_rejects "no emulator execution path"               drop_emulator
assert_rejects "no simulator execution path"              drop_simulator
assert_rejects "a missing harness"                        drop_harness
assert_rejects "a publishable harness"                    publish_harness
assert_rejects "qyro_ffi reaching qyro_crypto"            leak_crypto_to_ffi
assert_rejects "qyro_ffi builds passed off as crypto evidence" substitute_ffi_for_crypto

# And the real repository must pass, which is the point of the whole sprint.
if ! bash "$checker" >/dev/null 2>&1; then
  echo "FAIL: the repository does not prove qyro_crypto on its own platforms" >&2
  bash "$checker" >&2
  failures=$((failures + 1))
else
  echo "ok: the repository proves qyro_crypto on every product platform"
fi

if [[ "$failures" -gt 0 ]]; then
  echo "$failures contract failure(s)." >&2
  exit 1
fi
echo "All crypto platform evidence contracts hold."
