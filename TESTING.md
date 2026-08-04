# Estrategia de pruebas

## Regla

Comportamiento nuevo sigue rojo → verde → refactor. Un test debe fallar por la causa prevista antes de producción.

## Actual

- Rust: 3 tests de readiness/protocolo y 1 test ABI.
- Flutter: 5 tests del ScrambleDecodeEngine.
- CI: fmt, Clippy con warnings como error, cargo test, dart format, flutter analyze y flutter test.

## Pendiente

- Rust: property tests, fuzz, KAT de crypto, framing, manifest, traversal, resume, migraciones y RaptorQ.
- Flutter: widgets, golden 0/20/50/80/100, navegación, accesibilidad, responsive y errores.
- Nativas: instrumentation, XCTest y Windows.
- E2E: matriz Android/iOS/Windows, LAN/IP/óptica y fallos.
- Archivos: 0 B a 1 GiB, 1/10k items, Unicode, duplicados y largos.
- Fallos: red, corrupción, crash, permisos, almacenamiento, cámara, replay y peer malicioso.

No publicar benchmarks sin hardware, versión, resolución, FPS capturado/decodificado, CPU, memoria, distancia y condiciones.
