# Changelog

Basado en Keep a Changelog y Semantic Versioning.

## [Unreleased]

### Added

- Workspace Rust 1.88.0.
- Readiness, contrato QYRO/1 y ABI C mínima.
- Puente Dart FFI con validación de puntero, longitud y UTF-8.
- Test real Dart→qyro_ffi en Linux y Windows.
- Bibliotecas Rust Android arm64-v8a/x86_64 verificadas dentro del APK.
- qyro_ffi.dll empaquetada junto a qyro.exe.
- ScrambleDecodeEngine determinista, boot accesible y Home honesto.
- Runners Flutter Android/iOS/Windows y builds debug.
- doctor, bootstrap y test_all equivalentes en Bash y PowerShell.
- Contratos de categorías, códigos de salida y preservación de configuración.
- Validación del ledger de licencias desde test_all.
- Documentación, ADR, auditoría de referencias y prompt maestro.

### Security

- Política sin nube, telemetría ni servicios remotos.
- ABI con memoria estática sin transferencia de propiedad.
- Dart rechaza punteros nulos y longitudes fuera del límite antes de decodificar.
- bootstrap nunca sobrescribe configuraciones locales existentes.
- test_all declara como advertencia la ausencia de cargo-audit.
