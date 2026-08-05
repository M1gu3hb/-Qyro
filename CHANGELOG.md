# Changelog

Basado en Keep a Changelog y Semantic Versioning.

## [Unreleased]

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
