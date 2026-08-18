# Que `check_win7_imports.ps1` sepa fallar.
#
# Un script de puerta que nadie ha visto fallar es un script que puede estar
# saliendo 0 por la razon equivocada. Esto le da entradas que **tienen** que
# rechazarse y comprueba el codigo de salida.
#
# No compila nada: usa el binario que la tuberia ya produjo. Si no esta, lo dice
# y se salta -- **y saltada no es pasada**, asi que lo escribe en voz alta.

$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $root

$script = Join-Path $root 'scripts/check_win7_imports.ps1'

# `pwsh` en CI, `powershell` en una maquina que solo tiene el de Windows. Se
# elige en vez de suponerse: un script de prueba que no arranca en la maquina de
# quien programa es un script que solo corre cuando ya es tarde.
$shell = if (Get-Command pwsh -ErrorAction SilentlyContinue) { 'pwsh' } else { 'powershell' }
$normal = Join-Path $root 'target/x86_64-pc-windows-msvc/release/qyro.exe'
$failures = 0

function Assert-Fails([string]$what, [string[]]$argv) {
    $null = & $shell -NoProfile -File $script @argv 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "[FAIL] $what salio 0, y tenia que fallar"
        $script:failures++
    } else {
        Write-Host "[ok]   $what fallo, como debe"
    }
}

if (-not (Test-Path $normal)) {
    Write-Host "[SALTADA] no hay binario en $normal, asi que NO se comprobo nada."
    Write-Host "          Compila con: cargo build --release -p qyro_cli --target x86_64-pc-windows-msvc"
    exit 0
}

# 1. Un binario que SI importa el DLL prohibido, puesto en el hueco de win7.
#    Es el caso que este script existe para cazar.
Assert-Fails 'un binario con el import de Windows 8 en el hueco de win7' `
    @('-Win7Binary', $normal, '-NormalBinary', $normal)

# 2. Un archivo que no existe. Un script de puerta que se encoge de hombros ante
#    una ruta equivocada aprueba cualquier cosa que no este.
Assert-Fails 'un binario que no existe' `
    @('-Win7Binary', (Join-Path $root 'no-existe.exe'), '-NormalBinary', $normal)

# 3. Un control que no puede morder. Se le pasa el MISMO archivo inexistente
#    como control: si el script no distingue eso, su [PASS] no vale nada.
Assert-Fails 'un control que apunta a un archivo que no existe' `
    @('-Win7Binary', $normal, '-NormalBinary', (Join-Path $root 'no-existe.exe'))

if ($failures -gt 0) {
    Write-Error "[BLOCKER] check_win7_imports.ps1 no rechazo $failures entrada(s) que debia rechazar"
    exit 1
}
Write-Host '[OK] check_win7_imports.ps1 rechaza las tres entradas que tiene que rechazar'
exit 0
