# Changelog

Basado en Keep a Changelog y Semantic Versioning.

## [Unreleased]

### Added

- Workspace Rust 1.88.0.
- Readiness, contrato QYRO/1 y ABI C mínima.
- ScrambleDecodeEngine determinista y reduced motion.
- Boot accesible con logo y Home honesto.
- Runners Flutter Android/iOS/Windows y pubspec.lock.
- CI Rust/Flutter y builds debug en tres plataformas.
- doctor, bootstrap y test_all equivalentes en Bash y PowerShell.
- Contratos ejecutables para categorías, códigos de salida y preservación de configuración.
- Validación del ledger de licencias desde test_all.
- Documentación, ADR, auditoría de referencias y prompt maestro.

### Security

- Política sin nube, telemetría ni servicios remotos.
- ABI con memoria estática sin transferencia de propiedad.
- bootstrap nunca sobrescribe configuraciones locales existentes.
- test_all declara como advertencia la ausencia de cargo-audit.
- Workflow temporal de scaffolding retirado después de usarlo.
