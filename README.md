# Qyro

Qyro es un proyecto de transferencia privada y directa para Android, iOS y Windows. No requiere cuentas, nube, anuncios, telemetría ni backend central.

> Qyro todavía no transfiere archivos. Enviar y Recibir permanecen deshabilitados.

## Estado verificable

La fuente canónica es [STATUS.md](STATUS.md). Allí se distingue entre compilar, ejecutar, empaquetar y probar, con runs y bloqueos reales.

## Desarrollo

    bash scripts/doctor.sh
    bash scripts/bootstrap.sh
    bash scripts/test_all.sh

Equivalentes PowerShell:

    pwsh -NoProfile -File scripts/doctor.ps1
    pwsh -NoProfile -File scripts/bootstrap.ps1
    pwsh -NoProfile -File scripts/test_all.ps1

Para ejecutar la app:

    cd apps/qyro
    flutter run

Lee AGENTS.md, STATUS.md, HANDOFF.md y NEXT_STEPS.md antes de modificar código.
