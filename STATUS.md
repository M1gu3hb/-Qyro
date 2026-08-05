# Estado canónico de Qyro

Este archivo es la única fuente de verdad para el estado ejecutable actual. Las
especificaciones y ADR describen intención; no sustituyen evidencia.

- Updated UTC: 2026-08-05T01:35:00Z
- Branch: claude/qyro-protocol-manifest
- Verified commit: ff933d97beae7a98745fcfda9423f65135af94b8
- Milestone: Hito A cerrado; Hito C (protocolo y manifest) implementado; Hito 1 visual parcial

La rama reconcilia `audit/baseline-hardening` (`e9ed7f3`, 58 commits de trabajo)
con los dos commits del propietario en `main` (`e0041de`). Ninguna rama fue
reescrita. Auditoría completa: `docs/audits/CLAUDE_RECOVERY_AUDIT.md`.

## Implemented

- Flutter runners Android, iOS y Windows: IMPLEMENTED
- Rust qyro_core y qyro_ffi QYRO/1 mínima: IMPLEMENTED
- Native bridge Dart→Rust con fallos tipados: IMPLEMENTED, EJECUTADO en Linux/Windows
- Android arm64-v8a/x86_64 native library packaging: IMPLEMENTED
- Windows portable layout con qyro_ffi.dll junto a qyro.exe: IMPLEMENTED
- doctor, bootstrap y test_all en Bash/PowerShell: IMPLEMENTED
- Branding generado y validado desde configuración: IMPLEMENTED
- StartupCoordinator con tareas obligatorias, timeout, retry y cancelación: IMPLEMENTED
- Secuencia de arranque ASCII (modelo, painters, scramble, cipher rain): IMPLEMENTED
- Generador determinista logo→ASCII con modo `--check`: IMPLEMENTED
- Localización español/inglés con flutter_localizations: IMPLEMENTED
- Launch surfaces oscuras en Android, iOS y Windows: IMPLEMENTED
- Logo canónico fijado por checksum (ADR-0014): IMPLEMENTED
- Regla anti-deriva de STATUS.md en el job documental: IMPLEMENTED
- Framing binario QYRO/1 con decoder incremental acotado (ADR-0016): IMPLEMENTED
- Manifest canónico con validación estricta de rutas (ADR-0017): IMPLEMENTED
- Property tests y corpus smoke de fuzzing: IMPLEMENTED
- cargo audit obligatorio en CI: IMPLEMENTED
- Wordmark, tagline y firma configurable mediante scramble: IMPLEMENTED

- iOS staticlib linkage y XCTest en simulador: IMPLEMENTED, EJECUTADO (run 30963011815)
- Android runtime ABI en emulador: IMPLEMENTED, EJECUTADO (run 30963016390)

## Not implemented

- Golden tests de arranque: NOT_IMPLEMENTED
- Benchmark de arranque documentado: NOT_IMPLEMENTED
- Retained development artifacts and checksums: NOT_IMPLEMENTED
- Campaña real de fuzzing (solo hay corpus smoke): NOT_IMPLEMENTED
- Transporte, sockets y TLS: NOT_IMPLEMENTED
- File transfer: NOT_IMPLEMENTED
- File selection and manifest: NOT_IMPLEMENTED
- LAN/discovery/manual IP: NOT_IMPLEMENTED
- File encryption/integrity/resume: NOT_IMPLEMENTED
- Identidad, emparejamiento y dispositivos de confianza: NOT_IMPLEMENTED
- Database/history: NOT_IMPLEMENTED
- Optical QR/RaptorQ: NOT_IMPLEMENTED
- Wi-Fi Direct/Multipeer/Bluetooth transports: NOT_IMPLEMENTED
- Share Target Android, Share Extension iOS, drag and drop Windows: NOT_IMPLEMENTED
- SBOM y cargo-deny: NOT_IMPLEMENTED

## Platforms compiled

- Android debug APK: YES (CI, hasta `e9ed7f3`)
- Windows debug executable: YES (CI, hasta `e9ed7f3`)
- iOS Runner.app debug sin firma: YES en `ff933d9` (run 30963011815, paso
  «Build unsigned iOS application with qyro_ffi»). Estuvo roto entre `67fa795`
  y `565a78d`.

## Platforms executed

- Linux host Dart→Rust ABI test: YES (esta sesión, `flutter test`)
- Windows host Dart→DLL ABI test: YES (CI previo)
- Android emulator: **YES** en `ff933d9`. Run 30963016390, paso «Execute native
  ABI smoke test in an Android emulator»: success. Emulador API 35 `google_apis`
  x86_64 con KVM ejecutando `integration_test/native_abi_smoke_test.dart`.
- iOS simulator: **YES** en `ff933d9`. Run 30963011815, los diez pasos en
  success, incluidos «Verify native symbols in the unsigned application»
  (`nm -gU` encuentra `_qyro_protocol_version_ptr` y `_qyro_protocol_version_len`
  en el bundle) y «Execute qyro_ffi XCTest through the Runner host».
- iOS/Android hardware físico: NO
- Interactive Windows application smoke: NO

## Real tests

Host Linux, Flutter 3.44.8 (la versión que fija CI), Rust 1.88.0 y PowerShell
7.4.6:

- `cargo fmt --all --check`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS, sin avisos
- `cargo test --workspace`: PASS, **87 tests** (29 contratos de wire, 40 de
  manifest, 9 property, 3 corpus smoke, 4 previos, 2 doctests)
- `cargo audit`: PASS, 0 vulnerabilidades sobre 4 crates; el workspace no tiene
  dependencias externas
- `flutter pub get --enforce-lockfile`: PASS
- `dart tools/branding_generator/bin/generate.dart --check`: PASS
- `dart format --output=none --set-exit-if-changed .`: PASS
- `flutter analyze`: PASS, «No issues found!»
- `flutter test`: PASS, **58 tests**, incluye lectura real de `QYRO/1` desde
  `libqyro_ffi.so` por FFI
- 5 contratos Bash y 6 PowerShell: PASS
- `python3 -m unittest tools/logo_ascii_generator/…`: PASS, 7 tests
- `bash`/`pwsh scripts/check_docs_consistency`: PASS

Workflows remotos en esta rama, sobre `ff933d9`, lanzados con
`workflow_dispatch`:

| Workflow | Run | Conclusión | Duración |
|---|---|---|---|
| iOS runtime ABI | 30963011815 | **success** (10/10 pasos) | ~15 min |
| Android runtime ABI | 30963016390 | **success** (8/8 pasos) | ~7 min |

Referencia del estado anterior en `audit/baseline-hardening` (`e9ed7f3`):

- CI run 30961157153: success
- iOS runtime ABI run 30961153321: failure por el storyboard, corregido en `565a78d`
- Android runtime ABI run 30961153377: `in_progress` con `total_ms: 0`; nunca
  obtuvo runner y no es evidencia

`ci.yml` sí acepta `workflow_dispatch`. Ejecutado sobre `c7410cb` al abrir la
rama: **run 30964542743, success**, los cuatro jobs (rust, flutter, scripts,
documentation) en verde. Ese fue el baseline recuperado antes de tocar nada.

## Artifacts

- Las salidas del baseline existieron solo dentro de runners efímeros.
- No se retiene APK, ZIP de Windows ni Runner.app descargable.
- No existe release estable, IPA ni MSIX.

## Blockers

- Golden tests de arranque y benchmark documentado siguen ausentes. Este sprint
  los pedía y no se entregaron; ver «Next task».
- No se retienen artefactos de desarrollo con checksums.
- No se ha ejecutado una campaña de fuzzing: solo el corpus smoke.
- Ninguna de las tres plataformas se ha probado en hardware físico, solo en
  emulador, simulador y host.
- No hay SBOM ni `cargo-deny`.
- Autoría y licencia del logo siguen sin registrar.
- No existe ninguna función de transferencia: el producto no es usable todavía.
  El protocolo y el manifest existen y están probados, pero nada los usa aún:
  no hay sockets, transporte ni escritura en disco.

## Next task

Implementar los golden tests de la secuencia de arranque a 0/20/50/80/100 % con
seeds deterministas, dimensiones fijas y assets locales, más el benchmark
documentado del arranque. Aceptación: `flutter test` en verde con los archivos
golden versionados, y `docs/benchmarks/boot-baseline.md` con máquina, SO, versión
de Flutter, modo, resolución y número de muestras declarados.

Es la deuda que este sprint dejó abierta y debe cerrarse antes de continuar con
identidad y cifrado.

## Provisional values

Los siguientes valores son provisionales y deben bloquear el empaquetado público:

- Marcadores `REPLACE_WITH_*` en los ejemplos de branding.
- Base de identificador `com.owner.qyro`.
- Clearance del nombre de producto Qyro.
- Elección de licencia Apache-2.0.
- Autoría/licencia del logo suministrado (`design/brand/source/logo.png`).
