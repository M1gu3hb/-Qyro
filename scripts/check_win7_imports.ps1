# El binario de Windows 7 no puede importar lo que Windows 7 no tiene.
#
# ADR-0049 §4. `std` importa **estáticamente** `WaitOnAddress` y compañía de
# `api-ms-win-core-synch-l1-2-0.dll`, que es Windows 8 mínimo, así que el
# cargador falla **antes** de ejecutar una sola instrucción del programa. No hay
# forma de sortearlo en tiempo de ejecución: o no está en la tabla de imports, o
# el binario no arranca.
#
# ## El control, y es la mitad que importa
#
# Este script comprueba dos cosas y **la segunda es la que hace que la primera
# signifique algo**:
#
#   1. El binario de win7 **no** importa el DLL prohibido.
#   2. El binario del target **normal SÍ lo importa**.
#
# Sin (2), un patrón mal escrito, un `dumpbin` que no está o una ruta equivocada
# pasarían en verde para siempre, diciendo exactamente lo mismo que una
# comprobación que funciona. Es la forma que salvó a este proyecto en la fase 13:
# fue el **fallo** del control de `+crt-static` lo que destapó este bloqueo.
#
#   pwsh -File scripts/check_win7_imports.ps1 `
#        -Win7Binary  target/x86_64-win7-windows-msvc/release/qyro.exe `
#        -NormalBinary target/x86_64-pc-windows-msvc/release/qyro.exe

param(
    [Parameter(Mandatory = $true)][string]$Win7Binary,
    [Parameter(Mandatory = $true)][string]$NormalBinary
)

$ErrorActionPreference = 'Stop'

# Lo que Windows 7 no tiene. Nombres exactos, no patrones amplios: un `-like
# "*synch*"` tambien casaria con DLLs que si existen en 7.
$forbidden = @(
    'api-ms-win-core-synch-l1-2-0.dll',
    'vcruntime140.dll',
    'msvcp140.dll'
)

function Find-Dumpbin {
    $direct = Get-Command dumpbin -ErrorAction SilentlyContinue
    if ($direct) { return $direct.Source }
    $roots = @(
        "${env:ProgramFiles}\Microsoft Visual Studio",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio"
    ) | Where-Object { $_ -and (Test-Path $_) }
    foreach ($root in $roots) {
        $found = Get-ChildItem $root -Recurse -Filter dumpbin.exe -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if ($found) { return $found.FullName }
    }
    return $null
}

$dumpbin = Find-Dumpbin
if (-not $dumpbin) {
    # **No se pasa en verde por no poder mirar.** Una comprobacion que no puede
    # ejecutarse es una comprobacion que no se ejecuto, y decir OK ahi es la
    # mentira exacta que este script existe para no contar.
    Write-Error "[BLOCKER] no se encontro dumpbin, asi que los imports NO se comprobaron"
    exit 1
}

function Get-Imports([string]$path) {
    if (-not (Test-Path -LiteralPath $path)) {
        Write-Error "[BLOCKER] no existe el binario: $path"
        exit 1
    }
    return (& $dumpbin /imports $path | Out-String).ToLowerInvariant()
}

function Get-Offenders([string]$imports) {
    return @($forbidden | Where-Object { $imports.Contains($_) })
}

# ------------------------------------------------ 1. el binario de Windows 7
$win7 = Get-Imports $Win7Binary
$offenders = Get-Offenders $win7
if ($offenders.Count -gt 0) {
    Write-Error "[BLOCKER] $Win7Binary importa $($offenders -join ', '), que Windows 7 no tiene. No arranca alli."
    exit 1
}
Write-Host "[PASS] $Win7Binary no importa nada que falte en Windows 7"

# --------------------------------------- 2. el control: el normal debe fallar
$normal = Get-Imports $NormalBinary
$expected = Get-Offenders $normal
if ($expected.Count -eq 0) {
    Write-Error @"
[BLOCKER] EL CONTROL FALLO. $NormalBinary tampoco importa ninguno de:
    $($forbidden -join ', ')

Eso no es una buena noticia: significa que esta comprobacion **no distingue
nada**. O los nombres estan mal escritos, o dumpbin devolvio algo inesperado, o
se le paso el binario equivocado. El [PASS] de arriba no vale.
"@
    exit 1
}
Write-Host "[PASS] el control muerde: $NormalBinary importa $($expected -join ', '), como debe"

Write-Host "[OK] Windows 7 imports: el binario de win7 esta limpio y la comprobacion demostro que sabe encontrarlo"
exit 0
