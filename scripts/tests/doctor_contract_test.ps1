$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$doctor = Join-Path $repoRoot 'scripts/doctor.ps1'

if (-not (Test-Path -LiteralPath $doctor)) {
    throw "Expected $doctor to exist."
}

function Invoke-Doctor {
    param([string] $SimulateMissing)

    try {
        if ($SimulateMissing) {
            $env:QYRO_DOCTOR_SIMULATE_MISSING = $SimulateMissing
        }
        else {
            Remove-Item Env:QYRO_DOCTOR_SIMULATE_MISSING -ErrorAction SilentlyContinue
        }

        $output = & pwsh -NoProfile -File $doctor 2>&1
        [PSCustomObject]@{
            ExitCode = $LASTEXITCODE
            Output = [string[]] $output
        }
    }
    finally {
        Remove-Item Env:QYRO_DOCTOR_SIMULATE_MISSING -ErrorAction SilentlyContinue
    }
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

$normal = Invoke-Doctor
if ($normal.ExitCode -ne 0) {
    throw "Expected doctor.ps1 to succeed in the configured CI environment."
}
Assert-Contains $normal.Output '[OK] Git'
Assert-Contains $normal.Output '[OK] Flutter'
Assert-Contains $normal.Output '[OK] Dart'
Assert-Contains $normal.Output '[OK] Rust'
Assert-Contains $normal.Output '[OK] Cargo'
Assert-Contains $normal.Output '[N/A] Xcode'
Assert-Contains $normal.Output '[N/A] Visual Studio Build Tools'

$warning = Invoke-Doctor -SimulateMissing 'fvm'
if ($warning.ExitCode -ne 0) {
    throw 'An optional missing tool must not fail the diagnostic.'
}
Assert-Contains $warning.Output '[WARNING] FVM'

$blocker = Invoke-Doctor -SimulateMissing 'git'
if ($blocker.ExitCode -eq 0) {
    throw 'A required missing tool must return a non-zero exit code.'
}
Assert-Contains $blocker.Output '[BLOCKER] Git'
