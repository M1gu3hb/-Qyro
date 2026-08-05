# Estado canónico de Qyro

Este archivo es la única fuente de verdad para el estado ejecutable actual. Las
especificaciones y ADR describen intención; no sustituyen evidencia.

- Updated UTC: 2026-08-05T00:40:00Z
- Branch: claude/qyro-recovery-continuation-j53jgx
- Verified commit: ff933d97beae7a98745fcfda9423f65135af94b8
- Milestone: Hito 0 verificado; Hito A (recuperación) cerrado; Hito 1 en hardening

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

- iOS staticlib linkage y XCTest en simulador: IMPLEMENTED, EJECUTADO (run 30963011815)
- Android runtime ABI en emulador: IMPLEMENTED, EJECUTADO (run 30963016390)

## Not implemented

- Golden tests de arranque y benchmark documentado: NOT_IMPLEMENTED
- Retained development artifacts and checksums: NOT_IMPLEMENTED
- File transfer: NOT_IMPLEMENTED
- File selection and manifest: NOT_IMPLEMENTED
- LAN/discovery/manual IP: NOT_IMPLEMENTED
- File encryption/integrity/resume: NOT_IMPLEMENTED
- Identidad, emparejamiento y dispositivos de confianza: NOT_IMPLEMENTED
- Database/history: NOT_IMPLEMENTED
- Optical QR/RaptorQ: NOT_IMPLEMENTED
- Wi-Fi Direct/Multipeer/Bluetooth transports: NOT_IMPLEMENTED
- Share Target Android, Share Extension iOS, drag and drop Windows: NOT_IMPLEMENTED
- cargo audit obligatorio, SBOM y fuzzing: NOT_IMPLEMENTED

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

Ejecutado en esta sesión sobre `5825b50`, host Linux, Flutter 3.44.8 (la versión
que fija CI) y PowerShell 7.4.6:

- `cargo fmt --all --check`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS, sin avisos
- `cargo test --workspace`: PASS, 4 tests
- `flutter pub get --enforce-lockfile`: PASS
- `dart tools/branding_generator/bin/generate.dart --check`: PASS
- `dart format --output=none --set-exit-if-changed .`: PASS, 27 archivos, 0 cambiados
- `flutter analyze`: PASS, «No issues found!»
- `flutter test`: PASS, **51 tests**, ~5 s, incluye lectura real de `QYRO/1`
  desde `libqyro_ffi.so` por FFI
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

`ci.yml` todavía no se ha ejecutado en esta rama: se dispara por push a `main` o
por pull request. Su contenido sí se reprodujo íntegro en el host Linux (arriba).

## Artifacts

- Las salidas del baseline existieron solo dentro de runners efímeros.
- No se retiene APK, ZIP de Windows ni Runner.app descargable.
- No existe release estable, IPA ni MSIX.

## Blockers

- `ci.yml` no se ha ejecutado en esta rama (requiere pull request); el baseline
  equivalente sí se reprodujo localmente.
- Ninguna de las tres plataformas se ha probado en hardware físico, solo en
  emulador, simulador y host.
- Golden tests, benchmark de arranque y artefactos retenidos siguen ausentes.
- `cargo audit` no es obligatorio; no hay SBOM ni lockfile de licencias auditado.
- Autoría y licencia del logo siguen sin registrar.
- No existe ninguna función de transferencia: el producto no es usable todavía.

## Next task

Crear el crate `qyro_protocol` mediante TDD con el marco binario versionado de
QYRO/1: magic, versión, tipo, flags, session/transfer/stream/item ID, secuencia,
longitud y autenticación, rechazando longitudes fuera de límite **antes** de
reservar memoria. Aceptación: round-trip, truncamiento, corrupción de bytes y
límites comprobados con `cargo test --workspace` en verde.

## Provisional values

Los siguientes valores son provisionales y deben bloquear el empaquetado público:

- Marcadores `REPLACE_WITH_*` en los ejemplos de branding.
- Base de identificador `com.owner.qyro`.
- Clearance del nombre de producto Qyro.
- Elección de licencia Apache-2.0.
- Autoría/licencia del logo suministrado (`design/brand/source/logo.png`).
