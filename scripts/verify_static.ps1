<#
.SYNOPSIS
    Fails if a Windows binary needs a C runtime DLL.

.DESCRIPTION
    R8 §6 measured a case where `+crt-static` was **ignored in silence** and
    produced a byte-identical binary. So this looks at the imports rather than
    trusting the flag.

    Exit 0 when the binary is self-contained; exit 1 when it names
    `vcruntime140.dll` or `msvcp140.dll`. A machine that cannot install anything
    cannot install a redistributable either, so a dynamic link here is not a
    smaller problem than a missing feature -- it is the binary not starting.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Binary,
    # The control, kept and neutered on purpose -- see QYR-0360 below.
    [switch]$ExpectDynamic,
    # Demand a binary that starts on Windows 7. Fails today, by design.
    [switch]$ExpectWindows7
)

$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $Binary)) { throw "no such binary: $Binary" }

function Find-Dumpbin {
    $command = Get-Command 'dumpbin.exe' -ErrorAction SilentlyContinue
    if ($null -ne $command) { return $command.Source }
    $vswhere = Join-Path ([Environment]::GetFolderPath('ProgramFilesX86')) 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere)) { throw 'dumpbin is unavailable and vswhere is not installed' }
    $root = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1
    foreach ($toolset in (Get-ChildItem (Join-Path $root 'VC\Tools\MSVC') -Directory | Sort-Object Name -Descending)) {
        $candidate = Join-Path $toolset.FullName 'bin\Hostx64\x64\dumpbin.exe'
        if (Test-Path -LiteralPath $candidate) { return $candidate }
    }
    throw 'dumpbin is unavailable in the installed toolsets'
}

$imports = & (Find-Dumpbin) /imports $Binary | Out-String
$runtime = @('vcruntime140.dll', 'msvcp140.dll', 'msvcr120.dll') |
    Where-Object { $imports -match [regex]::Escape($_) }

# QYR-0360. The control was run and it FAILED, and that is the finding.
#
# Built with `+crt-static` and without it, this toolchain produces binaries with
# **identical import sets** and different hashes -- R8 §6's warning reproduced
# exactly. On Rust 1.88 MSVC a `qyro`-shaped program never names
# `vcruntime140.dll` either way, so a check for it passes for both and
# distinguishes nothing.
#
# The flag stays -- it costs nothing and it matters for programs that do pull
# the runtime in -- but **this script no longer claims to verify it**. What it
# verifies instead is the thing that actually decides whether the binary starts
# on an old machine, and that is `-ExpectWindows7`.
if ($ExpectDynamic) {
    Write-Host "[SKIP] +crt-static is not observable in the import set on this toolchain (QYR-0360). Nothing to control."
    exit 0
}

# The import that decides Windows 7, and it is present today.
#
# R8 §10: a stock Rust binary imports `WaitOnAddress`/`WakeByAddress*` from
# `api-ms-win-core-synch-l1-2-0.dll`, which is **Windows 8 minimum**, and because
# the import is static the loader fails before `main` -- no degradation, just
# "the DLL is missing". Fixing it needs the `*-win7-windows-msvc` targets, which
# are Tier 3 and need nightly with `-Z build-std`. That is phase 17.
$win7Blockers = @('api-ms-win-core-synch-l1-2-0.dll') |
    Where-Object { $imports -match [regex]::Escape($_) }

if ($ExpectWindows7) {
    if ($win7Blockers) {
        Write-Error "[BLOCKER] $Binary imports $($win7Blockers -join ', '), which is Windows 8 minimum. It will not start on Windows 7 (R8 §10). Phase 17 and the win7 targets."
        exit 1
    }
    Write-Host "[PASS] $Binary imports nothing newer than Windows 7"
    exit 0
}

if ($win7Blockers) {
    Write-Host "[NOTE] imports $($win7Blockers -join ', ') -- Windows 8 minimum, so not a Windows 7 binary yet (phase 17)."
}

if ($runtime) {
    Write-Error "[BLOCKER] $Binary needs $($runtime -join ', '). A machine that cannot install anything cannot install a redistributable, so this binary does not start there."
    exit 1
}

Write-Host "[PASS] $Binary imports no C runtime DLL"
exit 0
