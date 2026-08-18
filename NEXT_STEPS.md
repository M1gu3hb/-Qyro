# Próximos pasos

## P0 — y sólo hay uno

**Ejecutar la fase 07: el protocolo de hardware físico.**
[docs/testing/hardware-protocol.md](docs/testing/hardware-protocol.md).

- Aceptación: los veinte escenarios ejecutados y anotados en
  `docs/reports/fase-07-hardware-fisico.md`, **incluidos los que fallen y los que
  no se ejecutaron**, con el modelo del teléfono, su versión de Android, la
  versión de Windows, y el SHA-256 del APK y del `.exe` que se instalaron.
- **Un escenario sin marcar no es un aprobado.** No se escribe un resultado que
  nadie vio.
- Necesita dos aparatos, una Wi-Fi y una persona. No hay forma de hacerlo desde
  aquí, y fingir que la hay es la única cosa que arruinaría el proyecto.

Lo que la fase 07 encuentre decide la v1.1. Escribir hoy la lista sería adivinar
antes de la única medición que falta.

---

**Lo que había en esta sección hasta la fase 10**, y por qué ya no está: los
golden tests de arranque, el benchmark de arranque documentado y la retención de
artefactos eran los tres P0 heredados del sprint 4C. La fase 09 los dispositionó
uno a uno en `BUGS_PENDING.md` —dos descartados con argumento, el de artefactos
cerrado por `release.yml`— y `docs/reports/deuda-de-calidad.md` explica el
criterio. Lo de abajo se conserva como registro; no es trabajo pendiente.

## P1

- **QYR-0056: la guarda de material de clave no ve `Vec<u8>` ni `String`.**
  Razona sobre el tipo de retorno escrito en el fuente, así que un `pub fn` que
  devuelva bytes de clave dentro de un `Vec<u8>` no dispara ningún marcador. La
  corrección barata está identificada —congelar los sitios de origen;
  `signing_key.to_` aparece una sola vez en todo el crate— y quedó fuera del
  alcance de 4D.1.
- **QYR-0050: la ruta del blob depende de un nombre de producto provisional.**
  `%LOCALAPPDATA%\Qyro\identity.bin` deja de encontrar la identidad si Qyro pasa
  a llamarse otra cosa. Es un problema de migración, no de seguridad, y hay que
  decidirlo antes de que exista un usuario con una identidad guardada.
- **Atar el blob a un valor propio de la máquina.** `LOCALAPPDATA` evita que
  viaje con un perfil móvil; la MasterKey sí viaja, así que copiar el archivo a
  mano a otra máquina del mismo usuario de dominio lo abre. Cerrarlo estaba
  fuera del alcance de 4D.1 y sigue abierto.
- **Que algo del producto llame al almacén.** Hoy `qyro_ffi` no depende de
  `qyro_identity_store` y una prueba lo mantiene así, de modo que la aplicación
  Flutter no guarda ni carga identidad alguna. Conectarlo exige decidir primero
  qué cruza la frontera FFI, y **no puede ser la semilla**.
- **QYR-0039: cómo obtiene CI su `cargo-audit`.** Hoy lo compila desde fuente con
  pin exacto en cada run, lo que mete un centenar de crates sin auditar en el
  perímetro de confianza de CI y caduca cuando el advisory DB avanza. Binario con
  checksum, acción cacheada, o rango de versiones: las tres tienen contrapartidas
  distintas.

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

## Completado el 2026-08-07 (sprint 4C.3, cotas de recursos)

No añadió funcionalidad. Corrigió dos cotas y extendió una guarda, todo en la
ruta que tocará los bytes de un peer, y todo antes de que exista un consumidor
— que es exactamente cuándo conviene, porque el único perjudicado es una prueba.

- **El decoder ya no es cuadrático.** `next_frame` reclamaba cada frame
  entregado con `drain(..total)`, que memmovea todo lo que queda detrás. Llenar
  el búfer de heartbeats y drenarlo movía **11 476 501 344 bytes** para
  1 049 664 empujados; ahora mueve 0, y el bucle realista con backlog pasó de
  9,8 GB a 2 359 296 sobre 2 596 608. Contado, no cronometrado (QYR-0024,
  ADR-0016 enmendado).
- **La capacidad del búfer ya no dobla su límite.** Llegaba a 2 097 152 frente a
  `MAX_BUFFER_LEN` de 1 049 664, y dos pruebas ya afirmaban lo contrario sin
  llenar nunca el búfer (QYR-0027).
- **`qyro_protocol` y `qyro_manifest` no pueden terminar el proceso.** 33 y 22
  infracciones respectivamente, ninguna silenciada; la guarda encontró además un
  `debug_assert_eq!` que ningún lint cubre (QYR-0036).
- **Las exenciones de la guarda se derivan del código.** Quitar un
  `#[cfg(test)]` mueve el archivo al conjunto de producción en vez de eximirlo
  (QYR-0042).
- **Los seis workflows corren sobre cualquier rama `claude/**`.** En 4C.2 el
  nombre de la rama estaba escrito a mano en los seis YAML, así que la rama
  siguiente heredaba el defecto entero (QYR-0040).
- Registro completo: un `QYR-00xx` citado sin entrada es ahora un BLOCKER
  (QYR-0043); fecha de Unicode corregida (QYR-0041); consejo de regeneración
  condicionado (QYR-0044); `MAX_HASH_LEN` con prueba y `FrameTooLarge`
  documentado como inalcanzable donde lo es (QYR-0038).
- 307 → 323 tests. `cargo audit` sigue en 56 crates, `cargo tree -d` sin
  duplicados, cero dependencias nuevas.

## Completado el 2026-08-07 (sprint 4C.2, cierre de la auditoría independiente)

No añadió funcionalidad. Cerró trece hallazgos de una auditoría externa, de los
cuales uno era un fallo de seguridad real y tres eran garantías que sobrevivían
a su propio borrado.

- **La categoría Unicode `Cf` se rechaza en rutas.**
  `RelativePath::parse("invoice\u{202E}fdp.exe")` devolvía `Ok` y todo
  renderizador consciente de bidi mostraba ese nombre como `invoiceexe.pdf`.
  `char::is_control()` es la categoría `Cc` y nada más. Tabla de veintiún rangos
  transcrita de Unicode 16.0.0, citada y comprobada contra el archivo, sin
  dependencias nuevas (QYR-0021).
- **Un archivo ya no puede ser también el directorio padre de otro elemento.**
  Las claves de colisión se comparaban por igualdad, y `"a"` y `"a\0b"` son dos
  cadenas distintas (QYR-0028).
- **Tres garantías de `qyro_crypto` que sobrevivían a su borrado ahora tienen
  prueba**: la autenticación del iniciador, `verify_strict` frente a `verify`, y
  el transcript, que se verificaba llamándose a sí mismo (QYR-0022, QYR-0023,
  QYR-0025).
- **Ninguna ruta de producción de `qyro_crypto` puede terminar el proceso.** La
  guarda leía tres archivos bajo `src/aead/`; ahora lee los doce, y detecta
  además un módulo al que nadie le puso el `#![deny(...)]`. Dos pánicos
  eliminados y catorce indexaciones sin comprobar con ellos (QYR-0033).
- **La frontera FFI se comprueba sobre el cierre transitivo real**, con
  `cargo metadata` en vez de partiendo el manifest por una cadena (QYR-0030).
  **Superado en la fase 01:** el sujeto de la guarda ya no es el cierre sino el
  conjunto de dependencias directas. `cargo metadata` sigue siendo la fuente, que
  es lo que QYR-0030 arregló y sigue en pie. ADR-0032 §9.
- Cuatro controles de la ruta de decode con prueba propia (QYR-0032), nombres de
  dispositivo con superíndice rechazados (QYR-0029), variantes de error muertas
  eliminadas con guarda que impide su vuelta (QYR-0035), decisión sobre
  codificaciones X25519 no canónicas registrada (QYR-0034), seis sitios de
  documentación corregidos y marcados como corregidos (QYR-0031), y los seis
  workflows disparándose solos sobre la rama de trabajo (QYR-0026).
- 278 → 307 tests. `cargo audit` sigue en 56 crates y `cargo tree -d` sin
  duplicados.

**Lo que quedó abierto, a propósito y registrado**: QYR-0024 y QYR-0027 (sprint
4C.3), la mitad sin fuente de QYR-0029, la verificación pendiente de QYR-0034, y
QYR-0036, nuevo.

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
