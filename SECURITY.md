# Seguridad

**Estado: el protocolo está en el cable, y no ha tocado hardware.** Un archivo
cruza un socket TCP sellado frame a frame con ChaCha20-Poly1305 bajo una clave
derivada de un handshake autenticado, y se verifica con SHA-256 antes de
entregarse. Lo que sigue sin existir es la evidencia de aparato real: ver
`docs/testing/hardware-protocol.md`, veintiséis huecos en blanco.

> **Dos frases que estuvieron aquí hasta 2026-08-31, y las dos eran falsas.**
> Se dejan escritas para que no vuelvan; este documento se verifica contra el
> código desde una guarda que corre en el gate (QYR-0395).
>
> 1. *«Estado: diseño inicial; no hay transferencia real.»* Dejó de ser cierta
>    en la fase 12.
> 2. *«TLS 1.3 para red.»* **Nunca fue cierta, y es la peor frase que puede
>    tener un documento de seguridad: prometía un protocolo que este programa no
>    usa.** Qyro no habla TLS por ninguna parte. Su transporte es un handshake
>    propio —X25519, Ed25519, HKDF-SHA256— sobre un socket TCP desnudo, y el
>    secreto de cada frame lo da ChaCha20-Poly1305 (ADR-0021, ADR-0022,
>    ADR-0028). Quien leyera esta línea y dedujera «entonces tiene la
>    autenticación de servidor y la revocación de certificados de TLS» estaría
>    equivocado en las dos cosas: aquí la autenticación es **una huella que dos
>    personas comparan**, y no hay ninguna autoridad que revoque nada.

## Principios

- Primitivas revisadas, separación de identidad/sesión/contenido y nonces únicos.
- **Nada de TLS.** Handshake propio y sellado por frame, con la misma capa de
  contenido en los cuatro transportes: red local, cable directo, óptico y serie.
- Suite fijada por ADR: X25519, Ed25519, SHA-256, HKDF-SHA256 y HMAC-SHA256 para
  el handshake (ADR-0021); ChaCha20-Poly1305 para los frames (ADR-0022). Se
  descartó XChaCha20-Poly1305 —su nonce largo sirve para elegirlo al azar, y aquí
  es un contador— y AES-GCM, que necesita aceleración por hardware para ser rápido
  y resistente a canales laterales a la vez. BLAKE3 sigue sin usarse.
- Claves privadas en Keystore, Keychain o DPAPI/CNG.
- Metadata, manifest, nombres y rutas cifrados.
- Longitudes y conteos se validan antes de reservar memoria.
- Un frame no puede declarar `ENCRYPTED` sin llevar tag: solo el sellado activa
  ese flag, y produce el tag en la misma operación. Desde el sprint 4C ese tag lo
  calcula un AEAD real; ver `docs/security/frame-encryption.md`.
- El nonce de cada frame es prefijo derivado más secuencia, la asigna el sealer y
  no da la vuelta: agotarla es un error terminal. Ver
  `docs/security/nonce-lifecycle.md`.
- La ventana de replay se consulta antes del AEAD y se actualiza solo después: un
  tag inválido no consume secuencia y una sesión ajena no toca la ventana. Ver
  `docs/security/replay-window.md`.
- El nombre visible se deriva de la ruta; el peer no envía uno aparte.
- Todo archivo lleva digest final, incluidos los de cero bytes.
- Se rechazan rutas que un sistema de archivos real plegaría en una sola.
- Temporales .qyro-part y rename solo tras autenticidad, tamaño, flush e integridad.
- Sin autoaceptación de peers desconocidos.
- Logs locales, rotativos y redactados; sin contenido, claves o rutas completas.

## Reporte

No publicar vulnerabilidades con datos sensibles en issues. Hasta definir SECURITY.md con canal privado del propietario, describir solo el impacto mínimo y solicitar coordinación.

## Estado de auditoría

`cargo audit` es obligatorio en CI desde el sprint 2 y pasa. Este apartado decía
además que «el workspace no tiene dependencias externas»: era cierto hasta el
sprint 4A e incorrecto desde entonces. `qyro_crypto` depende de la pila dalek y
de RustCrypto, y `qyro_manifest` de `unicode-normalization`. El inventario por
crate, versión y licencia está en `docs/LICENSE_AUDIT.md`; la ruta de parsing de
`qyro_protocol` sigue sin dependencias de terceros.

**Sí hay KAT de criptografía**, en contra de lo que decía este archivo:

| Primitiva | Vectores | Archivo |
|---|---|---|
| Ed25519 | RFC 8032 §7.1, las cinco pruebas | `docs/security/test-vectors/rfc8032-ed25519.json` |
| X25519 | RFC 7748 §5 y §6.1 | `docs/security/test-vectors/rfc7748-x25519.json` |
| HMAC-SHA-256 | RFC 4231, los siete casos | `docs/security/test-vectors/rfc4231-hmac-sha256.json` |
| Identidad Qyro | construcción propia | `docs/security/test-vectors/identity-v1.json` |
| Handshake Qyro | ejecución completa | `docs/security/test-vectors/handshake-v1.json` |
| ChaCha20-Poly1305 | RFC 8439 §2.8.2 y apéndice A.5 | `docs/security/test-vectors/rfc8439-chacha20poly1305.json` |
| AEAD de frames Qyro | cinco frames sellados, encadenados al handshake | `docs/security/test-vectors/aead-v1.json` |

Desde el sprint 4C.1 hay una **campaña de fuzzing acotada**: seis targets, dos
minutos cada uno, semanal y bajo demanda, con las estadísticas de libFuzzer en el
log para que «se fuzzeó» sea un número y no una afirmación. No es fuzzing
exhaustivo, y este apartado no lo presentará como tal: dos minutos por target
encuentra defectos superficiales. El corpus smoke sigue corriendo en cada commit,
que es lo que protege contra regresiones ya conocidas. Antes de 4C.1 esta sección
decía que los targets existían, lo cual era cierto, y omitía que ninguno
compilaba.

Lo que **sigue sin existir**: prueba de tráfico, revisión externa y auditoría
criptográfica independiente. Deben añadirse antes de afirmar seguridad de
transferencia.

## Ciclo de vida de los secretos

`docs/security/secret-lifecycle-audit.md` inventaría cada valor secreto de
`qyro_crypto` —dueño, duración, borrado, copias y si puede salir de la
biblioteca— y enumera los límites que ningún `Drop` cierra: swap, hibernación,
core dumps, registros y la reasignación de un `Vec`. Escribirlo destapó que las
features `zeroize` de `sha2` y de `hmac` estaban apagadas, así que el estado de
compresión de cada transcript y el estado con clave de cada MAC quedaban en
memoria liberada. Ahora están activadas.

Ninguna de esas garantías se ha *observado*: leer memoria liberada es
comportamiento indefinido, y una prueba que afirmara verlo estaría mintiendo. Lo
que las pruebas comprueban es el tipo, que es donde vive la garantía.

## Criptografía por plataforma

`qyro_crypto` se compila para Android, iOS y Windows y **se ejecuta** en cuatro
entornos —Linux, Windows, emulador Android y simulador iOS— mediante un harness
aislado que no entra en la aplicación. Hasta el sprint 4C.1 no había evidencia de
ninguna de las tres: los workflows en verde construían y ejecutaban `qyro_ffi`,
que deliberadamente no depende de `qyro_crypto`. Detalles y la distinción entre
compilar y ejecutar, fila por fila, en `docs/testing/crypto-platform-matrix.md`.
Un emulador y un simulador no son hardware, y nada aquí se ha medido en un
teléfono.
