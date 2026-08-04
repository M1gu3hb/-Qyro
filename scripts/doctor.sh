#!/usr/bin/env bash
set -uo pipefail

blockers=0
simulated_missing=",$(printf '%s' "${QYRO_DOCTOR_SIMULATE_MISSING:-}" | tr '[:upper:]' '[:lower:]' | tr -d ' '),"

is_simulated_missing() {
  case "$simulated_missing" in
    *,"$1",*) return 0 ;;
    *) return 1 ;;
  esac
}

report() {
  printf '[%s] %s: %s\n' "$1" "$2" "$3"
}

check_command() {
  local token="$1"
  local label="$2"
  local command_name="$3"
  local required="$4"
  shift 4

  if is_simulated_missing "$token" || ! command -v "$command_name" >/dev/null 2>&1; then
    if [[ "$required" == "required" ]]; then
      report "BLOCKER" "$label" "not found on PATH"
      blockers=$((blockers + 1))
    else
      report "WARNING" "$label" "not found; optional workflow unavailable"
    fi
    return
  fi

  local detail
  detail="$("$command_name" "$@" 2>&1 | sed -n '1p')"
  if [[ -z "$detail" ]]; then
    detail="available"
  fi
  report "OK" "$label" "$detail"
}

check_command "git" "Git" "git" "required" --version
check_command "flutter" "Flutter" "flutter" "required" --version
check_command "dart" "Dart" "dart" "required" --version
check_command "rust" "Rust" "rustc" "required" --version
check_command "cargo" "Cargo" "cargo" "required" --version
check_command "fvm" "FVM" "fvm" "optional" --version
check_command "java" "Java" "java" "optional" -version
check_command "cmake" "CMake" "cmake" "optional" --version
check_command "ninja" "Ninja" "ninja" "optional" --version

android_sdk="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-}}"
if is_simulated_missing "android-sdk" || [[ -z "$android_sdk" || ! -d "$android_sdk" ]]; then
  report "WARNING" "Android SDK" "ANDROID_SDK_ROOT/ANDROID_HOME is not configured"
else
  report "OK" "Android SDK" "$android_sdk"
fi

if is_simulated_missing "ndk"; then
  report "WARNING" "Android NDK" "not found"
elif [[ -n "$android_sdk" ]] && compgen -G "$android_sdk/ndk/*" >/dev/null 2>&1; then
  ndk_path="$(find "$android_sdk/ndk" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort -V | tail -n 1)"
  report "OK" "Android NDK" "${ndk_path:-available}"
else
  report "WARNING" "Android NDK" "not installed under the Android SDK"
fi

platform="$(uname -s 2>/dev/null || printf 'Unknown')"
case "$platform" in
  Darwin)
    check_command "xcode" "Xcode" "xcodebuild" "optional" -version
    check_command "cocoapods" "CocoaPods" "pod" "optional" --version
    report "N/A" "Visual Studio Build Tools" "Windows only"
    report "N/A" "Windows SDK" "Windows only"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    report "N/A" "Xcode" "macOS only"
    report "N/A" "CocoaPods" "macOS only"
    if is_simulated_missing "visual-studio" || ! command -v cl.exe >/dev/null 2>&1; then
      report "WARNING" "Visual Studio Build Tools" "cl.exe not found on PATH"
    else
      report "OK" "Visual Studio Build Tools" "cl.exe available"
    fi
    if [[ -n "${WindowsSdkDir:-}" ]]; then
      report "OK" "Windows SDK" "$WindowsSdkDir"
    else
      report "WARNING" "Windows SDK" "WindowsSdkDir is not configured"
    fi
    ;;
  *)
    report "N/A" "Xcode" "macOS only"
    report "N/A" "CocoaPods" "macOS only"
    report "N/A" "Visual Studio Build Tools" "Windows only"
    report "N/A" "Windows SDK" "Windows only"
    ;;
esac

if is_simulated_missing "devices"; then
  report "WARNING" "Flutter devices" "device discovery was simulated as unavailable"
elif command -v flutter >/dev/null 2>&1; then
  devices="$(flutter devices --machine 2>/dev/null || true)"
  if [[ "$devices" == *'"id"'* ]]; then
    report "OK" "Flutter devices" "at least one device or simulator is discoverable"
  else
    report "WARNING" "Flutter devices" "no device or simulator is currently discoverable"
  fi
else
  report "WARNING" "Flutter devices" "Flutter is unavailable"
fi

if [[ $blockers -gt 0 ]]; then
  report "BLOCKER" "Doctor summary" "$blockers required tool(s) missing"
  exit 1
fi

report "OK" "Doctor summary" "required toolchain is ready"
