# Estado canónico de Qyro

Este archivo es la única fuente de verdad para el estado ejecutable actual. Las
especificaciones y ADR describen intención; no sustituyen evidencia.

- Updated UTC: 2026-08-16T12:00:00Z
- Branch: claude/qyro-net-6a
- Verified commit: 813eb95d96e4864a27b4299cf5d79e0e8ad3f7d7
- Milestone: **v1.0. El producto está completo en código y no lo ha usado
  nadie.** Un archivo se elige con el selector del sistema, viaja por un socket
  TCP cifrado y autenticado entre dos procesos, se verifica con SHA-256 y se
  entrega — y hay cuatro pantallas, en dos idiomas, con los botones **encendidos**
  desde la fase 05. Dos aparatos se emparejan **con un código tecleado**, que
  desde la fase 12 el receptor enseña de verdad (QYR-0322); **no se
  encuentran solos**, porque el descubrimiento no cruza la frontera C y es
  la fase 14. La identidad sobrevive al proceso en las dos plataformas
  (fase 11): DPAPI en Windows y **el sandbox por UID en Android, no
  Keystore** (ADR-0040 §7), y el paquete se llama `dev.qyro.app` y se
  firma con una clave real (fase 08). **Lo que sigue sin existir es la
  evidencia**: ningún teléfono ha ejecutado nunca esta aplicación, ninguna
  transferencia ha cruzado una Wi-Fi de verdad, y `flutter build` no corre en
  esta máquina por el Modo Desarrollador (QYR-0324). Dos procesos en
  `127.0.0.1` no son dos aparatos en una red. Los veinte escenarios que cierran
  ese hueco están escritos y **sin marcar** en `docs/testing/hardware-protocol.md`

### v1.0.0 — la etiqueta y sus artefactos

`v1.0.0` apunta a `98200e1`, cuyos tres workflows terminaron en **success** sobre
`386015a` (ver «Fase 11 — runs de cierre»).

Los artefactos los construyó el run **31994299360** de `release.yml` sobre
`4c297af`, un commit por delante de la etiqueta. La diferencia son tres archivos
—`release.yml`, `BUGS_PENDING.md`, `ESTADO-ACTUAL.md`— y **ninguna línea de
código del producto**, comprobable con `git diff --stat v1.0.0 4c297af`. La
etiqueta **no se movió**: mover una etiqueta es reescribir historia.

| Artefacto | SHA-256 |
|---|---|
| `app-release.apk`, firmado con la clave de release | `d0d7afaa…225f700a` |
| `qyro-windows-x64.zip` | `4e21923c…c17e0370` |
| `app-release-debugkey.apk`, el de CI | `f17125de…3a961a2e` |

Los tres completos están en `docs/release/v1.0.md`. El certificado del APK
firmado se verificó con `apksigner verify --print-certs`: **un solo firmante**,
`CN=Qyro`, RSA 4096, esquemas v2 y v3, y el digest que ese documento publica.

**No existe una GitHub Release.** La etiqueta es lo que se pidió; publicar
binarios en abierto es una decisión del propietario, sobre software que nadie ha
ejecutado nunca en un teléfono.

### Fase 11 — runs de cierre

Los tres sobre `386015a`, disparados por el `push`, y **es el primer commit de
este proyecto en el que las dos pruebas que importan existen y pasan**.

| Workflow | Run | Conclusión |
|---|---|---|
| CI | 31993329781 | **success** |
| Platform builds | 31993329863 | **success**, 3/3 jobs |
| Android runtime ABI | 31993329786 | **success** |

Dos pasos concretos, porque son los que sostienen esta fase y la 06:

- **`An identity survives a process, through the engine`** (job `rust`):
  **success**. Falla en cualquier commit anterior a la fase 11 — dos procesos
  abren el mismo archivo a través de `qyro_session` y comparan la huella, con su
  control de falsabilidad.
- **`The identity survives, checked inside an application process`** (job
  `android`): **success**. Estuvo en rojo dos commits por QYR-0350 y **no se
  había ejecutado nunca**, así que la evidencia que la fase 06 daba por hecha
  existe por primera vez aquí.

### Fase 11 — CERRADA. La identidad, y lo que costó no tenerla

Informe: `docs/reports/fase-11-la-identidad.md`. Decisión: ADR-0040.

**La aplicación no tenía identidad estable.** Los tres constructores de `Session`
llamaban a `DeviceIdentity::generate()` sin condición, así que cada transferencia
estrenaba un par de claves — y con eso ni la comparación de huella ni el código
de emparejamiento funcionaban. El mecanismo, el formato y el backend DPAPI eran
reales y nada los unía (QYR-0353, P0).

**La evidencia que lo ocultó cinco fases** era el paso «Persist an identity
across two separate process invocations», que ejecuta un arnés marcado «Never
shipped» y no pasa por `qyro_session` ni por `qyro_ffi`.

**Lo que existe ahora:** una identidad por proceso, cargada de un archivo que el
llamante nombra, que **nunca se regenera** si el blob no abre; tres símbolos
nuevos; Dart la abre antes de cualquier sesión, enseña su huella y pregunta al
libro. Y el paso de CI «An identity survives a process, through the engine», que
falla en cualquier commit anterior.

**Keystore está descartado para la v1.0** con argumento (QYR-0354): el mecanismo
de ADR-0037 no es implementable y el que sí funciona necesita un shim JNI que
nadie puede validar aquí. Android guarda la semilla sin envolver, bajo el sandbox
por UID, y `THREAT_MODEL.md` lo dice con estas palabras: **con Keystore, root
necesita además el TEE; sin él, root basta.**

### v1.0 — qué es y qué no es

Documento de release: `docs/release/v1.0.md`. Modelo de amenazas reescrito
contra el código que existe: `THREAT_MODEL.md`.

**Lo que existe y está ejecutado en Windows 10 real y en CI:**

- **Transferencia completa de extremo a extremo**, conducida desde Dart, entre
  dos procesos de sistema operativo, con verificación byte a byte.
- **Descubrimiento**: NO_ALCANZABLE. `NsdManager` con `FLAG_SHOW_PICKER` y
  `mdns-sd` bajo `cfg(windows)` están escritos y probados, y **ningún símbolo
  de la superficie C los alcanza**: `DiscoveryChannel.kt` está registrado y
  ningún archivo de Dart abre el canal `dev.qyro/discovery`. Se declara fuera
  de la v1.x aquí en vez de anunciarse; la fase 14 lo conecta.
- **Identidad persistente en las dos plataformas** que la v1.0 tiene.
- **Confianza explícita con interfaz**: una clave cambiada se rechaza por nombre
  y el botón de enviar **no existe** en ese estado.
- **Diecinueve símbolos C**, ninguno cruza un tipo.
- `cargo test --workspace`: **639 passed, 0 failed, 2 ignored**. `flutter test`:
  **92 pasadas, 9 saltadas** — las diez saltan sin la biblioteca nativa compilada
  o sin el manifiesto fusionado, y saltada no es pasada.
- `Cargo.lock`: **80 paquetes**. `pubspec.lock`: **45**.
- `BUGS_PENDING.md`: **155 fichas, 0 abiertas.**

**Lo que NO debe leerse como progreso:**

- **Cero evidencia de hardware físico.** Es la fase 07 y está sin ejecutar.
- **iOS está fuera** (ADR-0039): Xcode exige macOS.
- Sin transferencia en segundo plano, sin cola, sin ajustes, sin cámara.

### Fase 05 — CERRADA. La interfaz, y los botones ENCENDIDOS

Informe: `docs/reports/fase-05-la-interfaz-y-los-botones.md`.
Decisión de producto: ADR-0036. Superficie C: ADR-0032 enmienda 1.

**Los botones Enviar y Recibir están encendidos**, y el texto que explicaba por
qué estaban apagados está borrado del catálogo, de la pantalla y de las dos
pruebas que lo esperaban. Las cinco condiciones se cumplen y cada una tiene su
prueba con nombre; la que las gobierna a todas —«nadie ha visto esto en una
pantalla»— sigue siendo cierta y está en §15 de ese informe.

**Lo que existe y está ejecutado en Windows 10 real:**

- **La superficie C pasa de once símbolos a veintitrés** y ninguno cruza un tipo:
  enteros, o texto en un búfer que el llamante presta. Cuando no cabe **no se
  escribe nada**, porque media huella que coincide no prueba nada.
- **Una clave cambiada se refuta por nombre desde Dart**, con dos procesos
  receptores y por tanto dos identidades bajo un mismo nombre.
- **El receptor puede decir que no**, el emisor aprende el motivo exacto, y el
  destino queda sin un solo archivo — comprobado listando el directorio.
- **Cuatro pantallas** con todos sus estados feos probados contra un doble, y la
  entrada manual y el QR en la primera, nunca detrás de «avanzado».
- **Un peer con clave cambiada no ofrece botón de aceptar** — ausente, no
  atenuado: no hay «continuar de todos modos» que encontrar.
- 623 tests de Rust y 94 de Dart. `Cargo.lock` sigue en **64**.

**Lo que NO debe leerse como progreso:** nadie ha visto esta interfaz —
`flutter build` no corre en esta máquina (QYR-0324)—, no hay descubrimiento
automático, el QR no se lee con una cámara, y la confianza no sobrevive al cierre
de la aplicación hasta la fase 06.

### Fase 03 — CERRADA COMO PARCIAL. El selector de archivos

Informe completo: `docs/reports/fase-03-selector-de-archivos.md`.

**Lo que existe y está ejecutado en Windows 10 real:**

- **Android elige por descriptor y Windows por ruta**, que es ADR-0034. En
  Android un `MethodChannel` propio de ~140 líneas de Kotlin abre con `"rw"` y
  entrega el entero de `detachFd()`; `file_selector_android` **copia el archivo
  entero a la caché** antes de que Dart lo vea (QYR-0323) y por eso no se usa.
- **En Windows la dependencia es `file_selector_windows` 0.9.3+5**, la
  implementación endosada, y **no** el paraguas `file_selector`: el paraguas
  arrastra siete paquetes más y uno de ellos es la implementación que copia
  (ADR-0034, enmienda 1). Medido: 37 → 45 paquetes contra 37 → 52.
- **Una transferencia conducida por descriptor llega byte a byte idéntica**, y
  los nombres que sólo el selector conoce viajan correctos — un descriptor no
  tiene nombre propio.
- **Cero crates de Rust nuevos.** `Cargo.lock` sigue en **64 paquetes**, 50 de
  crates.io.
- `cargo test --workspace` en Windows: **603 passed, 0 failed, 2 ignored**;
  `flutter test`: **76 passed, 1 skipped**.

**Lo que NO debe leerse como progreso:**

- **Nadie ha visto ningún selector.** El diálogo de Windows no se abre en
  `flutter test` —no hay ventana— y `flutter build windows` no corre en esta
  máquina. El SAF de Android no se ha ejecutado en ningún emulador ni teléfono.
  El criterio 7 de la fase **no está cumplido** y por eso la fase es PARCIAL.
- **`the_descriptor_is_closed_exactly_once` es `cfg(unix)`** y no corre en
  Windows. Su evidencia viene del job de Linux.
- Sigue sin haber descubrimiento, sin UI y sin ninguna pantalla. **Los botones
  Enviar y Recibir siguen `onPressed: null`.**

**Tres defectos de verificación que esta fase encontró y cerró:** un byte NUL
crudo que hacía que `grep` saltara un archivo entero (QYR-0327), un `}` escrito
como literal de carácter que truncaba el análisis de las guardas en siete crates
(QYR-0328, P1, misma forma que QYR-0071), y la prueba del manifiesto fusionado,
que **no se había ejecutado nunca en ninguna parte** (QYR-0329, P1).

### Fase 02 — CERRADA. Dart conduce una transferencia real

**Lo que cambió y está ejecutado en Windows 10 real:**

- **`qyro_session` tiene pruebas de conducta por primera vez** (QYR-0309
  cerrada): diez, que conducen un emisor y un receptor en dos hilos por un socket
  de loopback real y comparan el archivo byte a byte. Antes tenía seis, todas
  estructurales, y ninguna abría un socket
- **Una transferencia íntegra se le reportaba al emisor como `PeerUnreachable`**
  (QYR-0316, P1, cerrada). El receptor producía su frame `IntegrityResult` y
  `advance` salía sin escribirlo. Dart conduce ese lado en esta fase
- **El checker de documentación estaba rojo en Windows PowerShell 5.1** desde
  antes de la fase 01 (QYR-0311): `-Include` no filtraba y recorría 5 962
  archivos donde declara 284. Arreglado, con un caso de contrato en las dos
  mitades que se vio fallar
- **ADR-0033 congelada** antes de una línea de código: el puente de progreso, con
  su presupuesto de emisiones acotado por una constante y no por el tamaño del
  archivo
- **Dart conduce una transferencia real.** Un archivo de 8 MiB + 13 bytes cruza
  **dos procesos de sistema operativo**, por un socket, conducido desde Dart a
  través de la superficie C, y llega **idéntico byte a byte**. El receptor es
  `qyro_net_smoke serve`. Con progreso monótono que termina en el total y dentro
  del presupuesto de 102 emisiones de ADR-0033
- **ADR-0038 congelada**: Dart no puede asignar memoria nativa sin
  `package:ffi`, así que los búferes se piden prestados a Rust. Sube la
  superficie `extern "C"` de ocho símbolos a diez
- `cargo test --workspace` en Windows: **595 passed, 0 failed, 2 ignored**;
  `flutter test`: **62 passed**

**Lo que sigue sin existir, y no ha cambiado:**

- Sin selector de archivos, sin descubrimiento, sin UI. **Los botones Enviar y
  Recibir siguen `onPressed: null`**, y el test que lo comprueba sigue pasando
- **Cero pruebas en hardware físico.** Dos procesos en `127.0.0.1` **no** son dos
  aparatos en una Wi-Fi: no hay descubrimiento, ni MTU real, ni pérdida de
  paquetes, ni dos sistemas operativos distintos
- El receptor **no informa de progreso** (QYR-0317) y `Progress::item` vale cero
  siempre (QYR-0318). Dart conduce el lado emisor, que sí informa bien

### Sprint 6A — `qyro_net`, hasta la Puerta 5 y la rama de Codex fusionada

**Esta rama contiene ya el trabajo de los dos agentes.**
`codex/qyro-gap-closure-5c` se fusionó en `157bd9f`: los dos únicos conflictos
fueron `BUGS_PENDING.md` y `STATUS.md`, resueltos conservando ambos lados. Las
secciones de Codex de este documento siguen abajo, intactas.

**Lo que existe y está ejecutado en Linux:**

- `qyro_net`: listener con presupuesto de conexiones, dialer con plazo, y un
  `FrameStream` que alimenta el decodificador incremental de `qyro_protocol` sin
  reimplementarlo. `#![forbid(unsafe_code)]`
- El handshake autenticado de cuatro mensajes de ADR-0021 **sobre un socket
  real**, y frames sellados en las dos direcciones después
- Finales tipados: ninguno se llama `Io`. Sólo envenenan los dos que significan
  que los bytes mintieron — un framing inválido y un tag que no verifica
- `qyro_net_smoke`: dos procesos de verdad, `serve` y `send`, con una prueba de
  integración que los lanza con `std::process::Command`
- Los cinco finales de ADR-0028 §5 provocados de verdad, incluido matar el
  proceso remoto con `Child::kill()`, y sin hilos ni descriptores supervivientes

**Lo que NO existe, y no debe leerse como progreso:**

- **Nada de esto es alcanzable desde Dart.** `qyro_ffi` sigue exponiendo sólo la
  versión del protocolo; el motor, el disco y la red son inalcanzables desde la
  app. Los botones siguen deshabilitados y deben seguirlo
- **No hay descubrimiento.** La dirección del peer se la pasa el llamante
- **`qyro_net` no se compila ni se ejecuta en Windows** (QYR-0078). Es el
  crate donde más diverge el sistema operativo, y es el que menos se prueba
- **Ninguna prueba en hardware físico ni entre dos máquinas.** Todo es loopback,
  que no pierde paquetes, no reordena y no se parece a ninguna red real
- El barrido de mutación completo sobre `qyro_net` y las guardas del crate
  todavía no están hechos: son las fases 6 en adelante

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

- Confianza explícita de peers (ADR-0031): almacén `QYRO-KPS` versionado y
  envuelto, huella humana de 128 bits y veredictos tipados
  `KnownAndMatches`/`KnownAndChanged`/`New`: IMPLEMENTED como mecanismo puro,
  sin UI ni autorización automática para un peer nuevo.
- Historial local append-only `QYRO-HST`: IMPLEMENTED en `qyro_fs`, con CRC por
  registro, recuperación truncando el primer tail inválido y consultas por
  últimos N, peer y estado. No hay sincronización, base de datos ni UI.
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
  IMPLEMENTED. Igualdad exacta con `{qyro_ffi, qyro_core}`. **Superado en la
  fase 01:** esa igualdad ya no se cumple ni puede cumplirse, y la guarda pasó a
  ser el conjunto de dependencias *directas*. ADR-0032 §9
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

- **Handshake y frames sobre transporte**: IMPLEMENTED, EJECUTADO. **Corregido en
  la fase 10:** esta línea decía NOT_IMPLEMENTED y era cierta hasta el sprint 6A.
  El handshake corre sobre un socket real desde entonces y Dart conduce la
  transferencia entera desde la fase 02.
- **Rotación y rekey de claves de sesión**: NOT_IMPLEMENTED. Una sesión usa una
  clave por dirección hasta agotar la secuencia.
- **Identidad persistente en la aplicación**: IMPLEMENTED, EJECUTADO entre dos
  procesos (ADR-0040). **Corregido dos veces, y la segunda importa más que la
  primera.** La fase 10 cambió «sólo en Windows» por «las dos plataformas», que
  era falso de otra manera: nada del producto llamaba al almacén y los tres
  constructores de `Session` generaban un par de claves por sesión. La fase 11
  lo arregló de verdad. Hoy:
  - **Windows**: envuelta con DPAPI de ámbito de usuario (ADR-0024).
  - **Android**: **sin envolver**, en el directorio privado de la aplicación,
    protegida por el sandbox por UID (ADR-0040 §7, etapa A). Keystore está
    descartado para la v1.0 con argumento en QYR-0354, y lo que eso cuesta está
    en `THREAT_MODEL.md` §4.5 con estas palabras: con Keystore, root necesita
    además el TEE; sin él, root basta.
  - iOS queda fuera con ADR-0039.
- **FFI criptográfico**: NOT_IMPLEMENTED, y deliberadamente. La biblioteca que
  Dart carga no depende de `qyro_crypto`, así que no hay nada de esto al otro
  lado de la frontera.
- Golden tests de arranque: NOT_IMPLEMENTED
- Benchmark de arranque documentado: NOT_IMPLEMENTED
- Retención de artefactos: IMPLEMENTED para la v1.0 (`release.yml`). El APK
  release y el ZIP portable de Windows se construyen sobre la etiqueta, cada uno
  con su `SHA256SUMS.txt` **dentro** del paquete y su SHA-256 publicado en
  `docs/release/v1.0.md`. Los artefactos de desarrollo de `platform-builds.yml`
  siguen siendo debug y lo dicen en su `BUILD-INFO.txt`.
- Campaña **exhaustiva** de fuzzing: NOT_IMPLEMENTED. Hay una acotada, semanal,
  de dos minutos por target, en `crypto-fuzz.yml`.
- Transporte y sockets: IMPLEMENTED, EJECUTADO (`qyro_net`, ADR-0028). **TLS:
  NOT_IMPLEMENTED y descartado** — ADR-0004 queda superada por ADR-0021 y
  ADR-0022, con la enmienda que dice por qué
- Transferencia de producto por red y por interfaz: IMPLEMENTED, EJECUTADA
  entre dos procesos y **nunca en hardware físico**. **Corregido en la fase 10.**
- **Interfaz de transferencia**: IMPLEMENTED (fase 05, ADR-0036). Cuatro
  pantallas, los dos idiomas, y **los botones encendidos**. NO vista en ninguna
  pantalla: `flutter build` no corre en la máquina de desarrollo (QYR-0324).
- **Confianza consultada desde la aplicación**: IMPLEMENTED (ADR-0032 enmienda
  1). Una clave cambiada se refuta por nombre desde Dart, y **persiste** desde la
  fase 06.
- **Rechazo del receptor**: IMPLEMENTED (QYR-0089, QYR-0088). `TransferReject` se
  emite y se entiende, `Phase::Rejected` no es `Cancelled`, y `FileSink::abandon`
  deja el destino como lo encontró.
- Selección de archivos: **IMPLEMENTED en código, NO EJECUTADA por nadie**
  (fase 03, ADR-0034). El `MethodChannel` de Android y el diálogo de Windows
  existen y están probados aguas abajo del diálogo; **ningún diálogo se ha
  abierto** en esta máquina ni en CI. La integración del manifest con el
  filesystem sí está ejecutada: `manifest_from_disk` y `manifest_from_open_files`
  construyen desde el disco y desde descriptores ya abiertos, con transferencias
  verificadas byte a byte en los dos caminos.
- LAN, descubrimiento y código manual: IMPLEMENTED (fase 04b, ADR-0035).
  `NsdManager` con `FLAG_SHOW_PICKER` en Android, `mdns-sd` bajo `cfg(windows)`,
  y el código `QYRO1|<socket-addr>|<32 hex>` tecleado, que es el camino que
  funciona con aislamiento de cliente. **No ejecutado en una red real.**
- Reanudación por red/UI: NOT_IMPLEMENTED. Los metadatos locales de
  `.qyro-resume` sí sobreviven entre procesos y `qyro_fs` los aplica.
- UI y política interactiva de emparejamiento: IMPLEMENTED (fase 05, ADR-0036;
  enmienda de ADR-0031 con la tabla de dónde aterrizó cada línea). **Corregido en
  la fase 10.**
- Base de datos o historial sincronizado: NOT_IMPLEMENTED. El historial local
  append-only sí existe y deliberadamente usa `Vec<T>` con iteradores.
- Optical QR/RaptorQ: NOT_IMPLEMENTED y **descartado para la v1.0** (ADR-0005
  enmendada). No hay cámara en la aplicación y hay una prueba que lo mantiene
  así (QYR-0348)
- Wi-Fi Direct/Multipeer/Bluetooth: NOT_IMPLEMENTED y **descartado para la
  v1.0** (ADR-0009 enmendada)
- Share Target Android, Share Extension iOS, drag and drop Windows: NOT_IMPLEMENTED
- SBOM y cargo-deny: NOT_IMPLEMENTED, **descartado con argumento** en la
  enmienda de ADR-0010: `Cargo.lock` y `pubspec.lock` están versionados y son la
  lista completa. `cargo audit --deny warnings` sí corre, sobre 80 paquetes

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

## Sprint 5C — cierre de gaps y deuda estructural

- Las Puertas 1–10 están cerradas. Fase 9 instaló un mínimo estructural común en
  diez de los once miembros del workspace; `qyro_ffi` conserva la única
  excepción presente, por su contrato ABI C dedicado. La meta-guarda falla si
  un crate no exceptuado pierde el archivo, su activación, la lista productiva,
  anti-panic, fin-de-análisis, antitautología o construcción de errores/veredictos.
- El barrido ampliado ejecutó 939/939 mutantes potenciales de `qyro_fs`,
  `qyro_protocol`, `qyro_manifest`, `qyro_identity_store` y `qyro_crypto`.
  La unión Windows/Linux contiene 161 supervivientes únicos: 25 cerrados y 136
  abiertos en QYR-0115–QYR-0275; 12 timeouts separados siguen abiertos en
  QYR-0276–QYR-0287. No se presentan los abiertos como cobertura lograda.
- Esto endurece evidencia y contratos; no añade red, FFI del motor, UI,
  selector de archivos ni persistencia móvil.
- Fase 10 consolidó el ledger, las decisiones, este estado y el informe. CI
  31549905688 sobre el commit documental `5736077` terminó en success con sus
  ocho jobs.

## Sprint 5D — ledger legible, confianza e historial

- El ledger conserva 99 fichas y 18 abiertas. Las 188 entradas mecánicas del
  barrido anterior se sustituyeron por diez fichas humanas QYR-0289–QYR-0298;
  el inventario completo de 939 mutantes vive en el informe de mutación.
- ADR-0031 y `qyro_identity_store` implementan la decisión local de confianza
  sin bool ni TOFU silencioso. Una clave conocida que cambia se rechaza por
  nombre y un peer nuevo se reporta como nuevo, no como confiable.
- `qyro_fs` implementa un historial local append-only con recuperación real de
  un registro escrito a medias. 10 000 registros ocuparon 720 012 bytes y se
  parsearon en 72.6051 ms en Windows debug; el test de falsabilidad rechaza
  deliberadamente 500 ms + 1 ns.
- El barrido final sobre todo el código nuevo produjo 209 mutantes: 180 caught,
  0 missed, 29 unviable y 0 timeout. `Cargo.lock` sigue byte-idéntico a
  `ebdffb9` y conserva 61 paquetes.
- Esto no añade red, descubrimiento, FFI, UI, selector de archivos ni política
  interactiva. Ninguna parte de 5D se ha ejecutado en hardware físico.
- Los seis workflows manuales sobre `ccc54ae` terminaron en success: CI
  31563755263, Platform builds 31563756867, Crypto platform 31563758336,
  Crypto fuzz 31563759613, Android runtime ABI 31563761200 e iOS runtime ABI
  31563762494. CI ejecutó 487/0/2 tests en Linux y 494/0/2 en Windows.

## Real tests

Evidencia actual: Linux y Windows en CI, además de Windows local, con Rust
1.88.0.

**Corregido el 2026-08-14 (fase 03):** este párrafo decía «el host local no trae
Flutter ni Dart». Ya los trae — Flutter 3.44.8 y Dart 3.12.2 en `D:\flutter`, la
misma versión que CI fija—, así que `flutter test`, `flutter analyze` y
`dart format` **sí** se ejecutan localmente y sus números están en el informe de
la fase 03. Lo que sigue sin correr aquí es `flutter build` y `flutter run` con
plugins, por el Modo Desarrollador (QYR-0324). Los números de esta lista son de
sprints anteriores y no se reescriben; los actuales están en el informe de fase.

- `cargo fmt --all --check`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS, sin avisos
- `cargo test --workspace`: PASS. Rama actual: **487 passed en Linux CI y 494
  passed en Windows CI** (run 31563755263), 0 failed y 2 ignored en ambos. La base era 388/394.
  El delta Windows +6 está explicado test por test: Windows añade los ocho
  `qyro_win_dpapi::tests::{a_data_blob_that_lies_does_not_round_trip,
  a_single_flipped_byte_is_a_typed_error_against_dpapi,
  a_wrapped_secret_needs_the_same_entropy,
  an_unreadable_store_is_not_an_absent_one, delete_leaves_nothing_loadable,
  load_on_an_empty_store_is_a_typed_absence, rotate_replaces_exactly_one_identity,
  two_creates_do_not_lose_data}`; Unix añade los dos
  `qyro_fs::tests::{a_symlink_at_the_final_part_component_is_refused_without_touching_its_target,
  a_symlink_in_the_destination_cannot_redirect_a_write}`. La novena prueba de
  DPAPI, `guards::the_unsafe_blocks_are_the_ones_we_listed`, corre en ambos y no
  altera la diferencia. Eran 369 al
  empezar el sprint 5B.1: quince del filesystem y cuatro de las guardas y los
  veredictos. Eran 352 al empezar el sprint 5A; las diecisiete nuevas son las del motor de transferencia,
  y cuatro de ellas existen porque el barrido de mutación encontró sin cubrir
  cuatro negativas de ADR-0026 §4. Eran 350 al empezar 4D.2a, y 323 al empezar
  4D.1: la guarda de caminos públicos, cuatro sobre el accesor
  de semilla, dieciocho sobre el formato del blob y dos sobre el `unsafe` del
  crate de plataforma.
- `cargo test --workspace --all-features`: PASS, **487 passed en Linux**, 0
  failed, 2 ignored. `qyro_fs` declara una feature de fixture Windows; no añade
  una prueba al conjunto Linux
- `cargo test --doc --workspace`: PASS
- `cargo audit --deny warnings`: PASS, 0 vulnerabilidades sobre **61 crates**.
  La entrada nueva del sprint 5B.1 es **`qyro_fs`, de primera parte**: el diff de
  `Cargo.lock` tiene exactamente una línea `name =`. `O_NOFOLLOW` sale de
  `std::os::unix` y `FILE_FLAG_OPEN_REPARSE_POINT` de `std::os::windows`, así que
  la política de symlinks no costó ninguna dependencia. Antes eran 60:
  La entrada nueva del sprint 5A es **`qyro_transfer`, de primera parte**: el
  diff de `Cargo.lock` tiene exactamente una línea `name =`. `sha2` pasó a ser
  dependencia directa suya y ya estaba en el grafo por `qyro_crypto`, así que es
  una arista nueva y no un paquete nuevo. **El sprint 5A no añadió
  ninguna dependencia externa.** Antes eran 59:
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
- Los cuatro scripts `check_*` en **Bash y Windows PowerShell 5.1**: PASS las
  ocho invocaciones. Dos rondas previas encontraron y corrigieron usos de
  `Join-Path` que sólo aceptaba PowerShell Core; el contrato criptográfico
  también se ejecutó completo bajo 5.1
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

**Los de la v1.0** (`release.yml`, sobre la etiqueta):

- `qyro-android-release`: el APK, su `SHA256SUMS.txt` y un `BUILD-INFO.txt` que
  dice en mayúsculas que **está firmado con la clave de depuración**. El APK
  firmado con la clave release se produce aparte, localmente, y su SHA-256 y el
  del certificado están en `docs/release/v1.0.md`. La clave privada **no entra en
  un secreto de repositorio**, así que el paso de firma no puede vivir en CI.
- `qyro-windows-x64-release`: el ZIP portable, con `SHA256SUMS.txt` de **cada
  archivo** dentro del paquete y su `BUILD-INFO.txt`. Sin firmar: MSIX y
  Authenticode quieren un certificado que cuesta dinero (ADR-0010 enmendada), y
  SmartScreen avisará, con razón.
- Retención 90 días.

**Los de desarrollo** (`platform-builds.yml`):

- `qyro-windows-x64-portable-debug`, 14 días, con su `SHA256SUMS.txt` y su
  etiqueta DEVELOPMENT / NOT FOR PUBLIC RELEASE. **Corregido en la fase 10:** su
  `BUILD-INFO.txt` afirmaba «Qyro does not transfer files: Send and Receive are
  disabled», cierto en la fase 02 y falso desde la 05. Una frase falsa dentro de
  un artefacto viaja con el binario y nadie la relee.
- El APK de debug y el `Runner.app` de iOS **no** se retienen: existen sólo
  dentro de runners efímeros. Para instalar algo, el artefacto es el de
  `release.yml`.
- El digest que GitHub imprime al subir un artefacto identifica el ZIP que
  produjo ese run, no el contenido que alguien desempaqueta; no se usa como
  sustituto del `SHA256SUMS.txt` que va dentro.
- `crypto-fuzz.yml` retiene corpus y artefactos de crash por target, 30 días.
  Son cadenas de bytes que eligió el fuzzer y no contienen material de clave: la
  única sesión en juego es la fija de `qyro_crypto::fuzzing`, cuyas semillas
  están publicadas en este repositorio y comprometidas por definición.
- No hay IPA ni MSIX, y no los habrá en la v1.0: iOS está fuera por ADR-0039 y
  MSIX quiere un certificado de pago (ADR-0010 enmendada).

## Blockers

**El bloqueador de la v1.0, y es uno solo:**

- **Nada se ha ejecutado en hardware físico.** Ni un teléfono, ni una Wi-Fi. Todo
  lo demás de esta lista es una limitación conocida y acotada; esto es un hueco
  de evidencia sobre el producto entero. `docs/testing/hardware-protocol.md`
  tiene los veinte escenarios con su comando literal y **sus veinte huecos en
  blanco**.

**Corregidos en la fase 10 — tres bloqueadores que ya no lo son:**

- «No hay transporte» — lo hay desde el sprint 6A, y Dart lo conduce desde la
  fase 02.
- «La identidad sólo vive en memoria en Android y en iOS» — persiste en Android
  desde la fase 06 (ADR-0037). En iOS no, y iOS está fuera de la v1.0 (ADR-0039).
- «Nada del producto llama al almacén» — la aplicación llama, por los dos
  punteros a función que Kotlin instala. Lo que sigue siendo cierto, y sigue
  siendo deliberado, es que `qyro_ffi` no depende de `qyro_crypto`.
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
- Los artefactos de la v1.0 sí llevan `SHA256SUMS.txt` dentro del paquete y su
  SHA-256 publicado. Lo que ninguna máquina de este proyecto puede hacer es
  **construirlos localmente**: el Modo Desarrollador de Windows está apagado
  (QYR-0324), así que los construye CI y la firma release se aplica aparte.
- La campaña de fuzzing es **acotada**: dos minutos por target, semanal. Lo que
  encuentre fuera de ese presupuesto sigue siendo desconocido.
- El plegado de colisiones aplica normalización NFC real y `to_lowercase`
  Unicode por segmento, no una tabla ASCII/Latin-1: pliega marcas combinantes
  fuera de ese rango, singletons y el plegado de griego y cirílico. Lo que **no**
  hace es plegar homoglifos, que son deliberadamente rutas distintas. Registrado
  en `docs/security/parser-threats.md`. La descripción anterior de este archivo
  describía la tabla que se sustituyó en el sprint 4A.
- Ninguna plataforma se ha probado en **hardware físico**. Emulador, simulador y
  dos hosts no son un teléfono. Android arm64 se compila y no se ejecuta.
- La zeroización **no se ha observado**: se comprueba el tipo, no la memoria.
  Leer memoria liberada es comportamiento indefinido, así que una prueba que
  afirmara verlo estaría mintiendo.
- No hay SBOM ni `cargo-deny`.
- Autoría y licencia del logo siguen sin registrar.
- **Corregido en la fase 10:** esta línea decía «no existe ninguna función de
  transferencia: el producto no es usable todavía». Existe y en código está
  completo. Lo que no existe es una sola persona que lo haya usado.

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

## Sprint 4D.2a en curso — Android está decidido y no está implementado

**No hay persistencia en Android.** Lo que hay es la decisión congelada y un
hallazgo que cambia la forma del sprint. Esta sección se escribe con el sprint a
medias, en vez de esperar al final, porque la alternativa es lo que QYR-0060
registró en 4D.1: una cabecera que dice una cosa y un cuerpo que dice otra.

Lo que existe:

- **ADR-0025 congelada antes de una sola línea de código de Android.** Envolver,
  no guardar: Keystore genera una clave AES-256-GCM no exportable y con ella se
  envuelve la semilla Ed25519, porque «key material never enters the application
  process» y Keystore no puede guardar una semilla ajena. Con eso Keystore ocupa
  exactamente el sitio de DPAPI y **`IdentityStore` y `SecretWrapper` no
  cambian**, que es la comprobación de que la costura estaba bien puesta.
- Las cuatro sub-decisiones con fuente citada y fechada: TEE sin StrongBox; sin
  autenticación de usuario; el blob en `getNoBackupFilesDir()`; y backup/restore
  derivado sólo hasta donde la no exportabilidad obliga, con lo demás abierto.
- El IV de GCM: lo genera Keystore por operación, viaja dentro de `wrapped` como
  `iv_len ‖ iv ‖ ciphertext`, y **no se deriva de nada**. Una derivación sobre un
  valor que se repite —reinstalar, restaurar, rotar dos veces— repite el IV, y un
  IV repetido bajo la misma clave no es una degradación: es la propiedad rota.
- **El byte `wrap` `0x02`** registrado como cambio de formato, y con él una
  comprobación que faltaba: `open_identity` nunca comparaba el `wrap` del blob
  con el del envoltorio. Con un solo envoltorio la pregunta no podía surgir; con
  dos, un blob de Windows entregado al envoltorio de Android habría fallado como
  archivo corrupto. Ahora es `WrapMismatch`, que nombra los dos lados.

Lo que **no** existe, y no debe leerse como progreso:

- **No hay crate de Android, no hay JNI y no hay nada que persista en Android.**
  Cerrar el proceso en Android sigue perdiendo la identidad.
- No hay barrido de corrupción contra Keystore, ni vector para `wrap = 0x02`, ni
  paso de CI que ejecute persistencia en el emulador.
- **QYR-0064 cambia la forma del sprint.** El harness de 4D.1 —un binario nativo
  empujado a `/data/local/tmp` y lanzado con `adb shell`— **no puede alcanzar
  Keystore**: no hay API en el NDK, `AndroidKeyStore` es código Java que corre en
  el proceso de la app, y las claves se separan por UID del llamante. Hace falta
  un test instrumentado bajo `am instrument`, con el andamiaje Gradle que eso
  implica. El prompt del sprint daba por buena la otra forma.
- **El sprint 4D.1 no añadió ninguna dependencia externa; 4D.2a sí lo hará.**
  ADR-0025 §1.4 decide `jni-sys` —dos entradas nuevas en el grafo, medidas en
  este árbol, frente a las once de `jni`— y argumenta por qué aquí la respuesta
  es la contraria a la de ADR-0024 §1: JNI no se alcanza por símbolos con nombre
  sino por una tabla de unos 233 punteros cuyo orden es la ABI. **Todavía no está
  añadida**: hoy el grafo sigue en 59 paquetes.
- QYR-0065 y QYR-0066 abiertos: falta fuente verbatim sobre invalidación de
  claves, y no está medido qué error da Keystore cuando el alias ya no existe.

## Sprint 5A — el motor existe y no mueve archivos

**`qyro_transfer` mueve una transferencia completa entre dos extremos del mismo
proceso.** Un emisor y un receptor, cada uno con su estado, intercambiando
**solo `Vec<u8>` de frames sellados**: ninguno toca al otro. Varios archivos,
varios chunks, verificación de SHA-256 contra el manifest y un veredicto por
elemento.

El sellado es **real**: `FrameSealer` y `FrameOpener` derivados de un handshake
de cuatro mensajes real. **No hay ningún doble criptográfico** en las pruebas del
motor; el sprint existía para comprobar que las piezas encajan, y un doble
probaría que encajan con otra cosa.

Lo que el motor hace, con prueba por cada cosa:

- Camino feliz completo sobre frames sellados, y los bytes que llegan son los que
  salieron —comprobado byte a byte en varias posiciones, no sólo por el veredicto.
- Un bit volteado en un chunk es `NotAuthenticated` y **envenena la sesión**;
  nada de ese frame llega al destino.
- El receptor calcula el digest él mismo y lo compara con el manifest. Un archivo
  cuyo contenido no case se rechaza al cerrar, con el otro archivo en `Ok` para
  que la prueba no pase por fallar todo.
- Control de flujo **medido**: el emisor produce exactamente una ventana y se
  para; con un ACK vuelve a producir. Que se parara sin volver a arrancar también
  sería cierto de un motor roto.
- Un chunk perdido se retransmite y la transferencia termina bien, con
  **go-back-N**: ACK acumulativo y sin buffer fuera de orden implican que
  reenviar sólo el que faltaba deja el resto sin llegar.
- Pausa, reanudación y cancelación **desde los dos lados**, dejando a los dos
  extremos de acuerdo.
- Cuatro transiciones ilegales rechazadas por tipo, más las cuatro negativas de
  ADR-0026 §4 que el barrido de mutación encontró **sin cubrir**.
- Un chunk repetido lo rechaza la ventana de replay que ya existía: la prueba
  comprueba que el motor pasa por ella y no por su lado.

**Memoria acotada, medida y no supuesta:** en una transferencia de 8 MiB el
emisor sostuvo **65 536 bytes** —un búfer de chunk— y el receptor entregó como
mucho 65 536 bytes de una vez. Contador instrumentado bajo `cfg(test)`, como el
`bytes_moved` del decoder; un cronómetro en un runner compartido mide el runner.
La fuente de contenido **genera** los bytes desde una semilla en vez de
guardarlos, así que lo que se mide es el motor y no el fixture.

Lo que **no** existe, y no debe leerse como progreso:

- **No hay red y no hay sockets.** El «transporte» es un `Vec<u8>` que una prueba
  pasa de un lado al otro.
- **No había filesystem en 5A**: la fuente y el destino eran memoria. El sprint
  5B.1 puso disco detrás de esas dos costuras **sin cambiarlas**; ver la sección
  de 5B.1 más arriba.
- **Nada del producto llama al motor.** Los botones Enviar y Recibir siguen
  deshabilitados y el README sigue diciendo que Qyro no transfiere archivos.
  **La razón cambió en la fase 01 y es más débil que antes:** `qyro_ffi` ya
  depende de `qyro_transfer` y de `qyro_crypto`, a través de `qyro_session`, y la
  prueba de cierre transitivo que lo impedía ya no existe con esa forma. Lo que
  sostiene la frase hoy es que `qyro_ffi` sigue exponiendo dos funciones y
  ninguna de ellas abre una sesión: Dart no puede pedir nada porque no hay qué
  pedir, no porque el camino no exista. Ver ADR-0032 §9.
- **No hay reanudación entre sesiones.** La pausa de 5A es dentro de una sesión
  viva; sobrevivir al cierre del proceso necesita disco.
- **El tamaño de chunk y la ventana no están medidos.** Son cotas argumentadas
  —64 KiB y 16, es decir 1 MiB en vuelo por dirección—, elegidas desde el límite
  de memoria. Sin transporte no hay contra qué medir un óptimo, y ADR-0026 §2 lo
  dice de sí misma.

**Dos desajustes de contrato**, que son el valor principal del sprint aparte del
motor. Los dos **registrados y no arreglados**:

- **QYR-0068**: la cabecera de 48 bytes reserva `transfer_id`, `stream_id` e
  `item_id` **dentro de los datos asociados autenticados**, y `Frame::new` los
  fija en cero sin que exista forma pública de cambiarlos. Hoy son tres campos
  autenticados que no dicen nada. Se descubrió al implementar ADR-0026 §1, que
  había decidido repetir `item_id` en el cuerpo sin saber que la cabecera ya lo
  llevaba. **No se añadieron setters**: ensanchar una superficie congelada como
  efecto secundario de otro sprint es cómo se pierde el control de un formato.
- **QYR-0069**: los constructores deterministas del handshake son `pub(crate)`,
  así que un crate dependiente no puede reproducir una sesión byte a byte.
  Probablemente correcto —un constructor determinista público acaba usándose en
  producción— y no cuesta nada aquí. Costará cuando haga falta un vector
  interoperable de una transferencia completa.

## Sprint 5B.1 — el disco de verdad, sin selector

**`qyro_fs` lee y escribe archivos reales detrás de las dos costuras que ya
existían.** `ContentSource` y `ContentSink` **no cambiaron**, que era la
comprobación de que ADR-0026 las puso en el sitio correcto: una costura que hay
que ensanchar para su segunda implementación era la costura equivocada.

Lo que existe, con prueba por cada cosa:

- Un archivo de **5 MiB + 777 bytes** y otro anidado viajan entre dos directorios
  y llegan **byte a byte idénticos** — comparados byte a byte, no por veredicto —
  y no queda ningún `.qyro-part` detrás.
- El manifest se construye **desde el disco** por streaming: **65 536 bytes** de
  lectura máxima sobre un archivo de 8 MiB, con contador instrumentado bajo
  `cfg(test)`. Y el digest resultante se compara contra un cálculo independiente,
  así que la cota no es pequeña porque no pasara nada.
- Un digest que no coincide **no produce el archivo final ni deja el
  `.qyro-part`**: nada verificable sobrevive a un desajuste.
- Un **symlink real** en el destino no redirige una escritura fuera de la raíz, y
  la prueba comprueba primero que el symlink existe: una prueba de symlinks que
  no crea uno no prueba nada de symlinks.
- Una ruta que se escapa de la raíz se rechaza **al materializar**, aunque el
  manifest la hubiera aceptado, y una ruta legítima sigue resolviendo.
- Una colisión en el destino se **rechaza**, y la prueba comprueba que el archivo
  del receptor sigue intacto.
- Una transferencia interrumpida **se reanuda desde sus metadatos**, con el sink
  soltado en medio para simular el proceso muerto.
- Metadatos de una versión futura se rechazan **nombrando la versión**, y las
  otras tres negativas del orden de lectura por su propio camino.

Lo que **no** existe, y no debe leerse como progreso:

- **No hay selector de archivos.** La lista de archivos se la pasa el llamante;
  SAF de Android y el picker de Windows cruzan el FFI y son 5B.2.
- **No hay red y no hay sockets.** El transporte sigue siendo un `Vec<u8>`.
- **Nada del producto llama a esto.** `qyro_ffi` no alcanza `qyro_crypto`,
  `qyro_transfer` ni `qyro_fs`. Los botones siguen deshabilitados.
- **Las garantías de `fsync` no se han comprobado cortando la corriente.** Lo que
  CI ejerce es la caída del proceso, que es un fallo distinto con garantías
  distintas, y ADR-0027 §4 separa los dos en vez de mezclarlos.
- **La carrera de los componentes intermedios sigue abierta** (QYR-0072).
  `O_NOFOLLOW` cierra por completo el último componente —comprobar y abrir son la
  misma llamada— y no los de en medio.

**Dos hallazgos, y el segundo es el que importa:**

- **QYR-0070**: `SizeMismatch` e `Incomplete` no los producía ninguna prueba.
  Escribirlas destapó que la **infra-entrega nunca llega a la fase de veredicto**
  —el control de `Complete` la rechaza antes—, así que el único camino a
  `SizeMismatch` es la **sobre-entrega**. E `Incomplete` es **inalcanzable y se
  puede demostrar**; queda exento por nombre con el argumento escrito, no
  borrado, porque su byte está congelado en ADR-0026 §1.
- **QYR-0071, P1**: el análisis compartido de guardas **leía 13 401 bytes de un
  archivo de 30 861**. `item_end` no sabía terminar un item en la coma de un
  campo, y `#[cfg(test)] peak_content_held: usize,` lo hizo comerse el resto del
  archivo. Desde el sprint 5A, `no_production_path_can_panic` sobre `session.rs`
  cubría el 43 % mientras decía cubrirlo entero. Corregido dos veces: la coma, y
  una comprobación que compara la última línea del archivo con lo que sobrevivió
  al análisis — que es la que atrapa **cualquier** forma futura, no sólo ésta.

Llevar la guarda de sitios de construcción al análisis compartido también destapó
dos variantes de 5A que nadie construía, `TransferError::UnsupportedMessage` y
`WindowExhausted`; las dos **borradas**, y la segunda tenía además un comentario
que afirmaba que se reportaba, y era falso.

## Runs de 5B.1

Todos los `push` de la rama, sin filtrar. **Ninguno falló y ninguno se canceló.**

| Workflow | Commit | Run | Conclusión |
|---|---|---|---|
| **CI #137** | **`e3fbaf1`** | **31232028441** | **success**, 4/4 jobs |
| **Platform builds #39** | **`e3fbaf1`** | **31232028378** | **success** |
| **Crypto platform #27** | **`e3fbaf1`** | **31232028429** | **success**, 4/4 jobs |
| **Crypto fuzz #14** | **`e3fbaf1`** | **31232028435** | **success**, 6 targets |
| **Android runtime ABI #62** | **`e3fbaf1`** | **31232028405** | **success**, emulador |
| **iOS runtime ABI #33** | **`e3fbaf1`** | **31232028433** | **success**, XCTest en simulador |

**Los seis sobre `e3fbaf1`, por `push`, y los seis en success al primer intento.**
Corrieron los seis porque ese push toca `rust/guards/**` —que vigilan `Crypto
fuzz` y `Crypto platform`—, `rust/**` para `Platform builds`, y `Cargo.toml` y
`Cargo.lock` para las dos ABI nativas. `ci.yml` no filtra.

A diferencia del sprint 5A, **el emulador de Android no necesitó un segundo
intento**: el paso arrancó a la primera. Se dice porque allí se registró la
cancelación, y una racha sin incidentes sólo significa algo si la anterior con
incidente también está escrita.

## Runs de 5A

Todos los `push` de la rama, sin filtrar.

| Workflow | Commit | Run | Conclusión |
|---|---|---|---|
| **CI #135** | **`94fe996`** | **31228301326** | **success**, 4/4 jobs |
| **Platform builds #38** | **`94fe996`** | **31228301291** | **success** |
| **Crypto platform #26** | **`94fe996`** | **31228301287** | **success**, 4/4 jobs |
| **Crypto fuzz #13** | **`94fe996`** | **31228301314** | **success**, 6 targets |
| **iOS runtime ABI #32** | **`94fe996`** | **31228301343** | **success**, XCTest en simulador |
| **Android runtime ABI #61** | **`94fe996`** | **31228301331** | **cancelled** en el intento 1, **success** en el 2 |

**Los seis sobre `94fe996`, por `push`, y los seis en success.** Corrieron los
seis porque ese push toca `rust/crates/qyro_protocol/**` —que vigilan `Crypto
fuzz` y `Crypto platform`—, `rust/**` para `Platform builds`, y `Cargo.toml` y
`Cargo.lock` para las dos ABI nativas. `ci.yml` no filtra.

### El primer intento de Android se canceló, y por qué

El **intento 1** de `Android runtime ABI #61` (job 93026874026) se quedó
**veintinueve minutos** en el paso «Execute native ABI smoke test in an Android
emulator» y lo canceló el `timeout-minutes` del job. No es un fallo del código y
tampoco es un éxito: **un run cancelado no es evidencia**, y se registra aquí en
vez de dejar sólo el intento que salió bien.

El **intento 2** (job 93031793097, mismo run y mismo commit, `run_attempt: 2`)
completó ese paso en **cinco minutos y medio** y salió en verde. La diferencia
entre veintinueve minutos y cinco y medio, con el mismo árbol, apunta al arranque
del emulador en el runner y no al cambio: este sprint no toca `apps/qyro/**` ni
`qyro_ffi`, y lo único que lo hizo recompilar fue `Cargo.lock`.

Se re-ejecutó el run, no se lanzó uno nuevo, para que la evidencia siguiera
siendo del evento `push` sobre `94fe996`.

## Runs de 4D.2a

Todos los `push` de la rama, sin filtrar. **Ninguno falló y ninguno se canceló**,
que es una frase que sólo vale escribir cuando la lista es exhaustiva.

| Workflow | Commit | Run | Conclusión |
|---|---|---|---|
| CI #132 | `5a2a576` | 31220388132 | **success** |
| CI #133 | `554f16d` | 31220738271 | **success** |
| Platform builds #37 | `554f16d` | 31220738191 | **success** |
| Crypto platform #25 | `554f16d` | 31220738176 | **success** |

Tres de los seis, y no es una omisión: `Crypto fuzz`, `Android runtime ABI` e
`iOS runtime ABI` filtran por rutas que estos commits no tocan. `Crypto platform`
y `Platform builds` sí corrieron porque `bdb2bf8` toca
`rust/crates/qyro_identity_store/**`.

**Estos tres no son la evidencia de cierre de 4D.2a**, porque no hay nada que
cerrar todavía: no existe el crate de Android. Los seis sobre un mismo commit
son el requisito del cierre y llegan cuando llegue la persistencia.

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
| **CI #130** | **`d20afd7`** | **31217230007** | **success** |
| **Platform builds #36** | **`d20afd7`** | **31217226979** | **success** |
| **Crypto platform #24** | **`d20afd7`** | **31217226445** | **success** |
| **Crypto fuzz #12** | **`d20afd7`** | **31217226480** | **success** |
| **Android runtime ABI #60** | **`d20afd7`** | **31217227701** | **success** |
| **iOS runtime ABI #31** | **`d20afd7`** | **31217226706** | **success** |

### Cierre del sprint: los seis sobre `d20afd7`

**Los seis workflows en success sobre un mismo commit, los seis por `push`.** Es
la evidencia de cierre de 4D.1, y el ancla apunta ahí.

| Workflow | Run | Evento | Conclusión |
|---|---|---|---|
| CI | 31217230007 | `push` | **success**, 4/4 jobs |
| Platform builds | 31217226979 | `push` | **success** |
| Crypto platform | 31217226445 | `push` | **success** — incluye la persistencia en dos procesos |
| Crypto fuzz | 31217226480 | `push` | **success** |
| Android runtime ABI | 31217227701 | `push` | **success**, emulador |
| iOS runtime ABI | 31217226706 | `push` | **success**, XCTest en simulador |

Corrieron los seis porque `d20afd7` toca `qyro_crypto`, `qyro_identity_store` y
`qyro_core`, que entre los tres cubren los filtros de rutas de los cinco
workflows filtrados; `ci.yml` no filtra. **Ese reparto es deliberado y su commit
lo dice**: sin él, un commit documental dispara solo CI, que es correcto y no
sirve como evidencia de los seis. Lo que el commit lleva son tres correcciones
reales de comentarios que este sprint volvió falsos, no un toque para activar un
filtro.

La persistencia se ejecutó **dos veces** sobre commits distintos: `b731276` (run
31215102331) y `d20afd7` (run 31217226445). La segunda no reemplaza a la primera
ni la primera a la segunda; están las dos porque las dos ocurrieron.

**Los seis sobre `3b2cf61`, por `push`, y los seis en success.** Es el primer
commit de este sprint con evidencia de los seis, y por eso el ancla apuntó ahí
durante ese tramo. **No es la evidencia de cierre**: en `3b2cf61` no existía
todavía el crate de plataforma, así que aquellos seis dicen que el repositorio
seguía en pie, no que algo persistiera. Los seis de cierre son los de `d20afd7`,
más abajo. Corrieron los seis porque ese commit tocó `rust/crates/**`,
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

**Ejecutar la fase 07: el protocolo de hardware físico.** Todo lo que esta
sesión podía construir está construido y etiquetado como `v1.0.0`. Lo único que
queda no es código: son dos aparatos, una Wi-Fi y una persona ejecutando los
veinte escenarios de `docs/testing/hardware-protocol.md` y anotando lo que pasó,
incluidos los que fallen.

**Lo que la fase 07 encuentre es lo que decide la v1.1**, y no al revés. Escribir
hoy una lista de mejoras sería adivinar antes de la única medición que falta.

## Provisional values

Los siguientes valores son provisionales y deben bloquear el empaquetado público:

- Marcadores `REPLACE_WITH_*` en los ejemplos de branding.
- Base de identificador `com.owner.qyro`.
- Clearance del nombre de producto Qyro.
- Elección de licencia Apache-2.0.
- Autoría/licencia del logo suministrado (`design/brand/source/logo.png`).
