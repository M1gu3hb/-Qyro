#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_all="$repo_root/scripts/test_all.sh"

if [[ ! -f "$test_all" ]]; then
  echo "Expected $test_all to exist." >&2
  exit 1
fi

output="$(bash "$test_all" --repo-root "$repo_root" --plan)"
for expected in   "[N/A] Rust quality"   "[N/A] Flutter quality"   "[N/A] Native tests"   "[N/A] License audit"   "[N/A] Security audit"   "[N/A] Protocol vectors"   "[OK] Test summary"; do
  if [[ "$output" != *"$expected"* ]]; then
    echo "Expected output to contain: $expected" >&2
    echo "$output" >&2
    exit 1
  fi
done

set +e
missing_output="$(bash "$test_all" --repo-root "$repo_root/does-not-exist" --plan 2>&1)"
missing_exit=$?
set -e
if [[ $missing_exit -eq 0 || "$missing_output" != *"[BLOCKER] Repository"* ]]; then
  echo "Missing repository roots must be blockers." >&2
  exit 1
fi
