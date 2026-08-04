#!/usr/bin/env bash
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
skip_dependencies=0
blockers=0

report() {
  printf '[%s] %s: %s\n' "$1" "$2" "$3"
}

usage() {
  printf 'Usage: %s [--repo-root PATH] [--skip-dependencies]\n' "$0"
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
    --skip-dependencies)
      skip_dependencies=1
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

run_step() {
  local label="$1"
  shift
  if "$@"; then
    report "OK" "$label" "completed"
  else
    report "BLOCKER" "$label" "command failed"
    blockers=$((blockers + 1))
  fi
}

if [[ $skip_dependencies -eq 1 ]]; then
  report "N/A" "Rust dependencies" "skipped by request"
  report "N/A" "Flutter dependencies" "skipped by request"
else
  if [[ -f "$repo_root/Cargo.toml" ]]; then
    run_step "Rust dependencies" cargo fetch --manifest-path "$repo_root/Cargo.toml"
  else
    report "BLOCKER" "Rust dependencies" "Cargo.toml not found"
    blockers=$((blockers + 1))
  fi

  if [[ -f "$repo_root/apps/qyro/pubspec.yaml" ]]; then
    run_step "Flutter dependencies" bash -c 'cd "$1" && flutter pub get' _ "$repo_root/apps/qyro"
  else
    report "BLOCKER" "Flutter dependencies" "apps/qyro/pubspec.yaml not found"
    blockers=$((blockers + 1))
  fi
fi

copy_config_if_missing() {
  local label="$1"
  local example="$2"
  local target="$3"

  if [[ -f "$target" ]]; then
    report "OK" "$label" "preserved existing local configuration"
  elif [[ -f "$example" ]]; then
    cp "$example" "$target"
    report "OK" "$label" "created local configuration from example"
  else
    report "BLOCKER" "$label" "example file missing"
    blockers=$((blockers + 1))
  fi
}

mkdir -p "$repo_root/config"
copy_config_if_missing "Branding config" "$repo_root/config/branding.example.json" "$repo_root/config/branding.json"
copy_config_if_missing "Feature config" "$repo_root/config/features.example.json" "$repo_root/config/features.json"

if [[ -f "$repo_root/apps/qyro/assets/brand/qyro-logo.png" ]]; then
  report "OK" "Brand assets" "Qyro logo is ready"
else
  report "BLOCKER" "Brand assets" "apps/qyro/assets/brand/qyro-logo.png is missing"
  blockers=$((blockers + 1))
fi

if [[ -f "$repo_root/apps/qyro/ffigen.yaml" ]]; then
  run_step "FFI bindings" bash -c 'cd "$1" && dart run ffigen --config ffigen.yaml' _ "$repo_root/apps/qyro"
else
  report "N/A" "FFI bindings" "no ffigen configuration is present yet"
fi

pubspec="$repo_root/apps/qyro/pubspec.yaml"
if [[ -f "$pubspec" ]] && grep -Eq '^[[:space:]]*build_runner:' "$pubspec"; then
  run_step "Code generation" bash -c 'cd "$1" && dart run build_runner build --delete-conflicting-outputs' _ "$repo_root/apps/qyro"
else
  report "N/A" "Code generation" "no build_runner configuration is present"
fi

if [[ $blockers -gt 0 ]]; then
  report "BLOCKER" "Bootstrap summary" "$blockers step(s) failed"
  exit 1
fi

report "OK" "Bootstrap summary" "workspace is prepared"
