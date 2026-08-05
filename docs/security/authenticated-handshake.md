# Handshake autenticado

Implementación: `rust/crates/qyro_crypto/src/handshake`. Decisión: ADR-0021.
Vectores: `docs/security/test-vectors/handshake-v1.json`.
Máquina de estados: `handshake-state-machine.md`.
Análisis de amenazas: `handshake-threat-analysis.md`.

## Qué establece

Dos dispositivos terminan con:

- el **mismo identificador de sesión** de ocho bytes;
- un **par de claves direccionales** —lo que uno escribe es lo que el otro lee—;
- la **identidad pública del otro**, la que realmente firmó el transcript.

Tener la identidad del otro **no es confiar en ella**. El emparejamiento y la
decisión de confiar son pasos posteriores y explícitos que todavía no existen.

## Qué **no** hace

- No cifra nada. Las claves derivadas no las consume ningún AEAD.
- No corre sobre ningún transporte: no hay sockets, ni descubrimiento, ni
  integración con el framing de `qyro_protocol`.
- No hay ventana de replay, ni rotación de claves, ni persistencia.
- No hay almacenamiento seguro: todo vive en memoria y se pierde al cerrar el
  proceso.

## Suite

| Elemento | Algoritmo | KAT |
|---|---|---|
| Acuerdo de clave | X25519 (RFC 7748) | `rfc7748-x25519.json` |
| Autenticación | Ed25519 (RFC 8032), dominio `HandshakeTranscript` | `rfc8032-ed25519.json` |
| Hash y transcript | SHA-256 | vía los anteriores |
| Derivación | HKDF-SHA256 (RFC 5869) | `handshake-v1.json` |
| Confirmación | HMAC-SHA256 (RFC 2104) | `rfc4231-hmac-sha256.json` |

`HANDSHAKE_VERSION = 1` y `CRYPTO_SUITE_ID = 1` identifican la combinación
completa. **No hay negociación**: un byte distinto en cualquiera de los dos es un
rechazo, no una degradación. Negociar suites es el mecanismo por el que un
atacante en el medio elige la más débil.

## Mensajes

Longitudes fijas, sin ningún campo de longitud declarado por el peer. El layout
exacto está en ADR-0021.

| Mensaje | Bytes | Contenido |
|---|---|---|
| `InitiatorHello` | 100 | prefijo, X25519 efímera, nonce, identidad |
| `ResponderHello` | 164 | los mismos 100 sin firmar, más 64 de firma |
| `InitiatorFinish` | 99 | prefijo, firma, MAC de confirmación |
| `ResponderFinish` | 35 | prefijo, MAC de confirmación |

Los tres bytes de prefijo son versión, suite y **tipo de mensaje**. El tipo va
dentro del mensaje y por tanto dentro del transcript: es lo que hace el
transcript asimétrico por rol.

## Identificador de sesión

Ocho bytes, el ancho exacto que reserva la cabecera QYRO/1, derivados con la
etiqueta `session-id`:

```
session_id = HKDF-Expand(PRK, "QYRO-HS-V1/session-id" || 0x00 || auth_transcript, 8)
```

Ocho bytes **derivados**, no 32 recortados. HKDF-Expand está definido para
cualquier longitud, así que pedir ocho es una derivación completa y no una clave
acortada.

Antes del sprint 4B.1 el schedule producía 32 bytes mientras la cabecera llevaba
ocho, sin ninguna conversión entre ambos. Quien conectara el transporte habría
tenido que inventar un truncamiento: una decisión sobre un formato congelado,
tomada en un call site. Ahora `qyro_protocol::SessionId` es el único tipo y lo
usan los dos lados.

## Claves de sesión

`SessionKey` es **privado del crate**. No se exporta, no hay accesores públicos
de clave, y la única salida es `into_secrets`, que es `pub(crate)` y que consume
el AEAD de frames desde el sprint 4C, a través de `into_frame_crypto`. Ese método
toma `self` por valor: el estado establecido deja de existir en el momento en que
se derivan el sealer y el opener, así que no hay forma de arrancar dos contadores
de nonce desde cero. Ver `docs/security/frame-encryption.md`.

Lo que sí expone un estado establecido: `session_id`, `role`, `peer_identity` y
`peer_fingerprint`. Nada más.

## Entropía

Los 64 bytes por lado se obtienen de `getrandom` de forma **falible**; si el
sistema no puede darlos, el handshake falla con `EntropyUnavailable` y no hay
ningún camino más débil.

El secreto X25519 se construye directamente desde esos bytes. No se usa
`EphemeralSecret::random_from_rng`, porque exige un `CryptoRng` cuyo
`fill_bytes` es **infalible**: ningún adaptador que lo alimente puede informar de
agotamiento, y la primera versión de este código respondía a una lectura de más
rellenando ceros y devolviendo éxito. Un secreto X25519 de ceros no es una clave
obviamente muerta: se *clampea* a un escalar válido y completa un handshake
normal sin entropía dentro.

La función `getrandom` de `x25519-dalek` queda **desactivada** a propósito: su
`random()` hace pánico cuando el CSPRNG falla.

El clamping y la aritmética de curva viven íntegramente en la biblioteca.

## Vectores

`handshake-v1.json` registra una ejecución completa: identidades, entropía,
claves efímeras, secreto compartido, los cuatro mensajes, ambos transcripts,
ambas entradas de firma, ambas firmas, cada `info` de HKDF, cada clave derivada,
el `session_id` de ocho bytes y ambos MAC.

`handshake-v1.schema.json` es estricto: `additionalProperties: false`, todos los
campos obligatorios, cada campo hex fijado a su longitud exacta y las versiones
fijadas como constantes.

Tres comprobaciones distintas lo protegen, y cada una detecta un fallo diferente:

1. **Regeneración byte a byte.** El archivo no puede quedar obsoleto respecto al
   código.
2. **Validación contra el schema.** El validador implementa solo el subconjunto
   que el schema usa y **falla ante cualquier palabra clave que no entienda**: un
   validador que ignora lo desconocido informa de éxito sobre restricciones que
   nunca comprobó.
3. **Verificación independiente.** Cada valor se reconstruye desde las
   primitivas sin pasar por la máquina de estados que lo produjo, así que un
   cambio que moviera código y vectores a la vez seguiría apareciendo.

## Regenerar

    cargo test -p qyro_crypto generate_handshake_vector -- --ignored --nocapture

No hay binario generador: exigiría un constructor determinista en la API pública
de `qyro_crypto`, que es justo lo que la biblioteca no debe exportar.
