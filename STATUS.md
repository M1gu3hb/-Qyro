# Estado canónico de Qyro

Este archivo es la única fuente de verdad para el estado ejecutable actual. Las
especificaciones y ADR describen intención; no sustituyen evidencia.

- Updated UTC: 2026-08-05T20:30:00Z
- Branch: claude/qyro-aead-replay
- Verified commit: bcca33906d93d500b79bf4ca2e668eaad3e75156
- Milestone: AEAD de frames implementado, documentado y con vectores;
  transporte y almacenamiento seguro NO iniciados

La rama continúa `claude/qyro-handshake-closure`, que a su vez reconcilió
`audit/baseline-hardening` con los commits del propietario en `main`. Ninguna
rama fue reescrita. Auditoría de este sprint:
`docs/audits/SPRINT4C_AEAD_AUDIT.md`.

## Implemented

- Flutter runners Android, iOS y Windows: IMPLEMENTED
- Rust qyro_core y qyro_ffi QYRO/1 mínima: IMPLEMENTED
- Native bridge Dart→Rust con fallos tipados: IMPLEMENTED, EJECUTADO en Linux y
  en Windows
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
- Flags protegidos fuera de la API pública: IMPLEMENTED
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
- Rechazo de claves Ed25519 de orden bajo y `verify_strict`: IMPLEMENTED
- Fingerprint con exactamente dos escrituras canónicas: IMPLEMENTED
- Identidad pública en el cable, 33 bytes con versión: IMPLEMENTED
- Constructor determinista fuera de la API pública (`cfg(test)`): IMPLEMENTED
- Cabecera protegida fuera de `Frame` (`ProtectedHeaderNotPlain`): IMPLEMENTED
- Plantilla de sobre probada por tipo (`from_plain_frame`): IMPLEMENTED
- **Handshake autenticado de cuatro mensajes (ADR-0021)**: IMPLEMENTED, en
  memoria. X25519 + Ed25519 + HKDF-SHA256 + HMAC-SHA256, máquina de estados
  con estados consumidos. **No corre sobre ningún transporte.**
- `SessionId` canónico de ocho bytes compartido por `qyro_protocol` y
  `qyro_crypto`: IMPLEMENTED. Sin truncamiento en ningún punto.
- Estado `ResponderFinishPending`: IMPLEMENTED. El responder no obtiene sesión
  hasta confirmar que entregó su último mensaje.
- Claves de sesión fuera de la API pública: IMPLEMENTED. `SessionKey` no se
  exporta y no hay accesores de clave.
- **Cifrado autenticado de frames QYRO/1 (ADR-0022)**: IMPLEMENTED, en memoria.
  ChaCha20-Poly1305 sobre la cabecera completa de 48 bytes como datos asociados,
  con `FrameSealer`, `FrameOpener`, `SealedFrame` y `AuthenticatedFrame`; los dos
  últimos con constructor privado. **Nada mueve estos frames a ninguna parte.**
- Derivación direccional de claves y prefijos de nonce con HKDF-SHA256:
  IMPLEMENTED. Dirección dentro de la etiqueta, `auth_transcript` y `SessionId`
  dentro de cada `info`, con pruebas unitarias sobre la derivación misma.
- Nonce monotónico `prefijo || secuencia` asignado por el sealer: IMPLEMENTED.
  No da la vuelta; agotarlo es `SequenceExhausted`, terminal.
- Ventana de replay fija de 1024 con bitmap: IMPLEMENTED. Se consulta antes del
  AEAD y se actualiza solo después de que el tag verifique.
- `into_frame_crypto` consumiendo el estado establecido: IMPLEMENTED. No hay
  forma de derivar dos sealers de la misma dirección.
- Frontera FFI sin acceso a claves, comprobada estructuralmente: IMPLEMENTED.
  `qyro_ffi → qyro_core → nada`; una prueba falla si alguien añade `qyro_crypto`.
- KAT RFC 8032 (5 vectores), RFC 4231 (7 casos), RFC 7748 (§5 y §6.1) y RFC 8439
  (§2.8.2 y apéndice A.5): IMPLEMENTED
- **Vectores interoperables del handshake y del AEAD**: IMPLEMENTED y
  encadenados. `handshake-v1.json` y `aead-v1.json` con sus schemas estrictos,
  regeneración byte a byte y verificación independiente contra las primitivas.
  Una prueba comprueba el encadenamiento campo a campo.

- iOS staticlib linkage y XCTest en simulador: IMPLEMENTED, EJECUTADO
- Android runtime ABI en emulador: IMPLEMENTED, EJECUTADO

## Not implemented

- **Handshake y frames sobre transporte**: NOT_IMPLEMENTED. El handshake existe,
  el sellado existe y ambos están probados, pero se ejecutan entre valores en un
  proceso. No hay sockets, ni descubrimiento, ni integración con el framing en un
  sentido que mueva bytes.
- **Rotación y rekey de claves de sesión**: NOT_IMPLEMENTED. Una sesión usa una
  clave por dirección hasta agotar la secuencia.
- **qyro_identity y almacenamiento seguro**: NOT_IMPLEMENTED en las tres
  plataformas. No hay Android Keystore, ni iOS Keychain, ni DPAPI/CNG.
- **FFI criptográfico**: NOT_IMPLEMENTED, y deliberadamente. La biblioteca que
  Dart carga no depende de `qyro_crypto`, así que no hay nada de esto al otro
  lado de la frontera.
- Golden tests de arranque: NOT_IMPLEMENTED
- Benchmark de arranque documentado: NOT_IMPLEMENTED
- Retained development artifacts and checksums: NOT_IMPLEMENTED
- Campaña real de fuzzing (solo hay corpus smoke): NOT_IMPLEMENTED
- Workflow `fuzz.yml` programado: NOT_IMPLEMENTED
- Transporte, sockets y TLS: NOT_IMPLEMENTED
- File transfer: NOT_IMPLEMENTED
- File selection and manifest: NOT_IMPLEMENTED
- LAN/discovery/manual IP: NOT_IMPLEMENTED
- Resume: NOT_IMPLEMENTED
- Identidad, emparejamiento y dispositivos de confianza: NOT_IMPLEMENTED
- Database/history: NOT_IMPLEMENTED
- Optical QR/RaptorQ: NOT_IMPLEMENTED
- Wi-Fi Direct/Multipeer/Bluetooth transports: NOT_IMPLEMENTED
- Share Target Android, Share Extension iOS, drag and drop Windows: NOT_IMPLEMENTED
- SBOM y cargo-deny: NOT_IMPLEMENTED

## Platforms compiled

- Android debug APK: YES en `bcca339` (run 31041951667, job `android`)
- Windows debug executable: YES en `bcca339` (run 31041951667, job `windows`)
- iOS Runner.app debug sin firma: YES en `bcca339` (run 31041951667, job `ios`)

## Platforms executed

- Linux host Dart→Rust ABI test: YES en `bcca339` (run 31041949268, job `flutter`)
- Windows host Dart→DLL ABI test: YES en `bcca339` (run 31041951667, paso
  «Verify Dart reads QYRO/1 from the Windows DLL»). El mismo job cubre el bundle
  x64, el smoke-launch de `qyro.exe` y el ZIP portable.
- Android emulator: YES en `bcca339` (run 31041953738). Emulador API 35
  `google_apis` x86_64 con KVM ejecutando `integration_test/native_abi_smoke_test.dart`.
- iOS simulator: **PENDIENTE en `bcca339`.** El run 31041956058 sigue en
  ejecución mientras se escribe esto; no se afirma su resultado. La última
  ejecución verde del XCTest en simulador es la del sprint 4B.1, run 31026229897
  sobre `c9cc0f3`, y entre ese commit y este no cambia nada de iOS: el diff es
  Rust, vectores, corpus y documentación.
- iOS/Android hardware físico: NO
- Interactive Windows application smoke: NO

## Real tests

Host Linux, Rust 1.88.0, Python 3 y PowerShell 7.4.6. **Este contenedor no trae
Flutter ni Dart**, así que todo lo que los necesita se ejecutó en CI y no aquí:

- `cargo fmt --all --check`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS, sin avisos
- `cargo test --workspace`: PASS, **262 tests**
- `cargo test --workspace --all-features`: PASS, **262 tests**. Ningún crate
  declara features, así que los dos conjuntos no pueden divergir
- `cargo test --doc --workspace`: PASS
- `cargo audit --deny warnings`: PASS, 0 vulnerabilidades sobre **55 crates**.
  Siete entran con `chacha20poly1305`; ver `docs/LICENSE_AUDIT.md`
- `rustfmt --check --edition 2024 rust/fuzz/fuzz_targets/*.rs`: PASS
- `python3 -m unittest tools/logo_ascii_generator/…`: PASS, 7 tests
- `bash`/`pwsh scripts/check_docs_consistency`: PASS
- `bash`/`pwsh scripts/check_repo_portability`: PASS
- Contratos de scripts: 5/6 Bash y 6/7 PowerShell PASS aquí.
  `doctor_contract_test` falla en este contenedor porque `doctor` reporta
  `BLOCKER` por Flutter y Dart ausentes. **No es una regresión**: es el
  comportamiento correcto de `doctor` en un entorno sin Flutter, y el contrato
  pasa en CI, donde Flutter existe
- `flutter analyze`, `flutter test`, `dart format` y el generador de branding:
  ejecutados solo en CI, run 31041949268

Workflows sobre `bcca339` (este sprint), lanzados con `workflow_dispatch`:

| Workflow | Run | Conclusión |
|---|---|---|
| CI | 31041949268 | **success**, 4/4 jobs |
| Platform builds | 31041951667 | **success**, 3/3 jobs: `android`, `ios` y `windows` |
| Android runtime ABI | 31041953738 | **success**, smoke de ABI en emulador |
| iOS runtime ABI | 31041956058 | en ejecución al escribir esto; sin afirmar |

Los cuatro sobre el mismo commit. Tres han terminado en success; el cuarto se
registrará cuando termine, y hasta entonces esta tabla no lo afirma.

Baseline previo a cualquier cambio funcional: CI 31037909391 sobre `cc4d7d9`,
**success**, lanzado sobre la rama nueva antes de tocar nada.

Este archivo apunta a `bcca339` y no al commit que lo contiene, porque el commit
que lo contiene es solo documentación: STATUS.md, BUGS_PENDING.md y tres pasajes
de `docs/` que seguían negando la existencia del AEAD. Ninguna línea de código
cambia entre `bcca339` y HEAD.

Runs intermedios sobre `69bd152` y `06780b6` quedaron **cancelled** para los dos
workflows de runtime: al relanzar sobre el commit final, el grupo de concurrencia
canceló los anteriores. No es un fallo, y ninguno se usa como evidencia. En ambos
commits, CI y Platform builds sí llegaron a **success** antes de la cancelación
de los otros dos.

Workflows previos sobre `c9cc0f3` (sprint 4B.1):

| Workflow | Run | Conclusión |
|---|---|---|
| CI | 31026203456 | **success**, 4/4 jobs |
| Platform builds | 31026211681 | **success**, 3/3 jobs |
| Android runtime ABI | 31026220463 | **success** |
| iOS runtime ABI | 31026229897 | **success** |

Workflows previos sobre `9f006b0` (sprint 4B, tras corregir QYR-0013):

| Workflow | Run | Conclusión |
|---|---|---|
| CI | 30976489548 | **success**, 4/4 jobs |
| Platform builds | 30976488367 | **success**, 3/3 jobs |

El fallo de Windows del sprint 4B **no** era una regresión de código:
`actions/checkout` moría con `invalid path` sobre un nombre de corpus reservado,
antes de compilar nada. Ver QYR-0013. Marcar como obsoleta la evidencia de
Windows fue lo que llevó a relanzar el workflow que lo destapó; antes, una tabla
de CI, Android e iOS pasaba por evidencia de las tres ABI nativas, y así el fallo
permaneció invisible durante tres sprints.

`ci.yml` acepta `workflow_dispatch`.

## Artifacts

- Las salidas del baseline existieron solo dentro de runners efímeros.
- No se retiene APK, ZIP de Windows ni Runner.app descargable.
- No existe release estable, IPA ni MSIX.

## Blockers

- **No hay transporte.** Hay identidad, handshake autenticado y cifrado de
  frames, y nada de eso mueve un byte: no hay sockets, ni descubrimiento, ni
  escritura en disco. Cifrar un frame en memoria no acerca la transferencia por
  sí solo.
- **La identidad solo vive en memoria.** No hay Keystore, Keychain ni DPAPI/CNG:
  generar una identidad y cerrar el proceso la pierde, así que ninguna decisión
  de confianza sobrevive a un reinicio.
- No hay FFI criptográfico; Dart no ve nada de esto, y una prueba lo mantiene así.
- **Ninguna segunda implementación ha verificado los vectores.** Existen,
  encadenados y comprobados contra las primitivas, pero «formato definido sin
  ambigüedad» seguirá siendo una intención hasta que alguien escriba el lado
  Swift o Kotlin.
- No hay medición de canales laterales. ChaCha20-Poly1305 en software es de
  tiempo constante por construcción y la comparación del tag la hace `subtle`,
  pero nada en este repositorio lo mide.
- Golden tests de arranque y benchmark documentado siguen ausentes por tercer
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

## Next task

Implementar persistencia segura de `DeviceIdentity` mediante Android Keystore,
iOS Keychain y Windows DPAPI/CNG, con rotación, borrado y pruebas en runtime.
**Todavía sin** conectar sockets ni transferencia.

Aceptación: una identidad generada sobrevive al cierre del proceso en las tres
plataformas, con evidencia de ejecución real y no solo de compilación; la clave
privada nunca sale del almacén en claro hacia Dart; rotación y borrado son
operaciones explícitas y probadas; y el fallo del almacén es un error tipado, no
una identidad silenciosamente nueva.

## Provisional values

Los siguientes valores son provisionales y deben bloquear el empaquetado público:

- Marcadores `REPLACE_WITH_*` en los ejemplos de branding.
- Base de identificador `com.owner.qyro`.
- Clearance del nombre de producto Qyro.
- Elección de licencia Apache-2.0.
- Autoría/licencia del logo suministrado (`design/brand/source/logo.png`).
