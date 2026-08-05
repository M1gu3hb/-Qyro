# Próximos pasos

## P0

1. Golden tests de la secuencia de arranque.
   - Aceptación: 0/20/50/80/100 %, teléfono, tablet, Windows ancho, reduced
     motion, branding provisional, branding válido con firma, fallo de
     biblioteca, fallo de asset, timeout y retry. Seeds deterministas,
     dimensiones fijas, assets locales y ninguna dependencia de hora o red.
     Archivos golden versionados y documentado cómo actualizarlos.
2. Benchmark de arranque documentado.
   - Aceptación: `docs/benchmarks/boot-baseline.md` con tiempo de preparación
     del modelo, tiempo de `frameAt`, build y paint por frame, tamaño del asset
     ASCII, y declaradas máquina, SO, versión de Flutter, modo, resolución y
     número de muestras. Sin afirmar 60 FPS sin medirlo.
3. Retener artefactos de desarrollo con SHA-256, etiquetados
   DEVELOPMENT / NOT FOR PUBLIC RELEASE.
   - Estado real: el ZIP portable de Windows **sí** se retiene (14 días,
     `qyro-windows-x64-portable-debug`). El APK de Android y el `Runner.app` de
     iOS no. Lo que falta en los tres casos es un checksum distribuido **dentro**
     del paquete y la etiqueta: el digest que GitHub imprime al subir un artefacto
     identifica el ZIP que produjo ese run, no el contenido que alguien descarga
     y desempaqueta, y presentarlo como sustituto sería un cambio silencioso de
     lo que se está afirmando.

## P1

- **Persistencia segura de `DeviceIdentity`.** Android Keystore, iOS Keychain y
  DPAPI/CNG en Windows, con rotación, borrado y pruebas en runtime. Hoy generar
  una identidad y cerrar el proceso la pierde, así que ninguna decisión de
  confianza puede sobrevivir a un reinicio. Sin conectar sockets ni transferencia
  todavía.
- **Una segunda implementación contra los vectores.** `handshake-v1.json` y
  `aead-v1.json` existen, están encadenados y verificados de forma independiente
  contra las primitivas, pero hasta que alguien escriba el lado Swift o Kotlin y
  encuentre las ambigüedades que queden, «formato definido sin ambigüedad» es una
  intención y no un hecho.
- Ampliar la campaña de `cargo-fuzz`. Desde el sprint 4C.1 corre semanalmente,
  seis targets, dos minutos cada uno, y eso es un suelo, no una revisión: la
  cobertura sobre entradas imprevistas más allá de ese presupuesto sigue siendo
  desconocida. Cualquier hallazgo entra al corpus antes de corregirse.
- Probar en hardware físico: hasta ahora solo emulador, simulador y host. El
  sprint 4C.1 añadió ejecución de `qyro_crypto` en cuatro entornos y ninguno es
  un teléfono; Android arm64 e iOS device se compilan y no se ejecutan.
- SBOM y `cargo-deny` para licencias, fuentes, duplicados y bans.
- Selección de archivos y construcción del manifest desde el filesystem real.

## P2

- SQLite/migración 0001.
- LAN e IP manual.
- Emparejamiento por QR e identidad local.

## P3

- RaptorQ/QR adaptativo.
- Wi-Fi Direct, Multipeer y Bluetooth experimental.

## Completado el 2026-08-05 (sprint 4C.1, endurecimiento del AEAD)

- `qyro_crypto` compilado para Android, iOS y Windows y **ejecutado** en cuatro
  entornos mediante un harness aislado que no entra en el producto (ADR-0023).
  Hasta aquí, cuatro workflows en verde no decían nada sobre `qyro_crypto` fuera
  de x86_64 Linux: todos ejercitaban `qyro_ffi`, que no depende de él.
- Sin `panic!`, `unreachable!`, `assert!` ni indexado sin comprobar en la ruta
  AEAD de producción, sostenido por un `deny` de Clippy y por una prueba que lee
  el fuente. Un sealer que falla queda envenenado, para que un reintento no
  reutilice una secuencia.
- Texto claro autenticado y búferes temporales en `Zeroizing`, con las features
  `zeroize` de `sha2` y `hmac` activadas: estaban apagadas, así que el estado de
  cada transcript y de cada MAC quedaba en memoria liberada.
- `rust/fuzz` construible por primera vez, seis targets y una campaña acotada
  semanal.
- `.gitattributes` con `eol=lf`: el repositorio se extraía con CRLF en Windows y
  tres pruebas de comparación byte a byte fallaban por eso y no por el código.

## Completado el 2026-08-05 (sprint 4B.1, cierre del handshake)

- `SessionId` canónico de ocho bytes compartido por `qyro_protocol` y
  `qyro_crypto`; la etiqueta `session-id` deriva ese ancho exacto en vez de 32
  bytes que alguien habría tenido que recortar.
- `ResponderFinishPending`: el respondedor no obtiene sesión hasta confirmar que
  entregó su último mensaje.
- `SessionKey` fuera de la API pública; sin accesores de clave.
- Entropía efímera sin ningún camino que pueda sustituir bytes.
- `handshake-v1.json` y su schema estricto, con regeneración byte a byte y
  verificación independiente desde las primitivas.
- KAT de X25519 (RFC 7748), que era la única primitiva sin vectores oficiales.
- Reglas nuevas en `check_docs_consistency` (Bash y PowerShell) contra la deriva
  documental, verificadas en rojo una por una.

## Completado el 2026-08-05 (sprint 4B, handshake autenticado)

- Handshake autenticado de cuatro mensajes en memoria: X25519, Ed25519 en el
  dominio `HandshakeTranscript`, HKDF-SHA256 y HMAC-SHA256, con máquina de
  estados de estados consumidos (ADR-0021).
- Cerradas las invariantes que quedaban: cabecera protegida fuera de `Frame`,
  plantilla de sobre probada por tipo, claves Ed25519 de orden bajo rechazadas,
  `verify_strict`, firma solo falible, fingerprint con dos escrituras canónicas,
  identidad pública de 33 bytes y constructor determinista fuera de la API.
- KAT completos: RFC 8032 §7.1 (5 vectores) y RFC 4231 (7 vectores), extraídos
  del texto de los RFC.
- Corregidas afirmaciones obsoletas sobre el plegado Unicode y sobre qué ABI
  nativas estaban verificadas.

## Completado el 2026-08-05 (sprint 2, protocolo y manifest)

- `qyro_protocol`: framing QYRO/1 con decoder incremental acotado; 29 contratos
  de wire y 4 property tests.
- `qyro_manifest`: manifest canónico y `RelativePath` estricto; 40 contratos y
  5 property tests.
- Targets `cargo-fuzz`, corpus de 65 entradas y smoke en CI.
- `cargo audit` obligatorio, en verde con cero dependencias externas.
- Wordmark, tagline y firma configurable mediante scramble, sin inventar nombre.
- ADR-0016, ADR-0017 y las especificaciones de wire y manifest.
- CI en verde sobre la rama: run 30964542743.

## Completado el 2026-08-05 (Hito A, recuperación)

- Reconciliadas main y audit/baseline-hardening sin force-push ni pérdida de
  commits; ambos cambios del propietario preservados.
- Logo canónico fijado en design/brand/source/logo.png por checksum, con el
  marcador rechazado excluido del producto y cubierto por pruebas (ADR-0014).
- Corregida la regresión que impedía compilar iOS desde 67fa795, con contrato
  estructural del storyboard verificado en rojo y verde.
- Cerrada la brecha que permitió a STATUS.md derivar 58 commits sin detección.
- Baseline completo reproducido en host Linux: Rust, Flutter (51 tests),
  11 contratos de script, 7 tests Python y el job documental.
- iOS y Android confirmados en CI sobre esta rama: run 30963011815 (iOS, 10/10
  pasos, incluye verificación de símbolos y XCTest) y run 30963016390 (Android,
  smoke test de ABI en emulador).

## Completado el 2026-08-04

- doctor/bootstrap/test_all en Bash y PowerShell mediante TDD.
- bootstrap crea configuración desde examples sin sobrescribir al usuario.
- Puente Dart↔qyro_ffi con lectura real QYRO/1 en Linux y Windows.
- APK contiene bibliotecas Rust arm64-v8a/x86_64 y Windows distribuye la DLL junto al ejecutable.
