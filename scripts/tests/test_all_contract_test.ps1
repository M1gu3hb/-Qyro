$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$testAll = Join-Path $repoRoot 'scripts/test_all.ps1'

if (-not (Test-Path -LiteralPath $testAll)) {
    throw "Expected $testAll to exist."
}

$output = & pwsh -NoProfile -File $testAll -RepoRoot $repoRoot -Plan 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "test_all.ps1 plan failed: $($output -join [Environment]::NewLine)"
}
$text = $output -join [Environment]::NewLine
foreach ($expected in @(
    '[N/A] Rust quality',
    '[N/A] Flutter quality',
    '[N/A] Native tests',
    '[N/A] License audit',
    '[N/A] Security audit',
    '[N/A] Protocol vectors',
    '[OK] Test summary'
)) {
    if (-not $text.Contains($expected)) {
        throw "Expected output to contain '$expected'. Actual output:$([Environment]::NewLine)$text"
    }
}

$missing = & pwsh -NoProfile -File $testAll -RepoRoot (Join-Path $repoRoot 'does-not-exist') -Plan 2>&1
if ($LASTEXITCODE -eq 0 -or -not (($missing -join [Environment]::NewLine).Contains('[BLOCKER] Repository'))) {
    throw 'Missing repository roots must be blockers.'
}
