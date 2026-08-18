[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string] $BundlePath,

    [string] $HeadersTextPath,

    [string] $ExportsTextPath
)

$ErrorActionPreference = 'Stop'

function Stop-Blocker {
    param([Parameter(Mandatory)][string] $Message)
    throw "[BLOCKER] $Message"
}

function Find-Dumpbin {
    $command = Get-Command 'dumpbin.exe' -ErrorAction SilentlyContinue
    if ($null -ne $command) {
        return $command.Source
    }

    $programFilesX86 = [Environment]::GetFolderPath('ProgramFilesX86')
    $vswhere = Join-Path $programFilesX86 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
        Stop-Blocker 'dumpbin.exe is unavailable; install the Visual C++ build tools'
    }

    $installationPath = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1)
    if ([string]::IsNullOrWhiteSpace($installationPath)) {
        Stop-Blocker 'Visual C++ x64 build tools are unavailable'
    }

    $toolRoot = Join-Path $installationPath 'VC\Tools\MSVC'
    foreach ($toolset in (Get-ChildItem -LiteralPath $toolRoot -Directory | Sort-Object Name -Descending)) {
        $candidate = Join-Path $toolset.FullName 'bin\Hostx64\x64\dumpbin.exe'
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            return $candidate
        }
    }

    Stop-Blocker 'dumpbin.exe is unavailable in the installed Visual C++ toolsets'
}

function Get-InspectionText {
    param(
        [Parameter(Mandatory)][string] $Label,
        [string] $TextPath,
        [Parameter(Mandatory)][string] $Switch,
        [Parameter(Mandatory)][string] $DllPath
    )

    if (-not [string]::IsNullOrWhiteSpace($TextPath)) {
        if (-not (Test-Path -LiteralPath $TextPath -PathType Leaf)) {
            Stop-Blocker "$Label fixture does not exist: $TextPath"
        }
        return Get-Content -LiteralPath $TextPath -Raw
    }

    $dumpbin = Find-Dumpbin
    $output = & $dumpbin $Switch $DllPath 2>&1
    if ($LASTEXITCODE -ne 0) {
        Stop-Blocker "dumpbin $Switch failed for qyro_ffi.dll"
    }
    return ($output | Out-String)
}

if (-not (Test-Path -LiteralPath $BundlePath -PathType Container)) {
    Stop-Blocker "Windows bundle directory does not exist: $BundlePath"
}

$resolvedBundle = (Resolve-Path -LiteralPath $BundlePath).Path
$exePath = Join-Path $resolvedBundle 'qyro.exe'
$dllPath = Join-Path $resolvedBundle 'qyro_ffi.dll'

if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) {
    Stop-Blocker 'Missing qyro.exe'
}
if (-not (Test-Path -LiteralPath $dllPath -PathType Leaf)) {
    Stop-Blocker 'Missing qyro_ffi.dll'
}

$headers = Get-InspectionText -Label 'Headers' -TextPath $HeadersTextPath -Switch '/headers' -DllPath $dllPath
if ($headers -notmatch '(?im)\b8664\s+machine\b|\bmachine\s+\(x64\)') {
    Stop-Blocker 'qyro_ffi.dll is not x64'
}

$exports = Get-InspectionText -Label 'Exports' -TextPath $ExportsTextPath -Switch '/exports' -DllPath $dllPath
foreach ($symbol in @('qyro_protocol_version_ptr', 'qyro_protocol_version_len')) {
    $pattern = '(?m)(^|\s)' + [regex]::Escape($symbol) + '(?=\s|$)'
    if ($exports -notmatch $pattern) {
        Stop-Blocker "Missing export $symbol"
    }
}

Write-Host '[PASS] Windows x64 package contains qyro.exe, qyro_ffi.dll, and required exports'
