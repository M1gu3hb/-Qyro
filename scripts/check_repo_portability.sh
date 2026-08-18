#!/usr/bin/env bash
# Refuses tracked paths that Windows cannot check out.
#
# This exists because the repository shipped one. `rust/fuzz/corpus/relative_path/nul.txt`
# held the NUL-*byte* corpus case and was named after its contents, but `NUL` is
# a reserved Windows device name, so `git checkout` failed with
# `invalid path` and the Windows CI job died at step 2 — before any Qyro code ran.
#
# The rule enforced here is the same one `qyro_manifest` enforces on a transfer.
# A project that rejects a peer's non-portable filename and then commits one of
# its own is not applying its own standard.
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
  exit 2
fi

cd "$repo_root" || exit 2

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  report "BLOCKER" "Repository" "$repo_root is not a git repository"
  exit 2
fi

# Reserved device names. Windows treats them as devices whatever the extension,
# so `nul.txt` is as unusable as `nul`.
reserved='^(CON|PRN|AUX|NUL|COM[0-9]|LPT[0-9])$'
# Characters Windows forbids in a path segment. `/` is the separator and is not
# part of a segment; `\` is a separator on Windows, so it is forbidden too.
illegal='[<>:"|?*\\]'

while IFS= read -r path; do
  [[ -n "$path" ]] || continue

  IFS='/' read -ra segments <<< "$path"
  for segment in "${segments[@]}"; do
    [[ -n "$segment" ]] || continue

    stem="${segment%%.*}"
    # Bash already has ASCII case conversion. Spawning `printf | tr` for every
    # segment made this O(paths) process launches and exceeded 120 s in Git Bash
    # on Windows even though the actual comparisons are tiny.
    upper="${stem^^}"
    if [[ "$upper" =~ $reserved ]]; then
      report "BLOCKER" "Portability" "$path uses the reserved Windows device name $upper; git checkout fails on Windows"
      blockers=$((blockers + 1))
    fi

    if [[ "$segment" =~ $illegal ]]; then
      report "BLOCKER" "Portability" "$path contains a character Windows forbids in a filename"
      blockers=$((blockers + 1))
    fi

    # Windows silently strips these, so two distinct paths become one file.
    if [[ "$segment" =~ [\ .]$ ]]; then
      report "BLOCKER" "Portability" "$path ends a segment with a space or dot, which Windows strips"
      blockers=$((blockers + 1))
    fi
  done
done < <(git ls-files)

if [[ "$blockers" -gt 0 ]]; then
  report "BLOCKER" "Repository portability" "$blockers path(s) cannot be checked out on Windows"
  exit 1
fi

report "OK" "Repository portability" "every tracked path can be checked out on Windows"
exit 0
