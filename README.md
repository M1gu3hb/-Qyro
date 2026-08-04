# Qyro

Qyro es una aplicación de transferencia privada y directa de archivos para Android, iOS y Windows. No requiere cuentas, nube, anuncios, telemetría ni un backend central.

> Estado: base técnica del Hito 0 alcanzada e interfaz del Hito 1 parcial. El nombre, los identificadores y Apache-2.0 son provisionales. Todavía no existe transferencia de archivos.

## Comprobado

- Workspace Rust 1.88.0 con núcleo y ABI C mínima QYRO/1.
- ScrambleDecodeEngine determinista.
- Boot de 5.5 s, omisión por toque/teclado tras 1 s y reduced motion.
- Home honesto con Enviar/Recibir deshabilitados hasta implementar transportes.
- Runners oficiales Android, iOS y Windows.
- CI verde y builds debug reales en las tres plataformas.

## Desarrollo

    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cd apps/qyro
    flutter pub get
    dart format --output=none --set-exit-if-changed .
    flutter analyze
    flutter test
    flutter run

Lee AGENTS.md, PROJECT_CONTEXT.md, HANDOFF.md y NEXT_STEPS.md antes de modificar código.
