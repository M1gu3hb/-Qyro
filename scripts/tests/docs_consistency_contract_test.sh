#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
checker="$repo_root/scripts/check_docs_consistency.sh"
if [[ ! -f "$checker" ]]; then echo "Expected $checker to exist." >&2; exit 1; fi

make_fixture() {
  local root="$1"
  mkdir -p "$root/scripts" "$root/config"
  touch "$root/scripts/doctor.sh" "$root/scripts/bootstrap.sh" "$root/scripts/test_all.sh"
  cat > "$root/STATUS.md" <<'EOF'
# Canonical project status
- Updated UTC: 2026-08-04T19:55:00Z
- Branch: audit/baseline-hardening
- Verified commit: CURRENT_HEAD
- Milestone: Hito 0 cerrado; Hito 1 en hardening
## Implemented
- Native bridge: IMPLEMENTED
## Not implemented
- File transfer: NOT_IMPLEMENTED
## Platforms compiled
- Android, iOS, Windows
## Platforms executed
- Linux CI, Windows test
## Real tests
- Baseline CI
## Artifacts
- None retained yet
## Blockers
- Android runtime ABI
## Next task
- Android runtime ABI
## Provisional values
- REPLACE_WITH_OWNER
- com.owner.qyro
EOF
  for doc in AGENTS.md PROJECT_CONTEXT.md README.md HANDOFF.md TESTING.md; do
    printf '# Document\n\nCurrent state: see STATUS.md.\n' > "$root/$doc"
  done
  printf '{"owner":"REPLACE_WITH_OWNER"}\n' > "$root/config/branding.example.json"
}

assert_fails_with() {
  local root="$1" expected="$2"
  set +e
  local output
  output="$(bash "$checker" --repo-root "$root" 2>&1)"
  local code=$?
  set -e
  if [[ $code -eq 0 || "$output" != *"$expected"* ]]; then
    echo "Expected failure containing: $expected" >&2
    echo "$output" >&2
    exit 1
  fi
}

valid="$(mktemp -d)"; missing="$(mktemp -d)"; stale="$(mktemp -d)"
scripts_pending="$(mktemp -d)"; false_claim="$(mktemp -d)"
trap 'rm -rf "$valid" "$missing" "$stale" "$scripts_pending" "$false_claim"' EXIT

make_fixture "$valid"
output="$(bash "$checker" --repo-root "$valid")"
[[ "$output" == *"[OK] Documentation consistency"* ]]

make_fixture "$missing"
sed -i '/^- Milestone:/d' "$missing/STATUS.md"
assert_fails_with "$missing" "[BLOCKER] STATUS fields"

make_fixture "$stale"
printf '\nCommit actual: 0000000000000000000000000000000000000000\n' >> "$stale/README.md"
assert_fails_with "$stale" "[BLOCKER] Stale current commit"

make_fixture "$scripts_pending"
printf '\ndoctor, bootstrap and test_all are pending\n' >> "$scripts_pending/AGENTS.md"
assert_fails_with "$scripts_pending" "[BLOCKER] AGENTS script state"

make_fixture "$false_claim"
printf '\nFile transfer: implemented\n' >> "$false_claim/README.md"
assert_fails_with "$false_claim" "[BLOCKER] Pending capability claim"
