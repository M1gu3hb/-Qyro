#requires -Version 5.1
# Refuses tracked paths that Windows cannot check out.
#
# Twin of scripts/check_repo_portability.sh. See that file for why this exists:
# the repository shipped `rust/fuzz/corpus/relative_path/nul.txt`, and `NUL` is a
# reserved Windows device name, so `git checkout` failed and the Windows CI job
# died before any Qyro code ran.

[CmdletBinding()]
param(
    [string]$RepoRoot
)

$ErrorActionPreference = 'Stop'
$blockers = 0

function Report {
    param([string]$Level, [string]$Area, [string]$Message)
    Write-Output ("[{0}] {1}: {2}" -f $Level, $Area, $Message)
}

if (-not $RepoRoot) {
    $RepoRoot = Split-Path -Parent $PSScriptRoot
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    Report 'BLOCKER' 'Repository' "$RepoRoot does not exist"
    exit 2
}

Push-Location -LiteralPath $RepoRoot
try {
    & git rev-parse --git-dir *> $null
    if ($LASTEXITCODE -ne 0) {
        Report 'BLOCKER' 'Repository' "$RepoRoot is not a git repository"
        exit 2
    }

    # Reserved device names. Windows treats them as devices whatever the
    # extension, so `nul.txt` is as unusable as `nul`.
    $reserved = '^(CON|PRN|AUX|NUL|COM[0-9]|LPT[0-9])$'
    # Characters Windows forbids in a path segment.
    $illegal = '[<>:"|?*\\]'

    $paths = & git ls-files
    foreach ($path in $paths) {
        if ([string]::IsNullOrWhiteSpace($path)) { continue }

        foreach ($segment in $path.Split('/')) {
            if ([string]::IsNullOrEmpty($segment)) { continue }

            $stem = $segment.Split('.')[0].ToUpperInvariant()
            if ($stem -match $reserved) {
                Report 'BLOCKER' 'Portability' "$path uses the reserved Windows device name $stem; git checkout fails on Windows"
                $blockers++
            }

            if ($segment -match $illegal) {
                Report 'BLOCKER' 'Portability' "$path contains a character Windows forbids in a filename"
                $blockers++
            }

            # Windows silently strips these, so two distinct paths become one file.
            if ($segment -match '[ .]$') {
                Report 'BLOCKER' 'Portability' "$path ends a segment with a space or dot, which Windows strips"
                $blockers++
            }
        }
    }
}
finally {
    Pop-Location
}

if ($blockers -gt 0) {
    Report 'BLOCKER' 'Repository portability' "$blockers path(s) cannot be checked out on Windows"
    exit 1
}

Report 'OK' 'Repository portability' 'every tracked path can be checked out on Windows'
exit 0
