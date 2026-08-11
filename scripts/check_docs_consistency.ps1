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

# ---------------------------------------------------------------- capability drift
#
# Twin of the Bash rules. A capability that exists in code but is denied in
# prose, or is still requested after it shipped, is how STATUS stopped being the
# source of truth three sprints running.

function Test-UnquotedClaim {
    # A document that quotes its own former wording while correcting it is doing
    # the right thing, so a claim inside guillemets does not count.
    param([string]$Content, [string]$Pattern)
    foreach ($line in ($Content -split "`n")) {
        if ($line -match $Pattern -and $line -notmatch '\u00ab') { return $true }
    }
    return $false
}

$handshakeModule = Join-Path $RepoRoot 'rust/crates/qyro_crypto/src/handshake/mod.rs'
if (Test-Path -LiteralPath $handshakeModule) {
    foreach ($doc in @('STATUS.md', 'SECURITY.md', 'THREAT_MODEL.md', 'PROTOCOL.md',
                       'ARCHITECTURE.md', 'rust/crates/qyro_crypto/src/lib.rs',
                       'docs/security/device-identity.md')) {
        $path = Join-Path $RepoRoot $doc
        if (-not (Test-Path -LiteralPath $path)) { continue }
        $content = Get-Content -LiteralPath $path -Raw
        if ($content -match '(?i)no (handshake|X25519|HKDF)|(sin|ni) handshake|no hay handshake|no existe handshake') {
            Write-Status 'BLOCKER' 'Capability drift' "$doc says there is no handshake, but rust/crates/qyro_crypto/src/handshake exists"
            $blockers++
        }
    }

    $nextSteps = Join-Path $RepoRoot 'NEXT_STEPS.md'
    if (Test-Path -LiteralPath $nextSteps) {
        $pending = (Get-Content -LiteralPath $nextSteps -Raw) -split '(?m)^## Completado' | Select-Object -First 1
        if ($pending -match '(?i)implementar el handshake|implement the handshake') {
            Write-Status 'BLOCKER' 'Capability drift' 'NEXT_STEPS.md still asks for the handshake, which is implemented'
            $blockers++
        }
    }

    $nextTask = ($status -split '(?m)^## Next task' | Select-Object -Skip 1 | Select-Object -First 1)
    if ($nextTask -and (($nextTask -split '(?m)^## ' | Select-Object -First 1) -match '(?i)implementar el handshake|implement the handshake')) {
        Write-Status 'BLOCKER' 'Capability drift' 'STATUS.md still lists the handshake as the next task'
        $blockers++
    }
}

# --------------------------------------------------------------- vector claims
foreach ($entry in @(
    @{ File = 'identity-v1.json'; Pattern = 'identidad|identity' },
    @{ File = 'handshake-v1.json'; Pattern = 'handshake' }
)) {
    $vector = Join-Path $RepoRoot "docs/security/test-vectors/$($entry.File)"
    if ($status -match "(?i)vectores? (de|del|interoperables? del)? ?($($entry.Pattern)).*: ?IMPLEMENTED") {
        if (-not (Test-Path -LiteralPath $vector)) {
            Write-Status 'BLOCKER' 'Vector claim' "STATUS.md marks $($entry.Pattern) vectors implemented but $($entry.File) is missing"
            $blockers++
        }
    }
}
if ((Test-Path -LiteralPath (Join-Path $RepoRoot 'docs/security/test-vectors/handshake-v1.json')) -and
    -not (Test-Path -LiteralPath (Join-Path $RepoRoot 'docs/security/test-vectors/handshake-v1.schema.json'))) {
    Write-Status 'BLOCKER' 'Vector claim' 'handshake-v1.json has no committed schema'
    $blockers++
}

# ------------------------------------------------------------- unicode folding
$pathRs = Join-Path $RepoRoot 'rust/crates/qyro_manifest/src/path.rs'
if ((Test-Path -LiteralPath $pathRs) -and ((Get-Content -LiteralPath $pathRs -Raw) -match 'unicode_normalization')) {
    foreach ($doc in @('docs/protocols/manifest-format.md', 'docs/security/parser-threats.md')) {
        $path = Join-Path $RepoRoot $doc
        if (-not (Test-Path -LiteralPath $path)) { continue }
        if (Test-UnquotedClaim (Get-Content -LiteralPath $path -Raw) '(?i)(pliega|folds|plegado de)[^.]*(ASCII|Latin-1)') {
            Write-Status 'BLOCKER' 'Folding claim' "$doc describes folding as ASCII/Latin-1 while path.rs uses unicode-normalization"
            $blockers++
        }
    }
}

# ------------------------------------------------------------ dependency claims
$lock = Join-Path $RepoRoot 'Cargo.lock'
if ((Test-Path -LiteralPath $lock) -and ((Get-Content -LiteralPath $lock -Raw) -match 'name = "ed25519-dalek"')) {
    foreach ($doc in @('SECURITY.md', 'STATUS.md')) {
        $path = Join-Path $RepoRoot $doc
        if (-not (Test-Path -LiteralPath $path)) { continue }
        $content = Get-Content -LiteralPath $path -Raw
        if (Test-UnquotedClaim $content '(?i)no tiene dependencias externas|sin dependencias externas|cero dependencias externas') {
            Write-Status 'BLOCKER' 'Dependency claim' "$doc says the workspace has no external dependencies, but Cargo.lock has ed25519-dalek"
            $blockers++
        }
        if ($content -match '(?i)no hay KAT|Tampoco hay KAT|sin KAT') {
            Write-Status 'BLOCKER' 'Dependency claim' "$doc says there are no cryptographic KATs; RFC 7748/8032/4231 vectors are committed"
            $blockers++
        }
    }
}

# ------------------------------------------------------------- finding ledger
#
# See the Bash half. A concrete identifier with no entry is a finding whose
# state nobody can look up (QYR-0043). Ownership ranges are not findings: their
# endpoints must not require placeholder records in somebody else's allocation
# (QYR-0100).
$ledger = Join-Path $RepoRoot 'BUGS_PENDING.md'
if (Test-Path -LiteralPath $ledger) {
    $recorded = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($line in (Get-Content -LiteralPath $ledger)) {
        if ($line -match '^##\s+(QYR-[0-9]{4})') { [void]$recorded.Add($Matches[1]) }
    }
    $cited = [System.Collections.Generic.HashSet[string]]::new()
    $extensions = @('*.md', '*.rs', '*.sh', '*.ps1', '*.yml')
    foreach ($file in (Get-ChildItem -LiteralPath $RepoRoot -Recurse -File -Include $extensions -ErrorAction SilentlyContinue)) {
        # `Get-Content -Raw` returns $null for an empty file. The fixtures keep
        # intentionally empty scripts, and range normalisation must treat those
        # as empty text rather than passing null to Regex.Replace.
        $content = [string](Get-Content -LiteralPath $file.FullName -Raw)
        $content = [regex]::Replace($content, 'QYR-[0-9]{4}\s*[-–—]\s*QYR-[0-9]{4}', '')
        $content = [regex]::Replace($content, 'QYR-[0-9]{4}\s+(?:onward|onwards|en adelante)', '')
        $content = [regex]::Replace($content, 'QYR-[0-9]{4}\+', '')
        foreach ($found in ([regex]::Matches($content, 'QYR-[0-9]{4}'))) {
            [void]$cited.Add($found.Value)
        }
    }
    foreach ($finding in ($cited | Sort-Object)) {
        if (-not $recorded.Contains($finding)) {
            Write-Status 'BLOCKER' 'Finding ledger' "$finding is cited but has no entry in BUGS_PENDING.md"
            $blockers++
        }
    }

    # ...and exactly one. `$recorded` is a HashSet, so it collapses a repeated
    # identifier exactly as the Bash half's `sort -u` does, and neither could
    # see that QYR-0036 was in the ledger twice with two different states
    # (QYR-0046). Counted here from the file rather than from the set.
    $headingCounts = @{}
    foreach ($line in (Get-Content -LiteralPath $ledger)) {
        if ($line -match '^##\s+(QYR-[0-9]{4})') {
            $id = $Matches[1]
            if ($headingCounts.ContainsKey($id)) { $headingCounts[$id]++ } else { $headingCounts[$id] = 1 }
        }
    }
    foreach ($id in ($headingCounts.Keys | Sort-Object)) {
        if ($headingCounts[$id] -gt 1) {
            Write-Status 'BLOCKER' 'Finding ledger' `
                "$id has $($headingCounts[$id]) entries in BUGS_PENDING.md; a finding has one state, not two"
            $blockers++
        }
    }
}

# --------------------------------------------------- workflow branch triggers
#
# See the Bash half for the reasoning. `main` is exempt because it is not a
# working branch; anything containing `*` is a pattern. A `branches:` form this
# check cannot read fails rather than passing.
$workflowDir = Join-Path (Join-Path $RepoRoot '.github') 'workflows'
if (Test-Path -LiteralPath $workflowDir) {
    foreach ($workflow in (Get-ChildItem -LiteralPath $workflowDir -Filter '*.yml' -File)) {
        foreach ($branchLine in (Get-Content -LiteralPath $workflow.FullName)) {
            if ($branchLine -notmatch '^\s*branches:') { continue }
            if ($branchLine -notmatch '^\s*branches:\s*\[(.*)\]') {
                Write-Status 'BLOCKER' 'Workflow branch trigger' "$($workflow.Name) uses a branches: form this check cannot read; write an inline list"
                $blockers++
                continue
            }
            foreach ($entry in ($Matches[1] -split ',')) {
                $entry = $entry.Trim().Trim("'").Trim('"')
                if ([string]::IsNullOrWhiteSpace($entry)) { continue }
                if ($entry -eq 'main') { continue }
                if ($entry.Contains('*')) { continue }
                Write-Status 'BLOCKER' 'Workflow branch trigger' "$($workflow.Name) names the branch '$entry' literally; use a pattern such as 'claude/**' so a new working branch needs no YAML edit"
                $blockers++
            }
        }
    }
}

# --------------------------------------------------------- platform evidence
$platformSection = ($status -split '(?m)^## Platforms executed' | Select-Object -Skip 1 | Select-Object -First 1)
if ($platformSection) {
    $platformSection = ($platformSection -split '(?m)^## ' | Select-Object -First 1)
    foreach ($line in ($platformSection -split "`n")) {
        if ($line -notmatch '^\s*-') { continue }
        # Whole word only: a case-insensitive substring match reads the "si"
        # inside "fisico" as an affirmative.
        if ($line -notmatch '\bYES\b') { continue }
        if ($line -match 'run [0-9]{6,}|[0-9]{9,}') { continue }
        if ($line -match '(?i)host local|esta sesi|this session') { continue }
        $label = ($line -replace '^\s*-\s*([^:]*):.*', '$1')
        Write-Status 'BLOCKER' 'Platform evidence' "STATUS.md marks '$label' executed without a run id"
        $blockers++
    }
}

if ($blockers -gt 0) {
    Write-Status 'BLOCKER' 'Documentation consistency' "$blockers inconsistency finding(s)"
    exit 1
}
Write-Status 'OK' 'Documentation consistency' 'STATUS.md and canonical references agree'
exit 0
