# Comprueba la tabla de paridad GUI/CLI por codigo de salida.
#
# ADR-0046 §3. Una tabla en prosa se desincroniza del codigo en la primera
# semana y sigue leyendose como verdad -- este taller ya lo pago: la fase 11
# anoto en su informe que `qyro_session_local_address` no tenia llamante y la
# observacion se quedo ahi hasta que la fase 12 tropezo con ella.
#
# Que comprueba, por celda:
#   * no esta vacia
#   * si es una referencia `ruta:linea`, el archivo EXISTE y tiene esa linea
#   * si empieza por `NO --`, el argumento tiene sustancia (>= 40 caracteres)
#
# Lo segundo es lo que la hace distinta de un linter de markdown: una referencia
# a un archivo que ya no existe es una celda que miente, y miente diciendo que
# si.

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$table = Join-Path $repo 'docs/PARIDAD-GUI-CLI.md'

if (-not (Test-Path -LiteralPath $table)) {
    Write-Error "[BLOCKER] no existe $table"
    exit 1
}

$lines = Get-Content -LiteralPath $table -Encoding UTF8
$start = ($lines | Select-String -SimpleMatch 'PARIDAD-INICIO' | Select-Object -First 1).LineNumber
$end = ($lines | Select-String -SimpleMatch 'PARIDAD-FIN' | Select-Object -First 1).LineNumber

if (-not $start -or -not $end) {
    Write-Error '[BLOCKER] la tabla no tiene sus marcas PARIDAD-INICIO / PARIDAD-FIN'
    exit 1
}

$rows = @()
for ($i = $start; $i -lt ($end - 1); $i++) {
    $line = $lines[$i]
    if ($line -notmatch '^\|') { continue }
    if ($line -match '^\|\s*-+') { continue }
    if ($line -match '^\|\s*Capacidad\s*\|') { continue }
    $rows += , $line
}

# El numero EXACTO de filas, no un piso.
#
# Empezo siendo un piso de 10 sobre una tabla de 12, y la prueba que el documento
# de fase exige -- borrar una fila y ver fallar el script -- **paso en verde**.
# Un piso deja desaparecer capacidades de una en una, que es precisamente como se
# pierde una: nadie borra doce, alguien borra una.
#
# La consecuencia es que añadir una capacidad obliga a tocar este numero, y eso
# es correcto: añadir una capacidad es un acto deliberado y esta linea es donde
# se declara.
$expected = 13
if ($rows.Count -ne $expected) {
    Write-Error "[BLOCKER] la tabla tiene $($rows.Count) filas y se esperaban exactamente $expected. Si se añadio una capacidad, sube este numero a proposito; si desaparecio una, esa es la razon por la que esta comprobacion existe."
    exit 1
}

$problems = @()

foreach ($row in $rows) {
    $cells = ($row -split '\|') | ForEach-Object { $_.Trim() }
    # split de '| a | b | c |' da '', 'a', 'b', 'c', ''
    if ($cells.Count -lt 5) {
        $problems += "fila con menos de tres columnas: $row"
        continue
    }
    $capability = $cells[1]
    foreach ($index in 2, 3) {
        $face = if ($index -eq 2) { 'GUI' } else { 'CLI' }
        $cell = $cells[$index] -replace '`', ''

        if ([string]::IsNullOrWhiteSpace($cell)) {
            $problems += "[$capability / $face] celda vacia. Una celda vacia no es un olvido, es un incumplimiento: se llena o se escribe por que esa cara no la tiene."
            continue
        }

        if ($cell.StartsWith('NO --')) {
            $argument = $cell.Substring(5).Trim()
            if ($argument.Length -lt 40) {
                $problems += "[$capability / $face] dice NO con un argumento de $($argument.Length) caracteres. Un 'no' sin argumento es una celda vacia con mas letras."
            }
            continue
        }

        if ($cell -match '^(.+):(\d+)$') {
            $path = Join-Path $repo $matches[1]
            $wanted = [int]$matches[2]
            if (-not (Test-Path -LiteralPath $path)) {
                $problems += "[$capability / $face] apunta a $($matches[1]), que no existe. Una referencia rota es una celda que miente, y miente diciendo que si."
                continue
            }
            $content = Get-Content -LiteralPath $path -Encoding UTF8
            $count = ($content | Measure-Object -Line).Lines
            if ($wanted -lt 1 -or $wanted -gt $count) {
                $problems += "[$capability / $face] apunta a $($matches[1]):$wanted y ese archivo tiene $count lineas."
                continue
            }

            # Y que la linea DIGA algo, que es lo que faltaba.
            #
            # Comprobar solo que el archivo tenga esa linea deja pasar una cita a
            # '}', a '};' o a '@override' -- y eso es exactamente lo que habia:
            # cinco filas de esta tabla apuntaban a nada. Un guardian que verifica
            # la existencia y no el contenido no protege el documento, lo AVALA.
            #
            # No se comprueba que la linea corresponda a la capacidad: eso no es
            # mecanizable y fingir que si lo es seria el mismo error otra vez. Se
            # comprueba lo unico que si lo es: que hay un nombre donde se apunta.
            $target = $content[$wanted - 1]
            $nameable = '(fn|def|class|struct|enum|impl|Future<|Stream<|void|String|bool|int|Widget|const|let|pub|static|Widget build)'
            if ($target -notmatch "\b$nameable\b" -or $target -match '^\s*[});\]]+\s*$') {
                $problems += "[$capability / $face] apunta a $($matches[1]):$wanted, y esa linea es '$($target.Trim())'. Una cita a un cierre de bloque o a un decorador no senala a un llamante: senala a donde estaba uno cuando alguien conto las lineas."
            }
            continue
        }

        $problems += "[$capability / $face] no es ni una referencia 'ruta:linea' ni un 'NO -- <argumento>': $cell"
    }
}

if ($problems.Count -gt 0) {
    foreach ($problem in $problems) { Write-Host "[BLOCKER] $problem" }
    Write-Error "[BLOCKER] paridad GUI/CLI: $($problems.Count) problema(s) en $($rows.Count) filas"
    exit 1
}

Write-Host "[OK] Paridad GUI/CLI: $($rows.Count) capacidades, cada celda con llamante o con argumento"
exit 0
