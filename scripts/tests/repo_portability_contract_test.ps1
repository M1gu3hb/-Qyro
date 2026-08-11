#requires -Version 5.1
# Twin of scripts/tests/repo_portability_contract_test.sh.

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path (Join-Path $PSScriptRoot '..') '..')).Path
$checker = Join-Path $repoRoot 'scripts/check_repo_portability.ps1'
$powerShellExecutable = (Get-Process -Id $PID).Path
if (-not (Test-Path -LiteralPath $checker)) {
    Write-Error "Expected $checker to exist."
    exit 1
}

$failures = 0

function New-Fixture {
    $root = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $root | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $root 'docs') | Out-Null
    Set-Content -LiteralPath (Join-Path $root 'docs/readme.md') -Value 'ok'
    & git -C $root init --quiet
    & git -C $root config user.email 'test@example.invalid'
    & git -C $root config user.name 'Contract test'
    # Hostile names live only in the index. Disable Git for Windows' early
    # refusal so the checker itself is the component the fixture exercises.
    & git -C $root config core.protectNTFS false
    & git -C $root add -A
    & git -C $root commit --quiet -m 'fixture'
    return $root
}

# Adds a tracked path through the index rather than the filesystem, so the
# hostile names can be tested on a host that cannot create them — which on
# Windows is exactly the set of names under test.
function Add-TrackedPath {
    param([string]$Root, [string]$Path)
    $blob = ('x' | & git -C $Root hash-object -w --stdin)
    & git -C $Root update-index --add --cacheinfo "100644,$blob,$Path"
}

function Assert-Rejects {
    param([string]$Path, [string]$Expected)
    $root = New-Fixture
    try {
        Add-TrackedPath -Root $root -Path $Path
        $output = & $powerShellExecutable -NoProfile -File $checker -RepoRoot $root 2>&1 | Out-String
        if ($LASTEXITCODE -eq 0) {
            Write-Host "FAIL: $Path must be rejected, but the checker passed"
            $script:failures++
        }
        elseif ($output -notmatch [regex]::Escape($Expected)) {
            Write-Host "FAIL: $Path was rejected without naming the reason ($Expected)"
            Write-Host $output
            $script:failures++
        }
        else {
            Write-Host "ok: rejects $Path"
        }
    }
    finally {
        Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Assert-Accepts {
    param([string]$Path)
    $root = New-Fixture
    try {
        Add-TrackedPath -Root $root -Path $Path
        & $powerShellExecutable -NoProfile -File $checker -RepoRoot $root *> $null
        if ($LASTEXITCODE -ne 0) {
            Write-Host "FAIL: $Path is portable and must be accepted"
            $script:failures++
        }
        else {
            Write-Host "ok: accepts $Path"
        }
    }
    finally {
        Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# The exact path that broke the Windows job.
Assert-Rejects 'rust/fuzz/corpus/relative_path/nul.txt' 'reserved Windows device name'
Assert-Rejects 'nul' 'reserved'
Assert-Rejects 'docs/CON.md' 'reserved'
Assert-Rejects 'a/com1.bin' 'reserved'
Assert-Rejects 'docs/lpt9.txt' 'reserved'
Assert-Rejects 'docs/a:b.md' 'forbid'
Assert-Rejects 'docs/what?.md' 'forbid'
Assert-Rejects 'docs/trailing ' 'strip'
Assert-Rejects 'docs/trailing.' 'strip'
Assert-Rejects 'docs/dir./file.md' 'strip'

# Names that only look reserved must still be accepted.
Assert-Accepts 'docs/console.md'
Assert-Accepts 'docs/com10.txt'
Assert-Accepts 'docs/nul_byte.txt'
Assert-Accepts 'docs/conf.md'
Assert-Accepts 'docs/release notes.md'
Assert-Accepts 'docs/trailing .md'
Assert-Accepts 'rust/fuzz/corpus/relative_path/reserved_con.txt'

& $powerShellExecutable -NoProfile -File $checker -RepoRoot $repoRoot *> $null
if ($LASTEXITCODE -ne 0) {
    Write-Host 'FAIL: the repository itself has a path Windows cannot check out'
    & $powerShellExecutable -NoProfile -File $checker -RepoRoot $repoRoot
    $failures++
}
else {
    Write-Host 'ok: the repository itself is checkout-clean on Windows'
}

if ($failures -gt 0) {
    Write-Error "$failures contract failure(s)."
    exit 1
}
Write-Host 'All repo portability contracts hold.'
