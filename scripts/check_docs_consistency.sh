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

# Phase 10, 2026-08-16. This rule used to forbid these four documents from
# saying "file transfer: implemented", and it was right for six sprints: the
# transfer did not exist. It exists now, so the rule had expired -- and an
# expired rule that still blocks is worse than no rule, because it stops the
# documents saying what is true.
#
# What replaces it guards the claim that is still worth guarding, and it is not
# a matter of opinion: no document may say the transfer is proven on hardware
# while `docs/testing/hardware-protocol.md` still has an unchecked box. The
# protocol is the evidence; the boxes are whether anybody ran it.
hardware_protocol="$repo_root/docs/testing/hardware-protocol.md"
hardware_unproven=1
if [[ -f "$hardware_protocol" ]] && ! grep -Fq '`[ ]`' "$hardware_protocol"; then
  hardware_unproven=0
fi
if [[ $hardware_unproven -eq 1 ]]; then
  for doc in PROJECT_CONTEXT.md README.md HANDOFF.md TESTING.md STATUS.md \
             docs/release/v1.0.md; do
    path="$repo_root/$doc"
    [[ -f "$path" ]] || continue
    # Two steps, because "no se ha probado en hardware" matches the same words
    # as "probado en hardware" and means the opposite. The first run of this
    # rule flagged a line whose whole point was to deny the claim -- which is
    # the textual guard failing the way textual guards fail, caught here rather
    # than by someone deleting a true sentence to make a check go green.
    if grep -Ein '(probad[oa]|verificad[oa]|tested|proven|validated)[^.]{0,40}(en|on) (hardware|un tel|a phone|dispositivos? f)' "$path" \
       | grep -Eiv '\<(no|nunca|jam.s|sin|not|never|ning[uú]n[ao]?|cero|zero)\>[^.]{0,40}(probad|verificad|tested|proven|validated)' \
       | grep -q .; then
      report "BLOCKER" "Hardware claim" "$doc claims hardware evidence that no one has recorded"
      blockers=$((blockers + 1))
    fi
  done
fi

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

# ------------------------------------------------------------- finding ledger
#
# Every concrete `QYR-00xx` citation must have exactly one entry in
# BUGS_PENDING.md. QYR-0043: two identifiers from an external audit had zero
# mentions in this repository, part of their content was fixed without being
# numbered, and QYR-0024 and QYR-0027 lived in STATUS.md and NEXT_STEPS.md while
# every other finding lived in the ledger. An identifier with no entry is a
# finding whose state nobody can look up. Ownership declarations such as
# `QYR-0100 onward` and `QYR-0076–QYR-0099` are ranges, not finding citations;
# counting their boundaries would require fake records in another owner's range
# (QYR-0100).
ledger="$repo_root/BUGS_PENDING.md"
if [[ -f "$ledger" ]]; then
  recorded="$(grep -oE '^## QYR-[0-9]{4}' "$ledger" | sed 's/^## //' | sort -u)"
  cited="$(
    grep -rhE 'QYR-[0-9]{4}' \
        --include='*.md' --include='*.rs' --include='*.sh' --include='*.ps1' \
        --include='*.yml' "$repo_root" 2>/dev/null |
      sed -E \
        -e 's/QYR-[0-9]{4}[[:space:]]*-[[:space:]]*QYR-[0-9]{4}//g' \
        -e 's/QYR-[0-9]{4}[[:space:]]*–[[:space:]]*QYR-[0-9]{4}//g' \
        -e 's/QYR-[0-9]{4}[[:space:]]*—[[:space:]]*QYR-[0-9]{4}//g' \
        -e 's/QYR-[0-9]{4}[[:space:]]+(onward|onwards|en adelante)//g' \
        -e 's/QYR-[0-9]{4}\+//g' |
      grep -oE 'QYR-[0-9]{4}' | sort -u || true
  )"
  while IFS= read -r finding; do
    [[ -z "$finding" ]] && continue
    if ! grep -qx -- "$finding" <<< "$recorded"; then
      report "BLOCKER" "Finding ledger" "$finding is cited but has no entry in BUGS_PENDING.md"
      blockers=$((blockers + 1))
    fi
  done <<< "$cited"

  # ...and *exactly* one entry, which the rule above cannot see. It builds
  # `recorded` with `sort -u`, so a second entry for the same identifier is
  # indistinguishable from the first: the comment said "exactly one entry" and
  # the code checked "at least one". QYR-0036 had two, one saying `abierto` and
  # one saying `resuelto`, and whether a reader believed the finding was open
  # depended on which one they scrolled to first (QYR-0046).
  duplicates="$(grep -oE '^## QYR-[0-9]{4}' "$ledger" | sed 's/^## //' | sort | uniq -d)"
  while IFS= read -r finding; do
    [[ -z "$finding" ]] && continue
    count="$(grep -cE "^## $finding" "$ledger")"
    report "BLOCKER" "Finding ledger" \
      "$finding has $count entries in BUGS_PENDING.md; a finding has one state, not two"
    blockers=$((blockers + 1))
  done <<< "$duplicates"

  # BUGS_PENDING is an operating queue. Once it grows past a screenful of
  # unresolved work, raw tool output has displaced prioritised findings and the
  # ledger no longer tells a human what to act on. Sprint 5D establishes 59 as
  # the hard ceiling; mutation inventories belong in reports, grouped here by
  # behaviour and severity.
  open_finding_limit=59
  open_finding_count="$(grep -cE '^- Estado:[[:space:]]*abierto([;[:space:]]|$)' "$ledger" || true)"
  if [[ "$open_finding_count" -gt "$open_finding_limit" ]]; then
    report "BLOCKER" "Finding ledger" \
      "$open_finding_count open findings exceed the ledger limit of $open_finding_limit"
    blockers=$((blockers + 1))
  fi
fi

# --------------------------------------------------- workflow branch triggers
#
# A workflow whose `branches:` names a branch literally only runs on the branch
# somebody remembered to write down. QYR-0026 fixed that symptom by writing the
# then-current branch into six files, which made it a property of one branch
# rather than of the repository; the next branch inherited the defect. QYR-0040
# is the rule that stops it recurring: a pattern is the property, a name is a
# reminder.
#
# `main` is exempt because it is not a working branch. Anything containing `*`
# is a pattern and therefore fine.
workflow_dir="$repo_root/.github/workflows"
if [[ -d "$workflow_dir" ]]; then
  for workflow in "$workflow_dir"/*.yml; do
    [[ -f "$workflow" ]] || continue
    workflow_name="$(basename "$workflow")"
    while IFS= read -r branch_line; do
      # An inline list is the only form this check can read. A block sequence
      # makes it fail rather than pass: a guard with a form it silently skips
      # is the defect it exists to catch.
      if ! grep -Eq '^[[:space:]]*branches:[[:space:]]*\[.*\]' <<< "$branch_line"; then
        report "BLOCKER" "Workflow branch trigger" \
          "$workflow_name uses a branches: form this check cannot read; write an inline list"
        blockers=$((blockers + 1))
        continue
      fi
      branch_items="$(sed -E 's/^[[:space:]]*branches:[[:space:]]*\[(.*)\].*$/\1/' <<< "$branch_line")"
      IFS=',' read -ra branch_entries <<< "$branch_items"
      for entry in "${branch_entries[@]}"; do
        entry="$(sed -E "s/^[[:space:]]*[\"']?//; s/[\"']?[[:space:]]*$//" <<< "$entry")"
        [[ -z "$entry" ]] && continue
        [[ "$entry" == "main" ]] && continue
        [[ "$entry" == *"*"* ]] && continue
        report "BLOCKER" "Workflow branch trigger" \
          "$workflow_name names the branch '$entry' literally; use a pattern such as 'claude/**' so a new working branch needs no YAML edit"
        blockers=$((blockers + 1))
      done
    done < <(grep -E '^[[:space:]]*branches:' "$workflow")
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
