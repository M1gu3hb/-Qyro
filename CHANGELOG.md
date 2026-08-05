# Changelog

Basado en Keep a Changelog y Semantic Versioning.

## [Unreleased]

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
