# Estado canónico de Qyro

Este archivo es la única fuente de verdad para el estado ejecutable actual. Las especificaciones y ADR describen intención; no sustituyen evidencia.

- Updated UTC: 2026-08-04T20:01:00Z
- Branch: audit/baseline-hardening
- Verified commit: 7ca3973cd1928ffaa3e7b112d121587d83d5092c
- Milestone: Hito 0 verificado; Hito 1 en hardening

## Implemented

- Flutter runners Android, iOS y Windows: IMPLEMENTED
- Boot visual básico y Home con acciones deshabilitadas: IMPLEMENTED
- Rust qyro_core y qyro_ffi QYRO/1 mínima: IMPLEMENTED
- Native bridge Dart→Rust en Linux y Windows: IMPLEMENTED
- Android arm64-v8a/x86_64 native library packaging: IMPLEMENTED
- Windows portable build layout con qyro_ffi.dll junto a qyro.exe: IMPLEMENTED
- doctor, bootstrap y test_all en Bash/PowerShell: IMPLEMENTED

## Not implemented

- Android runtime ABI execution: NOT_IMPLEMENTED
- iOS staticlib linkage and runtime symbol resolution: NOT_IMPLEMENTED
- Validated generated branding: NOT_IMPLEMENTED
- ASCII boot sequence and StartupCoordinator: NOT_IMPLEMENTED
- Spanish/English localization: NOT_IMPLEMENTED
- Retained development artifacts and checksums: NOT_IMPLEMENTED
- File transfer: NOT_IMPLEMENTED
- File selection and manifest: NOT_IMPLEMENTED
- LAN/discovery/manual IP: NOT_IMPLEMENTED
- File encryption/integrity/resume: NOT_IMPLEMENTED
- Database/history: NOT_IMPLEMENTED
- Optical QR/RaptorQ: NOT_IMPLEMENTED
- Wi-Fi Direct/Bluetooth transports: NOT_IMPLEMENTED

## Platforms compiled

- Android debug APK: YES
- Windows debug executable: YES
- iOS Runner.app debug without signing: YES

## Platforms executed

- Linux host Dart→Rust ABI test: YES
- Windows host Dart→DLL ABI test: YES
- Android emulator/device: NO
- iOS simulator/device: NO
- Interactive Windows application smoke: NO

## Real tests

Baseline captured on this branch before hardening:

- CI run 30945236666: Rust fmt/Clippy/4 tests, Dart format/analyze/9 tests and 6 script contracts passed.
- Platform builds run 30945238593: Android, Windows and iOS jobs passed.
- Android baseline verifies APK contents only; it does not prove runtime loading.
- iOS baseline is a no-codesign build only; it does not prove qyro_ffi linkage.

## Artifacts

- Baseline outputs existed only inside ephemeral runners.
- No downloadable APK, Windows ZIP or Runner.app ZIP is retained yet.
- No stable release, IPA or MSIX exists.

## Blockers

- Android runtime ABI smoke test is absent.
- qyro_ffi staticlib is not linked into iOS Runner.
- Branding values and identifiers remain provisional.
- cargo audit is not mandatory yet.
- Golden tests, license lockfile audit and development packaging are absent.
- Logo authorship/license confirmation and scramble visual reference remain pending.

## Next task

Implement and enforce documentation consistency scripts, then execute Android runtime ABI smoke testing.

## Provisional values

The following values are intentionally provisional and must block public packaging:

- REPLACE_WITH_* markers in branding examples.
- com.owner.qyro bundle identifier base.
- Qyro product name clearance.
- Apache-2.0 project license choice.
- Supplied PNG logo authorship/license.
