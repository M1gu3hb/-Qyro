#!/usr/bin/env bash
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
blockers=0

report() {
  printf '[%s] %s: %s\n' "$1" "$2" "$3"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-root)
      [[ $# -ge 2 ]] || { report "BLOCKER" "Arguments" "--repo-root requires a path"; exit 2; }
      repo_root="$2"
      shift 2
      ;;
    --help|-h)
      printf 'Usage: %s [--repo-root PATH]\n' "$0"
      exit 0
      ;;
    *)
      report "BLOCKER" "Arguments" "unknown argument: $1"
      exit 2
      ;;
  esac
done

if [[ ! -d "$repo_root" ]]; then
  report "BLOCKER" "Repository" "$repo_root does not exist"
  exit 1
fi
repo_root="$(cd "$repo_root" && pwd)"
status_file="$repo_root/STATUS.md"

if [[ ! -f "$status_file" ]]; then
  report "BLOCKER" "STATUS fields" "STATUS.md is missing"
  exit 1
fi

required_patterns=(
  '^- Updated UTC:'
  '^- Branch:'
  '^- Verified commit:'
  '^- Milestone:'
  '^## Implemented$'
  '^## Not implemented$'
  '^## Platforms compiled$'
  '^## Platforms executed$'
  '^## Real tests$'
  '^## Artifacts$'
  '^## Blockers$'
  '^## Next task$'
  '^## Provisional values$'
)
missing_fields=()
for pattern in "${required_patterns[@]}"; do
  if ! grep -Eq "$pattern" "$status_file"; then
    missing_fields+=("$pattern")
  fi
done
if [[ ${#missing_fields[@]} -gt 0 ]]; then
  report "BLOCKER" "STATUS fields" "missing ${#missing_fields[@]} required field(s)"
  blockers=$((blockers + 1))
fi

canonical_docs=(AGENTS.md PROJECT_CONTEXT.md README.md HANDOFF.md TESTING.md)
for doc in "${canonical_docs[@]}"; do
  path="$repo_root/$doc"
  if [[ ! -f "$path" ]] || ! grep -Fq 'STATUS.md' "$path"; then
    report "BLOCKER" "Canonical reference" "$doc must point to STATUS.md"
    blockers=$((blockers + 1))
  fi
done

for doc in "${canonical_docs[@]}"; do
  path="$repo_root/$doc"
  [[ -f "$path" ]] || continue
  if grep -Eiq '(commit (actual|current|verificado|comprobado)|current commit)[^0-9a-f]*[0-9a-f]{40}' "$path"; then
    report "BLOCKER" "Stale current commit" "$doc declares a current commit outside STATUS.md"
    blockers=$((blockers + 1))
  fi
done

agents="$repo_root/AGENTS.md"
if [[ -f "$agents" ]] && [[ -f "$repo_root/scripts/doctor.sh" ]] && [[ -f "$repo_root/scripts/bootstrap.sh" ]] && [[ -f "$repo_root/scripts/test_all.sh" ]]; then
  if grep -Eiq '((doctor|bootstrap|test_all).*(pending|pendiente)|(pending|pendiente).*(doctor|bootstrap|test_all))' "$agents"; then
    report "BLOCKER" "AGENTS script state" "existing scripts are described as pending"
    blockers=$((blockers + 1))
  fi
fi

for doc in PROJECT_CONTEXT.md README.md HANDOFF.md TESTING.md; do
  path="$repo_root/$doc"
  [[ -f "$path" ]] || continue
  if grep -Eiq '(file transfer|transferencia de archivos)[[:space:]]*:[[:space:]]*(implemented|complete|ready|implementada|completa|lista)' "$path"; then
    report "BLOCKER" "Pending capability claim" "$doc marks file transfer implemented"
    blockers=$((blockers + 1))
  fi
done

if grep -Rqs --exclude-dir=.git --exclude-dir=build --exclude-dir=target 'REPLACE_WITH_' "$repo_root"; then
  if ! grep -Fq 'REPLACE_WITH_' "$status_file"; then
    report "BLOCKER" "Provisional markers" "REPLACE_WITH_* exists but is absent from STATUS.md"
    blockers=$((blockers + 1))
  fi
fi
if grep -Rqs --exclude-dir=.git --exclude-dir=build --exclude-dir=target 'com.owner.qyro' "$repo_root"; then
  if ! grep -Fq 'com.owner.qyro' "$status_file"; then
    report "BLOCKER" "Provisional markers" "com.owner.qyro exists but is absent from STATUS.md"
    blockers=$((blockers + 1))
  fi
fi

if [[ $blockers -gt 0 ]]; then
  report "BLOCKER" "Documentation consistency" "$blockers inconsistency finding(s)"
  exit 1
fi
report "OK" "Documentation consistency" "STATUS.md and canonical references agree"
