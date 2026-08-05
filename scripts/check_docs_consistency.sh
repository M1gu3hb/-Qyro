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

# STATUS.md is the canonical executable state, so a verified commit that no longer
# tracks HEAD silently invalidates every claim in it. STATUS cannot name the commit
# that introduces it, so a small lead is expected; a large one means real drift.
max_status_commit_lag="${QYRO_MAX_STATUS_COMMIT_LAG:-10}"
verified_commit="$(sed -n 's/^- Verified commit:[[:space:]]*//p' "$status_file" | head -n 1 | tr -d '[:space:]')"

if [[ -z "$verified_commit" ]]; then
  report "BLOCKER" "Malformed verified commit" "STATUS.md does not record a verified commit"
  blockers=$((blockers + 1))
elif [[ ! "$verified_commit" =~ ^[0-9a-f]{40}$ ]]; then
  report "BLOCKER" "Malformed verified commit" "$verified_commit is not a full 40-character SHA"
  blockers=$((blockers + 1))
elif ! git -C "$repo_root" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  report "SKIP" "Verified commit freshness" "$repo_root is not a Git work tree"
elif [[ "$(git -C "$repo_root" rev-parse --is-shallow-repository 2>/dev/null)" == "true" ]]; then
  report "SKIP" "Verified commit freshness" "shallow clone cannot prove reachability"
elif ! git -C "$repo_root" cat-file -e "${verified_commit}^{commit}" 2>/dev/null; then
  report "BLOCKER" "Unknown verified commit" "$verified_commit is not a commit in this repository"
  blockers=$((blockers + 1))
elif ! git -C "$repo_root" merge-base --is-ancestor "$verified_commit" HEAD 2>/dev/null; then
  report "BLOCKER" "Unknown verified commit" "$verified_commit is not reachable from HEAD"
  blockers=$((blockers + 1))
else
  lag="$(git -C "$repo_root" rev-list --count "${verified_commit}..HEAD" 2>/dev/null || echo 0)"
  if [[ "$lag" -gt "$max_status_commit_lag" ]]; then
    report "BLOCKER" "Stale verified commit" \
      "HEAD is $lag commits ahead of the verified commit (limit $max_status_commit_lag)"
    blockers=$((blockers + 1))
  fi
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

# ---------------------------------------------------------------- capability drift
#
# A capability that exists in code but is denied in prose, or is asked for in
# NEXT_STEPS after it shipped, is how STATUS stopped being the source of truth
# three sprints running. Each rule below names one contradiction that was
# actually present, so none of them is hypothetical.

crypto_lib="$repo_root/rust/crates/qyro_crypto/src/handshake/mod.rs"
if [[ -f "$crypto_lib" ]]; then
  # The handshake module exists. Nothing may claim these primitives are absent.
  for doc in STATUS.md SECURITY.md THREAT_MODEL.md PROTOCOL.md ARCHITECTURE.md \
             rust/crates/qyro_crypto/src/lib.rs docs/security/device-identity.md; do
    path="$repo_root/$doc"
    [[ -f "$path" ]] || continue
    if grep -Eiq 'no (handshake|X25519|HKDF)|(sin|ni) handshake|no hay handshake|no existe handshake' "$path"; then
      report "BLOCKER" "Capability drift" "$doc says there is no handshake, but rust/crates/qyro_crypto/src/handshake exists"
      blockers=$((blockers + 1))
    fi
  done

  # NEXT_STEPS must not ask for a milestone that already shipped.
  next_steps="$repo_root/NEXT_STEPS.md"
  if [[ -f "$next_steps" ]]; then
    pending="$(sed -n '1,/^## Completado/p' "$next_steps")"
    if grep -Eiq 'implementar el handshake|implement the handshake' <<< "$pending"; then
      report "BLOCKER" "Capability drift" "NEXT_STEPS.md still asks for the handshake, which is implemented"
      blockers=$((blockers + 1))
    fi
  fi

  # STATUS must not carry it as the next task either.
  if sed -n '/^## Next task/,/^## /p' "$status_file" | grep -Eiq 'implementar el handshake|implement the handshake'; then
    report "BLOCKER" "Capability drift" "STATUS.md still lists the handshake as the next task"
    blockers=$((blockers + 1))
  fi
fi

# --------------------------------------------------------------- vector claims
#
# "Vectors implemented" must mean the files are on disk. A claim with no file
# behind it is exactly the kind of thing a reader has no way to check.
declare -A vector_files=(
  ["identity-v1.json"]="identidad|identity"
  ["handshake-v1.json"]="handshake"
)
for file in "${!vector_files[@]}"; do
  path="$repo_root/docs/security/test-vectors/$file"
  pattern="${vector_files[$file]}"
  if grep -Eiq "vectores? (de|del|interoperables? del)? ?($pattern).*: ?IMPLEMENTED" "$status_file"; then
    if [[ ! -f "$path" ]]; then
      report "BLOCKER" "Vector claim" "STATUS.md marks $pattern vectors implemented but $file is missing"
      blockers=$((blockers + 1))
    fi
  fi
done

# A schema is required alongside the handshake vectors: an interoperable file
# nothing validates is a file that drifts.
if [[ -f "$repo_root/docs/security/test-vectors/handshake-v1.json" ]] \
   && [[ ! -f "$repo_root/docs/security/test-vectors/handshake-v1.schema.json" ]]; then
  report "BLOCKER" "Vector claim" "handshake-v1.json has no committed schema"
  blockers=$((blockers + 1))
fi

# ------------------------------------------------------------- unicode folding
#
# The folding is full Unicode. Describing it as ASCII or Latin-1 understates a
# security guarantee, which invites someone to re-add the case the text says is
# missing.
if [[ -f "$repo_root/rust/crates/qyro_manifest/src/path.rs" ]] \
   && grep -q 'unicode_normalization' "$repo_root/rust/crates/qyro_manifest/src/path.rs"; then
  for doc in docs/protocols/manifest-format.md docs/security/parser-threats.md; do
    path="$repo_root/$doc"
    [[ -f "$path" ]] || continue
    if grep -Eih '(pliega|folds|plegado de)[^.]*(ASCII|Latin-1)' "$path" \
       | grep -qv '«'; then
      report "BLOCKER" "Folding claim" "$doc describes folding as ASCII/Latin-1 while path.rs uses unicode-normalization"
      blockers=$((blockers + 1))
    fi
  done
fi

# ------------------------------------------------------------ dependency claims
if [[ -f "$repo_root/Cargo.lock" ]] && grep -q 'name = "ed25519-dalek"' "$repo_root/Cargo.lock"; then
  for doc in SECURITY.md STATUS.md; do
    path="$repo_root/$doc"
    [[ -f "$path" ]] || continue
    # A document that quotes its own former wording while correcting it is
    # doing the right thing, so a claim inside guillemets does not count. Only
    # an unquoted assertion is a live claim.
    if grep -Eih 'no tiene dependencias externas|sin dependencias externas|cero dependencias externas' "$path" \
       | grep -qv '«'; then
      report "BLOCKER" "Dependency claim" "$doc says the workspace has no external dependencies, but Cargo.lock has ed25519-dalek"
      blockers=$((blockers + 1))
    fi
    if grep -Eiq 'no hay KAT|Tampoco hay KAT|sin KAT' "$path"; then
      report "BLOCKER" "Dependency claim" "$doc says there are no cryptographic KATs; RFC 7748/8032/4231 vectors are committed"
      blockers=$((blockers + 1))
    fi
  done
fi

# --------------------------------------------------------- platform evidence
#
# A platform marked executed must name the run that executed it. "YES (CI)" is
# what let Windows read as verified for three sprints while it could not even
# be checked out.
# `YES` as a whole word only: a case-insensitive substring match reads the
# "si" inside "físico" as an affirmative, which it did on the first attempt.
# Host-local evidence has no run id by nature, so it must say so explicitly
# rather than being silently exempt.
platform_section="$(sed -n '/^## Platforms executed/,/^## /p' "$status_file")"
while IFS= read -r line; do
  [[ "$line" == -* ]] || continue
  grep -Eqw 'YES' <<< "$line" || continue
  grep -Eq 'run [0-9]{6,}|[0-9]{9,}' <<< "$line" && continue
  grep -Eiq 'host local|esta sesión|this session' <<< "$line" && continue
  label="$(sed -E 's/^- ([^:]*):.*/\1/' <<< "$line")"
  report "BLOCKER" "Platform evidence" "STATUS.md marks '$label' executed without a run id"
  blockers=$((blockers + 1))
done <<< "$platform_section"

if [[ $blockers -gt 0 ]]; then
  report "BLOCKER" "Documentation consistency" "$blockers inconsistency finding(s)"
  exit 1
fi
report "OK" "Documentation consistency" "STATUS.md and canonical references agree"
