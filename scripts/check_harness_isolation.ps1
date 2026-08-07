#!/usr/bin/env pwsh
# Keeps the crypto smoke harness out of the product.
#
# The harness exists so Android, iOS and Windows have real evidence about
# `qyro_crypto`. It links the crate that holds keys, and it exports a symbol
# whose whole job is to run a handshake. Shipping it would mean buying a test
# with an attack surface, which is a bad trade at any price.
#
# Two halves. This script checks what can be checked from the source tree, on
# every platform, in CI and locally. The artifacts themselves — the APK, the
# Runner.app and the Windows ZIP — are searched for the exported symbol inside
# `platform-builds.yml`, where they exist. Neither half is sufficient: a source
# tree can be clean while a build script copies the library in, and an artifact
# check only runs where that artifact is produced.
#
# See docs/adr/ADR-0023-crypto-platform-test-harness.md.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

$script:status = 0
$symbol = 'qyro_crypto_smoke_run'
# Both harnesses. See the Bash half: naming one instance of a category stops
# covering the category the moment a second appears, and `qyro_store_smoke`
# arrived in sprint 4D.1.
$harnesses = @('qyro_crypto_smoke', 'qyro_store_smoke')

function Write-Failure([string] $Message) {
    Write-Error -Message "[FAIL] $Message" -ErrorAction Continue
    $script:status = 1
}

# --- nothing the product builds may name the harness -------------------------

foreach ($crate in @('qyro_ffi', 'qyro_core', 'qyro_crypto', 'qyro_protocol', 'qyro_manifest')) {
    $manifest = Join-Path 'rust' 'crates' $crate 'Cargo.toml'
    foreach ($harness in $harnesses) {
        if ((Get-Content -LiteralPath $manifest -Raw) -match [regex]::Escape($harness)) {
            Write-Failure "$manifest depends on the test harness $harness"
        }
    }
}

# The FFI boundary itself, checked here too so this script alone answers "can
# Dart reach a key?" without anyone having to also run the Rust test suite.
#
# A comment naming the crate is not a dependency on it. This used to match the
# whole manifest, so writing the crate's name in a comment failed the check and
# the only way to keep it green was to avoid saying the word. Comment lines are
# dropped first (sprint 4C.2, QYR-0031). A target-specific dependency table is
# still caught: it is not a comment.
foreach ($crate in @('qyro_ffi', 'qyro_core')) {
    $manifest = Join-Path 'rust' 'crates' $crate 'Cargo.toml'
    $declarations = (Get-Content -LiteralPath $manifest) |
        Where-Object { $_ -notmatch '^\s*#' }
    if (($declarations -join "`n") -match 'qyro_crypto') {
        Write-Failure "$manifest reaches qyro_crypto; the library Dart loads must not"
    }
}

# --- no application source may reference it ----------------------------------

$appRoot = Join-Path 'apps' 'qyro'
if (Test-Path -LiteralPath $appRoot) {
    $offenders = Get-ChildItem -LiteralPath $appRoot -Recurse -File -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -notmatch '[\\/](build|\.dart_tool)[\\/]' } |
        Where-Object {
            $text = Get-Content -LiteralPath $_.FullName -Raw -ErrorAction SilentlyContinue
            $text -and ($text -match [regex]::Escape($symbol) -or
                ($harnesses | Where-Object { $text -match [regex]::Escape($_) }))
        }
    if ($offenders) {
        Write-Failure 'the Flutter application references a test harness'
    }
}

# --- no staged native library may be the harness -----------------------------
#
# These directories are what the platform build copies into the APK and the iOS
# bundle. A file here goes into the product whatever any manifest says.

foreach ($staged in @(
        (Join-Path 'apps' 'qyro' 'android' 'app' 'src' 'main' 'jniLibs'),
        (Join-Path 'apps' 'qyro' 'ios' 'Native'))) {
    if (-not (Test-Path -LiteralPath $staged)) { continue }
    foreach ($library in Get-ChildItem -LiteralPath $staged -Recurse -File) {
        if ($harnesses | Where-Object { $library.Name -match [regex]::Escape($_) }) {
            Write-Failure "$($library.FullName) stages a test harness into a product bundle"
            continue
        }
        $bytes = [System.IO.File]::ReadAllBytes($library.FullName)
        $text = [System.Text.Encoding]::ASCII.GetString($bytes)
        if ($text.Contains($symbol)) {
            Write-Failure "$($library.FullName) contains $symbol"
        }
    }
}

# --- the harness must declare itself unshippable -----------------------------

foreach ($harness in $harnesses) {
    $harnessManifest = Join-Path 'rust' 'tools' $harness 'Cargo.toml'
    if (-not (Test-Path -LiteralPath $harnessManifest)) {
        Write-Failure "the manifest for $harness is missing"
    }
    elseif ((Get-Content -LiteralPath $harnessManifest -Raw) -notmatch '(?m)^publish = false') {
        Write-Failure "$harnessManifest must set publish = false"
    }
}

# --- and it must never be built in release for distribution ------------------
#
# Not a rule about the `--release` flag: a release *build* of the harness is
# fine on a runner. What must not exist is a workflow step that puts it into
# something a user downloads.

foreach ($workflow in Get-ChildItem -LiteralPath (Join-Path '.github' 'workflows') -Filter '*.yml') {
    $text = Get-Content -LiteralPath $workflow.FullName -Raw
    # Naming the harness is not the offence — `platform-builds.yml` names it
    # precisely to search the APK, the ZIP and Runner.app for it, which is the
    # opposite of shipping it. What matters is *building* it next to an upload.
    foreach ($harness in $harnesses) {
        if ($text -match "(--package|-p) $([regex]::Escape($harness))" -and $text -match 'upload-artifact') {
            if ($workflow.Name -ne 'crypto-platform.yml') {
                Write-Failure "$($workflow.Name) both builds $harness and uploads an artifact"
            }
        }
    }
}

if ($script:status -ne 0) {
    Write-Error -Message '[FAIL] Harness isolation: the crypto smoke harness can reach the product' -ErrorAction Continue
    exit 1
}

Write-Host '[OK] Harness isolation: the crypto smoke harness cannot reach the product'
