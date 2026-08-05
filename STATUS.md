# Estado canónico de Qyro

Este archivo es la única fuente de verdad para el estado ejecutable actual. Las
especificaciones y ADR describen intención; no sustituyen evidencia.

- Updated UTC: 2026-08-05T04:45:00Z
- Branch: claude/qyro-authenticated-handshake
- Verified commit: 779fb168c5595fb16f888f08766c60dd9be9938c
- Milestone: handshake autenticado en memoria; AEAD y transporte NO iniciados

La rama reconcilia `audit/baseline-hardening` (`e9ed7f3`, 58 commits de trabajo)
con los dos commits del propietario en `main` (`e0041de`). Ninguna rama fue
reescrita. Auditoría completa: `docs/audits/CLAUDE_RECOVERY_AUDIT.md`.

## Implemented

- Flutter runners Android, iOS y Windows: IMPLEMENTED
- Rust qyro_core y qyro_ffi QYRO/1 mínima: IMPLEMENTED
- Native bridge Dart→Rust con fallos tipados: IMPLEMENTED, EJECUTADO en Linux
  (esta sesión) y en Windows solo hasta `e9ed7f3`
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
- Rechazo de rutas rastreadas que Windows no puede extraer: IMPLEMENTED
- Framing binario QYRO/1 con decoder incremental acotado (ADR-0016): IMPLEMENTED
- Manifest canónico con validación estricta de rutas (ADR-0017): IMPLEMENTED
- Property tests y corpus smoke de fuzzing: IMPLEMENTED
- cargo audit obligatorio en CI: IMPLEMENTED
- Wordmark, tagline y firma configurable mediante scramble: IMPLEMENTED
- Política de errores estructurales/semánticos del decoder (ADR-0018): IMPLEMENTED
- Cabecera QYRO/1.0 sin extensiones no preservables, campos privados: IMPLEMENTED
- Flags protegidos fuera de la API pública; envoltura cifrada con tag obligatorio: IMPLEMENTED
- Nombre visible derivado de la ruta (ADR-0019, manifest v2): IMPLEMENTED
- Digest final obligatorio para todo archivo: IMPLEMENTED
- Rechazo de caracteres no portables y de colisiones case/NFC-NFD: IMPLEMENTED
- Preflight de longitud serializada del manifest: IMPLEMENTED
- Tipo desconocido representado sin sustitución (ParsedHeader): IMPLEMENTED
- Construcción de cabecera totalmente acotada: IMPLEMENTED
- `EncryptedEnvelope` con garantías honestas, sin afirmar autenticación: IMPLEMENTED
- `DecodedFrame` sin centinelas y sin panic (`plaintext`/`try_encode`): IMPLEMENTED
- Normalización Unicode canónica real (unicode-normalization): IMPLEMENTED
- SHA-256 como único digest final de archivo: IMPLEMENTED
- Identidad Ed25519 con fingerprint versionado y firma con dominios: IMPLEMENTED
- Vectores interoperables RFC 8032 + Qyro: IMPLEMENTED
- Rechazo de claves Ed25519 de orden bajo y `verify_strict`: IMPLEMENTED
- Fingerprint con exactamente dos escrituras canónicas: IMPLEMENTED
- Identidad pública en el cable, 33 bytes con versión: IMPLEMENTED
- Constructor determinista fuera de la API pública (`cfg(test)`): IMPLEMENTED
- Cabecera protegida fuera de `Frame` (`ProtectedHeaderNotPlain`): IMPLEMENTED
- Plantilla de sobre probada por tipo (`from_plain_frame`): IMPLEMENTED
- **Handshake autenticado de cuatro mensajes (ADR-0021)**: IMPLEMENTED, en
  memoria. X25519 + Ed25519 + HKDF-SHA256 + HMAC-SHA256, máquina de estados
  con estados consumidos. **No corre sobre ningún transporte y no cifra nada.**
- KAT RFC 8032 (5 vectores) y RFC 4231 (7 vectores): IMPLEMENTED

- iOS staticlib linkage y XCTest en simulador: IMPLEMENTED, EJECUTADO (run 30963011815)
- Android runtime ABI en emulador: IMPLEMENTED, EJECUTADO (run 30963016390)

## Not implemented

- **AEAD y replay protection**: NOT_IMPLEMENTED. El handshake deriva claves de
  sesión, pero **nada las usa**: no hay ChaCha20-Poly1305 ni ningún otro cifrado.
  `EncryptedEnvelope` define la forma de un frame cifrado y expone los datos
  asociados, pero ningún AEAD los consume. `qyro_crypto` **todavía no puede
  cifrar nada.**
- **Handshake sobre transporte**: NOT_IMPLEMENTED. El handshake existe y está
  probado, pero se ejecuta entre dos valores en un proceso. No hay sockets, ni
  descubrimiento, ni integración con el framing de `qyro_protocol`.
- **Vectores del handshake**: NOT_IMPLEMENTED. `identity-v1.json` fija la
  identidad; el transcript del handshake solo está fijado por ADR-0021 y por los
  tests de Rust, que prueban que Rust es consistente consigo mismo.
- **SealedFrame / AuthenticatedFrame**: NOT_IMPLEMENTED. Vivirán en
  `qyro_crypto` con constructores privados cuando exista el sellado.
- **qyro_identity y almacenamiento seguro**: NOT_IMPLEMENTED en las tres
  plataformas. No hay Android Keystore, ni iOS Keychain, ni DPAPI/CNG.
- Golden tests de arranque: NOT_IMPLEMENTED
- Benchmark de arranque documentado: NOT_IMPLEMENTED
- Retained development artifacts and checksums: NOT_IMPLEMENTED
- Campaña real de fuzzing (solo hay corpus smoke): NOT_IMPLEMENTED
- Workflow `fuzz.yml` programado: NOT_IMPLEMENTED
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
- Windows debug executable: **NO desde el sprint 2**. No es una regresión de
  compilación: el checkout fallaba antes de compilar. Ver QYR-0013.
- iOS Runner.app debug sin firma: YES en `ff933d9` (run 30963011815, paso
  «Build unsigned iOS application with qyro_ffi»). Estuvo roto entre `67fa795`
  y `565a78d`.

## Platforms executed

- Linux host Dart→Rust ABI test: YES (esta sesión, `flutter test`)
- Windows host Dart→DLL ABI test: **NO**, y ahora se sabe por qué. El job
  `windows` de `Platform builds` moría en `actions/checkout` desde el sprint 2:
  `rust/fuzz/corpus/relative_path/nul.txt` usa un nombre de dispositivo
  reservado y `git` no puede extraerlo en Windows (run 30976026135). Corregido
  en este sprint (QYR-0013); **queda pendiente volver a ejecutarlo** para tener
  evidencia real sobre el código actual.
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
- `cargo test --workspace`: PASS, **191 tests**
- `cargo test --workspace --all-features`: PASS, **191 tests**. Ya no difieren:
  ningún crate declara features, así que los vectores de identidad y los KAT ya
  no pueden saltarse ejecutando solo el conjunto por defecto
- `cargo test --doc --workspace`: PASS
- `cargo audit --deny warnings`: PASS, 0 vulnerabilidades sobre **48 crates**.
  El workspace ya no está libre de dependencias: ver `docs/LICENSE_AUDIT.md`
- `flutter pub get --enforce-lockfile`: PASS
- `dart tools/branding_generator/bin/generate.dart --check`: PASS
- `dart format --output=none --set-exit-if-changed .`: PASS
- `flutter analyze`: PASS, «No issues found!»
- `flutter test`: PASS, **58 tests**, incluye lectura real de `QYRO/1` desde
  `libqyro_ffi.so` por FFI (sin cambios en este sprint)
- 6 contratos Bash y 7 PowerShell: PASS, incluido el nuevo de portabilidad de
  rutas, verificado en rojo contra el nombre real que rompía Windows
- `python3 -m unittest tools/logo_ascii_generator/…`: PASS, 7 tests
- `bash`/`pwsh scripts/check_docs_consistency`: PASS

Workflows sobre `abe6601` (este sprint), lanzados con `workflow_dispatch`:

| Workflow | Run | Conclusión |
|---|---|---|
| CI | 30970737104 | **success**, 4/4 jobs |
| Android runtime ABI | 30970738398 | **success** |
| iOS runtime ABI | 30970744000 | **success** |

Esos tres runs cubren **Linux, Android e iOS**, no las tres ABI de plataforma
del producto. **Windows no se ha revalidado**: su última evidencia es CI sobre
`e9ed7f3`, muy por detrás de este commit, y ningún workflow de Windows se ha
ejecutado desde entonces. Decir «las tres ABI nativas siguen intactas» —como
decía antes esta línea— hace pasar la ABI que sí se comprobó (Linux, dentro del
job de CI) por la que no.

Workflows previos sobre `f78522a`:

| Workflow | Run | Conclusión |
|---|---|---|
| CI | 30966182205 | **success**, 4/4 jobs (rust, flutter, scripts, documentation) |
| Android runtime ABI | 30966196087 | **success** |
| iOS runtime ABI | 30966197144 | **success** |

El job `rust` incluye `cargo audit` como paso obligatorio y pasó. Linux, Android
e iOS siguieron intactas tras añadir `qyro_protocol` y `qyro_manifest`; Windows
tampoco se revalidó aquí.

Evidencia previa sobre `ff933d9` (rama de recuperación):

| Workflow | Run | Conclusión |
|---|---|---|
| iOS runtime ABI | 30963011815 | success, 10/10 pasos |
| Android runtime ABI | 30963016390 | success, 8/8 pasos |

Referencia del estado anterior en `audit/baseline-hardening` (`e9ed7f3`):

- CI run 30961157153: success
- iOS runtime ABI run 30961153321: failure por el storyboard, corregido en `565a78d`
- Android runtime ABI run 30961153377: `in_progress` con `total_ms: 0`; nunca
  obtuvo runner y no es evidencia

`ci.yml` acepta `workflow_dispatch`. El baseline recuperado se verificó sobre
`c7410cb` antes de tocar nada: run 30964542743, success.

## Artifacts

- Las salidas del baseline existieron solo dentro de runners efímeros.
- No se retiene APK, ZIP de Windows ni Runner.app descargable.
- No existe release estable, IPA ni MSIX.

## Blockers

- **No hay cifrado.** Existe identidad (firmar/verificar) pero ni handshake, ni
  derivación de claves, ni AEAD. Qyro no puede proteger un payload todavía.
- **La identidad solo vive en memoria.** No hay Keystore, Keychain ni DPAPI/CNG:
  generar una identidad y cerrar el proceso la pierde.
- No hay FFI criptográfico; Dart no ve nada de esto.
- Golden tests de arranque y benchmark documentado siguen ausentes por segundo
  sprint consecutivo.
- No se retienen artefactos de desarrollo con checksums.
- No se ha ejecutado una campaña de fuzzing: solo el corpus smoke.
- El plegado de colisiones cubre ASCII y marcas combinantes sobre Latin-1; dos
  rutas que difieran solo por una marca fuera de ese rango se consideran
  distintas. Registrado en `docs/security/parser-threats.md`.
- Ninguna de las tres plataformas se ha probado en hardware físico, solo en
  emulador, simulador y host.
- No hay SBOM ni `cargo-deny`.
- Autoría y licencia del logo siguen sin registrar.
- No existe ninguna función de transferencia: el producto no es usable todavía.
  El protocolo y el manifest existen y están probados, pero nada los usa aún:
  no hay sockets, transporte ni escritura en disco.

## Next task

Implementar el handshake autenticado con X25519 efímero, HKDF-SHA256, transcript
firmado, claves direccionales y vectores conocidos, **sin** conectar todavía
sockets ni almacenamiento seguro.

Aceptación: handshake completo en memoria entre Initiator y Responder; fallo ante
firma incorrecta, transcript alterado, identidad sustituida, reflexión y roles
intercambiados; claves distintas por dirección; vectores en
`docs/security/test-vectors/`; y liberar el dominio `HandshakeTranscript`, hoy
rechazado a propósito hasta que su formato esté congelado.

## Provisional values

Los siguientes valores son provisionales y deben bloquear el empaquetado público:

- Marcadores `REPLACE_WITH_*` en los ejemplos de branding.
- Base de identificador `com.owner.qyro`.
- Clearance del nombre de producto Qyro.
- Elección de licencia Apache-2.0.
- Autoría/licencia del logo suministrado (`design/brand/source/logo.png`).
