# Estado canónico de Qyro

Este archivo es la única fuente de verdad para el estado ejecutable actual. Las
especificaciones y ADR describen intención; no sustituyen evidencia.

- Updated UTC: 2026-08-05T00:30:00Z
- Branch: claude/qyro-recovery-continuation-j53jgx
- Verified commit: 5825b50b40792a1fb588a969dc7411db6ff04a17
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

## Not implemented

- iOS staticlib linkage verificado en HEAD: NOT_VERIFIED (ver Blockers)
- Android runtime ABI verificado en HEAD: NOT_VERIFIED (ver Blockers)
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
- iOS Runner.app debug sin firma: NO desde `67fa795`. La corrección está en
  `565a78d` pero todavía no se ha compilado en un runner macOS.

## Platforms executed

- Linux host Dart→Rust ABI test: YES (esta sesión, `flutter test`)
- Windows host Dart→DLL ABI test: YES (CI previo)
- Android emulator/device: NO en HEAD. Único `success` histórico: run
  30957598982 (SHA `c971c9a`).
- iOS simulator/device: NO. XCTest nunca llegó a ejecutarse en HEAD.
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

Workflows remotos en el último HEAD de `audit/baseline-hardening` (`e9ed7f3`):

- CI run 30961157153: success
- iOS runtime ABI run 30961153321: **failure** (storyboard, corregido en `565a78d`)
- Android runtime ABI run 30961153377: `in_progress` desde 2026-08-04T23:47Z con
  `total_ms: 0`; nunca obtuvo runner. No es evidencia.

Esta rama todavía no tiene ejecución de CI: `ci.yml` se dispara por push a `main`
o por pull request, y los workflows de runtime por push a
`audit/baseline-hardening` o `workflow_dispatch`.

## Artifacts

- Las salidas del baseline existieron solo dentro de runners efímeros.
- No se retiene APK, ZIP de Windows ni Runner.app descargable.
- No existe release estable, IPA ni MSIX.

## Blockers

- La corrección del storyboard de iOS (`565a78d`) no está confirmada en un runner
  macOS. Hasta entonces, la vinculación de `qyro_ffi` en iOS sigue sin probar.
- El runtime ABI de Android no tiene ejecución válida en HEAD.
- Golden tests, benchmark de arranque y artefactos retenidos siguen ausentes.
- `cargo audit` no es obligatorio; no hay SBOM ni lockfile de licencias auditado.
- Autoría y licencia del logo siguen sin registrar.
- No existe ninguna función de transferencia: el producto no es usable todavía.

## Next task

Disparar `ios-runtime.yml` y `android-runtime.yml` con `workflow_dispatch` sobre
`claude/qyro-recovery-continuation-j53jgx` y confirmar que el job de iOS supera el
paso «Build unsigned iOS application with qyro_ffi» y ejecuta el XCTest que lee
`QYRO/1`. Registrar los IDs de run y su conclusión en este archivo.

## Provisional values

Los siguientes valores son provisionales y deben bloquear el empaquetado público:

- Marcadores `REPLACE_WITH_*` en los ejemplos de branding.
- Base de identificador `com.owner.qyro`.
- Clearance del nombre de producto Qyro.
- Elección de licencia Apache-2.0.
- Autoría/licencia del logo suministrado (`design/brand/source/logo.png`).
