# Handoff operativo

- Actualizado: 2026-08-04 18:12 UTC
- Rama: main
- Commit comprobado: 908307d9248cdedf27d5d064c645b282904e65fe
- Hito: 0 — auditoría y scaffolding, en progreso

## Funciona

- Workspace Rust 1.88.0.
- qyro_core informa estados reales y QYRO/1.
- qyro_ffi exporta puntero/longitud de QYRO/1 con memoria estática.
- ScrambleDecodeEngine es determinista, clampa progreso y soporta reduced motion.
- CI Ubuntu ejecuta formato, lint, análisis y tests.

## No funciona todavía

No hay aplicación Flutter ejecutable, runners Android/iOS/Windows, enlace Dart-FFI, protocolo de transferencia, criptografía, LAN, base de datos ni modo óptico.

## Pruebas ejecutadas

GitHub Actions CI, run 30937447915:

- cargo fmt --all --check: éxito.
- cargo clippy --workspace --all-targets -- -D warnings: éxito, 0.12 s.
- cargo test --workspace: éxito, 4 tests, 0 fallos, build/test 0.49 s.
- dart format --output=none --set-exit-if-changed .: éxito, 2 archivos, 0 cambios.
- flutter analyze: éxito, sin issues, 7.8 s.
- flutter test: éxito, 5 tests.
- Jobs: Rust 14 s; Flutter 36 s.

## Builds

Ninguno. No se ejecutó build de Android, iOS o Windows.

## Bloqueos

- Falta generar y versionar runners Flutter por plataforma.
- No hay entorno macOS/hardware iOS comprobado.
- Identidad legal, bundle IDs y licencia requieren aprobación.
- Falta captura design/reference/scramble-decode-reference.jpg.
- El logo proporcionado no incluye declaración de licencia/propiedad en el repositorio.

## Archivos modificados relevantes

Cargo.toml, rust-toolchain.toml, rust/crates/qyro_core, rust/crates/qyro_ffi, apps/qyro y .github/workflows/ci.yml.

## Próxima tarea exacta

Crear tests widget rojos para la pantalla de arranque y Home; después implementar una app Flutter ejecutable con runners Android/Windows y artefacto de inicialización iOS, manteniendo CI verde.

Criterio: flutter test pasa; flutter analyze queda limpio; builds Android debug y Windows debug se ejecutan en runners adecuados; iOS se valida sin firma en macOS o se documenta el bloqueo real.
