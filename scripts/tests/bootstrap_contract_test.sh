#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
bootstrap="$repo_root/scripts/bootstrap.sh"

if [[ ! -f "$bootstrap" ]]; then
  echo "Expected $bootstrap to exist." >&2
  exit 1
fi

workspace="$(mktemp -d)"
trap 'rm -rf "$workspace"' EXIT
mkdir -p "$workspace/config" "$workspace/apps/qyro/assets/brand"
printf '{"app":"example"}\n' > "$workspace/config/branding.example.json"
printf '{"feature":false}\n' > "$workspace/config/features.example.json"
printf 'asset' > "$workspace/apps/qyro/assets/brand/qyro-logo.png"

output="$(bash "$bootstrap" --repo-root "$workspace" --skip-dependencies)"

[[ -f "$workspace/config/branding.json" ]]
[[ -f "$workspace/config/features.json" ]]
cmp "$workspace/config/branding.example.json" "$workspace/config/branding.json"
cmp "$workspace/config/features.example.json" "$workspace/config/features.json"

for expected in   "[N/A] Rust dependencies"   "[N/A] Flutter dependencies"   "[OK] Branding config"   "[OK] Feature config"   "[OK] Brand assets"   "[N/A] FFI bindings"   "[N/A] Code generation"; do
  if [[ "$output" != *"$expected"* ]]; then
    echo "Expected output to contain: $expected" >&2
    echo "$output" >&2
    exit 1
  fi
done

printf '{"app":"custom"}\n' > "$workspace/config/branding.json"
bash "$bootstrap" --repo-root "$workspace" --skip-dependencies >/dev/null
if [[ "$(cat "$workspace/config/branding.json")" != '{"app":"custom"}' ]]; then
  echo "bootstrap.sh overwrote a user configuration." >&2
  exit 1
fi
