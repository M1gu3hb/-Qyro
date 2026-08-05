#!/usr/bin/env bash
# Runs the Qyro crypto smoke inside a booted Android emulator.
#
# Split out of crypto-platform.yml because the emulator action takes a single
# `script:` string, and a multi-step shell pipeline inlined into YAML is the
# kind of thing that breaks silently on quoting.
#
# Pushes an ordinary Android executable rather than a Flutter integration test:
# the Dart test proves qyro_ffi loads, and qyro_ffi cannot reach qyro_crypto.
set -euo pipefail

binary="target/x86_64-linux-android/debug/qyro_crypto_smoke"
device_path="/data/local/tmp/qyro_crypto_smoke"

test -f "$binary"
mkdir -p reports

adb wait-for-device
adb shell 'rm -f /data/local/tmp/qyro_crypto_smoke'
adb push "$binary" "$device_path"
adb shell "chmod 755 $device_path"

# Human-readable first, so a failing run says which step failed in the log.
adb shell "$device_path"

# Then the machine-readable report. `adb shell` mangles line endings, so the
# carriage returns are stripped before the file is written.
adb shell "$device_path --json" | tr -d '\r' > reports/android.json
cat reports/android.json

# `adb shell` reports the shell's exit status, not the binary's, on some
# images. Ask for the code explicitly rather than trusting the pipeline.
status="$(adb shell "$device_path >/dev/null 2>&1; echo \$?" | tr -d '\r')"
if [[ "$status" != "0" ]]; then
    echo "[FAIL] qyro crypto smoke exited $status inside the emulator" >&2
    exit 1
fi

# And leave nothing behind on the device.
adb shell "rm -f $device_path"
echo "[PASS] qyro crypto smoke ran on x86_64-linux-android in the emulator"
