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
- Verified commit: 7ca3973cd1928ffaa3e7b112d121587d83d5092c
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
range_refs="$(mktemp -d)"; concrete_finding="$(mktemp -d)"
at_open_limit="$(mktemp -d)"; too_many_open="$(mktemp -d)"
out_of_scope="$(mktemp -d)"
trap 'rm -rf "$valid" "$missing" "$stale" "$scripts_pending" "$false_claim" "$range_refs" "$concrete_finding" "$at_open_limit" "$too_many_open" "$out_of_scope"' EXIT

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

# Reserved ranges describe ownership, not findings. Their endpoints must not
# force agents to create placeholder ledger records outside their allocation.
make_fixture "$range_refs"
printf '## QYR-0001 — fixture\n\n- Estado: cerrado\n' > "$range_refs/BUGS_PENDING.md"
printf '\nReserved: QYR-0076–QYR-0099; this agent owns QYR-0100 onward.\n' >> "$range_refs/README.md"
output="$(bash "$checker" --repo-root "$range_refs")"
[[ "$output" == *"[OK] Documentation consistency"* ]]

# A concrete citation remains subject to the ledger rule.
make_fixture "$concrete_finding"
printf '## QYR-0001 — fixture\n\n- Estado: cerrado\n' > "$concrete_finding/BUGS_PENDING.md"
missing_id='QYR-''0101'
printf '\n%s is a concrete missing finding.\n' "$missing_id" >> "$concrete_finding/README.md"
assert_fails_with "$concrete_finding" "$missing_id is cited but has no entry"

# ...and only inside the five declared extensions. The twin of this case exists
# in the PowerShell half, where the scope was not enforced at all: `-Include`
# beside `-LiteralPath` is inert on Windows PowerShell 5.1, so that checker read
# `.txt`, `.o` and `.exe` as documentation and blocked on a citation this half
# could not see (QYR-0311). Asserted here too because the two halves are only
# equivalent for as long as something checks that they are.
#
# Both directions on purpose: the same citation out of scope must pass and in
# scope must fail. A checker that ignores extensions fails the first; a checker
# that scans nothing passes the first and fails the second.
make_fixture "$out_of_scope"
printf '## QYR-0001 — fixture\n\n- Estado: cerrado\n' > "$out_of_scope/BUGS_PENDING.md"
out_of_scope_id='QYR-''0102'
printf '\n%s is cited where the checker must not look.\n' "$out_of_scope_id" > "$out_of_scope/notes.txt"
output="$(bash "$checker" --repo-root "$out_of_scope")"
[[ "$output" == *"[OK] Documentation consistency"* ]]
printf '\n%s is cited where the checker must look.\n' "$out_of_scope_id" >> "$out_of_scope/README.md"
assert_fails_with "$out_of_scope" "$out_of_scope_id is cited but has no entry"

# The ledger is an operational list, not an unbounded tool-output sink. Sixty
# simultaneously open findings make the instrument fail its explicit ceiling.
make_fixture "$at_open_limit"
for index in $(seq 1 59); do
  printf '## QYR-%04d — human-readable fixture %d\n\n- Estado: abierto\n\n' \
    "$index" "$index" >> "$at_open_limit/BUGS_PENDING.md"
done
output="$(bash "$checker" --repo-root "$at_open_limit")"
[[ "$output" == *"[OK] Documentation consistency"* ]]

make_fixture "$too_many_open"
for index in $(seq 1 60); do
  printf '## QYR-%04d — human-readable fixture %d\n\n- Estado: abierto\n\n' \
    "$index" "$index" >> "$too_many_open/BUGS_PENDING.md"
done
assert_fails_with "$too_many_open" "60 open findings exceed the ledger limit of 59"

# STATUS.md drifted 58 commits behind audit/baseline-hardening without any check
# noticing, because only the field layout was validated. These fixtures pin the
# freshness rule: the verified commit must be reachable from HEAD and close to it.
make_git_fixture() {
  local root="$1"
  make_fixture "$root"
  git -C "$root" init --quiet --initial-branch=main
  git -C "$root" config user.email "contract@qyro.test"
  git -C "$root" config user.name "Qyro Contract"
  git -C "$root" add -A
  git -C "$root" commit --quiet -m "chore: fixture baseline"
}

set_verified_commit() {
  local root="$1" value="$2"
  local escaped=${value//\//\\/}
  sed -i "s/^- Verified commit:.*/- Verified commit: $escaped/" "$root/STATUS.md"
}

fresh="$(mktemp -d)"; drifted="$(mktemp -d)"
unreachable="$(mktemp -d)"; malformed="$(mktemp -d)"
trap 'rm -rf "$valid" "$missing" "$stale" "$scripts_pending" "$false_claim" "$range_refs" "$concrete_finding" "$at_open_limit" "$too_many_open" "$fresh" "$drifted" "$unreachable" "$malformed"' EXIT

# A commit recorded one revision back is normal: STATUS cannot contain the SHA of
# the very commit that introduces it.
make_git_fixture "$fresh"
set_verified_commit "$fresh" "$(git -C "$fresh" rev-parse HEAD)"
printf '\nfollow-up\n' >> "$fresh/README.md"
git -C "$fresh" commit --quiet -am "docs: follow-up"
output="$(bash "$checker" --repo-root "$fresh")"
[[ "$output" == *"[OK] Documentation consistency"* ]]

make_git_fixture "$drifted"
set_verified_commit "$drifted" "$(git -C "$drifted" rev-parse HEAD)"
for index in $(seq 1 12); do
  printf 'change %s\n' "$index" >> "$drifted/README.md"
  git -C "$drifted" commit --quiet -am "chore: change $index"
done
assert_fails_with "$drifted" "[BLOCKER] Stale verified commit"

make_git_fixture "$unreachable"
set_verified_commit "$unreachable" "0123456789abcdef0123456789abcdef01234567"
assert_fails_with "$unreachable" "[BLOCKER] Unknown verified commit"

make_git_fixture "$malformed"
set_verified_commit "$malformed" "not-a-sha"
assert_fails_with "$malformed" "[BLOCKER] Malformed verified commit"
