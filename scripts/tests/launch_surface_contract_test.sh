#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

require_text() {
  local path="$1"
  local text="$2"
  if [[ ! -f "$path" ]] || ! grep -Fq "$text" "$path"; then
    echo "[FAIL] Expected $path to contain: $text" >&2
    exit 1
  fi
}

reject_text() {
  local path="$1"
  local text="$2"
  if [[ -f "$path" ]] && grep -Fq "$text" "$path"; then
    echo "[FAIL] Expected $path not to contain: $text" >&2
    exit 1
  fi
}

android="$repo_root/apps/qyro/android/app/src/main/res"
require_text "$android/values/colors.xml" '<color name="qyro_background">#03070D</color>'
require_text "$android/drawable/launch_background.xml" '@color/qyro_background'
require_text "$android/drawable-v21/launch_background.xml" '@color/qyro_background'
require_text "$android/values/styles.xml" '@drawable/launch_background'
require_text "$android/values-night/styles.xml" '@drawable/launch_background'
reject_text "$android/drawable/launch_background.xml" '@android:color/white'
reject_text "$android/drawable-v21/launch_background.xml" '?android:colorBackground'
reject_text "$android/values/styles.xml" 'Theme.Light'

ios="$repo_root/apps/qyro/ios/Runner/Base.lproj/LaunchScreen.storyboard"
require_text "$ios" 'red="0.01176470588"'
require_text "$ios" 'green="0.02745098039"'
require_text "$ios" 'blue="0.05098039216"'
reject_text "$ios" 'image="LaunchImage"'
reject_text "$ios" 'red="1" green="1" blue="1"'

# Interface Builder refuses to open a storyboard whose document element omits
# toolsVersion, and fails the whole iOS build with "com.apple.InterfaceBuilder
# error -1" long before any Dart code runs. Assert the loadable structure here so
# a Linux runner catches it without a macOS host.
if ! python3 - "$ios" <<'PY'
import sys
import xml.etree.ElementTree as ElementTree

path = sys.argv[1]
try:
    document = ElementTree.parse(path).getroot()
except ElementTree.ParseError as error:
    print(f"[FAIL] {path} is not well-formed XML: {error}", file=sys.stderr)
    raise SystemExit(1)

if document.tag != "document":
    print(f"[FAIL] {path} root element must be <document>", file=sys.stderr)
    raise SystemExit(1)

for attribute in ("toolsVersion", "targetRuntime", "initialViewController"):
    if not document.get(attribute):
        print(
            f"[FAIL] {path} <document> must declare {attribute} "
            "or Interface Builder cannot open it",
            file=sys.stderr,
        )
        raise SystemExit(1)

if document.get("launchScreen") != "YES":
    print(f"[FAIL] {path} must stay a launch screen", file=sys.stderr)
    raise SystemExit(1)

for capability in document.iter("capability"):
    minimum = capability.get("minToolsVersion")
    if minimum and minimum > document.get("toolsVersion", ""):
        print(
            f"[FAIL] {path} declares capability "
            f"{capability.get('name')!r} above its toolsVersion",
            file=sys.stderr,
        )
        raise SystemExit(1)
PY
then
  exit 1
fi

windows="$repo_root/apps/qyro/windows/runner/win32_window.cpp"
require_text "$windows" 'RGB(3, 7, 13)'
require_text "$windows" 'window_class.hbrBackground ='
reject_text "$windows" 'window_class.hbrBackground = 0;'

echo "[PASS] Native launch surfaces use the Qyro dark background"
