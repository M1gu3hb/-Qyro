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

windows="$repo_root/apps/qyro/windows/runner/win32_window.cpp"
require_text "$windows" 'RGB(3, 7, 13)'
require_text "$windows" 'window_class.hbrBackground ='
reject_text "$windows" 'window_class.hbrBackground = 0;'

echo "[PASS] Native launch surfaces use the Qyro dark background"
