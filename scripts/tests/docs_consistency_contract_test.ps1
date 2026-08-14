#requires -Version 5.1
$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$checker = Join-Path $repoRoot 'scripts/check_docs_consistency.ps1'
$powerShellExecutable = (Get-Process -Id $PID).Path
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
'@ | Set-Content -LiteralPath (Join-Path $root 'STATUS.md') -Encoding UTF8
    foreach ($doc in @('AGENTS.md', 'PROJECT_CONTEXT.md', 'README.md', 'HANDOFF.md', 'TESTING.md')) {
        @('# Document', '', 'Current state: see STATUS.md.') | Set-Content -LiteralPath (Join-Path $root $doc) -Encoding UTF8
    }
    '{"owner":"REPLACE_WITH_OWNER"}' | Set-Content -LiteralPath (Join-Path $root 'config/branding.example.json') -Encoding UTF8
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
    (Get-Content -LiteralPath $path -Encoding UTF8) -replace '^- Verified commit:.*', "- Verified commit: $Value" |
        Set-Content -LiteralPath $path -Encoding UTF8
}

function Get-FixtureHead {
    param([string] $Root)
    return (& git -C $Root rev-parse HEAD).Trim()
}

function Assert-FailsWith {
    param([string] $Root, [string] $Expected)
    $output = & $powerShellExecutable -NoProfile -File $checker -RepoRoot $Root 2>&1
    if ($LASTEXITCODE -eq 0 -or -not (($output -join [Environment]::NewLine).Contains($Expected))) {
        throw "Expected failure containing '$Expected'. Output: $($output -join [Environment]::NewLine)"
    }
}

$fixtures = @()
try {
    $valid = New-Fixture; $fixtures += $valid
    $output = & $powerShellExecutable -NoProfile -File $checker -RepoRoot $valid 2>&1
    if ($LASTEXITCODE -ne 0 -or -not (($output -join [Environment]::NewLine).Contains('[OK] Documentation consistency'))) {
        throw "Valid fixture failed: $($output -join [Environment]::NewLine)"
    }
    $missing = New-Fixture; $fixtures += $missing
    (Get-Content (Join-Path $missing 'STATUS.md') -Encoding UTF8) | Where-Object { $_ -notmatch '^- Milestone:' } | Set-Content (Join-Path $missing 'STATUS.md') -Encoding UTF8
    Assert-FailsWith $missing '[BLOCKER] STATUS fields'
    $stale = New-Fixture; $fixtures += $stale
    Add-Content (Join-Path $stale 'README.md') 'Commit actual: 0000000000000000000000000000000000000000' -Encoding UTF8
    Assert-FailsWith $stale '[BLOCKER] Stale current commit'
    $scriptsPending = New-Fixture; $fixtures += $scriptsPending
    Add-Content (Join-Path $scriptsPending 'AGENTS.md') 'doctor, bootstrap and test_all are pending' -Encoding UTF8
    Assert-FailsWith $scriptsPending '[BLOCKER] AGENTS script state'
    $falseClaim = New-Fixture; $fixtures += $falseClaim
    Add-Content (Join-Path $falseClaim 'README.md') 'File transfer: implemented' -Encoding UTF8
    Assert-FailsWith $falseClaim '[BLOCKER] Pending capability claim'

    # Reserved ranges describe ownership, not findings. Their endpoints must
    # not require placeholder ledger records outside an agent's allocation.
    $rangeRefs = New-Fixture; $fixtures += $rangeRefs
    @('## QYR-0001 - fixture', '', '- Estado: cerrado') |
        Set-Content -LiteralPath (Join-Path $rangeRefs 'BUGS_PENDING.md') -Encoding UTF8
    $rangeStart = 'QYR-' + '0076'
    $rangeEnd = 'QYR-' + '0099'
    $rangeOnward = 'QYR-' + '0100'
    Add-Content (Join-Path $rangeRefs 'README.md') "Reserved: $rangeStart$([char]0x2013)$rangeEnd; this agent owns $rangeOnward onward." -Encoding UTF8
    $output = & $powerShellExecutable -NoProfile -File $checker -RepoRoot $rangeRefs 2>&1
    if ($LASTEXITCODE -ne 0 -or -not (($output -join [Environment]::NewLine).Contains('[OK] Documentation consistency'))) {
        throw "Reserved range fixture failed: $($output -join [Environment]::NewLine)"
    }

    # A concrete citation remains subject to the ledger rule.
    $concreteFinding = New-Fixture; $fixtures += $concreteFinding
    @('## QYR-0001 - fixture', '', '- Estado: cerrado') |
        Set-Content -LiteralPath (Join-Path $concreteFinding 'BUGS_PENDING.md') -Encoding UTF8
    $missingId = 'QYR-' + '0101'
    Add-Content (Join-Path $concreteFinding 'README.md') "$missingId is a concrete missing finding." -Encoding UTF8
    Assert-FailsWith $concreteFinding "$missingId is cited but has no entry"

    # ...and only inside the five declared extensions. This case exists because
    # the scope was not enforced at all: `-Include` beside `-LiteralPath` is
    # inert on Windows PowerShell 5.1, so the checker read `.txt`, `.o` and
    # `.exe` as documentation and blocked on a citation the Bash half could not
    # see (QYR-0311). The case is written the way it is on purpose -- the same
    # citation in an out-of-scope file and in an in-scope one, so a checker that
    # ignores extensions cannot satisfy both halves, and neither can a checker
    # that scans nothing.
    $outOfScope = New-Fixture; $fixtures += $outOfScope
    @('## QYR-0001 - fixture', '', '- Estado: cerrado') |
        Set-Content -LiteralPath (Join-Path $outOfScope 'BUGS_PENDING.md') -Encoding UTF8
    $outOfScopeId = 'QYR-' + '0102'
    Add-Content (Join-Path $outOfScope 'notes.txt') "$outOfScopeId is cited where the checker must not look." -Encoding UTF8
    $output = & $powerShellExecutable -NoProfile -File $checker -RepoRoot $outOfScope 2>&1
    if ($LASTEXITCODE -ne 0 -or -not (($output -join [Environment]::NewLine).Contains('[OK] Documentation consistency'))) {
        throw "A citation in an out-of-scope file must not block: $($output -join [Environment]::NewLine)"
    }
    # The other half of the same measurement: move that citation into a file the
    # checker does declare, and it must block. Without this, a checker that
    # scanned no files at all would pass the case above.
    Add-Content (Join-Path $outOfScope 'README.md') "$outOfScopeId is cited where the checker must look." -Encoding UTF8
    Assert-FailsWith $outOfScope "$outOfScopeId is cited but has no entry"

    # The ledger is an operational list, not an unbounded tool-output sink.
    # The boundary itself remains usable; the next open record must fail.
    $atOpenLimit = New-Fixture; $fixtures += $atOpenLimit
    $ledgerLines = [System.Collections.Generic.List[string]]::new()
    foreach ($index in 1..59) {
        $ledgerLines.Add("## QYR-$($index.ToString('0000')) - human-readable fixture $index")
        $ledgerLines.Add('')
        $ledgerLines.Add('- Estado: abierto')
        $ledgerLines.Add('')
    }
    $ledgerLines | Set-Content -LiteralPath (Join-Path $atOpenLimit 'BUGS_PENDING.md') -Encoding UTF8
    $output = & $powerShellExecutable -NoProfile -File $checker -RepoRoot $atOpenLimit 2>&1
    if ($LASTEXITCODE -ne 0 -or -not (($output -join [Environment]::NewLine).Contains('[OK] Documentation consistency'))) {
        throw "Ledger boundary fixture failed: $($output -join [Environment]::NewLine)"
    }

    $tooManyOpen = New-Fixture; $fixtures += $tooManyOpen
    $ledgerLines = [System.Collections.Generic.List[string]]::new()
    foreach ($index in 1..60) {
        $ledgerLines.Add("## QYR-$($index.ToString('0000')) - human-readable fixture $index")
        $ledgerLines.Add('')
        $ledgerLines.Add('- Estado: abierto')
        $ledgerLines.Add('')
    }
    $ledgerLines | Set-Content -LiteralPath (Join-Path $tooManyOpen 'BUGS_PENDING.md') -Encoding UTF8
    Assert-FailsWith $tooManyOpen '60 open findings exceed the ledger limit of 59'

    # A commit recorded one revision back is normal: STATUS cannot contain the SHA
    # of the very commit that introduces it.
    $fresh = New-GitFixture; $fixtures += $fresh
    Set-VerifiedCommit $fresh (Get-FixtureHead $fresh)
    Add-Content (Join-Path $fresh 'README.md') 'follow-up' -Encoding UTF8
    & git -C $fresh commit --quiet -am 'docs: follow-up'
    $output = & $powerShellExecutable -NoProfile -File $checker -RepoRoot $fresh 2>&1
    if ($LASTEXITCODE -ne 0 -or -not (($output -join [Environment]::NewLine).Contains('[OK] Documentation consistency'))) {
        throw "Fresh Git fixture failed: $($output -join [Environment]::NewLine)"
    }

    $drifted = New-GitFixture; $fixtures += $drifted
    Set-VerifiedCommit $drifted (Get-FixtureHead $drifted)
    foreach ($index in 1..12) {
        Add-Content (Join-Path $drifted 'README.md') "change $index" -Encoding UTF8
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
