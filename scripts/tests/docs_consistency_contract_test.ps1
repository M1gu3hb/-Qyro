$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$checker = Join-Path $repoRoot 'scripts/check_docs_consistency.ps1'
if (-not (Test-Path -LiteralPath $checker)) { throw "Expected $checker to exist." }

function New-Fixture {
    $root = Join-Path ([IO.Path]::GetTempPath()) "qyro-docs-$([Guid]::NewGuid())"
    New-Item -ItemType Directory -Path (Join-Path $root 'scripts') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $root 'config') -Force | Out-Null
    foreach ($script in @('doctor.sh', 'bootstrap.sh', 'test_all.sh')) {
        New-Item -ItemType File -Path (Join-Path $root "scripts/$script") | Out-Null
    }
    @'
# Canonical project status
- Updated UTC: 2026-08-04T19:55:00Z
- Branch: audit/baseline-hardening
- Verified commit: CURRENT_HEAD
- Milestone: Hito 0 cerrado; Hito 1 en hardening
## Implemented
- Native bridge: IMPLEMENTED
## Not implemented
- File transfer: NOT_IMPLEMENTED
## Platforms compiled
- Android, iOS, Windows
## Platforms executed
- Linux CI, Windows test
## Real tests
- Baseline CI
## Artifacts
- None retained yet
## Blockers
- Android runtime ABI
## Next task
- Android runtime ABI
## Provisional values
- REPLACE_WITH_OWNER
- com.owner.qyro
'@ | Set-Content -LiteralPath (Join-Path $root 'STATUS.md')
    foreach ($doc in @('AGENTS.md', 'PROJECT_CONTEXT.md', 'README.md', 'HANDOFF.md', 'TESTING.md')) {
        @('# Document', '', 'Current state: see STATUS.md.') | Set-Content -LiteralPath (Join-Path $root $doc)
    }
    '{"owner":"REPLACE_WITH_OWNER"}' | Set-Content -LiteralPath (Join-Path $root 'config/branding.example.json')
    return $root
}

function Assert-FailsWith {
    param([string] $Root, [string] $Expected)
    $output = & pwsh -NoProfile -File $checker -RepoRoot $Root 2>&1
    if ($LASTEXITCODE -eq 0 -or -not (($output -join [Environment]::NewLine).Contains($Expected))) {
        throw "Expected failure containing '$Expected'. Output: $($output -join [Environment]::NewLine)"
    }
}

$fixtures = @()
try {
    $valid = New-Fixture; $fixtures += $valid
    $output = & pwsh -NoProfile -File $checker -RepoRoot $valid 2>&1
    if ($LASTEXITCODE -ne 0 -or -not (($output -join [Environment]::NewLine).Contains('[OK] Documentation consistency'))) {
        throw "Valid fixture failed: $($output -join [Environment]::NewLine)"
    }
    $missing = New-Fixture; $fixtures += $missing
    (Get-Content (Join-Path $missing 'STATUS.md')) | Where-Object { $_ -notmatch '^- Milestone:' } | Set-Content (Join-Path $missing 'STATUS.md')
    Assert-FailsWith $missing '[BLOCKER] STATUS fields'
    $stale = New-Fixture; $fixtures += $stale
    Add-Content (Join-Path $stale 'README.md') 'Commit actual: 0000000000000000000000000000000000000000'
    Assert-FailsWith $stale '[BLOCKER] Stale current commit'
    $scriptsPending = New-Fixture; $fixtures += $scriptsPending
    Add-Content (Join-Path $scriptsPending 'AGENTS.md') 'doctor, bootstrap and test_all are pending'
    Assert-FailsWith $scriptsPending '[BLOCKER] AGENTS script state'
    $falseClaim = New-Fixture; $fixtures += $falseClaim
    Add-Content (Join-Path $falseClaim 'README.md') 'File transfer: implemented'
    Assert-FailsWith $falseClaim '[BLOCKER] Pending capability claim'
}
finally {
    foreach ($fixture in $fixtures) {
        if (Test-Path -LiteralPath $fixture) { Remove-Item -LiteralPath $fixture -Recurse -Force }
    }
}
