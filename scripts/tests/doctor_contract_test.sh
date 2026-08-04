#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
doctor="$repo_root/scripts/doctor.sh"

if [[ ! -f "$doctor" ]]; then
  echo "Expected $doctor to exist." >&2
  exit 1
fi

assert_contains() {
  local output="$1"
  local expected="$2"
  if [[ "$output" != *"$expected"* ]]; then
    echo "Expected output to contain: $expected" >&2
    echo "$output" >&2
    exit 1
  fi
}

output="$(bash "$doctor")"
assert_contains "$output" "[OK] Git"
assert_contains "$output" "[OK] Flutter"
assert_contains "$output" "[OK] Dart"
assert_contains "$output" "[OK] Rust"
assert_contains "$output" "[OK] Cargo"
assert_contains "$output" "[N/A] Xcode"
assert_contains "$output" "[N/A] Visual Studio Build Tools"

warning_output="$(QYRO_DOCTOR_SIMULATE_MISSING=fvm bash "$doctor")"
assert_contains "$warning_output" "[WARNING] FVM"

set +e
blocker_output="$(QYRO_DOCTOR_SIMULATE_MISSING=git bash "$doctor" 2>&1)"
blocker_exit=$?
set -e

if [[ $blocker_exit -eq 0 ]]; then
  echo "Expected a required missing tool to return a non-zero exit code." >&2
  exit 1
fi
assert_contains "$blocker_output" "[BLOCKER] Git"
