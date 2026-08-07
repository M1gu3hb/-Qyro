# Estado canónico de Qyro

Este archivo es la única fuente de verdad para el estado ejecutable actual. Las
especificaciones y ADR describen intención; no sustituyen evidencia.

- Updated UTC: 2026-08-07T23:10:00Z
- Branch: claude/qyro-secure-storage-4d1
- Verified commit: 91355a84f54c210423d1b4b5a34e3f2a8be78a47
- Milestone: **una identidad sobrevive al cierre del proceso en Windows**,
  ejecutado en CI en dos invocaciones separadas. La persistencia está
  **IMPLEMENTED solo en Windows y NOT_IMPLEMENTED en Android y en iOS**, y nada
  se ha probado en hardware físico

**Qué es y qué no es «Verified commit».** Es el ancla de frescura que comprueba
`check_docs_consistency`: el commit hasta el que este archivo describe el estado.
No es, por sí solo, una afirmación de que se ejecutaron seis workflows sobre él.
La evidencia ejecutada está en las tablas de runs de más abajo, y **cada fila
dice sobre qué commit corrió**. Los runs de cierre del sprint 4C.2 se ejecutan
sobre el commit que lleva los disparadores de CI y se registran en el commit
siguiente, que es la misma secuencia que usó el sprint 4C.1.

La rama continúa `claude/qyro-resource-bounds-4c3`, que continúa
`claude/qyro-crypto-platform-hardening`, que continúa `claude/qyro-aead-replay`,
que continúa `claude/qyro-handshake-closure`, que a su vez reconcilió
`audit/baseline-hardening` con los commits del propietario en `main`. Ninguna
rama fue reescrita ni fusionada a `main`. Auditoría de este sprint:
`docs/audits/SPRINT4D1_SECURE_STORAGE.md`.

**El sprint 4D.1 sí añadió función**, y es el primero desde 4A que lo hace: una
identidad sobrevive al cierre del proceso, en una plataforma de tres. No añadió
ninguna dependencia externa, no tocó transporte ni UI, y no habilitó Enviar ni
Recibir.

**El sprint 4C.3 no añadió funcionalidad.** Corrigió un coste cuadrático medido
en la única ruta que tocará los bytes de un peer, corrigió una cota de memoria
que las propias pruebas del repositorio afirmaban mal, y extendió a los dos
crates de parsing la denegación de pánico e indexado que solo tenía
`qyro_crypto`. Auditoría: `docs/audits/SPRINT4C3_RESOURCE_BOUNDS.md`.

**El sprint 4C.2 no añadió funcionalidad.** Cerró un fallo de seguridad real en
`qyro_manifest`, convirtió en pruebas tres garantías de `qyro_crypto` que
sobrevivían a su propio borrado, y corrigió la documentación que contradecía al
código. Trece hallazgos de una auditoría independiente, QYR-0021 … QYR-0035;
cuatro quedan abiertos y registrados, no omitidos.

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

- **AEAD probado en host Linux** desde el sprint 4C: IMPLEMENTED, EJECUTADO.
  Es lo único que la evidencia de aquel sprint sostenía.
- **AEAD probado en cada plataforma**: IMPLEMENTED, EJECUTADO, pero **solo desde
  el workflow `crypto-platform.yml` de este sprint**. Antes de él, los cuatro
  workflows en verde construían y ejecutaban `qyro_ffi`, que no depende de
  `qyro_crypto` y tiene una prueba que falla si alguien lo añade. Ver la tabla
  de «Platforms executed».
- Ruta AEAD de producción sin `panic!`, `unreachable!`, `assert!` ni indexado sin
  comprobar: IMPLEMENTED. Sostenido por `deny` de Clippy y por una prueba que lee
  el propio fuente descartando antes los bloques `cfg(test)`.
- Sealer envenenado ante cualquier error: IMPLEMENTED. Un reintento no puede
  reutilizar una secuencia ya consumida.
- Texto claro autenticado y búferes temporales en `Zeroizing`: IMPLEMENTED, con
  `sha2/zeroize` y `hmac/zeroize` activadas —estaban apagadas—. El alcance y los
  límites, en `docs/security/secret-lifecycle-audit.md`.
- Harness de criptografía por plataforma aislado del producto (ADR-0023):
  IMPLEMENTED. `publish = false`, sin dependientes en el producto, con dos
  guardas que lo mantienen fuera de los bundles.
- Campaña de fuzzing acotada, seis targets: IMPLEMENTED, EJECUTADA. No es
  exhaustiva y no se presenta como tal.

- iOS staticlib linkage y XCTest en simulador: IMPLEMENTED, EJECUTADO
- Android runtime ABI en emulador: IMPLEMENTED, EJECUTADO

### Sprint 4C.3 — cotas de recursos

- Coste de drenado del decoder acotado (ADR-0016 enmendado): IMPLEMENTED. Un
  byte se copia un número acotado de veces entre entrar al búfer y salir de él.
  Llenar `MAX_BUFFER_LEN` de frames mínimos y drenarlo pasó de **11 476 501 344
  bytes movidos a 0**; el bucle con backlog, de 9 830 400 000 a 2 359 296 sobre
  2 596 608 empujados. Contado con un contador instrumentado, no cronometrado
- `buffer_capacity()` nunca supera `MAX_BUFFER_LEN`: IMPLEMENTED, con una prueba
  que llena el búfer de verdad. Llegaba a 2 097 152 frente a 1 049 664
- Familia de pánico e `indexing_slicing` denegados en `qyro_protocol` y
  `qyro_manifest`, con guarda estructural: IMPLEMENTED. 33 y 22 infracciones
  respectivamente, ninguna silenciada con `allow` fuera de los módulos de prueba
- Análisis de la guarda compartido por los tres crates y exenciones **derivadas**
  de las declaraciones `mod`: IMPLEMENTED. Quitar un `#[cfg(test)]` mueve el
  archivo al conjunto de producción en vez de eximirlo
- Los seis workflows se disparan sobre **cualquier** rama `claude/**` sin editar
  un solo YAML: IMPLEMENTED. Antes era propiedad de una rama concreta
- Un `QYR-00xx` citado sin entrada en `BUGS_PENDING.md` es un BLOCKER:
  IMPLEMENTED
- Consejo de regeneración de vectores condicionado a que el formato siga
  coincidiendo con el ADR: IMPLEMENTED

### Sprint 4C.2 — cierre de la auditoría independiente

- Rechazo de la categoría Unicode `Cf` completa en rutas (ADR-0019 enmendado):
  IMPLEMENTED. Tabla de veintiún rangos de Unicode 16.0.0 citada en el fuente,
  170 puntos de código, sin dependencias nuevas. `invoice<RLO>fdp.exe` ya no
  puede mostrarse como `invoiceexe.pdf`
- Rechazo de colisión ancestro/descendiente (ADR-0017 enmendado): IMPLEMENTED.
  Un archivo no puede ser además el directorio padre de otro elemento
- Nombres de dispositivo de Windows con superíndice: IMPLEMENTED para `COM¹`,
  `COM²`, `COM³`, `LPT¹`, `LPT²`, `LPT³`, con la fuente citada. `COM0`, `LPT0`,
  `CONIN$`, `CONOUT$` y `CLOCK$` **siguen aceptados**: sin fuente, no se añade
  la regla (QYR-0029 abierto)
- Autenticación del iniciador con prueba que falla al borrar el control:
  IMPLEMENTED
- `verify_strict` con prueba que falla al sustituirlo por `verify`: IMPLEMENTED.
  Firma de `R` de orden pequeño sobre la clave de RFC 8032 §7.1 TEST 1
- Transcript verificado contra las primitivas y no contra sí mismo:
  IMPLEMENTED. SHA-256 sobre concatenación literal y HMAC escrito desde
  RFC 2104; `Schedule::derive` fijado contra los valores ya verificados
- Cuatro controles de la ruta de decode con prueba propia: IMPLEMENTED. Cada uno
  borrado por turno hace fallar su propia prueba
- Ninguna ruta de producción de `qyro_crypto` puede terminar el proceso:
  IMPLEMENTED. Doce archivos bajo guarda estructural, `#![deny(...)]` de Clippy
  extendido a `handshake/`, `identity.rs`, `signature.rs` y `fingerprint.rs`,
  y catorce indexaciones sin comprobar eliminadas
- Frontera FFI comprobada sobre el cierre transitivo real (`cargo metadata`):
  IMPLEMENTED. Igualdad exacta con `{qyro_ffi, qyro_core}`
- Variantes de `HandshakeError` sin sitio de construcción: ELIMINADAS, con
  guarda que impide que vuelvan
- Decisión sobre codificaciones X25519 no canónicas (ADR-0021 enmendado):
  REGISTRADA. Se aceptan, conforme a RFC 7748 §5; la verificación de
  libsodium/CryptoKit queda abierta (QYR-0034)
- Los seis workflows se disparan solos sobre la rama de trabajo: IMPLEMENTED.
  **Corregido en 4C.3 (QYR-0040)**: en 4C.2 esto era cierto de *una* rama, cuyo
  nombre estaba escrito a mano en los seis YAML, y este archivo lo registró como
  propiedad del repositorio. Ahora lo es: el disparador es `claude/**`

## Not implemented

- **Handshake y frames sobre transporte**: NOT_IMPLEMENTED. El handshake existe,
  el sellado existe y ambos están probados, pero se ejecutan entre valores en un
  proceso. No hay sockets, ni descubrimiento, ni integración con el framing en un
  sentido que mueva bytes.
- **Rotación y rekey de claves de sesión**: NOT_IMPLEMENTED. Una sesión usa una
  clave por dirección hasta agotar la secuencia.
- **Almacenamiento seguro de identidad**: IMPLEMENTED **solo en Windows**,
  NOT_IMPLEMENTED en Android y en iOS. Hay DPAPI de ámbito de usuario
  (`qyro_win_dpapi`, ADR-0024); no hay Android Keystore ni iOS Keychain.
- **FFI criptográfico**: NOT_IMPLEMENTED, y deliberadamente. La biblioteca que
  Dart carga no depende de `qyro_crypto`, así que no hay nada de esto al otro
  lado de la frontera.
- Golden tests de arranque: NOT_IMPLEMENTED
- Benchmark de arranque documentado: NOT_IMPLEMENTED
- Retención de artefactos de desarrollo: **PARCIAL**. El ZIP portable de Windows
  sí se retiene (`qyro-windows-x64-portable-debug`, 14 días). El APK de Android y
  el `Runner.app` de iOS **no**. Lo que falta en los tres es el checksum
  distribuido dentro del paquete y la etiqueta DEVELOPMENT / NOT FOR PUBLIC
  RELEASE: el digest que GitHub imprime al subir un artefacto identifica el ZIP
  de ese run, no el contenido que alguien desempaqueta.
- Campaña **exhaustiva** de fuzzing: NOT_IMPLEMENTED. Hay una acotada, semanal,
  de dos minutos por target, en `crypto-fuzz.yml`.
- Transporte, sockets y TLS: NOT_IMPLEMENTED
- File transfer: NOT_IMPLEMENTED
- Selección de archivos e integración del manifest con el filesystem:
  NOT_IMPLEMENTED. **El manifest sí existe** y está probado
  (`qyro_manifest`, ADR-0017/0019); lo que falta es elegir archivos reales y
  construirlo desde el disco.
- LAN/discovery/manual IP: NOT_IMPLEMENTED
- Resume: NOT_IMPLEMENTED
- Emparejamiento y dispositivos de confianza: NOT_IMPLEMENTED. La identidad
  existe (`DeviceIdentity`, Ed25519, ADR-0020), el handshake la autentica y desde
  este sprint **sobrevive al cierre del proceso en Windows**; lo que falta es el
  paso de confianza, y que sobreviva también en Android y en iOS.
- Database/history: NOT_IMPLEMENTED
- Optical QR/RaptorQ: NOT_IMPLEMENTED
- Wi-Fi Direct/Multipeer/Bluetooth transports: NOT_IMPLEMENTED
- Share Target Android, Share Extension iOS, drag and drop Windows: NOT_IMPLEMENTED
- SBOM y cargo-deny: NOT_IMPLEMENTED

## Platforms compiled

Aplicación (`qyro_ffi` dentro del bundle):

- Android debug APK: YES en `2c3b3b5` (run 31052477356, job `android`)
- Windows debug executable: YES en `2c3b3b5` (run 31052477356, job `windows`)
- iOS Runner.app debug sin firma: YES en `2c3b3b5` (run 31052477356, job `ios`)

`qyro_crypto`, por target explícito (run 31052478940):

| Target | Compila | Ejecuta |
|---|---|---|
| `x86_64-unknown-linux-gnu` | YES | YES, harness nativo |
| `x86_64-pc-windows-msvc` | YES | YES, harness nativo |
| `x86_64-linux-android` | YES | YES, emulador API 35 vía `adb` |
| `aarch64-linux-android` | YES | **NO** — no hay hardware |
| `aarch64-apple-ios-sim` | YES | YES, simulador vía `xcodebuild test` |
| `aarch64-apple-ios` | YES | **NO** — no hay hardware |

Seis targets compilados, cuatro ejecutados. La distinción no es cosmética: hasta
este sprint no había evidencia de ninguna de las tres plataformas, porque los
workflows en verde ejercitaban `qyro_ffi`, que deliberadamente no depende de
`qyro_crypto`. Detalle en `docs/testing/crypto-platform-matrix.md`.

## Platforms executed

- Linux host Dart→Rust ABI test: YES en `2c3b3b5` (run 31052475631, job `flutter`)
- Windows host Dart→DLL ABI test: YES en `2c3b3b5` (run 31052477356, paso
  «Verify Dart reads QYRO/1 from the Windows DLL»). El mismo job cubre el bundle
  x64, el smoke-launch de `qyro.exe` y el ZIP portable.
- Android emulator, ABI de `qyro_ffi`: YES en `2c3b3b5` (run 31052488810).
  Emulador API 35 `google_apis` x86_64 con KVM ejecutando
  `integration_test/native_abi_smoke_test.dart`.
- iOS simulator, ABI de `qyro_ffi`: YES en `2c3b3b5` (run 31052490644),
  incluidos «Verify native symbols in the unsigned application» y «Execute
  qyro_ffi XCTest through the Runner host».
- **Criptografía en las cuatro plataformas con entorno**: YES en `2c3b3b5` (run 31052478940).
  Jobs `linux-crypto`, `windows-crypto`, `android-crypto` e `ios-crypto`. El
  harness ejecuta identidad, handshake, derivación, sellado, round trip de cable,
  apertura, replay y manipulación, y devuelve un código de salida estable por
  variante de fallo.
- **Persistencia de identidad en Windows, en dos procesos distintos**: YES en `b731276` (run 31215102331, job `windows-crypto`).
  Paso «Persist an identity across two separate process invocations». Un proceso
  llama a `create` y termina; **otro proceso**, lanzado después, llama a `load` y
  obtiene el mismo fingerprint:

      created fingerprint: 49eff48e-89bf12b0-…-0bff77f7
      loaded  fingerprint: 49eff48e-89bf12b0-…-0bff77f7

  `"process_invocations":2` en el informe JSON, checksum SHA-256
  `209cb450100c0dc3f4cb55a65f71f0416d0fb81ebebbce499247d99652046a79`. Dos
  llamadas dentro de un proceso no habrían probado nada: el sistema operativo
  entre ellas es el sujeto de la prueba.
- **`qyro_win_dpapi` contra la API real**: YES en `b731276` (run 31215102331, job `windows-crypto`).
  Nueve pruebas, incluido el barrido de 448 posiciones contra DPAPI y no contra
  el doble. No hay ninguna ejecución de este crate fuera de Windows: es
  `cfg(windows)` entero.
- Persistencia en Android o en iOS: **NO**. No existe.
- iOS/Android **hardware físico**: NO. Un emulador y un simulador no son
  hardware, y este archivo no los va a contar como tal. `windows-latest` tampoco
  es una máquina de usuario: es un perfil recién creado, sin dominio y sin perfil
  móvil, que son justo los casos que ADR-0024 §2 no puede ejercitar allí.
- Interactive Windows application smoke: NO

## Real tests

Host Linux, Rust 1.88.0, Python 3 y PowerShell 7.4.6. **Este contenedor no trae
Flutter ni Dart**, así que todo lo que los necesita se ejecutó en CI y no aquí:

- `cargo fmt --all --check`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS, sin avisos
- `cargo test --workspace`: PASS, **350 tests**, 0 failed, 2 ignored. Eran 323 al
  empezar el sprint 4D.1: la guarda de caminos públicos, cuatro sobre el accesor
  de semilla, dieciocho sobre el formato del blob y dos sobre el `unsafe` del
  crate de plataforma. **Las nueve pruebas de `qyro_win_dpapi` no están en esa
  cuenta**: el crate entero es `cfg(windows)` y en este host no compila ninguna.
  Corren en CI, y ese es su único sitio
- `cargo test --workspace --all-features`: PASS, **350 tests**. Ningún crate
  declara features, así que los dos conjuntos no pueden divergir
- `cargo test --doc --workspace`: PASS
- `cargo audit --deny warnings`: PASS, 0 vulnerabilidades sobre **59 crates**.
  Eran 56: las tres entradas nuevas son `qyro_identity_store`, `qyro_win_dpapi` y
  `qyro_store_smoke`, los tres miembros de este workspace. Este sprint **no añadió
  ninguna dependencia externa**, como fija ADR-0024: las tres entradas nuevas del
  grafo son de primera parte, y el `extern` a Win32 no es una dependencia de
  Cargo. `serde_json` pasó a ser también dev-dependency de `qyro_ffi` y ya estaba
  en el lock como dev-dependency de `qyro_crypto`, así que el grafo auditado no
  cambia. Siete entran con `chacha20poly1305`; ver `docs/LICENSE_AUDIT.md`
- `cargo tree --workspace -d`: PASS, sin duplicados
- `cargo run --package qyro_crypto_smoke -- --json`: PASS,
  `{"target":"linux-x86_64-unix","outcome":"success","code":0}`
- `bash scripts/check_crypto_platform_evidence.sh`: PASS
- `bash scripts/check_harness_isolation.sh`: PASS
- `python3 -m unittest tools/logo_ascii_generator/…`: PASS, 7 tests
- `bash`/`pwsh scripts/check_docs_consistency`: PASS
- `bash`/`pwsh scripts/check_repo_portability`: PASS
- Contratos de scripts: **6/7 Bash y 7/8 PowerShell** PASS aquí, contados
  ejecutando los dieciséis archivos de `scripts/tests/`. Este archivo decía «5/6
  y 6/7», que era la cuenta de antes de que existiera
  `crypto_platform_evidence_contract_test`. El único fallo, en los dos shells,
  es `doctor_contract_test`, porque `doctor` reporta `BLOCKER` por Flutter y Dart
  ausentes. **No es una regresión**: es el comportamiento correcto de `doctor` en
  un entorno sin Flutter, y el contrato pasa en CI, donde Flutter existe
- Los cuatro scripts `check_*` en **Bash y en PowerShell**: PASS los ocho. Este
  contenedor sí trae `pwsh` 7.4.6, a diferencia del de los sprints 4C.2 y 4C.3,
  cuyas secciones más abajo dicen lo contrario de sus propios contenedores
- `flutter analyze`, `flutter test`, `dart format` y el generador de branding:
  ejecutados solo en CI, run 31041949268

### Sprint 4C.2 — línea base sobre `9f79e55`

Antes de tocar una línea, los seis workflows se lanzaron con
`workflow_dispatch` sobre el HEAD heredado. Eso establece la línea base y cierra
de paso el hueco de evidencia que ese commit tenía: los tres commits
documentales del sprint 4C.1 no habían sido ejecutados por nada.

| Workflow | Run | Conclusión |
|---|---|---|
| CI | 31142702190 | **success** |
| Platform builds | 31142703382 | **success** |
| Crypto platform | 31142704701 | **success** |
| Crypto fuzz | 31142705935 | **success** |
| Android runtime ABI | 31142707020 | **success** |
| iOS runtime ABI | 31142708306 | **success** |

### Sprint 4C.3 — runs de cierre

<!-- SPRINT_4C3_CLOSING_RUNS -->
Los seis sobre `c21dd72`, **todos disparados por el `push`**.

| Workflow | Run | Evento | Conclusión |
|---|---|---|---|
| CI | 31150759605 | `push` | **success** |
| Platform builds | 31150759609 | `push` | **success** |
| Crypto platform | 31150759608 | `push` | **success** |
| Crypto fuzz | 31150759628 | `push` | **success** |
| Android runtime ABI | 31150759604 | `push` | **success** |
| iOS runtime ABI | 31150759597 | `push` | **success** |

**Ningún YAML menciona el nombre de esta rama.** El disparador es
`[main, 'claude/**']`, así que los seis corrieron sobre
`claude/qyro-resource-bounds-4c3` sin que nadie editara un archivo de workflow
para permitirlo. Esa es la evidencia de QYR-0040, y no un efecto secundario de
ella: en el sprint anterior este mismo enunciado era cierto de una rama cuyo
nombre estaba escrito a mano en los seis archivos.

El primer push de la rama, en `a579673`, ya lo había demostrado: runs
31148575003 (CI), 31148574819 (Platform builds), 31148574804 (Crypto platform),
31148574796 (Crypto fuzz), 31148574815 (Android runtime ABI) y 31148574808
(iOS runtime ABI), los seis **success**.

Un push documental intermedio, `3b45705`, disparó **cuatro** de los seis. Eso es
correcto —los filtros de rutas existen para eso— y mirar por qué destapó
QYR-0045: dos filtros que no cubrían el código que su workflow construye. Se
registra aquí en vez de omitirse, porque la diferencia entre «no corrió» y «no
tenía que correr» es el hallazgo.

Ningún run falló en esta rama.

El job `documentation` de CI ejecuta los cuatro scripts `check_*` en Bash **y**
en PowerShell. El contenedor de aquella sesión no traía `pwsh`, así que las dos reglas nuevas de
`check_docs_consistency` —nombre de rama literal y registro de hallazgos— solo
tienen esa ejecución como evidencia de su mitad PowerShell.

### Sprint 4C.2 — runs de cierre

<!-- SPRINT_4C2_CLOSING_RUNS -->
Los seis sobre `496e066`, **todos disparados por el `push`** y no a mano. Ese es
el commit que este archivo nombra como verificado, y que los disparadores hayan
funcionado solos es a la vez el resultado y la prueba de QYR-0026: es el primer
commit de la historia de este repositorio en el que empujar a una rama de
trabajo ejecuta los seis workflows sin que nadie los invoque.

| Workflow | Run | Evento | Conclusión |
|---|---|---|---|
| CI | 31145547953 | `push` | **success**, 4/4 jobs |
| Platform builds | 31145547793 | `push` | **success**, 3/3 jobs |
| Crypto platform | 31145547809 | `push` | **success** |
| Crypto fuzz | 31145547827 | `push` | **success** |
| Android runtime ABI | 31145547798 | `push` | **success** |
| iOS runtime ABI | 31145547805 | `push` | **success** |

El job `documentation` de CI ejecuta los cuatro scripts `check_*` en Bash **y**
en PowerShell, y los ocho pasos pasaron. El contenedor de aquella sesión no traía `pwsh`, así que
las dos ediciones PowerShell de este sprint no pudieron probarse localmente y
esa ejecución es su única evidencia; se dice aquí en vez de omitirlo.

Ningún run intermedio falló en esta rama. Los dos commits documentales
posteriores a `496e066` no vuelven a disparar los seis: solo `ci.yml` corre sin
filtro de rutas, y los otros cinco filtran por rutas que esos commits no tocan.
Por eso la evidencia se ancla en `496e066` y no en HEAD.

### Sprint 4C.1 — workflows sobre `2c3b3b5`

Los seis lanzados con `workflow_dispatch` sobre **el mismo commit**:

| Workflow | Run | Conclusión |
|---|---|---|
| CI | 31052475631 | **success**, 4/4 jobs |
| Platform builds | 31052477356 | **success**, 3/3 jobs: `android`, `ios` y `windows` |
| Android runtime ABI | 31052488810 | **success**, smoke de ABI en emulador |
| iOS runtime ABI | 31052490644 | **success**, XCTest en simulador |
| Crypto platform | 31052478940 | **success**, 4/4 jobs: `linux-crypto`, `windows-crypto`, `android-crypto` e `ios-crypto` |
| Crypto fuzz | 31052486806 | **success**, 6/6 targets, 0 artefactos de crash |

Los seis sobre el mismo commit y los seis en success. Ningún run de un commit
anterior se usa como evidencia final, y ninguno de otra rama como baseline.

Este archivo apunta a `2c3b3b5` y no al commit que lo contiene, porque lo que
viene después es solo documentación y un commit no puede nombrar su propio SHA.
Es el patrón que la regla de deriva —hasta diez commits— existe para permitir.

Baseline previo a cualquier cambio de este sprint: CI 31047932017 sobre
`f7ae943`, **success**, lanzado sobre la rama nueva antes de tocar nada.

Runs intermedios de este sprint que **no** son evidencia, listados porque
omitirlos daría una impresión más limpia que la real:

- `crypto-platform.yml` falló en `b05c57c` y en `09b9e8e`. El segundo por el job
  `ios-crypto`: una cabecera dentro de un XCFramework no es un módulo Clang, y
  Swift respondía «no such module», que se lee como si no hubiera encontrado la
  cabecera cuando la había encontrado y copiado bien.
- `crypto-fuzz.yml` falló entero en `09b9e8e` por el `--fuzz-dir` que faltaba.
- **CI falló en `358c64f`** (run 31051825788), y solo en el job `documentation`:
  `check_docs_consistency` rechazó STATUS.md porque dentro de aquel commit
  todavía apuntaba a `bcca339`, dieciocho commits por detrás y en otra rama. Es
  la regla de QYR-0007 haciendo su trabajo, no una regresión de código. Los
  otros cinco workflows sí pasaron allí.
- El run de Android runtime sobre `358c64f` (31051829401) quedó **cancelled** al
  lanzar su reemplazo: el grupo de concurrencia `android-runtime-${{ github.ref }}`
  cancela el anterior sobre la misma ref. No es un fallo, y no se usa como
  evidencia.

Workflows previos sobre `bcca339` (sprint 4C):

| Workflow | Run | Conclusión |
|---|---|---|
| CI | 31041949268 | **success**, 4/4 jobs |
| Platform builds | 31041951667 | **success**, 3/3 jobs |
| Android runtime ABI | 31041953738 | **success** |
| iOS runtime ABI | 31041956058 | **success** |

Esos cuatro estaban en verde y **no** demostraban que `qyro_crypto` compilara ni
corriera en Android, iOS o Windows. Es el hallazgo que motivó este sprint; queda
aquí para que la tabla no se vuelva a leer como lo que no es.

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

- **El ZIP portable de Windows sí se retiene**: `qyro-windows-x64-portable-debug`,
  14 días, desde el job `windows` de `platform-builds.yml`. Este archivo afirmó
  lo contrario durante varios sprints.
- El APK de Android y el `Runner.app` de iOS **no** se retienen. Sus salidas
  existen solo dentro de runners efímeros.
- Ninguno de los tres lleva checksum distribuido **dentro** del paquete ni la
  etiqueta DEVELOPMENT / NOT FOR PUBLIC RELEASE. El digest que GitHub imprime al
  subir un artefacto identifica el ZIP que produjo ese run, no el contenido que
  alguien descarga y desempaqueta; no se usa como sustituto.
- `crypto-fuzz.yml` retiene corpus y artefactos de crash por target, 30 días.
  Son cadenas de bytes que eligió el fuzzer y no contienen material de clave: la
  única sesión en juego es la fija de `qyro_crypto::fuzzing`, cuyas semillas
  están publicadas en este repositorio y comprometidas por definición.
- No existe release estable, IPA ni MSIX.

## Blockers

- **No hay transporte.** Hay identidad, handshake autenticado y cifrado de
  frames, y nada de eso mueve un byte: no hay sockets, ni descubrimiento, ni
  escritura en disco. Cifrar un frame en memoria no acerca la transferencia por
  sí solo.
- **La identidad solo vive en memoria en Android y en iOS.** No hay Keystore ni
  Keychain: en esas dos plataformas, generar una identidad y cerrar el proceso la
  pierde. En Windows sí persiste, y aun así **ninguna decisión de confianza
  sobrevive a un reinicio en ninguna plataforma**, porque no existe el paso de
  confianza que la usaría.
- **Nada del producto llama al almacén.** `qyro_ffi` no depende de
  `qyro_identity_store`, y una prueba lo mantiene así; la aplicación Flutter no
  guarda ni carga identidad alguna. Lo que persiste en CI es un harness aislado,
  no la app.
- No hay FFI criptográfico; Dart no ve nada de esto, y una prueba lo mantiene
  así. Por eso mismo, **la aplicación Flutter no ejercita `qyro_crypto` en
  ninguna plataforma**: lo que corre en el emulador y en el simulador es un
  harness aislado, no la app.
- **Ninguna segunda implementación ha verificado los vectores.** Existen,
  encadenados y comprobados contra las primitivas, pero «formato definido sin
  ambigüedad» seguirá siendo una intención hasta que alguien escriba el lado
  Swift o Kotlin.
- No hay medición de canales laterales. ChaCha20-Poly1305 en software es de
  tiempo constante por construcción y la comparación del tag la hace `subtle`,
  pero nada en este repositorio lo mide.
- Golden tests de arranque y benchmark documentado siguen ausentes por tercer
  sprint consecutivo.
- No se retiene ningún artefacto con checksum distribuido dentro del paquete. El
  ZIP de Windows sí se retiene; el APK y el `Runner.app` no. Ver «Artifacts».
- La campaña de fuzzing es **acotada**: dos minutos por target, semanal. Lo que
  encuentre fuera de ese presupuesto sigue siendo desconocido.
- El plegado de colisiones aplica normalización NFC real y `to_lowercase`
  Unicode por segmento, no una tabla ASCII/Latin-1: pliega marcas combinantes
  fuera de ese rango, singletons y el plegado de griego y cirílico. Lo que **no**
  hace es plegar homoglifos, que son deliberadamente rutas distintas. Registrado
  en `docs/security/parser-threats.md`. La descripción anterior de este archivo
  describía la tabla que se sustituyó en el sprint 4A.
- Ninguna de las tres plataformas se ha probado en **hardware físico**. Este
  sprint añadió ejecución de `qyro_crypto` en cuatro entornos y ninguno es un
  teléfono: emulador, simulador y dos hosts. Android arm64 e iOS device se
  compilan y no se ejecutan.
- La zeroización **no se ha observado**: se comprueba el tipo, no la memoria.
  Leer memoria liberada es comportamiento indefinido, así que una prueba que
  afirmara verlo estaría mintiendo.
- No hay SBOM ni `cargo-deny`.
- Autoría y licencia del logo siguen sin registrar.
- No existe ninguna función de transferencia: el producto no es usable todavía.

## Sprint 4D.1 — qué existe y qué no

**Hay persistencia en Windows y no la hay en Android ni en iOS.** Una identidad
generada por un proceso la carga otro proceso distinto, ejecutado en CI sobre
`windows-latest`; en las otras dos plataformas no hay nada y cerrar el proceso
sigue perdiendo la identidad.

**Esta sección se contradijo con la línea `Milestone` de arriba durante un
commit** (QYR-0060). En `91355a8` la cabecera ya decía IMPLEMENTED en Windows y
este párrafo seguía diciendo «no hay persistencia en ninguna plataforma», con
cuatro viñetas más abajo negando el crate, el harness, los vectores y el
`unsafe`. Las cinco afirmaciones eran falsas en ese mismo commit. Es la forma
exacta de QYR-0055 —registrada en este sprint, doce commits antes— repitiéndose:
registrar una forma de fallo no la previene. Este encabezado, antes de aquello,
había dicho «decisión y especificación, no código» mientras tres viñetas más
abajo listaba el crate.

- ADR-0024 congelada, con las cuatro preguntas de diseño resueltas y sus fuentes
  primarias citadas y fechadas: la estrategia de `unsafe`, DPAPI frente a CNG con
  sus parámetros, el formato del blob byte a byte, y el accesor de semilla.
- `docs/security/identity-storage.md` con el formato.
- Filas nuevas en `THREAT_MODEL.md`, incluida la que dice qué **no** protege
  DPAPI: un atacante que ya ejecuta código como ese usuario descifra el blob
  llamando a la misma API.

- **El accesor de semilla existe**, que es el cambio de superficie que este
  sprint tenía que revisar dos veces: `DeviceIdentity::export_secret` y
  `DeviceIdentity::from_secret`, sobre un `IdentitySecret` que se borra al
  soltarse, no es `Clone` y tiene `Debug` redactado. `identity.rs` ya **no**
  dice «there is no accessor for the seed or the private key»; decía eso hasta
  este sprint y habría quedado contradicho por el código.
- La guarda que lo acota: `every_public_path_returning_key_material_is_listed`
  enumera **por nombre** los caminos públicos que devuelven material de clave.
  Antes del sprint la lista estaba vacía; ahora tiene **tres** entradas,
  `identity.rs::export_secret`, `identity.rs::as_bytes` y
  `aead/mod.rs::into_zeroizing_payload`. Se escribió con la lista vacía y pasó;
  añadir el accesor la puso en rojo con los dos de `identity.rs`, y ampliar los
  marcadores de retorno al arreglar QYR-0053 destapó el tercero, que llevaba en
  el árbol desde el sprint 4C.1 sin que ninguna guarda lo viera.

- **El formato del blob está implementado y probado**: `qyro_identity_store` con
  `blob.rs`, **doce** variantes de `StoreError` —una por paso del orden de
  lectura, más las de escritura y las del almacén— y dieciocho pruebas
  adversariales. Voltear un bit en cualquier posición produce un error tipado,
  comprobado posición por posición y bit por bit, y **la prueba dice por qué
  camino espera cada tramo**. El mismo barrido corre después contra DPAPI real,
  donde 128 posiciones **no** producen error: ver QYR-0059.
- **Tres guardas que no guardaban, ahora verificadas por su propia mutación**
  (QYR-0052, QYR-0053, QYR-0054). Las tres sobrevivían a su propio borrado, que
  es la definición que este proyecto usa para «no cubierto»:
  - la ligadura de la cabecera a la entropía: sustituirla por doce ceros dejaba
    toda la suite en verde, porque el único test comparaba `entropy_for(V, W)`
    consigo misma. Tercera vez con esta forma exacta, tras QYR-0025 y el target
    `encrypted_envelope`;
  - la guarda de material de clave: un `pub fn` que devolvía la semilla en claro
    pasaba, porque la lista de marcadores era una lista de permitidos disfrazada
    de prohibidos. Ahora todo retorno público con forma de bytes debe estar
    clasificado, y ampliarla destapó `into_zeroizing_payload`, que la anterior no
    veía;
  - `forbid(unsafe_code)`: no lo comprobaba nada, y escribir la guarda demostró
    que **la afirmación era falsa**. Ver abajo.
- **Corrección: `forbid(unsafe_code)` no lo llevaban todos.** Este archivo decía
  «todos los crates conservan `forbid(unsafe_code)`, incluido el nuevo» y
  ADR-0024 §1 decía lo mismo. Eran cinco de siete. `qyro_ffi` y
  `qyro_crypto_smoke` **no pueden** llevarlo —`#[unsafe(no_mangle)]` es un
  atributo unsafe en edición 2024, comprobado añadiéndolo y viendo fallar la
  compilación—; `qyro_core` sí podía y no lo llevaba, así que ahora lo lleva. La
  lista de excepciones tiene **tres** entradas —las dos anteriores más
  `qyro_win_dpapi`, que es el crate que ADR-0024 §1 decide— y una prueba la
  vigila. Añadir la tercera fue el acto central de este sprint: es la única forma
  de que exista `unsafe` en este repositorio, y exige escribirla a mano.
- **QYR-0048 corregido antes de escribir el blob**: la entropía congelada era
  circular. La enmienda va en `df9f574`, **anterior al primer commit del blob**
  (`3f25874`). Este párrafo decía «anterior al primer commit de implementación» y
  eso era falso: `0ff21bd`, el accesor de semilla, son 217 líneas de Rust y es
  anterior a la enmienda. La intención —especificar antes de implementar lo que
  la enmienda gobierna— se cumplió; la frase que la describía, no (QYR-0055).

- **El crate de plataforma existe y llama a DPAPI**: `qyro_win_dpapi`, con
  `DpapiWrapper` implementando `SecretWrapper` y `WindowsIdentityStore`
  implementando `IdentityStore` sobre `%LOCALAPPDATA%\Qyro\identity.bin`.
  `CryptProtectData`/`CryptUnprotectData` declaradas a mano, `#[repr(C)]
  DATA_BLOB`, `CRYPTPROTECT_UI_FORBIDDEN`, ámbito de usuario. Nueve pruebas, solo
  en Windows.
- **`unsafe` existe, en un crate y en tres funciones**, enumeradas por nombre:
  `ffi.rs::take_and_free`, `store.rs::wrap` y `store.rs::unwrap`. La guarda que
  lo acota se escribió con la lista **vacía** antes de que hubiera un solo
  bloque, y el primero la puso en rojo. La lista de crates que pueden relajar
  `forbid(unsafe_code)` tiene tres entradas —`qyro_ffi`, `qyro_crypto_smoke`,
  `qyro_win_dpapi`—, cada una argumentada.
- **El harness de dos procesos existe y corre en CI**: `qyro_store_smoke`, con
  `create` y `load` como invocaciones separadas y códigos de salida estables por
  variante de fallo. El paso «Persist an identity across two separate process
  invocations» del job `windows-crypto` es lo que ejecuta la persistencia.
- **`storage-v1.json` existe**, con su schema estricto. Congela la cabecera y la
  construcción de la entropía, y **no** un blob sellado completo: la salida de
  DPAPI está atada a la máquina que la produjo, así que un blob comprometido en
  el repositorio no lo podría abrir nadie más. El archivo lo dice de sí mismo en
  `_what_is_and_is_not_here` en vez de dejar el hueco sin explicar.

Lo que **no** existe todavía, y no debe leerse como progreso:

- **No hay persistencia en Android ni en iOS.** No hay Keystore ni Keychain, y
  nada de lo anterior aplica a esas dos plataformas: en ellas, cerrar el proceso
  sigue perdiendo la identidad. Es el sprint 4D.2.
- **Nada llama al almacén desde el producto.** `qyro_ffi` no depende de
  `qyro_identity_store` —una prueba falla si alguien lo añade—, así que la
  aplicación Flutter no persiste ni carga ninguna identidad. Lo que corre en CI
  es el harness aislado.
- No hay emparejamiento ni dispositivos de confianza. Que una identidad
  sobreviva al proceso no crea por sí solo ninguna decisión de confianza.
- **No se ha probado en hardware físico.** `windows-latest` es un perfil recién
  creado, sin dominio, sin perfil móvil y sin historial de contraseñas, que son
  exactamente los casos que ADR-0024 §2 investigó y allí no se pueden ejercitar.
- El blob **no está atado a ningún valor propio de la máquina**. `LOCALAPPDATA`
  evita que viaje con un perfil móvil, pero la MasterKey sí viaja: copiar el
  archivo a mano a otra máquina del mismo usuario de dominio lo abre. Cerrarlo
  estaba fuera del alcance de este sprint y sigue abierto.
- QYR-0050 sigue abierto: la ruta del blob depende del nombre de producto, que
  sigue siendo provisional.
- QYR-0059 sigue abierto en P3: DPAPI no autentica el GUID de provider de su
  propio envoltorio, así que 128 mutaciones del blob abren igual. Devuelven **la
  misma** identidad, comprobado en el bucle del barrido; es maleabilidad en un
  campo ignorado, no sustitución de identidad.

## Runs de 4D.1

**Todos los `push` de la rama, en orden, sin filtrar.** Doce runs de este sprint
no salieron en verde: siete fallos, cuatro cancelaciones y uno que el registro
anterior contaba mal. La tabla es exhaustiva a propósito; una lista de la que se
pueden caer los fallos no es evidencia, es un resumen favorable.

| Workflow | Commit | Run | Conclusión |
|---|---|---|---|
| CI #107 | `7e272f3` | 31203268535 | **success** |
| CI #108 | `f5ed985` | 31204272720 | **success** |
| CI #109 | `8c30304` | 31204477154 | **success** |
| CI #110 | `0ff21bd` | 31205179103 | **success** |
| Platform builds #26 | `0ff21bd` | 31205179363 | **success** |
| Crypto fuzz #10 | `0ff21bd` | 31205179585 | **success** |
| Crypto platform #13 | `0ff21bd` | 31205179748 | **success** |
| CI #111 | `e0786ee` | 31205271929 | **success** |
| CI #112 | `df9f574` | 31205754229 | **success** |
| Crypto platform #14 | `3f25874` | 31206167733 | **cancelled** por concurrencia |
| Android runtime ABI #57 | `3f25874` | 31206168276 | **success** |
| CI #113 | `3f25874` | 31206168355 | **success** |
| Platform builds #27 | `3f25874` | 31206168849 | **success** |
| iOS runtime ABI #28 | `3f25874` | 31206170678 | **success** |
| CI #114 | `3527db7` | 31206287397 | **success**, 4/4 |
| **CI #115** | **`940b49d`** | **31206358256** | **FAILURE**, job `documentation` |
| Crypto platform #15 | `940b49d` | 31206358892 | **success** |
| CI #116 | `0cb18ec` | 31207950941 | **success** — la rama vuelve al verde |
| **CI #117** | **`3b2cf61`** | **31208710992** | **success** |
| **Platform builds #28** | **`3b2cf61`** | **31208710511** | **success** |
| **Android runtime ABI #58** | **`3b2cf61`** | **31208710528** | **success** |
| **iOS runtime ABI #29** | **`3b2cf61`** | **31208711030** | **success** |
| **Crypto platform #16** | **`3b2cf61`** | **31208710546** | **success** |
| **Crypto fuzz #11** | **`3b2cf61`** | **31208710539** | **success** |
| CI #118 | `0a37573` | 31208802150 | **success** |
| CI #119 | `a607550` | 31209622943 | **success** |
| Android runtime ABI #59 | `97756ad` | 31211250788 | **success** |
| Crypto platform #17 | `97756ad` | 31211250812 | **cancelled** por concurrencia; no es evidencia |
| iOS runtime ABI #30 | `97756ad` | 31211251001 | **success** |
| CI #120 | `97756ad` | 31211251308 | **success** |
| Platform builds #29 | `97756ad` | 31211252764 | **success** |
| **Crypto platform #18** | **`5d44ec8`** | **31211402008** | **FAILURE**, `LNK2019`: `Crypt32.lib` sin enlazar |
| CI #121 | `5d44ec8` | 31211402056 | **success** |
| Platform builds #30 | `5d44ec8` | 31211402323 | **success** |
| CI #122 | `23a5660` | 31211535849 | **success** |
| CI #123 | `89022c6` | 31211958948 | **success** |
| **Crypto platform #19** | **`89022c6`** | **31211959010** | **FAILURE**, QYR-0059: el byte 20 sobrevivió |
| Platform builds #31 | `89022c6` | 31211959312 | **success** |
| Platform builds #32 | `dd568a4` | 31212493685 | **success** |
| CI #124 | `dd568a4` | 31212493906 | **success** |
| **Crypto platform #20** | **`dd568a4`** | **31212494494** | **FAILURE**, la prueba seguía en rojo; su log respondió que la identidad era la misma |
| CI #125 | `764aa32` | 31212853501 | **success** |
| Platform builds #33 | `1269229` | 31213767572 | **success** |
| CI #126 | `1269229` | 31213767707 | **success** |
| **Crypto platform #21** | **`1269229`** | **31213769557** | **FAILURE**, la cota «≤16 posiciones» era falsa: eran 128 |
| **Crypto platform #22** | **`ec912ef`** | **31214233989** | **FAILURE**, la aserción exacta no llegó a aplicarse |
| Platform builds #34 | `ec912ef` | 31214234042 | **success** |
| **CI #127** | **`ec912ef`** | **31214234093** | **FAILURE**, job `documentation`, regla de deriva |
| **Crypto platform #23** | **`b731276`** | **31215102331** | **success**, 4/4 jobs — **persistencia ejecutada** |
| **CI #128** | **`b731276`** | **31215102373** | **FAILURE**, job `documentation`, misma regla de deriva |
| Platform builds #35 | `b731276` | 31215102388 | **success** |
| **CI #129** | **`91355a8`** | **31215543466** | **success**, 4/4 — la rama vuelve al verde |

**Los seis sobre `3b2cf61`, por `push`, y los seis en success.** Es el primer
commit de este sprint con evidencia de los seis, y por eso el ancla apuntó ahí
durante ese tramo. Corrieron los seis porque ese commit tocó `rust/crates/**`,
incluido el filtro de rutas que `940b49d` añadió para que `crypto-platform.yml`
vigile el crate nuevo (QYR-0045).

### Dos filas de esta tabla estaban mal (QYR-0061)

La versión anterior de esta tabla decía **`Crypto platform #14` sobre `3f25874`:
success**. Fue **cancelled**, por el grupo de concurrencia. Y decía **`CI` sobre
`0cb18ec`: run 31207659962**, que **no existe**: la API responde 404. El run real
es 31207950941.

Ninguna de las dos cambia una conclusión —la primera se sustituyó por el run #15
sobre `940b49d`, que sí pasó; la segunda tuvo su run y sí fue success—, y por eso
mismo merecen quedar escritas: una cancelación contada como éxito y un
identificador que no resuelve son las dos formas de que una tabla de evidencia
deje de serlo sin que nada falle. Se encontraron listando **todos** los runs de la
rama por API en vez de reescribir la tabla desde la memoria de la sesión, que es
exactamente de donde salieron los dos errores.

### La rama estuvo en rojo, y por qué

**CI #115 falló** (run 31206358256) sobre `940b49d`, job `documentation`:

    [BLOCKER] Stale verified commit: HEAD is 11 commits ahead of the verified
              commit (limit 10)

No fue un fallo de código. Fue **este archivo**: `Verified commit` seguía en
`c21dd72`, del sprint 4C.3, y `3527db7` estaba a exactamente diez commits —pasó
por un margen de uno— mientras `940b49d` cruzó el umbral.

La causa raíz es una política que escribí aquí y que no puede ser cierta:
«`Verified commit` se moverá cuando este sprint tenga sus propios seis en verde
sobre un mismo commit». Esa regla y el límite de diez commits **no pueden
sostenerse a la vez en un sprint largo**, y la primera no tenía por qué existir:
`HANDOFF.md` ya decía «STATUS.md debe actualizarse dentro del mismo tramo de
trabajo, no al final».

**Y volvió a pasar dos veces más**: en `ec912ef` (CI #127, 31214234093) y en
`b731276` (CI #128, 31215102373), por la misma razón y con el ancla en `3b2cf61`
—once y doce commits de deriva—. Tres ocurrencias del mismo blocker en un solo
sprint. La política de abajo es correcta y **mover el ancla hay que hacerlo, no
solo escribirlo**; el registro anterior mencionaba dos de las tres y omitía la de
`ec912ef`.

**La política que manda, y queda escrita aquí para que nadie la vuelva a
inventar:** `Verified commit` es *el commit hasta el que este archivo describe el
estado*, y se mueve **por tramo de trabajo**, no al cerrar el sprint. No es una
afirmación de que seis workflows corrieron sobre él —eso lo dice la tabla de
runs, fila por fila, con su commit—. Confundir las dos cosas es lo que dejó la
rama en rojo.

**Solo CI hasta `3f25874`, y no es una omisión.** `ci.yml` no tiene filtro de
rutas a propósito: es el job que dice si el repositorio sigue en pie, y filtrarlo
sería filtrar esa pregunta. Los otros cinco sí filtran, y hasta `3f25874` este
sprint no había tocado ninguna ruta que vigilen.

## Next task

**Sprint 4D.2: Android Keystore e iOS Keychain, detrás del mismo trait.** La
persistencia existe en una plataforma de tres, y ese desequilibrio es el estado
menos estable posible: una app que guarda la identidad en Windows y la pierde en
el teléfono se comporta de dos maneras distintas sin decirlo.

Lo que 4D.2 tiene que reproducir, no reinventar:

1. `IdentityStore` y `SecretWrapper` ya existen y no deberían cambiar. Si cambian
   para acomodar Keystore o Keychain, el trait estaba mal y eso es el hallazgo.
2. El mismo barrido de corrupción, posición por posición, **contra la API real de
   cada plataforma** y no contra un doble. Es lo que destapó QYR-0059 en Windows,
   y no hay razón para esperar que Keystore y Keychain autentiquen todos los
   bytes de su propio envoltorio solo porque sería cómodo.
3. Rotación y borrado probados, como en `rotate_replaces_exactly_one_identity` y
   `delete_leaves_nothing_loadable`.
4. Emulador y simulador según ADR-0023, con sus runs nombrados. **Ni el emulador
   ni el simulador son hardware**, y el resultado se registra como lo que es.

Preguntas abiertas que 4D.2 tiene que decidir **antes** de escribir código, con
fuente primaria citada y fechada, igual que hizo ADR-0024:

- **Secure Enclave solo hace P-256**, no Ed25519. O la identidad de Qyro deja de
  ser Ed25519 en iOS —lo que rompe el handshake congelado en ADR-0021—, o la
  semilla se envuelve con una clave del Enclave en vez de vivir en él. Son
  decisiones distintas con propiedades distintas y hay que argumentar cuál.
- `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` frente a
  `…AfterFirstUnlockThisDeviceOnly`: la primera impide leer la identidad con el
  dispositivo bloqueado, la segunda no. `ThisDeviceOnly` en las dos, porque una
  identidad de dispositivo que viaja en un respaldo deja de identificar un
  dispositivo.
- Respaldo, restauración y migración en Android. Si la identidad viaja en un
  respaldo, dos teléfonos presentan la misma; si no viaja, cambiar de teléfono la
  pierde sin aviso. Hay que elegir y decirlo.

Fuera de 4D.2 y sin fecha: atar el blob a un valor propio de la máquina, el paso
de confianza, y que algo del producto llame al almacén.

## Provisional values

Los siguientes valores son provisionales y deben bloquear el empaquetado público:

- Marcadores `REPLACE_WITH_*` en los ejemplos de branding.
- Base de identificador `com.owner.qyro`.
- Clearance del nombre de producto Qyro.
- Elección de licencia Apache-2.0.
- Autoría/licencia del logo suministrado (`design/brand/source/logo.png`).
