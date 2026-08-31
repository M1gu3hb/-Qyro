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
    # **Sin valor fijo, y la version importa** (QYR-0387). Estaba clavado en
    # `34.0.0`, y el flag `-P` de `zipalign` -- el que alinea a 16 KB, que es lo
    # que Android 15 exige -- no existe antes de build-tools 35. Un valor por
    # omision que no puede hacer el trabajo es peor que ninguno: falla en la
    # linea de zipalign, a mitad de una firma, en vez de al empezar.
    #
    # Se resuelve el mas nuevo que haya instalado. Si no hay ninguno, se dice
    # aqui y no tres pasos mas tarde.
    [string]$BuildTools
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $InputApk)) { throw "No such APK: $InputApk" }
if (-not (Test-Path -LiteralPath $KeyProperties)) {
    throw "No key.properties at $KeyProperties. It is deliberately not in the repository; see apps/qyro/android/key.properties.example."
}

if (-not $BuildTools) {
    $sdk = if ($env:ANDROID_SDK_ROOT) { $env:ANDROID_SDK_ROOT }
           elseif ($env:ANDROID_HOME) { $env:ANDROID_HOME }
           else { Join-Path $env:LOCALAPPDATA 'Android\Sdk' }
    $root = Join-Path $sdk 'build-tools'
    if (-not (Test-Path -LiteralPath $root)) {
        throw "No hay build-tools en $root. Instalalos desde Android Studio (SDK Manager, SDK Tools), o pasa -BuildTools con la ruta."
    }
    # Orden por version y no alfabetico: '9.0.0' es mayor que '35.0.0' en texto.
    $newest = Get-ChildItem -LiteralPath $root -Directory |
        Where-Object { $_.Name -as [version] } |
        Sort-Object { [version]$_.Name } -Descending |
        Select-Object -First 1
    if (-not $newest) { throw "No hay ninguna version de build-tools en $root" }
    if ([version]$newest.Name -lt [version]'35.0.0') {
        throw "build-tools $($newest.Name) es demasiado antiguo: el flag -P de zipalign, que alinea a 16 KB para Android 15, existe desde la 35. Instala una mas nueva."
    }
    $BuildTools = $newest.FullName
    Write-Host "build-tools: $BuildTools"
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
    #
    # **`-P 16`, y sin eso este script deshacia el trabajo del enlazador.**
    # QYR-0387: `-p` alinea los `.so` sin comprimir a la pagina, y hasta
    # build-tools 34 esa pagina son **4 KB**. Android 15 corre con paginas de
    # **16 KB** en aparatos nuevos, asi que re-alinear a 4 aqui tiraba la
    # alineacion que el NDK habia puesto -- despues de medirla, y sobre el
    # artefacto que se publica.
    #
    # `-P <kb>` existe desde build-tools 35. Si esta caja tiene una mas vieja,
    # el flag se rechaza y el script para: parar es correcto, porque firmar con
    # una herramienta que no sabe alinear a 16 KB produce un APK que no carga en
    # un telefono nuevo, y eso no se ve hasta que alguien lo instala.
    $aligned = Join-Path $work 'aligned.apk'
    & $zipalign -P 16 -f 4 $stripped $aligned
    if ($LASTEXITCODE -ne 0) {
        throw "zipalign -P 16 fallo con $LASTEXITCODE. Necesita build-tools 35 o mas nuevo, porque el flag -P no existe antes. Sin el, el .so queda alineado a 4 KB y no carga en Android 15."
    }

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
    Write-Host '=== las ABIs y los 16 KB, medidos sobre el APK firmado ==='
    # QYR-0387. Firmar es lo ultimo que toca el paquete, asi que es lo ultimo
    # que puede romperlo: medir antes de firmar mide otro archivo.
    $inspector = Join-Path $PSScriptRoot '..\tools\apk_inspector\inspect_apk.py'
    if (Test-Path -LiteralPath $inspector) {
        & python3 $inspector $OutputApk --require-abi arm64-v8a --require-abi armeabi-v7a
        if ($LASTEXITCODE -ne 0) { throw 'el APK firmado no pasa la inspeccion' }
    }
    else {
        Write-Host "[SKIP] no esta $inspector, asi que nadie mide el APK firmado"
    }

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
