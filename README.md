# Qyro

Qyro es una aplicación de transferencia privada y directa de archivos para Android, iOS y Windows. No requiere cuentas, nube, anuncios, telemetría ni un backend central.

> Estado: Hito 0 en progreso. El nombre, los identificadores y la licencia Apache-2.0 son provisionales hasta aprobación del propietario. No existe todavía una transferencia de archivos funcional.

## Comprobado

- Workspace Rust fijado a 1.88.0.
- Estado de arranque basado en resultados reales.
- ABI C mínima para consultar QYRO/1.
- Motor Dart determinista de scramble/reveal con reduced motion.
- CI en Ubuntu con formato, Clippy, análisis y tests.

## Desarrollo

    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cd apps/qyro
    flutter pub get
    dart format --output=none --set-exit-if-changed .
    flutter analyze
    flutter test

Lee AGENTS.md, PROJECT_CONTEXT.md, HANDOFF.md y NEXT_STEPS.md antes de modificar código.
