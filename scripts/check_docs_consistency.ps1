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
