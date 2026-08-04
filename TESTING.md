# Estrategia de pruebas

## Regla

Comportamiento nuevo sigue rojo → verde → refactor. Un test debe fallar por la causa prevista antes de producción.

## Actual

- Rust: 3 tests de readiness/protocolo y 1 test ABI.
- Flutter: 5 tests del ScrambleDecodeEngine, 2 de app/widgets y 2 del puente FFI.
- ABI real: Linux y Windows cargan la biblioteca compilada y leen QYRO/1.
- Android: el APK se inspecciona para exigir .so arm64-v8a y x86_64.
- Scripts: 6 contratos Bash/PowerShell de doctor, bootstrap y test_all.
- CI: fmt, Clippy, cargo test, Dart format/analyze/test y contratos de scripts.
- Evidencia: CI 30942981584 y Platform builds 30942981789, ambos verdes.

## Pendiente

- Android: ejecutar el contrato ABI dentro de emulador/dispositivo.
- iOS: enlazar staticlib y resolver símbolos desde el proceso.
- Rust: property tests, fuzz, KAT de crypto, framing, manifest, traversal, resume, migraciones y RaptorQ.
- Flutter: golden 0/20/50/80/100, navegación, accesibilidad, responsive y errores.
- Nativas: instrumentation, XCTest y Windows UI.
- Seguridad: instalar cargo-audit en CI.
- Protocolo: crear y ejecutar corpus de vectores.
- E2E: matriz Android/iOS/Windows, LAN/IP/óptica y fallos.

No publicar benchmarks sin hardware, versión, resolución, FPS capturado/decodificado, CPU, memoria, distancia y condiciones.
