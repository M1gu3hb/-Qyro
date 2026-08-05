# ADR-0021: handshake autenticado de cuatro mensajes

Estado: aceptada. Sustituye la reserva del dominio `HandshakeTranscript` que
ADR-0020 dejó abierta.

Este ADR **congela el transcript antes de que exista el código**. Es deliberado:
un transcript definido después de implementarlo documenta lo que el código hace,
no lo que el protocolo exige, y las dos cosas se parecen justo hasta que dejan de
parecerse. Cualquier implementación futura en Swift, Kotlin o Dart debe poder
escribirse contra este documento sin leer el Rust.

## Contexto

`qyro_crypto` tiene identidad Ed25519 desde el sprint 4A, pero nada que
establezca una clave compartida. Sin handshake, dos dispositivos pueden probar
quiénes son y aun así no tener con qué cifrar; `EncryptedEnvelope` sigue siendo
una forma de cable sin nadie que calcule el tag.

## Alcance

Solo el handshake, **en memoria**. Fuera de alcance en esta decisión:
ChaCha20-Poly1305, sellado y apertura de frames, ventana de replay, sockets, TLS,
LAN, transferencia, SQLite, persistencia de identidad, almacenes seguros, FFI,
interfaz de emparejamiento y QR. El handshake produce secretos; nada los usa
todavía.

## Suite

| Elemento | Algoritmo |
|---|---|
| Acuerdo de clave | X25519 (RFC 7748) |
| Autenticación | Ed25519 (RFC 8032), dominio `HandshakeTranscript` |
| Hash y transcript | SHA-256 |
| Derivación | HKDF-SHA256 (RFC 5869) |
| Confirmación | HMAC-SHA256 (RFC 2104) |

`HANDSHAKE_VERSION = 1` y `CRYPTO_SUITE_ID = 1` identifican esta combinación
completa. No hay negociación: un byte distinto en cualquiera de los dos es un
rechazo, no una degradación. Negociar suites es el mecanismo por el que un
atacante en el medio elige la más débil, y con una sola suite definida no hay
nada que negociar.

## Constantes

| Nombre | Valor |
|---|---|
| `HANDSHAKE_VERSION` | 1 |
| `CRYPTO_SUITE_ID` | 1 |
| `NONCE_LEN` | 32 |
| `X25519_PUBLIC_LEN` | 32 |
| `PUBLIC_IDENTITY_WIRE_LEN` | 33 |
| `SIGNATURE_LEN` | 64 |
| `FINISHED_MAC_LEN` | 32 |

## Mensajes

Los cuatro empiezan con el mismo prefijo de 3 bytes: versión, suite y tipo. El
tipo va **dentro** del mensaje y por tanto dentro del transcript, que es lo que
hace el transcript asimétrico por rol: un `InitiatorHello` no puede reinterpretarse
como un `ResponderHello` porque el byte 2 difiere y entra en el hash.

Todas las longitudes son fijas. No hay campos de longitud variable, así que no
hay ningún lugar donde un peer declare un tamaño.

### 1. `InitiatorHello` — 100 bytes

| Offset | Bytes | Contenido |
|---|---|---|
| 0 | 1 | `HANDSHAKE_VERSION` |
| 1 | 1 | `CRYPTO_SUITE_ID` |
| 2 | 1 | tipo = 1 |
| 3 | 32 | clave pública X25519 efímera del iniciador |
| 35 | 32 | nonce del iniciador |
| 67 | 33 | identidad pública del iniciador (forma de 33 bytes) |

### 2. `ResponderHello` — 164 bytes

| Offset | Bytes | Contenido |
|---|---|---|
| 0 | 1 | `HANDSHAKE_VERSION` |
| 1 | 1 | `CRYPTO_SUITE_ID` |
| 2 | 1 | tipo = 2 |
| 3 | 32 | clave pública X25519 efímera del respondedor |
| 35 | 32 | nonce del respondedor |
| 67 | 33 | identidad pública del respondedor |
| **100** | — | **fin del prefijo sin firmar** |
| 100 | 64 | firma del respondedor |

Los primeros 100 bytes son `responder_hello_unsigned` y son lo que entra en
`base_transcript`. La firma no puede estar dentro de lo que firma.

### 3. `InitiatorFinish` — 99 bytes

| Offset | Bytes | Contenido |
|---|---|---|
| 0 | 1 | `HANDSHAKE_VERSION` |
| 1 | 1 | `CRYPTO_SUITE_ID` |
| 2 | 1 | tipo = 3 |
| 3 | 64 | firma del iniciador |
| 67 | 32 | MAC de confirmación del iniciador |

### 4. `ResponderFinish` — 35 bytes

| Offset | Bytes | Contenido |
|---|---|---|
| 0 | 1 | `HANDSHAKE_VERSION` |
| 1 | 1 | `CRYPTO_SUITE_ID` |
| 2 | 1 | tipo = 4 |
| 3 | 32 | MAC de confirmación del respondedor |

## Transcript

```
base_transcript = SHA-256(
      "QYRO-HANDSHAKE-BASE-V1" || 0x00
   || len(initiator_hello)          as u32 BE || initiator_hello
   || len(responder_hello_unsigned) as u32 BE || responder_hello_unsigned )
```

Las longitudes son explícitas aunque los mensajes sean de tamaño fijo. Cuestan
ocho bytes y eliminan una clase entera de ataque: sin ellas, dos concatenaciones
distintas pueden producir los mismos bytes, y añadir un campo en una versión
futura convierte esa ambigüedad en real sin que nadie lo note.

## Firmas

Ambas se emiten en el dominio `SignatureDomain::HandshakeTranscript`, que
ADR-0020 reservó exactamente para esto y que deja de estar reservado con esta
decisión.

```
responder_signature = Sign(responder_identity, base_transcript)                        // 32 bytes firmados
initiator_signature = Sign(initiator_identity, base_transcript || responder_signature) // 96 bytes firmados
```

El iniciador firma **incluyendo la firma del respondedor**, de modo que su firma
queda atada a esa respuesta concreta.

Límite honesto de esa afirmación, encontrado borrando el enlace y comprobando
que **ninguna prueba de extremo a extremo fallaba**: Ed25519 aquí es
determinista (RFC 8032) y la identidad del respondedor está dentro de
`base_transcript`, así que `responder_signature` es una función pura de
`base_transcript` —mismo transcript, misma firma, siempre—. El enlace no aporta
nada que `base_transcript` no aporte ya, y ningún ataque de extremo a extremo
distingue las dos construcciones.

Se conserva como defensa en profundidad para un firmante **no** determinista —un
token hardware, o una variante futura que aleatorice— donde existen varias firmas
válidas sobre un mismo transcript. Lo que sí se comprueba es la construcción: que
la firma del respondedor forme realmente parte de lo que firma el iniciador.

Una firma de respondedor no puede presentarse como firma de iniciador: la entrada
de firma de ADR-0020 incluye `len(message) as u64 BE`, y 32 nunca es 96. La
separación de rol no depende de un prefijo añadido a mano.

```
auth_transcript = SHA-256(
      "QYRO-HANDSHAKE-AUTH-V1" || 0x00
   || base_transcript      (32)
   || responder_signature  (64)
   || initiator_signature  (64) )
```

## Derivación de claves

```
PRK = HKDF-Extract( salt = base_transcript, IKM = x25519_shared_secret )

info(label) = "QYRO-HS-V1/" || label || 0x00 || auth_transcript
key(label)  = HKDF-Expand( PRK, info(label), 32 )
```

Etiquetas: `initiator-finished`, `responder-finished`, `initiator-to-responder`,
`responder-to-initiator`, `session-id`.

`auth_transcript` entra en cada `info`, así que **ninguna clave derivada existe
sin que ambas firmas hayan entrado en su derivación**. Dos ejecuciones que
difieran en cualquier byte de cualquier mensaje producen claves distintas, y no
existe un secreto «solo con Diffie-Hellman» del que sacar provecho si la
autenticación falla.

`base_transcript` como salt es redundante —`auth_transcript` ya lo contiene—
pero es inofensivo y mantiene el extract atado al intercambio aunque una versión
futura cambie qué entra en el info.

Las direcciones son separadas: la clave con la que el iniciador escribe nunca es
la clave con la que lee. Reutilizar una sola clave en ambos sentidos permite
reflejar los propios mensajes de un peer de vuelta hacia él.

## Confirmación

```
initiator_finished = HMAC-SHA256( key("initiator-finished"), auth_transcript )
responder_finished = HMAC-SHA256( key("responder-finished"), auth_transcript )
```

Se comparan en **tiempo constante**, nunca con `==`. Un `==` sobre un MAC filtra
por temporización cuántos bytes iniciales acertó quien lo envió, que es
suficiente para construirlo byte a byte.

## Claves rechazadas

- Identidad pública de orden bajo: `WeakPublicIdentity`. Mismo motivo que en
  ADR-0020: la firma verificaría para casi cualquier mensaje.
- Secreto compartido X25519 no contributorio: `NonContributorySharedSecret`. Si
  el peer envía un punto de orden pequeño, el secreto resultante es todo ceros y
  ambos lados «coinciden» sin que ninguno haya aportado nada.

## Máquina de estados

Los estados se **consumen**. Cada transición toma `self` por valor y devuelve el
siguiente estado, así que reutilizar uno anterior no compila:

```
InitiatorStart → InitiatorAwaitResponder → InitiatorAwaitResponderFinish → EstablishedInitiator
ResponderStart → ResponderAwaitInitiatorFinish → EstablishedResponder
```

No es cosmético. Un handshake con estado mutable y un campo `step` permite
procesar dos veces el mismo mensaje, o procesarlos en otro orden, y esos son
precisamente los errores que un revisor no ve leyendo el camino feliz. Aquí el
compilador los rechaza.

## Alternativas descartadas

- **Noise (IK/XX).** Bien estudiado, y probablemente el destino a largo plazo.
  Descartado ahora porque adoptarlo bien significa adoptar su framing, su
  gestión de nonces y su rekey completos, no solo el patrón; hacerlo a medias
  daría la apariencia de Noise sin sus garantías. Esta construcción es más
  pequeña y está enteramente escrita aquí.
- **TLS 1.3.** ADR-0004 lo prevé para el transporte. No sirve como handshake de
  emparejamiento: su modelo de identidad son certificados y una PKI, y Qyro no
  tiene ni tendrá autoridad certificadora.
- **Firmar `auth_transcript` en vez de `base_transcript`.** Circular: el
  `auth_transcript` contiene las firmas.
- **Un solo MAC de confirmación.** Confirma un sentido. Con dos, cada lado
  demuestra al otro que derivó las mismas claves.

## Consecuencias

- Tres dependencias nuevas: `x25519-dalek` (BSD-3-Clause), `hkdf` y `hmac`
  (MIT/Apache-2.0). `x25519-dalek` comparte `curve25519-dalek 5.0` con
  `ed25519-dalek`, y `hkdf`/`hmac` comparten `digest 0.11` con `sha2`, así que no
  entra ninguna versión duplicada de una primitiva.
- El dominio `HandshakeTranscript` deja de estar reservado.
- El handshake **no** se ejecuta sobre ningún transporte. No hay sockets, no hay
  descubrimiento, y los botones Enviar/Recibir siguen deshabilitados.
- Las claves derivadas no cifran nada todavía: no hay AEAD.
