#!/usr/bin/env bash
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
plan=0
blockers=0

report() {
  printf '[%s] %s: %s\n' "$1" "$2" "$3"
}

usage() {
  printf 'Usage: %s [--repo-root PATH] [--plan]\n' "$0"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-root)
      if [[ $# -lt 2 ]]; then
        usage >&2
        exit 2
      fi
      repo_root="$2"
      shift 2
      ;;
    --plan)
      plan=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      printf 'Unknown argument: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ ! -d "$repo_root" ]]; then
  report "BLOCKER" "Repository" "$repo_root does not exist"
  exit 1
fi
repo_root="$(cd "$repo_root" && pwd)"

if [[ $plan -eq 1 ]]; then
  report "N/A" "Rust quality" "plan: format, Clippy, workspace tests"
  report "N/A" "Flutter quality" "plan: dependencies, format, analyze, tests"
  report "N/A" "Native tests" "plan: run when dedicated suites exist"
  report "N/A" "License audit" "plan: validate the reviewed dependency ledger"
  report "N/A" "Security audit" "plan: cargo-audit when installed"
  report "N/A" "Protocol vectors" "plan: run when a vector corpus exists"
  report "OK" "Test summary" "test plan is valid"
  exit 0
fi

run_step() {
  local label="$1"
  shift
  if "$@"; then
    report "OK" "$label" "passed"
  else
    report "BLOCKER" "$label" "failed"
    blockers=$((blockers + 1))
  fi
}

if [[ -f "$repo_root/Cargo.toml" ]]; then
  run_step "Rust quality" bash -c     'cd "$1" && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace'     _ "$repo_root"
else
  report "BLOCKER" "Rust quality" "Cargo.toml not found"
  blockers=$((blockers + 1))
fi

flutter_root="$repo_root/apps/qyro"
if [[ -f "$flutter_root/pubspec.yaml" ]]; then
  run_step "Flutter quality" bash -c     'cd "$1" && flutter pub get && dart format --output=none --set-exit-if-changed . && flutter analyze && flutter test'     _ "$flutter_root"
else
  report "BLOCKER" "Flutter quality" "apps/qyro/pubspec.yaml not found"
  blockers=$((blockers + 1))
fi

report "N/A" "Native tests" "no dedicated instrumentation, XCTest, or Windows test suite is configured"

license_ledger="$repo_root/docs/LICENSE_AUDIT.md"
if [[ ! -f "$license_ledger" ]]; then
  report "BLOCKER" "License audit" "docs/LICENSE_AUDIT.md not found"
  blockers=$((blockers + 1))
elif ! grep -q '^| Dependencia |' "$license_ledger"; then
  report "BLOCKER" "License audit" "dependency ledger table is missing"
  blockers=$((blockers + 1))
elif grep -Eiq '^\|.*\|[[:space:]]*(GPL|AGPL|LGPL|MPL|UNKNOWN|DESCONOCIDA)[^|]*\|' "$license_ledger"; then
  report "BLOCKER" "License audit" "review-required dependency found in the ledger"
  blockers=$((blockers + 1))
else
  report "OK" "License audit" "reviewed dependency ledger has no blocked entries"
fi

if command -v cargo-audit >/dev/null 2>&1; then
  run_step "Security audit" bash -c 'cd "$1" && cargo audit' _ "$repo_root"
else
  report "WARNING" "Security audit" "cargo-audit is not installed; advisory scan was not executed"
fi

if [[ -d "$repo_root/tests/protocol_vectors" ]]; then
  run_step "Protocol vectors" bash -c 'cd "$1" && cargo test --workspace protocol_vector' _ "$repo_root"
else
  report "N/A" "Protocol vectors" "vector corpus is not implemented; QYRO/1 unit contract ran with Rust tests"
fi

if [[ $blockers -gt 0 ]]; then
  report "BLOCKER" "Test summary" "$blockers required suite(s) failed"
  exit 1
fi

report "OK" "Test summary" "all available required suites passed"
