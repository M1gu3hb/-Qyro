#!/usr/bin/env pwsh
# Contract for the checker that refuses "qyro_ffi built for Android" as evidence
# that qyro_crypto works on Android.
#
# Run against fixtures rather than the repository alone, so the checker is shown
# to reject each specific omission instead of passing for an unrelated reason.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Windows PowerShell 5.1 accepts only one child per Join-Path invocation. The
# contract deliberately runs in the current host, so keep its fixture builder
# compatible with both 5.1 and PowerShell Core.
function Join-Path {
    param(
        [Parameter(Position = 0, Mandatory = $true)]
        [string] $Path,
        [Parameter(Position = 1, Mandatory = $true, ValueFromRemainingArguments = $true)]
        [string[]] $ChildPath
    )
    $joined = $Path
    foreach ($child in $ChildPath) {
        $joined = Microsoft.PowerShell.Management\Join-Path -Path $joined -ChildPath $child
    }
    return $joined
}

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$checker = Join-Path $repoRoot 'scripts' 'check_crypto_platform_evidence.ps1'
$powerShellHost = (Get-Process -Id $PID).Path
if (-not (Test-Path -LiteralPath $checker)) {
    Write-Error "Expected $checker to exist."
    exit 1
}

$script:failures = 0

$workflowFixture = @'
name: Crypto platform
jobs:
  linux-crypto:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test --locked --package qyro_crypto
      - run: cargo run --package qyro_crypto_smoke
  windows-crypto:
    runs-on: windows-latest
    steps:
      - run: cargo test --locked --package qyro_crypto
      - run: cargo run --package qyro_crypto_smoke
  android-crypto:
    runs-on: ubuntu-latest
    steps:
      - run: cargo build --locked --package qyro_crypto --target x86_64-linux-android
      - run: cargo build --locked --package qyro_crypto --target aarch64-linux-android
      - run: adb push qyro_crypto_smoke /data/local/tmp/
  ios-crypto:
    runs-on: macos-latest
    steps:
      - run: cargo build --locked --package qyro_crypto --target aarch64-apple-ios
      - run: cargo build --locked --package qyro_crypto --target aarch64-apple-ios-sim
      - run: xcodebuild test -scheme CryptoSmoke
'@

function New-Fixture {
    $root = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
    foreach ($relative in @(
            (Join-Path '.github' 'workflows'),
            (Join-Path 'rust' 'tools' 'qyro_crypto_smoke'),
            (Join-Path 'rust' 'crates' 'qyro_ffi'),
            (Join-Path 'rust' 'crates' 'qyro_core'),
            'scripts')) {
        New-Item -ItemType Directory -Path (Join-Path $root $relative) -Force | Out-Null
    }
    Copy-Item -LiteralPath $checker -Destination (Join-Path $root 'scripts' 'check_crypto_platform_evidence.ps1')
    Set-Content -LiteralPath (Join-Path $root 'rust' 'tools' 'qyro_crypto_smoke' 'Cargo.toml') -Value 'publish = false'
    Set-Content -LiteralPath (Join-Path $root 'rust' 'crates' 'qyro_ffi' 'Cargo.toml') `
        -Value "[dependencies]`nqyro_core = { path = `"../qyro_core`" }"
    Set-Content -LiteralPath (Join-Path $root 'rust' 'crates' 'qyro_core' 'Cargo.toml') -Value '[dependencies]'
    Set-Content -LiteralPath (Join-Path $root '.github' 'workflows' 'crypto-platform.yml') -Value $workflowFixture
    return $root
}

function Invoke-Checker([string] $Root) {
    $target = Join-Path $Root 'scripts' 'check_crypto_platform_evidence.ps1'
    $savedPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        & $powerShellHost -NoProfile -File $target *>&1 | Out-String | Out-Null
        return $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $savedPreference
    }
}

function Assert-Rejects([string] $Description, [scriptblock] $Mutate) {
    $root = New-Fixture
    try {
        & $Mutate $root
        if ((Invoke-Checker $root) -eq 0) {
            Write-Host "FAIL: $Description must be rejected, but the checker passed"
            $script:failures++
        }
        else {
            Write-Host "ok: rejects $Description"
        }
    }
    finally {
        Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Remove-WorkflowLine([string] $Root, [string] $Pattern) {
    $path = Join-Path $Root '.github' 'workflows' 'crypto-platform.yml'
    (Get-Content -LiteralPath $path) | Where-Object { $_ -notmatch $Pattern } |
        Set-Content -LiteralPath $path
}

# The fixture is valid before any mutation. Without this, a checker that
# rejected everything would pass every assertion below.
$root = New-Fixture
try {
    if ((Invoke-Checker $root) -ne 0) {
        Write-Host 'FAIL: the complete fixture must be accepted, or every rejection below proves nothing'
        $script:failures++
    }
    else {
        Write-Host 'ok: accepts a complete fixture'
    }
}
finally {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}

Assert-Rejects 'a missing crypto workflow' {
    param($Root)
    Remove-Item -LiteralPath (Join-Path $Root '.github' 'workflows' 'crypto-platform.yml') -Force
}
Assert-Rejects 'a missing per-platform job' { param($Root) Remove-WorkflowLine $Root '^  windows-crypto:' }
Assert-Rejects 'Android arm64 never being built' { param($Root) Remove-WorkflowLine $Root 'aarch64-linux-android' }
Assert-Rejects 'the iOS simulator target never being built' { param($Root) Remove-WorkflowLine $Root 'aarch64-apple-ios-sim' }
Assert-Rejects 'the smoke never being executed' { param($Root) Remove-WorkflowLine $Root 'qyro_crypto_smoke' }
Assert-Rejects 'no emulator execution path' { param($Root) Remove-WorkflowLine $Root 'adb push' }
Assert-Rejects 'no simulator execution path' { param($Root) Remove-WorkflowLine $Root 'xcodebuild test' }
Assert-Rejects 'a missing harness' {
    param($Root)
    Remove-Item -LiteralPath (Join-Path $Root 'rust' 'tools' 'qyro_crypto_smoke') -Recurse -Force
}
Assert-Rejects 'a publishable harness' {
    param($Root)
    Set-Content -LiteralPath (Join-Path $Root 'rust' 'tools' 'qyro_crypto_smoke' 'Cargo.toml') -Value 'publish = true'
}
Assert-Rejects 'qyro_ffi reaching qyro_crypto' {
    param($Root)
    Set-Content -LiteralPath (Join-Path $Root 'rust' 'crates' 'qyro_ffi' 'Cargo.toml') `
        -Value "[dependencies]`nqyro_crypto = { path = `"../qyro_crypto`" }"
}
# The exact substitution this checker exists to refuse: building qyro_ffi for a
# target and calling it crypto evidence.
Assert-Rejects 'qyro_ffi builds passed off as crypto evidence' {
    param($Root)
    $path = Join-Path $Root '.github' 'workflows' 'crypto-platform.yml'
    (Get-Content -LiteralPath $path -Raw).Replace('--package qyro_crypto --target', '--package qyro_ffi --target') |
        Set-Content -LiteralPath $path
}

# And the real repository must pass, which is the point of the whole sprint.
& $powerShellHost -NoProfile -File $checker *>&1 | Out-String | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host 'FAIL: the repository does not prove qyro_crypto on its own platforms'
    $script:failures++
}
else {
    Write-Host 'ok: the repository proves qyro_crypto on every product platform'
}

if ($script:failures -gt 0) {
    Write-Error "$($script:failures) contract failure(s)."
    exit 1
}
Write-Host 'All crypto platform evidence contracts hold.'
