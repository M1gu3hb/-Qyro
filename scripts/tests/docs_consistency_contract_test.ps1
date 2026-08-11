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
- Verified commit: 7ca3973cd1928ffaa3e7b112d121587d83d5092c
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

# STATUS.md drifted 58 commits behind audit/baseline-hardening without any check
# noticing, because only the field layout was validated. These fixtures pin the
# freshness rule: the verified commit must be reachable from HEAD and close to it.
function New-GitFixture {
    $root = New-Fixture
    & git -C $root init --quiet --initial-branch=main
    & git -C $root config user.email 'contract@qyro.test'
    & git -C $root config user.name 'Qyro Contract'
    & git -C $root add -A
    & git -C $root commit --quiet -m 'chore: fixture baseline'
    return $root
}

function Set-VerifiedCommit {
    param([string] $Root, [string] $Value)
    $path = Join-Path $Root 'STATUS.md'
    (Get-Content -LiteralPath $path) -replace '^- Verified commit:.*', "- Verified commit: $Value" |
        Set-Content -LiteralPath $path
}

function Get-FixtureHead {
    param([string] $Root)
    return (& git -C $Root rev-parse HEAD).Trim()
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

    # Reserved ranges describe ownership, not findings. Their endpoints must
    # not require placeholder ledger records outside an agent's allocation.
    $rangeRefs = New-Fixture; $fixtures += $rangeRefs
    @('## QYR-0001 — fixture', '', '- Estado: cerrado') |
        Set-Content -LiteralPath (Join-Path $rangeRefs 'BUGS_PENDING.md')
    Add-Content (Join-Path $rangeRefs 'README.md') 'Reserved: QYR-0076–QYR-0099; this agent owns QYR-0100 onward.'
    $output = & pwsh -NoProfile -File $checker -RepoRoot $rangeRefs 2>&1
    if ($LASTEXITCODE -ne 0 -or -not (($output -join [Environment]::NewLine).Contains('[OK] Documentation consistency'))) {
        throw "Reserved range fixture failed: $($output -join [Environment]::NewLine)"
    }

    # A concrete citation remains subject to the ledger rule.
    $concreteFinding = New-Fixture; $fixtures += $concreteFinding
    @('## QYR-0001 — fixture', '', '- Estado: cerrado') |
        Set-Content -LiteralPath (Join-Path $concreteFinding 'BUGS_PENDING.md')
    $missingId = 'QYR-' + '0101'
    Add-Content (Join-Path $concreteFinding 'README.md') "$missingId is a concrete missing finding."
    Assert-FailsWith $concreteFinding "$missingId is cited but has no entry"

    # A commit recorded one revision back is normal: STATUS cannot contain the SHA
    # of the very commit that introduces it.
    $fresh = New-GitFixture; $fixtures += $fresh
    Set-VerifiedCommit $fresh (Get-FixtureHead $fresh)
    Add-Content (Join-Path $fresh 'README.md') 'follow-up'
    & git -C $fresh commit --quiet -am 'docs: follow-up'
    $output = & pwsh -NoProfile -File $checker -RepoRoot $fresh 2>&1
    if ($LASTEXITCODE -ne 0 -or -not (($output -join [Environment]::NewLine).Contains('[OK] Documentation consistency'))) {
        throw "Fresh Git fixture failed: $($output -join [Environment]::NewLine)"
    }

    $drifted = New-GitFixture; $fixtures += $drifted
    Set-VerifiedCommit $drifted (Get-FixtureHead $drifted)
    foreach ($index in 1..12) {
        Add-Content (Join-Path $drifted 'README.md') "change $index"
        & git -C $drifted commit --quiet -am "chore: change $index"
    }
    Assert-FailsWith $drifted '[BLOCKER] Stale verified commit'

    $unreachable = New-GitFixture; $fixtures += $unreachable
    Set-VerifiedCommit $unreachable '0123456789abcdef0123456789abcdef01234567'
    Assert-FailsWith $unreachable '[BLOCKER] Unknown verified commit'

    $malformed = New-GitFixture; $fixtures += $malformed
    Set-VerifiedCommit $malformed 'not-a-sha'
    Assert-FailsWith $malformed '[BLOCKER] Malformed verified commit'
}
finally {
    foreach ($fixture in $fixtures) {
        if (Test-Path -LiteralPath $fixture) { Remove-Item -LiteralPath $fixture -Recurse -Force }
    }
}
