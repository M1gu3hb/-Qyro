# Changelog

Basado en Keep a Changelog y Semantic Versioning.

## [Unreleased]

### Security (sprint 4C.3)

- El decoder de `qyro_protocol` ya no es cuadrático. `next_frame` reclamaba cada
  frame entregado con `drain(..total)`, que memmovea todo lo que queda detrás,
  así que un peer enviando heartbeats bien formados —tráfico válido, ningún
  error, nada que un limitador basado en validez vea— hacía trabajar al bucle de
  recepción con el cuadrado de lo que enviaba. Medido: 21 868 frames,
  1 049 664 bytes empujados, **11 476 501 344 bytes movidos**. Ahora 0 en esa
  forma y 2 359 296 sobre 2 596 608 en el bucle con backlog, que es donde la
  compactación corre de verdad (QYR-0024, ADR-0016 enmendado).
- `buffer_capacity()` ya no supera `MAX_BUFFER_LEN`. Goteando un byte por push
  llegaba a 2 097 152 frente a 1 049 664, y dos pruebas del repositorio ya
  afirmaban lo contrario porque nunca llenaban el búfer (QYR-0027).
- Ninguna ruta de producción de `qyro_protocol` ni de `qyro_manifest` puede
  terminar el proceso. Son la primera superficie que toca los bytes de un peer y
  no tenían ninguna de las denegaciones que `qyro_crypto` lleva desde 4C.2: 33 y
  22 infracciones respectivamente (QYR-0036).

### Added (sprint 4C.3)

- `rust/guards/source_guard.rs`: el análisis de la guarda anti-pánico,
  compartido por los tres crates con `include!`. Un archivo, porque un análisis
  duplicado son tres análisis que pueden discrepar sobre qué es código de
  producción — que es justo la deriva que la guarda existe para detectar.
- Pruebas de coste del decoder que cuentan bytes movidos en vez de cronometrar:
  un reloj de pared en un runner compartido mide el runner.
- Cinco formas adversariales fijadas en el decoder: búfer lleno de frames
  mínimos, el mismo goteando un byte por push, un frame máximo goteando un byte
  por push, un frame mayor que un techo personalizado (rechazado y envenenado,
  nunca esperado), y frames válidos alternando con basura.
- Dos reglas nuevas en `check_docs_consistency` (Bash y PowerShell): un nombre
  de rama literal en cualquier `branches:` es un BLOCKER (QYR-0040), y un
  `QYR-00xx` citado sin entrada en `BUGS_PENDING.md` también (QYR-0043).

### Changed (sprint 4C.3)

- Los seis workflows disparan sobre `[main, 'claude/**']`. El sprint 4C.2 había
  escrito el nombre de la rama de entonces en los seis, lo que hacía de «CI
  corre sobre la rama de trabajo» una propiedad de esa rama y no del
  repositorio (QYR-0040).
- Las exenciones de la guarda se derivan de las declaraciones `mod` en vez de
  una lista escrita a mano, así que quitar un `#[cfg(test)]` mueve el archivo al
  conjunto de producción en vez de eximirlo (QYR-0042).
- El consejo de regenerar un vector es condicional: si este build ya no calcula
  el transcript que ADR-0021 especifica, dice «Do not regenerate», porque el
  archivo comprometido es entonces lo único que sostiene la especificación
  (QYR-0044).
- `ParsedHeader::parse` toma un `&[u8; HEADER_LEN]` en vez de comparar una
  longitud y confiar en ella cuarenta veces.

### Removed (sprint 4C.3)

- El `debug_assert_eq!` al final de `codec::encode`. Duplicaba un invariante que
  `encoded_len_matches_the_bytes_actually_produced` ya fija en todos los
  perfiles, y lo hacía en una forma ausente de un build de release y letal en
  uno de debug. Ningún lint de Clippy cubre la familia `debug_assert`; lo
  encontró la guarda estructural.

### Fixed (sprint 4C.3)

- La cita de Unicode 16.0.0 llevaba la fecha de generación del archivo de datos
  en lugar de la de publicación de la versión, equivocada por cuatro meses
  (QYR-0041).
- `MAX_HASH_LEN` y `FrameError::FrameTooLarge` se presentaban como cotas vivas.
  El primero lo es —por el constructor, nunca por el cable— y ahora tiene
  prueba; la comprobación del segundo no puede dispararse, lo dice donde está, y
  una aserción `const` fija la aritmética que lo garantiza (QYR-0038).
- Diez identificadores `QYR-00xx` sin entrada en el registro, incluidos dos que
  nunca habían entrado en este repositorio y uno cuyo contenido sigue sin
  conocerse y se registra como tal (QYR-0043).

### Security (sprint 4C.2)

- `qyro_manifest` rechaza toda la categoría general Unicode `Cf` en una ruta,
  con `PathError::FormatCharacter`. `RelativePath::parse("invoice\u{202E}fdp.exe")`
  devolvía `Ok`, y todo renderizador consciente de bidi muestra ese nombre como
  `invoiceexe.pdf`: un receptor confirmaba un documento y recibía un ejecutable.
  El filtro anterior era `char::is_control()`, que es la categoría `Cc` y nada
  más. La tabla son veintiún rangos transcritos de
  `DerivedGeneralCategory.txt` de Unicode 16.0.0, 170 puntos de código, citados
  en el fuente y comprobados contra el archivo; no se añade ninguna dependencia
  a la ruta que analiza bytes de un peer (QYR-0021, ADR-0019 enmendado).
- `TransferManifest::new` y el decoder rechazan un elemento que es un archivo y
  además el directorio padre de otro. Las claves de colisión se comparaban por
  igualdad, y `"a"` y `"a\0b"` son dos cadenas distintas, así que un receptor
  habría tenido que crear `a` como archivo y como directorio (QYR-0028,
  ADR-0017 enmendado).
- Ninguna ruta de producción de `qyro_crypto` puede terminar el proceso.
  `handshake/transcript.rs` tenía un `expect` y `handshake/schedule.rs` un
  `unreachable!`, ambos alcanzables desde bytes elegidos por un peer; con ellos
  se fueron catorce indexaciones sin comprobar (QYR-0033).
- `COM¹`, `COM²`, `COM³`, `LPT¹`, `LPT²` y `LPT³` añadidos a los nombres de
  dispositivo reservados de Windows, con la fuente citada (QYR-0029, parcial).

### Added (sprint 4C.2)

- `rust/crates/qyro_crypto/src/guards.rs`: guarda estructural sobre los doce
  archivos de producción del crate. Detecta lo que un `#![deny(...)]` no puede —
  un módulo al que nadie le puso el atributo, y `assert!`, que no tiene lint— y
  comprueba además que cada variante de `HandshakeError` tiene un sitio de
  construcción.
- Pruebas que fallan al borrar el control que cubren: la autenticación del
  iniciador (QYR-0022), `verify_strict` frente a `verify` con una firma de `R`
  de orden pequeño (QYR-0023), el transcript calculado desde SHA-256 sobre
  concatenación literal (QYR-0025) y los cuatro controles de la ruta de decode
  del manifest (QYR-0032).
- `#![deny(...)]` de Clippy en `handshake/`, `identity.rs`, `signature.rs` y
  `fingerprint.rs`, con la familia de pánico y `indexing_slicing`.
- Los seis workflows se disparan por `push` sobre la rama de trabajo. Cuatro
  listaban solo `main` y dos una rama muerta desde hacía cuatro sprints
  (QYR-0026).

### Changed (sprint 4C.2)

- `c_abi_contract.rs` pide el cierre transitivo a `cargo metadata` en vez de
  partir `Cargo.toml` por la cadena `"[dependencies]"`. Una tabla
  `[target.'cfg(target_os = "android")'.dependencies]` es otra sección y no se
  miraba nunca (QYR-0030).
- `handshake/vectors.rs` recalcula ambos transcripts y los MAC desde las
  primitivas —SHA-256 y HMAC escrito desde RFC 2104— en vez de llamar a las
  funciones que produjeron el archivo, y fija `Schedule::derive` contra los
  valores ya verificados (QYR-0025).
- El campo `RelativePath::normalized` pasa a llamarse `verbatim`, que es lo que
  siempre fue (QYR-0031).
- Los dos scripts de guarda de la frontera FFI descartan las líneas de
  comentario antes de buscar, en Bash y en PowerShell.

### Removed (sprint 4C.2)

- `HandshakeError::UnexpectedRole`, `InvalidEphemeralPublicKey`,
  `TranscriptMismatch` y `SequenceViolation`. Nada las construía, así que
  ADR-0021 y `handshake-state-machine.md` describían cuatro controles que no
  existían (QYR-0035, ADR-0021 enmendado).
- La comprobación redundante de `U+007F` en `validate_segment`: es `Cc`, así que
  `is_control()` ya la había rechazado.

### Fixed (sprint 4C.2)

- Seis sitios de documentación que contradecían al código: rutas descritas como
  normalizadas, bytes de cabecera desconocidos descritos como saltados cuando se
  rechazan, trailer descrito como cero cuando un frame sellado exige `1..=64`,
  `cfg(test)` donde el atributo es `cfg(any(test, fuzzing))`, y tres filas de
  THREAT_MODEL.md. Todos marcados como corregidos, no reescritos en silencio
  (QYR-0031).

### Added (sprint 4C.1)

- `.github/workflows/crypto-platform.yml`: compila `qyro_crypto` por target
  explícito para Android x86_64 y arm64, iOS device y simulator, Windows x64 y
  Linux, y **lo ejecuta** en cuatro de esos seis mediante un harness aislado.
- `rust/tools/qyro_crypto_smoke`: el harness. `publish = false`, ningún crate del
  producto depende de él, y dos guardas lo mantienen fuera de los bundles
  (ADR-0023).
- `.github/workflows/crypto-fuzz.yml`: seis targets, un job cada uno, sin
  `fail-fast`, con las estadísticas finales de libFuzzer en el log. Campaña
  acotada, no exhaustiva.
- Tres targets nuevos —`encrypted_envelope`, `frame_opener` y `replay_window`—
  y `qyro_crypto::fuzzing`, disponible solo bajo `--cfg fuzzing` y no como
  feature de Cargo, porque las features son aditivas y cualquier crate del grafo
  podría encenderla para todos.
- `scripts/check_crypto_platform_evidence.{sh,ps1}` y
  `scripts/check_harness_isolation.{sh,ps1}`.
- `docs/security/secret-lifecycle-audit.md`,
  `docs/testing/crypto-platform-matrix.md`, `docs/testing/crypto-fuzzing.md`,
  ADR-0023 y `docs/audits/SPRINT4C1_CRYPTO_PLATFORM_AUDIT.md`.
- `.gitattributes`: el repositorio se extraía con CRLF en Windows y tres pruebas
  de comparación byte a byte fallaban allí y solo allí.

### Changed (sprint 4C.1)

- `AuthenticatedFrame::payload` pasa a `Zeroizing<Vec<u8>>`, y los búferes
  temporales de `seal` y `open` con él.
- El estado del sealer pasa de `Option<u64>` a un enum de tres variantes:
  cualquier `Err` lo envenena de forma permanente, para que un reintento no
  reutilice una secuencia que ya se consumió.
- Features `zeroize` activadas en `sha2` y `hmac`: estaban apagadas, así que el
  estado de compresión de cada transcript y el estado con clave de cada MAC
  quedaban en memoria liberada.
- `rust/fuzz` declara su propio `[workspace]`. Sin él Cargo no podía construir
  ningún target, y ninguno se había construido nunca.

### Removed (sprint 4C.1)

- `AuthenticatedFrame::into_payload`, que entregaba el texto claro descifrado
  como un `Vec<u8>` que nadie borra. Lo sustituye `into_zeroizing_payload`.
- `panic!`, `unreachable!` y `assert!` de la ruta AEAD de producción, con un
  `deny` de Clippy que lo mantiene así. Un `assert!` no era un control de
  seguridad: `debug_assertions` está apagado en release.

### Added (sprint 4C)

- Cifrado autenticado de frames QYRO/1 con ChaCha20-Poly1305 (ADR-0022):
  `FrameSealer`, `FrameOpener`, `SealedFrame` y `AuthenticatedFrame`, los dos
  últimos con constructor privado.
- Claves y prefijos de nonce derivados por dirección con HKDF-SHA256 sobre los
  secretos de tráfico del handshake, con la dirección dentro de la etiqueta y el
  transcript y el `SessionId` dentro de cada `info`.
- Nonce monotónico `prefijo || secuencia`, asignado por el sealer, sin
  envolvimiento: agotar la secuencia es un error terminal.
- Ventana de replay fija de 1024, consultada antes del AEAD y actualizada solo
  después de que el tag verifique.
- `EstablishedInitiator::into_frame_crypto` y su equivalente en el respondedor,
  que consumen la sesión establecida.
- KAT de ChaCha20-Poly1305 (RFC 8439 §2.8.2 y apéndice A.5) y vectores propios
  del sellado, encadenados a los del handshake.
- Trece semillas selladas en el corpus de fuzzing y un smoke del opener.
- `docs/security/frame-encryption.md`, `docs/security/nonce-lifecycle.md`,
  `docs/security/replay-window.md` y
  `docs/audits/SPRINT4C_AEAD_AUDIT.md`.

### Fixed (sprint 4C)

- Cuatro afirmaciones de ADR-0022 sobre lo que la derivación liga no las cubría
  ninguna prueba: quitar la dirección de la etiqueta, o el transcript, o el
  `SessionId`, no rompía nada, porque los secretos de tráfico ya difieren una
  capa más arriba.
- La documentación del código dejaba de ser cierta al implementar el AEAD:
  `lib.rs` decía «There is still no AEAD», el módulo del handshake decía que sus
  claves no cifran nada, y `envelope.rs` describía `SealedFrame` y
  `AuthenticatedFrame` como tipos futuros.

### Removed (sprint 4C)

- `AeadError::NotEncrypted`, `AeadError::PayloadTooLarge` y
  `AeadError::InvalidNonceState`: ADR-0022 los congeló antes del código y
  ninguno resultó alcanzable. Registrado como enmienda en la propia ADR.

### Added (sprint 4B.1)

- Handshake autenticado de cuatro mensajes cerrado y documentado: X25519,
  Ed25519 en el dominio `HandshakeTranscript`, HKDF-SHA256 y HMAC-SHA256, con
  estados consumidos (ADR-0021).
- `qyro_protocol::SessionId`: ocho bytes, un solo tipo, compartido por la
  cabecera QYRO/1 y por el schedule del handshake.
- `ResponderFinishPending`: el respondedor no obtiene una sesión hasta confirmar
  que entregó su `ResponderFinish`.
- Vectores interoperables del handshake con schema estricto, regeneración byte a
  byte y verificación independiente contra las primitivas.
- KAT de X25519 (RFC 7748 §5 y §6.1).
- `docs/security/authenticated-handshake.md`,
  `docs/security/handshake-state-machine.md`,
  `docs/security/handshake-threat-analysis.md` y
  `docs/audits/SPRINT4B_HANDSHAKE_AUDIT.md`.

### Fixed (sprint 4B.1)

- El identificador de sesión ya no existe en dos anchos incompatibles. La
  etiqueta `session-id` deriva ocho bytes; antes producía 32 mientras la
  cabecera reservaba ocho, sin conversión, de modo que quien conectara el
  transporte habría tenido que inventar un truncamiento.
- Las claves de sesión salen de la API pública: `SessionKey` deja de exportarse
  y desaparecen `sending_key()` y `receiving_key()`.
- La entropía efímera ya no puede sustituirse. El adaptador RNG rellenaba con
  ceros y devolvía éxito ante una lectura de más, y un secreto X25519 de ceros
  se clampea a un escalar válido y completa un handshake sin entropía dentro. No
  se pudo reparar el adaptador —`random_from_rng` exige un `CryptoRng`
  infalible— así que el secreto se construye directamente desde bytes obtenidos
  de forma falible.
- `check_docs_consistency` detecta la deriva documental que se acumuló durante
  tres sprints: capacidades negadas en prosa que existen en código, hitos
  pedidos después de entregarse, vectores declarados sin archivo, plegado
  descrito como ASCII/Latin-1 y plataformas marcadas como ejecutadas sin run id.
- `SECURITY.md` decía que el workspace no tiene dependencias externas y que no
  hay KAT de criptografía. Ambas cosas eran falsas desde el sprint 4A.

### Fixed (sprint 3)

- El decoder ya no envenena el stream ante un tipo de mensaje desconocido: lo
  consume delimitado y lo reporta, así que un peer con una versión menor más
  nueva recibe respuesta en vez de perder la conexión (ADR-0018).
- QYRO/1.0 rechaza extensiones de cabecera que no puede preservar, en lugar de
  saltarlas y romper la reserialización byte-exacta.
- `ENCRYPTED` y `COMPRESSED` salen de la API pública. Un frame no puede declarar
  protección que no tiene; `SealedFrame` no existe sin tag.
- El manifest ya no lleva `display_name`: `factura.pdf.exe` no puede presentarse
  como `factura.pdf` (ADR-0019, `MANIFEST_VERSION` 2).
- Todo archivo exige digest final, incluidos los de cero bytes.
- Se rechazan caracteres ilegales en Windows y colisiones por mayúsculas o por
  composición Unicode.
- `codec::encoded_len` valida el tamaño antes de reservar.
- Corregida una aserción de travesía por subcadena heredada del sprint 2.

### Added (sprint 2)

- `qyro_protocol`: framing binario QYRO/1 con cabecera fija de 48 bytes
  big-endian, 17 tipos de mensaje congelados, flags, errores tipados y decoder
  incremental. Las longitudes declaradas se validan contra constantes de
  compilación antes de reservar memoria (ADR-0016).
- `qyro_manifest`: manifest canónico y `RelativePath`, que rechaza travesía,
  rutas absolutas, prefijos de unidad, UNC, barras invertidas, NUL, nombres
  reservados de Windows y punto o espacio final, con reglas Unix y Windows
  aplicadas en todas las plataformas (ADR-0017).
- Property tests con generador sembrado (~30 000 casos por ejecución), targets
  `cargo-fuzz` y un corpus de 65 entradas reproducido como smoke en CI.
- `cargo audit` obligatorio en CI. Pasa con 0 vulnerabilidades: el workspace no
  tiene dependencias externas.
- Intro: QYRO se revela mediante scramble en vez de aparecer estático, más una
  línea secundaria localizada y una firma configurable que nunca inventa un
  nombre.
- Especificaciones `docs/protocols/qyro1-wire-format.md` y
  `docs/protocols/manifest-format.md`, y `docs/security/parser-threats.md`.

### Fixed

- iOS vuelve a compilar: se restauró un LaunchScreen.storyboard que Interface
  Builder puede abrir. Desde 67fa795 faltaban `toolsVersion`/`systemVersion` y
  todas las builds de iOS fallaban con `com.apple.InterfaceBuilder error -1`.
  Confirmado por el run 30963011815: los diez pasos en verde, incluida la
  verificación de que `_qyro_protocol_version_ptr` y `_qyro_protocol_version_len`
  quedan enlazados en el bundle, y el XCTest en simulador.
- Recuperado el runtime ABI de Android, sin ejecución válida desde c971c9a:
  run 30963016390 ejecuta el smoke test en un emulador API 35 con KVM.
- El merge de `main` dejó silenciosamente el logo real dentro del archivo que el
  propietario marcó como inutilizable; se restauró byte a byte.

### Added

- Ruta canónica del logo `design/brand/source/logo.png`, fijada por SHA-256 y
  cubierta por cinco pruebas que impiden que el marcador rechazado vuelva a los
  activos empaquetados (ADR-0014).
- Regla anti-deriva en el job documental: STATUS.md falla si su `Verified commit`
  no es alcanzable desde HEAD o queda más de 10 commits por detrás.
- Validación estructural del storyboard de iOS en los contratos Bash y
  PowerShell, ejecutable sin host macOS.
- `docs/audits/CLAUDE_RECOVERY_AUDIT.md` con la recuperación completa del estado
  real, y ADR-0015 sobre la reconciliación de ramas.

### Changed

- STATUS.md reescrito con evidencia real: 51 tests de Flutter y 4 de Rust, iOS
  sin verificar y Android runtime sin ejecución válida en HEAD. Antes declaraba
  como no implementadas seis funciones que sí existían y fijaba un commit 58
  revisiones atrás.

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
