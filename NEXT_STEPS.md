# Próximos pasos

## P0

1. Confirmar en CI la corrección del storyboard de iOS.
   - Acción: `workflow_dispatch` de `ios-runtime.yml` sobre la rama de trabajo.
   - Aceptación: el paso «Build unsigned iOS application with qyro_ffi» pasa y el
     XCTest lee QYRO/1. Registrar run ID y conclusión en STATUS.md.
2. Recuperar el runtime ABI de Android en HEAD.
   - Acción: `workflow_dispatch` de `android-runtime.yml`.
   - Aceptación: el emulador ejecuta `native_abi_smoke_test.dart` y lee QYRO/1.
3. Crear modelo QYRO/1 y manifest mediante TDD.
   - Aceptación: round-trip, límites, Unicode y path traversal comprobados.

## P1

- Golden tests boot 0/20/50/80/100, teléfono, tablet, Windows, reduced motion,
  fallo de FFI y branding provisional.
- Benchmark de arranque documentado con condiciones explícitas.
- Retener artefactos debug y checksums.
- Instalar cargo-audit, hacerlo obligatorio y generar SBOM.
- Crear suites nativas y corpus de vectores de protocolo.
- Selección de archivos y manifest.

## P2

- SQLite/migración 0001.
- LAN e IP manual.
- Emparejamiento por QR e identidad local.

## P3

- RaptorQ/QR adaptativo.
- Wi-Fi Direct, Multipeer y Bluetooth experimental.

## Completado el 2026-08-05 (Hito A, recuperación)

- Reconciliadas main y audit/baseline-hardening sin force-push ni pérdida de
  commits; ambos cambios del propietario preservados.
- Logo canónico fijado en design/brand/source/logo.png por checksum, con el
  marcador rechazado excluido del producto y cubierto por pruebas (ADR-0014).
- Corregida la regresión que impedía compilar iOS desde 67fa795, con contrato
  estructural del storyboard verificado en rojo y verde.
- Cerrada la brecha que permitió a STATUS.md derivar 58 commits sin detección.
- Baseline completo reproducido en host Linux: Rust, Flutter (51 tests),
  11 contratos de script, 7 tests Python y el job documental.

## Completado el 2026-08-04

- doctor/bootstrap/test_all en Bash y PowerShell mediante TDD.
- bootstrap crea configuración desde examples sin sobrescribir al usuario.
- Puente Dart↔qyro_ffi con lectura real QYRO/1 en Linux y Windows.
- APK contiene bibliotecas Rust arm64-v8a/x86_64 y Windows distribuye la DLL junto al ejecutable.
