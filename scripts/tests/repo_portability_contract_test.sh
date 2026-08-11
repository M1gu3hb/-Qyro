#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
checker="$repo_root/scripts/check_repo_portability.sh"
if [[ ! -f "$checker" ]]; then echo "Expected $checker to exist." >&2; exit 1; fi

failures=0

make_fixture() {
  local root="$1"
  git -C "$root" init --quiet
  git -C "$root" config user.email "test@example.invalid"
  git -C "$root" config user.name "Contract test"
  # The fixture must be able to put a Windows-hostile path in the *index* so
  # the checker, rather than Git for Windows' pre-emptive protection, rejects
  # it. No such path is created on disk.
  git -C "$root" config core.protectNTFS false
  mkdir -p "$root/docs"
  printf 'ok\n' > "$root/docs/readme.md"
  git -C "$root" add -A
  git -C "$root" commit --quiet -m "fixture"
}

# Adds a tracked path. Uses `git update-index` rather than a real file so the
# hostile names can be tested on any host, including one that cannot create
# them. That is the point: this checker must catch a name the *checkout* host
# rejects, and a Linux runner can create names Windows cannot.
track_path() {
  local root="$1" path="$2"
  local blob
  blob="$(printf 'x' | git -C "$root" hash-object -w --stdin)"
  git -C "$root" update-index --add --cacheinfo "100644,$blob,$path"
}

assert_rejects() {
  local path="$1" expected="$2"
  local root
  root="$(mktemp -d)"
  make_fixture "$root"
  track_path "$root" "$path"

  set +e
  local output status
  output="$(bash "$checker" --repo-root "$root" 2>&1)"
  status=$?
  set -e

  if [[ "$status" -eq 0 ]]; then
    echo "FAIL: $path must be rejected, but the checker passed" >&2
    failures=$((failures + 1))
  elif ! grep -qi "$expected" <<< "$output"; then
    echo "FAIL: $path was rejected without naming the reason ($expected)" >&2
    echo "$output" >&2
    failures=$((failures + 1))
  else
    echo "ok: rejects $path"
  fi
  rm -rf "$root"
}

assert_accepts() {
  local path="$1"
  local root
  root="$(mktemp -d)"
  make_fixture "$root"
  track_path "$root" "$path"

  set +e
  bash "$checker" --repo-root "$root" >/dev/null 2>&1
  local status=$?
  set -e

  if [[ "$status" -ne 0 ]]; then
    echo "FAIL: $path is portable and must be accepted" >&2
    failures=$((failures + 1))
  else
    echo "ok: accepts $path"
  fi
  rm -rf "$root"
}

# The exact path that broke the Windows job. A NUL-byte corpus case named after
# its contents, on a system where NUL is a device.
assert_rejects "rust/fuzz/corpus/relative_path/nul.txt" "reserved windows device name"
assert_rejects "nul" "reserved"
assert_rejects "docs/CON.md" "reserved"
assert_rejects "a/com1.bin" "reserved"
assert_rejects "docs/lpt9.txt" "reserved"
assert_rejects 'docs/a:b.md' "forbid"
assert_rejects 'docs/what?.md' "forbid"
# Windows strips a trailing space or dot from the whole segment, so two paths
# that differ only by one become the same file.
assert_rejects 'docs/trailing ' "strip"
assert_rejects 'docs/trailing.' "strip"
assert_rejects 'docs/dir./file.md' "strip"

# Names that only look reserved must still be accepted, exactly as
# qyro_manifest accepts them. Over-rejection would make legitimate files
# impossible to commit.
assert_accepts "docs/console.md"
assert_accepts "docs/com10.txt"
assert_accepts "docs/nul_byte.txt"
assert_accepts "docs/conf.md"
# A space *inside* a segment is fine; only a trailing one is stripped. Rejecting
# this would be over-rejection, which makes legitimate files uncommittable.
assert_accepts 'docs/release notes.md'
assert_accepts 'docs/trailing .md'
assert_accepts "rust/fuzz/corpus/relative_path/reserved_con.txt"

# The real repository must pass.
if ! bash "$checker" --repo-root "$repo_root" >/dev/null 2>&1; then
  echo "FAIL: the repository itself has a path Windows cannot check out" >&2
  bash "$checker" --repo-root "$repo_root" >&2
  failures=$((failures + 1))
else
  echo "ok: the repository itself is checkout-clean on Windows"
fi

if [[ "$failures" -gt 0 ]]; then
  echo "$failures contract failure(s)." >&2
  exit 1
fi
echo "All repo portability contracts hold."
