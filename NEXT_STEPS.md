# Próximos pasos

## P0

1. Crear el crate `qyro_protocol` mediante TDD.
   - Aceptación: marco binario versionado con magic, versión, tipo, flags,
     session/transfer/stream/item ID, secuencia, longitud y autenticación;
     round-trip, truncamiento, corrupción y límites comprobados. Las longitudes
     se validan antes de reservar memoria.
2. Crear `qyro_manifest` mediante TDD.
   - Aceptación: rutas relativas, tamaños, MIME, carpetas y hash; rechaza `..`,
     rutas absolutas, NUL, nombres reservados y manifests gigantes.
3. Golden tests de arranque y benchmark documentado.

## P1

- Golden tests boot 0/20/50/80/100, teléfono, tablet, Windows, reduced motion,
  fallo de FFI y branding provisional.
- Benchmark de arranque documentado con condiciones explícitas.
- Ejecutar `ci.yml` en esta rama mediante pull request.
- Probar en hardware físico: hasta ahora solo emulador, simulador y host.
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
- iOS y Android confirmados en CI sobre esta rama: run 30963011815 (iOS, 10/10
  pasos, incluye verificación de símbolos y XCTest) y run 30963016390 (Android,
  smoke test de ABI en emulador).

## Completado el 2026-08-04

- doctor/bootstrap/test_all en Bash y PowerShell mediante TDD.
- bootstrap crea configuración desde examples sin sobrescribir al usuario.
- Puente Dart↔qyro_ffi con lectura real QYRO/1 en Linux y Windows.
- APK contiene bibliotecas Rust arm64-v8a/x86_64 y Windows distribuye la DLL junto al ejecutable.
