$ErrorActionPreference = 'Stop'

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$checker = Join-Path $repositoryRoot 'scripts\verify_windows_package.ps1'
if (-not (Test-Path -LiteralPath $checker -PathType Leaf)) {
    throw '[BLOCKER] Missing scripts/verify_windows_package.ps1'
}

$fixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('qyro-windows-contract-' + [guid]::NewGuid())
$bundle = Join-Path $fixtureRoot 'bundle'
$headers = Join-Path $fixtureRoot 'headers.txt'
$exports = Join-Path $fixtureRoot 'exports.txt'

function Invoke-Checker {
    & $checker -BundlePath $bundle -HeadersTextPath $headers -ExportsTextPath $exports
}

function Assert-Blocked {
    param(
        [Parameter(Mandatory)]
        [string] $ExpectedPattern
    )

    try {
        Invoke-Checker
        throw "Expected checker to fail with: $ExpectedPattern"
    }
    catch {
        if ($_.Exception.Message -notmatch $ExpectedPattern) {
            throw "Unexpected error: $($_.Exception.Message)"
        }
    }
}

try {
    New-Item -ItemType Directory -Path $bundle -Force | Out-Null
    New-Item -ItemType File -Path (Join-Path $bundle 'qyro.exe') -Force | Out-Null
    Set-Content -LiteralPath $headers -Value '8664 machine (x64)'
    Set-Content -LiteralPath $exports -Value @(
        'qyro_protocol_version_ptr'
        'qyro_protocol_version_len'
    )

    Assert-Blocked -ExpectedPattern '\[BLOCKER\] Missing qyro_ffi\.dll'

    New-Item -ItemType File -Path (Join-Path $bundle 'qyro_ffi.dll') -Force | Out-Null
    Set-Content -LiteralPath $headers -Value '14C machine (x86)'
    Assert-Blocked -ExpectedPattern '\[BLOCKER\] qyro_ffi\.dll is not x64'

    Set-Content -LiteralPath $headers -Value '8664 machine (x64)'
    Set-Content -LiteralPath $exports -Value 'qyro_protocol_version_ptr'
    Assert-Blocked -ExpectedPattern '\[BLOCKER\] Missing export qyro_protocol_version_len'

    Set-Content -LiteralPath $exports -Value @(
        'qyro_protocol_version_ptr'
        'qyro_protocol_version_len'
    )
    Invoke-Checker
    Write-Host '[PASS] Windows package verification contracts'
}
finally {
    if (Test-Path -LiteralPath $fixtureRoot) {
        Remove-Item -LiteralPath $fixtureRoot -Recurse -Force
    }
}
