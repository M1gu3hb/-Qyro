param(
    [string] $RepoRoot = (Join-Path $PSScriptRoot '..'),
    [switch] $Plan
)

$ErrorActionPreference = 'Continue'
$script:Blockers = 0

function Write-Status {
    param(
        [ValidateSet('OK', 'WARNING', 'BLOCKER', 'N/A')]
        [string] $Kind,
        [string] $Label,
        [string] $Detail
    )
    Write-Output "[$Kind] $($Label): $Detail"
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    Write-Status 'BLOCKER' 'Repository' "$RepoRoot does not exist"
    exit 1
}
$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path

if ($Plan) {
    Write-Status 'N/A' 'Rust quality' 'plan: format, Clippy, workspace tests'
    Write-Status 'N/A' 'Flutter quality' 'plan: dependencies, format, analyze, tests'
    Write-Status 'N/A' 'Native tests' 'plan: run when dedicated suites exist'
    Write-Status 'N/A' 'License audit' 'plan: validate the reviewed dependency ledger'
    Write-Status 'N/A' 'Security audit' 'plan: cargo-audit when installed'
    Write-Status 'N/A' 'Protocol vectors' 'plan: run when a vector corpus exists'
    Write-Status 'OK' 'Test summary' 'test plan is valid'
    exit 0
}

function Invoke-Step {
    param(
        [string] $Label,
        [scriptblock] $Action
    )

    try {
        & $Action
        if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) {
            throw "command exited with $LASTEXITCODE"
        }
        Write-Status 'OK' $Label 'passed'
    }
    catch {
        Write-Status 'BLOCKER' $Label $_.Exception.Message
        $script:Blockers++
    }
}

$cargoManifest = Join-Path $RepoRoot 'Cargo.toml'
if (Test-Path -LiteralPath $cargoManifest) {
    Invoke-Step 'Rust quality' {
        Push-Location $RepoRoot
        try {
            & cargo fmt --all --check
            if ($LASTEXITCODE -ne 0) { throw 'cargo fmt failed' }
            & cargo clippy --workspace --all-targets -- -D warnings
            if ($LASTEXITCODE -ne 0) { throw 'cargo clippy failed' }
            & cargo test --workspace
            if ($LASTEXITCODE -ne 0) { throw 'cargo test failed' }
        }
        finally {
            Pop-Location
        }
    }
}
else {
    Write-Status 'BLOCKER' 'Rust quality' 'Cargo.toml not found'
    $script:Blockers++
}

$flutterRoot = Join-Path $RepoRoot 'apps/qyro'
$pubspec = Join-Path $flutterRoot 'pubspec.yaml'
if (Test-Path -LiteralPath $pubspec) {
    Invoke-Step 'Flutter quality' {
        Push-Location $flutterRoot
        try {
            & flutter pub get
            if ($LASTEXITCODE -ne 0) { throw 'flutter pub get failed' }
            & dart format --output=none --set-exit-if-changed .
            if ($LASTEXITCODE -ne 0) { throw 'dart format failed' }
            & flutter analyze
            if ($LASTEXITCODE -ne 0) { throw 'flutter analyze failed' }
            & flutter test
            if ($LASTEXITCODE -ne 0) { throw 'flutter test failed' }
        }
        finally {
            Pop-Location
        }
    }
}
else {
    Write-Status 'BLOCKER' 'Flutter quality' 'apps/qyro/pubspec.yaml not found'
    $script:Blockers++
}

Write-Status 'N/A' 'Native tests' 'no dedicated instrumentation, XCTest, or Windows test suite is configured'

$licenseLedger = Join-Path $RepoRoot 'docs/LICENSE_AUDIT.md'
if (-not (Test-Path -LiteralPath $licenseLedger)) {
    Write-Status 'BLOCKER' 'License audit' 'docs/LICENSE_AUDIT.md not found'
    $script:Blockers++
}
else {
    $ledger = Get-Content -LiteralPath $licenseLedger -Raw
    if (-not $ledger.Contains('| Dependencia |')) {
        Write-Status 'BLOCKER' 'License audit' 'dependency ledger table is missing'
        $script:Blockers++
    }
    elseif ($ledger -match '(?im)^\|.*\|\s*(GPL|AGPL|LGPL|MPL|UNKNOWN|DESCONOCIDA)[^|]*\|') {
        Write-Status 'BLOCKER' 'License audit' 'review-required dependency found in the ledger'
        $script:Blockers++
    }
    else {
        Write-Status 'OK' 'License audit' 'reviewed dependency ledger has no blocked entries'
    }
}

if (Get-Command 'cargo-audit' -ErrorAction SilentlyContinue) {
    Invoke-Step 'Security audit' {
        Push-Location $RepoRoot
        try {
            & cargo audit
        }
        finally {
            Pop-Location
        }
    }
}
else {
    Write-Status 'WARNING' 'Security audit' 'cargo-audit is not installed; advisory scan was not executed'
}

$vectorRoot = Join-Path $RepoRoot 'tests/protocol_vectors'
if (Test-Path -LiteralPath $vectorRoot -PathType Container) {
    Invoke-Step 'Protocol vectors' {
        Push-Location $RepoRoot
        try {
            & cargo test --workspace protocol_vector
        }
        finally {
            Pop-Location
        }
    }
}
else {
    Write-Status 'N/A' 'Protocol vectors' 'vector corpus is not implemented; QYRO/1 unit contract ran with Rust tests'
}

if ($script:Blockers -gt 0) {
    Write-Status 'BLOCKER' 'Test summary' "$($script:Blockers) required suite(s) failed"
    exit 1
}

Write-Status 'OK' 'Test summary' 'all available required suites passed'
exit 0
