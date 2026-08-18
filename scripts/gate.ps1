# La puerta, corrida con LOS MISMOS comandos que corre CI.
#
# **Comprobación 18, y existe porque una vez no se cumplió.** La 17 decía
# «compila en Linux» y se cumplía con `cargo check`; CI corre `cargo clippy
# --all-targets -- -D warnings`, que es un comando *parecido* y no el mismo.
# Resultado: un `clippy::ptr_arg` de Linux pasó la puerta local y tumbó CI. Y
# `cargo test --workspace --all-features`, que CI también corre, no lo corría
# nadie aquí — la primera vez que se ejecutó, falló.
#
# Por eso este script **no lleva su propia lista**. Lee `.github/workflows/ci.yml`
# y ejecuta los `cargo` que encuentra: una lista escrita a mano se separa del
# flujo el día que alguien toca uno de los dos, y separarse es justo el defecto
# que esto evita.
#
#   pwsh -File scripts/gate.ps1              # todo
#   pwsh -File scripts/gate.ps1 -SkipLinux   # sin el objetivo de Linux

param([switch]$SkipLinux)

$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$yml = Get-Content .github/workflows/ci.yml -Raw
$commands = [System.Collections.Generic.List[string]]::new()
foreach ($line in ($yml -split "`n")) {
    if ($line -match '^\s*-\s+run:\s+(cargo\s+.+?)\s*$') {
        $cmd = $Matches[1].Trim()
        if (-not $commands.Contains($cmd)) { $commands.Add($cmd) }
    }
}
if ($commands.Count -eq 0) {
    Write-Error "[GATE] no se encontro ni un comando cargo en ci.yml -- el parser se rompio, no el repositorio"
    exit 1
}

Write-Host "[GATE] $($commands.Count) comandos leidos de ci.yml"
$failed = @()

foreach ($cmd in $commands) {
    Write-Host "`n[GATE] > $cmd"
    Invoke-Expression $cmd | Out-Host
    if ($LASTEXITCODE -ne 0) { $failed += $cmd }
}

# El objetivo de Linux. `check` no basta: el defecto que creo esta comprobacion
# era un lint, y los lints solo los ve clippy.
if (-not $SkipLinux) {
    $linux = 'x86_64-unknown-linux-gnu'
    $installed = (rustup target list --installed) -split "`n" | ForEach-Object { $_.Trim() }
    if ($installed -notcontains $linux) {
        Write-Host "[GATE] falta el objetivo $linux -- instalalo con: rustup target add $linux"
        $failed += "rustup target add $linux"
    } else {
        foreach ($cmd in @(
            "cargo clippy --workspace --all-targets --target $linux -- -D warnings",
            "cargo check --workspace --all-targets --target $linux"
        )) {
            Write-Host "`n[GATE] > $cmd"
            Invoke-Expression $cmd | Out-Host
            if ($LASTEXITCODE -ne 0) { $failed += $cmd }
        }
    }
}

Write-Host ""
if ($failed.Count -gt 0) {
    Write-Host "[GATE] ROJO. Fallaron $($failed.Count):"
    $failed | ForEach-Object { Write-Host "  $_" }
    exit 1
}
Write-Host "[GATE] VERDE: $($commands.Count) comandos de ci.yml + el objetivo de Linux"
exit 0
