$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$doctor = Join-Path $repoRoot 'scripts/doctor.ps1'

if (-not (Test-Path -LiteralPath $doctor)) {
    throw "Expected $doctor to exist."
}

function Assert-Contains {
    param(
        [string[]] $Output,
        [string] $Expected
    )

    $text = $Output -join [Environment]::NewLine
    if (-not $text.Contains($Expected)) {
        throw "Expected output to contain '$Expected'. Actual output:$([Environment]::NewLine)$text"
    }
}

$output = & $doctor
if ($LASTEXITCODE -ne 0) {
    throw "Expected doctor.ps1 to succeed in the configured CI environment."
}
Assert-Contains $output '[OK] Git'
Assert-Contains $output '[OK] Flutter'
Assert-Contains $output '[OK] Dart'
Assert-Contains $output '[OK] Rust'
Assert-Contains $output '[OK] Cargo'
Assert-Contains $output '[N/A] Xcode'
Assert-Contains $output '[N/A] Visual Studio Build Tools'

try {
    $env:QYRO_DOCTOR_SIMULATE_MISSING = 'fvm'
    $warningOutput = & $doctor
    if ($LASTEXITCODE -ne 0) {
        throw 'An optional missing tool must not fail the diagnostic.'
    }
    Assert-Contains $warningOutput '[WARNING] FVM'

    $env:QYRO_DOCTOR_SIMULATE_MISSING = 'git'
    $blockerOutput = & $doctor 2>&1
    if ($LASTEXITCODE -eq 0) {
        throw 'A required missing tool must return a non-zero exit code.'
    }
    Assert-Contains $blockerOutput '[BLOCKER] Git'
}
finally {
    Remove-Item Env:QYRO_DOCTOR_SIMULATE_MISSING -ErrorAction SilentlyContinue
}
