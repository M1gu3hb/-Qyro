# Estado canónico de Qyro

Este archivo es la única fuente de verdad para el estado ejecutable actual. Las
especificaciones y ADR describen intención; no sustituyen evidencia.

- Updated UTC: 2026-08-07T20:20:00Z
- Branch: claude/qyro-secure-storage-4d1
- Verified commit: 3b2cf613baa006545b6509c59ba8fd416694d0a8
- Milestone: formato del blob implementado y probado adversarialmente, accesor
  de semilla abierto con su guarda; **la persistencia no está implementada en
  ninguna plataforma todavía**, ni en Windows ni en Android ni en iOS, porque no
  existe el crate de plataforma que llama a DPAPI

**Qué es y qué no es «Verified commit».** Es el ancla de frescura que comprueba
`check_docs_consistency`: el commit hasta el que este archivo describe el estado.
No es, por sí solo, una afirmación de que se ejecutaron seis workflows sobre él.
La evidencia ejecutada está en las tablas de runs de más abajo, y **cada fila
dice sobre qué commit corrió**. Los runs de cierre del sprint 4C.2 se ejecutan
sobre el commit que lleva los disparadores de CI y se registran en el commit
siguiente, que es la misma secuencia que usó el sprint 4C.1.

La rama continúa `claude/qyro-crypto-platform-hardening`, que continúa
`claude/qyro-aead-replay`, que continúa `claude/qyro-handshake-closure`, que a su
vez reconcilió `audit/baseline-hardening` con los commits del propietario en
`main`. Ninguna rama fue reescrita ni fusionada a `main`. Auditoría de este
sprint: `docs/audits/SPRINT4C2_AUDIT_CLOSURE.md`.

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
- **qyro_identity y almacenamiento seguro**: NOT_IMPLEMENTED en las tres
  plataformas. No hay Android Keystore, ni iOS Keychain, ni DPAPI/CNG.
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
- Persistencia de identidad, emparejamiento y dispositivos de confianza:
  NOT_IMPLEMENTED. **La identidad sí existe** (`DeviceIdentity`, Ed25519,
  ADR-0020) y el handshake la autentica; lo que falta es que sobreviva al cierre
  del proceso, y que exista un paso de confianza.
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
- iOS/Android **hardware físico**: NO. Un emulador y un simulador no son
  hardware, y este archivo no los va a contar como tal.
- Interactive Windows application smoke: NO

## Real tests

Host Linux, Rust 1.88.0, Python 3 y PowerShell 7.4.6. **Este contenedor no trae
Flutter ni Dart**, así que todo lo que los necesita se ejecutó en CI y no aquí:

- `cargo fmt --all --check`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS, sin avisos
- `cargo test --workspace`: PASS, **348 tests**, 0 failed, 2 ignored. Eran 323
  al empezar el sprint 4D.1: la guarda de caminos públicos, cuatro sobre el
  accesor de semilla y dieciocho sobre el formato del blob
- `cargo test --workspace --all-features`: PASS, **348 tests**. Ningún crate
  declara features, así que los dos conjuntos no pueden divergir
- `cargo test --doc --workspace`: PASS
- `cargo audit --deny warnings`: PASS, 0 vulnerabilidades sobre **57 crates**.
  Eran 56: la entrada nueva es `qyro_identity_store`, un miembro del workspace.
  Este sprint **no añadió ninguna dependencia externa**, como fija ADR-0024: la
  única entrada nueva del grafo es de primera parte. `serde_json` pasó a ser también
  dev-dependency de `qyro_ffi` y ya estaba en el lock como dev-dependency de
  `qyro_crypto`, así que el grafo auditado no cambia. Siete entran con
  `chacha20poly1305`; ver `docs/LICENSE_AUDIT.md`
- `cargo tree --workspace -d`: PASS, sin duplicados
- `cargo run --package qyro_crypto_smoke -- --json`: PASS,
  `{"target":"linux-x86_64-unix","outcome":"success","code":0}`
- `bash scripts/check_crypto_platform_evidence.sh`: PASS
- `bash scripts/check_harness_isolation.sh`: PASS
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
en PowerShell. Este contenedor no trae `pwsh`, así que las dos reglas nuevas de
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
en PowerShell, y los ocho pasos pasaron. Este contenedor no trae `pwsh`, así que
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
- **La identidad solo vive en memoria.** No hay Keystore, Keychain ni DPAPI/CNG:
  generar una identidad y cerrar el proceso la pierde, así que ninguna decisión
  de confianza sobrevive a un reinicio.
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

## Sprint 4D.1 en curso — qué existe y qué no

**No hay persistencia en ninguna plataforma.** Lo que existe es la decisión, la
especificación y **el formato del blob en código y probado**; lo que no existe es
nada que persista. Este encabezado decía «decisión y especificación, no código» y
tres viñetas más abajo listaba el crate: se quedó atrás cuando el crate entró
(QYR-0055).

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
  Antes del sprint la lista estaba vacía; ahora tiene dos entradas,
  `identity.rs::export_secret` e `identity.rs::as_bytes`. Se escribió con la
  lista vacía y pasó; añadir el accesor la puso en rojo con exactamente esos dos.

- **El formato del blob está implementado y probado**: `qyro_identity_store` con
  `blob.rs`, once variantes de `StoreError` —una por paso del orden de lectura,
  más las de escritura— y dieciocho pruebas adversariales. Voltear un bit en
  cualquier posición produce un error tipado, comprobado posición por posición y
  bit por bit, y **la prueba dice por qué camino espera cada tramo**.
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
  lista de excepciones tiene **dos** entradas y una prueba la vigila.
- **QYR-0048 corregido antes de escribir el blob**: la entropía congelada era
  circular. La enmienda va en `df9f574`, **anterior al primer commit del blob**
  (`3f25874`). Este párrafo decía «anterior al primer commit de implementación» y
  eso era falso: `0ff21bd`, el accesor de semilla, son 217 líneas de Rust y es
  anterior a la enmienda. La intención —especificar antes de implementar lo que
  la enmienda gobierna— se cumplió; la frase que la describía, no (QYR-0055).

Lo que **no** existe todavía, y no debe leerse como progreso:

- **No hay crate de plataforma y no hay DPAPI.** Nada persiste nada: el trait
  `IdentityStore` está declarado y **no tiene implementaciones**. `seal_identity`
  y `open_identity` funcionan contra un envoltorio que solo existe en `cfg(test)`.
- No hay harness de dos procesos ni paso de CI que ejecute persistencia.
- No hay `storage-v1.json`.
- **No hay `unsafe` en ninguna parte del producto**, y ahora una prueba lo
  sostiene en vez de la costumbre. El crate de plataforma que ADR-0024 §1 decide
  **no existe todavía**.
- QYR-0050 sigue abierto: la ruta del blob depende del nombre de producto, que
  sigue siendo provisional.

## Runs de 4D.1

| Workflow | Commit | Run | Conclusión |
|---|---|---|---|
| CI | `7e272f3` | 31203268535 | **success** |
| CI | `f5ed985` | 31204272720 | **success** |
| CI | `8c30304` | 31204477154 | **success** |
| CI | `e0786ee` | 31205271929 | **success** |
| CI #112 | `df9f574` | 31205754229 | **success** |
| CI #113 | `3f25874` | 31206168355 | **success** |
| CI #114 | `3527db7` | 31206287397 | **success**, 4/4 |
| **CI #115** | **`940b49d`** | **31206358256** | **FAILURE**, job `documentation` |
| CI | `0cb18ec` | 31207659962 | **success** — la rama vuelve al verde |
| **CI** | **`3b2cf61`** | **31208710992** | **success** |
| **Platform builds** | **`3b2cf61`** | **31208710511** | **success** |
| **Android runtime ABI** | **`3b2cf61`** | **31208710528** | **success** |
| **iOS runtime ABI** | **`3b2cf61`** | **31208711030** | **success** |
| **Crypto platform** | **`3b2cf61`** | **31208710546** | **success** |
| **Crypto fuzz** | **`3b2cf61`** | **31208710539** | **success** |
| CI | `0a37573` | 31208802150 | **success** |

**Los seis sobre `3b2cf61`, por `push`, y los seis en success.** Es el primer
commit de este sprint con evidencia de los seis, y por eso el ancla apunta ahí.
Corrieron los seis porque ese commit tocó `rust/crates/**`, incluido el filtro de
rutas que `940b49d` añadió para que `crypto-platform.yml` vigile el crate nuevo
(QYR-0045).
| Crypto platform #14 | `3f25874` | ver §runs | **success** |
| Platform builds #27 | `3f25874` | ver §runs | **success** |
| Android runtime ABI #57 | `3f25874` | ver §runs | **success** |
| iOS runtime ABI #28 | `3f25874` | ver §runs | **success** |

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

**El crate de plataforma Windows.** Es lo único que falta para que algo persista;
todo lo demás del sprint está hecho y verificado. En este orden:

1. `only_the_listed_crates_may_relax_forbid_unsafe` ya existe y la lista de
   excepciones tiene dos entradas. Añadir el crate de plataforma es **una tercera
   entrada argumentada**, no un `forbid` que nunca estuvo: eso es exactamente por
   qué la guarda se escribió antes.
2. `the_unsafe_blocks_are_the_ones_we_listed`, **antes** de escribir un bloque
   `unsafe`. Por nombre de función contenedora, no por número: sustituir un bloque
   por otro deja el número igual.
3. El `extern "system"` a mano para `CryptProtectData`, `CryptUnprotectData` y
   `LocalFree`, más `#[repr(C)] DATA_BLOB`. `CRYPTPROTECT_UI_FORBIDDEN`, ámbito de
   usuario, `pPromptStruct = NULL` (ADR-0024 §2). Borrar el búfer **antes** de
   `LocalFree`: liberar sin borrar es el defecto que QYR-0018 ya cerró una vez.
4. `a_data_blob_survives_a_protect_and_unprotect` en `windows-latest`. Es la
   mitigación que ADR-0024 §1 prometió a cambio de transcribir el `extern` a mano.
5. La implementación de `IdentityStore` sobre `%LOCALAPPDATA%`, con QYR-0050
   resuelto o registrado, y `two_creates_do_not_lose_data` decidido y probado.
6. El barrido de 448 posiciones **contra DPAPI**, no contra el doble. Los tres
   tramos deben caer por los mismos caminos; si alguno cae por otro, eso es el
   hallazgo.
7. El harness de dos procesos (ADR-0023) y su paso en `windows-crypto`.
8. `storage-v1.json` verificado **desde las primitivas**, no desde el módulo que
   lo produce (QYR-0025).

Después: los seis workflows en verde sobre un mismo commit, y `Verified commit`
movido a él.

## Provisional values

Los siguientes valores son provisionales y deben bloquear el empaquetado público:

- Marcadores `REPLACE_WITH_*` en los ejemplos de branding.
- Base de identificador `com.owner.qyro`.
- Clearance del nombre de producto Qyro.
- Elección de licencia Apache-2.0.
- Autoría/licencia del logo suministrado (`design/brand/source/logo.png`).
