#!/usr/bin/env pwsh
# Rejects a workflow set that claims platform coverage it does not have.
#
# The four workflows that existed before this check all build and run
# `qyro_ffi`. None of them built `qyro_crypto` for Android, iOS or Windows, and
# `qyro_ffi` deliberately does not depend on `qyro_crypto` — so a green Platform
# builds run was evidence about the minimal ABI and nothing at all about the
# handshake or the AEAD on those targets.
#
# Building qyro_ffi for a target is not building qyro_crypto for it. This script
# exists because that sentence is easy to agree with and easy to forget.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

$workflows = Join-Path '.github' 'workflows'
$cryptoWorkflow = Join-Path $workflows 'crypto-platform.yml'
$script:status = 0

function Write-Failure([string] $Message) {
    Write-Error -Message "[FAIL] $Message" -ErrorAction Continue
    $script:status = 1
}

function Require-Pattern([string] $File, [string] $Pattern, [string] $Description) {
    if (-not (Test-Path -LiteralPath $File)) {
        Write-Failure "${Description}: $File does not exist"
        return
    }
    $content = Get-Content -LiteralPath $File -Raw
    if ($content -notmatch $Pattern) {
        Write-Failure "${Description}: $File does not match /$Pattern/"
    }
}

# --- the dedicated workflow must exist at all --------------------------------

if (-not (Test-Path -LiteralPath $cryptoWorkflow)) {
    Write-Failure "no workflow builds or runs qyro_crypto on a target platform: $cryptoWorkflow is missing"
    Write-Error -Message '[FAIL] Crypto platform evidence: the existing workflows cover qyro_ffi only' -ErrorAction Continue
    exit 1
}

# --- one job per platform ----------------------------------------------------

foreach ($job in @('linux-crypto', 'windows-crypto', 'android-crypto', 'ios-crypto')) {
    Require-Pattern $cryptoWorkflow "(?m)^  $([regex]::Escape($job)):" "job $job"
}

# --- qyro_crypto itself must be built for every target -----------------------
#
# The package name is checked together with the target, so building qyro_ffi for
# the same triple cannot satisfy the rule.

foreach ($target in @(
        'x86_64-linux-android',
        'aarch64-linux-android',
        'aarch64-apple-ios',
        'aarch64-apple-ios-sim')) {
    Require-Pattern $cryptoWorkflow "--package qyro_crypto( .*)? --target $([regex]::Escape($target))" `
        "qyro_crypto build for $target"
}

# --- and its tests must actually run where a host can run them ---------------

Require-Pattern $cryptoWorkflow 'cargo test( .*)? --package qyro_crypto' 'qyro_crypto unit tests'
Require-Pattern $cryptoWorkflow 'runs-on: windows' 'a Windows runner'
Require-Pattern $cryptoWorkflow 'runs-on: macos' 'a macOS runner'

# --- the smoke has to run, not just compile ----------------------------------
#
# Compiling proves the toolchain accepts the code. It does not prove X25519,
# Ed25519, HKDF, ChaCha20-Poly1305 and the replay window behave on that
# platform's word size, endianness and CPU features.

Require-Pattern $cryptoWorkflow 'qyro_crypto_smoke' 'a crypto smoke execution'
Require-Pattern $cryptoWorkflow 'adb' 'an Android emulator execution path'
Require-Pattern $cryptoWorkflow 'xcodebuild test' 'an iOS simulator execution path'

# --- the harness must exist and stay out of the product ----------------------

$harness = Join-Path (Join-Path 'rust' 'tools') 'qyro_crypto_smoke'
if (-not (Test-Path -LiteralPath $harness)) {
    Write-Failure 'the isolated crypto smoke harness does not exist'
}
else {
    $manifest = Get-Content -LiteralPath (Join-Path $harness 'Cargo.toml') -Raw
    if ($manifest -notmatch '(?m)^publish = false') {
        Write-Failure 'the crypto smoke harness must set publish = false'
    }
}

foreach ($manifest in @(
        (Join-Path 'rust' 'crates' 'qyro_ffi' 'Cargo.toml'),
        (Join-Path 'rust' 'crates' 'qyro_core' 'Cargo.toml'))) {
    if ((Get-Content -LiteralPath $manifest -Raw) -match 'qyro_crypto') {
        Write-Failure "$manifest reaches qyro_crypto; the FFI boundary must stay crypto-free"
    }
}

if ($script:status -ne 0) {
    Write-Error -Message '[FAIL] Crypto platform evidence: qyro_crypto is not proven on the product platforms' -ErrorAction Continue
    exit 1
}

Write-Host '[OK] Crypto platform evidence: qyro_crypto is built and executed on every product platform'
