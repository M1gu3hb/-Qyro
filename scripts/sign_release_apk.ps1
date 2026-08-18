<#
.SYNOPSIS
    Signs a CI-built APK with the release key, and prints both hashes.

.DESCRIPTION
    The release key is not in the repository and is not in a CI secret, so the
    APK that `release.yml` builds is signed with the debug key. This script is
    the step that turns it into the release artifact, and it runs on the machine
    that holds `key.properties` -- which is the only place the private key ever
    is.

    It strips the existing signature, re-signs with apksigner, verifies the
    certificate it actually ended up with, and prints the SHA-256 of both files
    so that `docs/release/v1.0.md` records measured numbers rather than
    remembered ones.

.PARAMETER InputApk
    The APK downloaded from the `qyro-android-release` artifact.

.PARAMETER OutputApk
    Where to write the release-signed APK.

.EXAMPLE
    gh run download <run-id> --name qyro-android-release --dir C:\temp\qyro
    .\scripts\sign_release_apk.ps1 -InputApk C:\temp\qyro\app-release-debugkey.apk `
                                   -OutputApk C:\temp\qyro\app-release.apk
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$InputApk,
    [Parameter(Mandatory = $true)][string]$OutputApk,
    [string]$KeyProperties = "$PSScriptRoot\..\apps\qyro\android\key.properties",
    [string]$BuildTools = 'D:\android-sdk\build-tools\34.0.0'
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $InputApk)) { throw "No such APK: $InputApk" }
if (-not (Test-Path -LiteralPath $KeyProperties)) {
    throw "No key.properties at $KeyProperties. It is deliberately not in the repository; see apps/qyro/android/key.properties.example."
}

$apksigner = Join-Path $BuildTools 'apksigner.bat'
$zipalign = Join-Path $BuildTools 'zipalign.exe'
foreach ($tool in @($apksigner, $zipalign)) {
    if (-not (Test-Path -LiteralPath $tool)) { throw "Missing $tool" }
}

$props = @{}
foreach ($line in (Get-Content -LiteralPath $KeyProperties)) {
    if ($line -match '^\s*([^#=]+?)\s*=\s*(.*)$') { $props[$matches[1]] = $matches[2] }
}
$store = $props['storeFile'] -replace '\\\\', '\'
if (-not (Test-Path -LiteralPath $store)) { throw "key.properties points at $store, which does not exist" }

$work = Join-Path ([IO.Path]::GetTempPath()) "qyro-sign-$([Guid]::NewGuid())"
New-Item -ItemType Directory -Path $work -Force | Out-Null
try {
    # Strip the debug signature. apksigner replaces v2/v3 blocks itself, but the
    # v1 META-INF files are ordinary zip entries and a leftover one makes the
    # result verify against two certificates.
    $stripped = Join-Path $work 'stripped.apk'
    Copy-Item -LiteralPath $InputApk -Destination $stripped
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::Open($stripped, 'Update')
    try {
        $doomed = @($zip.Entries | Where-Object {
            $_.FullName -match '^META-INF/.*\.(SF|RSA|DSA|EC)$' -or $_.FullName -eq 'META-INF/MANIFEST.MF'
        })
        foreach ($entry in $doomed) {
            Write-Host "  removing $($entry.FullName)"
            $entry.Delete()
        }
    }
    finally { $zip.Dispose() }

    # zipalign before signing: apksigner preserves alignment, the other order
    # does not.
    $aligned = Join-Path $work 'aligned.apk'
    & $zipalign -p -f 4 $stripped $aligned
    if ($LASTEXITCODE -ne 0) { throw "zipalign failed with $LASTEXITCODE" }

    & $apksigner sign `
        --ks $store `
        --ks-key-alias $props['keyAlias'] `
        --ks-pass "pass:$($props['storePassword'])" `
        --key-pass "pass:$($props['keyPassword'])" `
        --out $OutputApk `
        $aligned
    if ($LASTEXITCODE -ne 0) { throw "apksigner sign failed with $LASTEXITCODE" }

    Write-Host ''
    Write-Host '=== the certificate it is actually signed with ==='
    & $apksigner verify --print-certs --verbose $OutputApk
    if ($LASTEXITCODE -ne 0) { throw "apksigner verify failed with $LASTEXITCODE" }

    Write-Host ''
    Write-Host '=== SHA-256, for docs/release/v1.0.md ==='
    foreach ($file in @($InputApk, $OutputApk)) {
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash.ToLower()
        $size = (Get-Item -LiteralPath $file).Length
        "{0}  {1}  ({2} bytes)" -f $hash, (Split-Path -Leaf $file), $size | Write-Host
    }
}
finally {
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
