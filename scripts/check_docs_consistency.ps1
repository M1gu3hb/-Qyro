param([string] $RepoRoot = (Join-Path $PSScriptRoot '..'))
$ErrorActionPreference = 'Stop'
$blockers = 0

function Write-Status {
    param([string] $Kind, [string] $Label, [string] $Detail)
    Write-Output "[$Kind] $($Label): $Detail"
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    Write-Status 'BLOCKER' 'Repository' "$RepoRoot does not exist"
    exit 1
}
$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
$statusPath = Join-Path $RepoRoot 'STATUS.md'
if (-not (Test-Path -LiteralPath $statusPath)) {
    Write-Status 'BLOCKER' 'STATUS fields' 'STATUS.md is missing'
    exit 1
}
$status = Get-Content -LiteralPath $statusPath -Raw
$required = @(
    '(?m)^- Updated UTC:',
    '(?m)^- Branch:',
    '(?m)^- Verified commit:',
    '(?m)^- Milestone:',
    '(?m)^## Implemented$',
    '(?m)^## Not implemented$',
    '(?m)^## Platforms compiled$',
    '(?m)^## Platforms executed$',
    '(?m)^## Real tests$',
    '(?m)^## Artifacts$',
    '(?m)^## Blockers$',
    '(?m)^## Next task$',
    '(?m)^## Provisional values$'
)
$missing = @($required | Where-Object { $status -notmatch $_ })
if ($missing.Count -gt 0) {
    Write-Status 'BLOCKER' 'STATUS fields' "missing $($missing.Count) required field(s)"
    $blockers++
}

# STATUS.md is the canonical executable state, so a verified commit that no longer
# tracks HEAD silently invalidates every claim in it. STATUS cannot name the commit
# that introduces it, so a small lead is expected; a large one means real drift.
$maxStatusCommitLag = 10
if ($env:QYRO_MAX_STATUS_COMMIT_LAG) {
    $maxStatusCommitLag = [int] $env:QYRO_MAX_STATUS_COMMIT_LAG
}

function Invoke-Git {
    param([string[]] $Arguments)
    $output = & git @Arguments 2>$null
    return [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output   = ($output | Out-String).Trim()
    }
}

$verifiedMatch = [regex]::Match($status, '(?m)^- Verified commit:\s*(\S*)\s*$')
$verifiedCommit = if ($verifiedMatch.Success) { $verifiedMatch.Groups[1].Value } else { '' }

if ([string]::IsNullOrEmpty($verifiedCommit)) {
    Write-Status 'BLOCKER' 'Malformed verified commit' 'STATUS.md does not record a verified commit'
    $blockers++
} elseif ($verifiedCommit -notmatch '^[0-9a-f]{40}$') {
    Write-Status 'BLOCKER' 'Malformed verified commit' "$verifiedCommit is not a full 40-character SHA"
    $blockers++
} elseif ((Invoke-Git @('-C', $RepoRoot, 'rev-parse', '--is-inside-work-tree')).ExitCode -ne 0) {
    Write-Status 'SKIP' 'Verified commit freshness' "$RepoRoot is not a Git work tree"
} elseif ((Invoke-Git @('-C', $RepoRoot, 'rev-parse', '--is-shallow-repository')).Output -eq 'true') {
    Write-Status 'SKIP' 'Verified commit freshness' 'shallow clone cannot prove reachability'
} elseif ((Invoke-Git @('-C', $RepoRoot, 'cat-file', '-e', "$verifiedCommit^{commit}")).ExitCode -ne 0) {
    Write-Status 'BLOCKER' 'Unknown verified commit' "$verifiedCommit is not a commit in this repository"
    $blockers++
} elseif ((Invoke-Git @('-C', $RepoRoot, 'merge-base', '--is-ancestor', $verifiedCommit, 'HEAD')).ExitCode -ne 0) {
    Write-Status 'BLOCKER' 'Unknown verified commit' "$verifiedCommit is not reachable from HEAD"
    $blockers++
} else {
    $lagResult = Invoke-Git @('-C', $RepoRoot, 'rev-list', '--count', "$verifiedCommit..HEAD")
    $lag = if ($lagResult.ExitCode -eq 0 -and $lagResult.Output) { [int] $lagResult.Output } else { 0 }
    if ($lag -gt $maxStatusCommitLag) {
        Write-Status 'BLOCKER' 'Stale verified commit' "HEAD is $lag commits ahead of the verified commit (limit $maxStatusCommitLag)"
        $blockers++
    }
}

$canonicalDocs = @('AGENTS.md', 'PROJECT_CONTEXT.md', 'README.md', 'HANDOFF.md', 'TESTING.md')
foreach ($doc in $canonicalDocs) {
    $path = Join-Path $RepoRoot $doc
    if (-not (Test-Path -LiteralPath $path) -or -not (Select-String -LiteralPath $path -SimpleMatch 'STATUS.md' -Quiet)) {
        Write-Status 'BLOCKER' 'Canonical reference' "$doc must point to STATUS.md"
        $blockers++
        continue
    }
    $content = Get-Content -LiteralPath $path -Raw
    if ($content -match '(?i)(commit (actual|current|verificado|comprobado)|current commit)[^0-9a-f]*[0-9a-f]{40}') {
        Write-Status 'BLOCKER' 'Stale current commit' "$doc declares a current commit outside STATUS.md"
        $blockers++
    }
}

$agentsPath = Join-Path $RepoRoot 'AGENTS.md'
$requiredScripts = @('doctor.sh', 'bootstrap.sh', 'test_all.sh') |
    ForEach-Object { Test-Path -LiteralPath (Join-Path $RepoRoot "scripts/$_") }
if (($requiredScripts -notcontains $false) -and (Test-Path -LiteralPath $agentsPath)) {
    $agents = Get-Content -LiteralPath $agentsPath -Raw
    if ($agents -match '(?i)((doctor|bootstrap|test_all).*(pending|pendiente)|(pending|pendiente).*(doctor|bootstrap|test_all))') {
        Write-Status 'BLOCKER' 'AGENTS script state' 'existing scripts are described as pending'
        $blockers++
    }
}

foreach ($doc in @('PROJECT_CONTEXT.md', 'README.md', 'HANDOFF.md', 'TESTING.md')) {
    $path = Join-Path $RepoRoot $doc
    if (-not (Test-Path -LiteralPath $path)) { continue }
    $content = Get-Content -LiteralPath $path -Raw
    if ($content -match '(?i)(file transfer|transferencia de archivos)\s*:\s*(implemented|complete|ready|implementada|completa|lista)') {
        Write-Status 'BLOCKER' 'Pending capability claim' "$doc marks file transfer implemented"
        $blockers++
    }
}

$textFiles = Get-ChildItem -LiteralPath $RepoRoot -Recurse -File |
    Where-Object {
        $_.FullName -notmatch '[\\/](\.git|build|target)[\\/]' -and
        $_.Extension -in @('.md', '.json', '.yaml', '.yml', '.dart', '.toml', '.gradle', '.kts', '.sh', '.ps1')
    }
$allText = ($textFiles | ForEach-Object { Get-Content -LiteralPath $_.FullName -Raw -ErrorAction SilentlyContinue }) -join [Environment]::NewLine
if ($allText.Contains('REPLACE_WITH_') -and -not $status.Contains('REPLACE_WITH_')) {
    Write-Status 'BLOCKER' 'Provisional markers' 'REPLACE_WITH_* exists but is absent from STATUS.md'
    $blockers++
}
if ($allText.Contains('com.owner.qyro') -and -not $status.Contains('com.owner.qyro')) {
    Write-Status 'BLOCKER' 'Provisional markers' 'com.owner.qyro exists but is absent from STATUS.md'
    $blockers++
}

if ($blockers -gt 0) {
    Write-Status 'BLOCKER' 'Documentation consistency' "$blockers inconsistency finding(s)"
    exit 1
}
Write-Status 'OK' 'Documentation consistency' 'STATUS.md and canonical references agree'
exit 0
