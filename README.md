# Qyro

Qyro es una aplicación de transferencia privada y directa de archivos para Android, iOS y Windows. No requiere cuentas, nube, anuncios, telemetría ni un backend central.

> Estado: base técnica del Hito 0 alcanzada e interfaz del Hito 1 parcial. El nombre, los identificadores y Apache-2.0 son provisionales. Todavía no existe transferencia de archivos.

## Comprobado

- Workspace Rust 1.88.0 con núcleo y ABI C mínima QYRO/1.
- ScrambleDecodeEngine determinista.
- Boot de 5.5 s, omisión por toque/teclado tras 1 s y reduced motion.
- Home honesto con Enviar/Recibir deshabilitados hasta implementar transportes.
- Runners oficiales Android, iOS y Windows.
- doctor, bootstrap y test_all equivalentes en Bash y PowerShell.
- CI verde y builds debug reales en las tres plataformas.

## Desarrollo

Diagnóstico:

    bash scripts/doctor.sh
    pwsh -NoProfile -File scripts/doctor.ps1

Preparación segura del workspace:

    bash scripts/bootstrap.sh
    pwsh -NoProfile -File scripts/bootstrap.ps1

Suite completa disponible:

    bash scripts/test_all.sh
    pwsh -NoProfile -File scripts/test_all.ps1

Ejecución de la app:

    cd apps/qyro
    flutter run

Lee AGENTS.md, PROJECT_CONTEXT.md, HANDOFF.md y NEXT_STEPS.md antes de modificar código.
