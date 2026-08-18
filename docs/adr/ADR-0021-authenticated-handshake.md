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
key(label)  = HKDF-Expand( PRK, info(label), len(label) )
```

| Etiqueta | Bytes |
|---|---|
| `initiator-finished` | 32 |
| `responder-finished` | 32 |
| `initiator-to-responder` | 32 |
| `responder-to-initiator` | 32 |
| `session-id` | **8** |

El identificador de sesión se deriva a ocho bytes, el ancho exacto que reserva la
cabecera QYRO/1. No son 32 recortados: HKDF-Expand está definido para cualquier
longitud, así que pedir ocho es una derivación completa. Esta tabla la fijó la
enmienda del sprint 4B.1; antes este ADR no decía el ancho, y el código derivaba
32 mientras la cabecera llevaba ocho.

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
- Las claves derivadas no cifraban nada al aceptarse esta decisión. Desde
  ADR-0022 las consume el AEAD de frames, que no cambia nada de lo anterior: el
  handshake sigue sin correr sobre ningún transporte.

## Enmienda (sprint 4B.1)

Cuatro endurecimientos que **no cambian el formato de cable de los cuatro
mensajes ni los transcripts**, más una precisión sobre el identificador de
sesión que sí lo fija.

1. **`SessionId` de ocho bytes.** La etiqueta `session-id` deriva exactamente
   ocho bytes con HKDF-Expand, y ese es el tipo que lleva la cabecera QYRO/1.
   Este ADR no había dicho el ancho, y el código derivaba 32 bytes mientras la
   cabecera reservaba ocho. Ocho **derivados**, no 32 recortados: HKDF-Expand
   está definido para cualquier longitud, así que pedir ocho es una derivación
   completa y no una clave acortada. Queda congelado aquí.
2. **`ResponderFinishPending`.** El respondedor no obtiene una sesión hasta
   confirmar que entregó su `ResponderFinish`. Es un estado más en la máquina,
   no un mensaje más en el cable.
3. **Claves fuera de la API pública.** `SessionKey` es privado del crate y no
   hay accesores de clave.
4. **Entropía sin sustitución posible.** El secreto X25519 se construye
   directamente desde bytes obtenidos de forma falible. No se usa
   `EphemeralSecret::random_from_rng`: exige un `CryptoRng` infalible, de modo
   que ningún adaptador puede informar de agotamiento, y el relleno con ceros que
   había estaba forzado por la forma del trait. Un secreto de ceros se clampea a
   un escalar válido y completa un handshake sin entropía, así que el modo de
   fallo se elimina en vez de gestionarse.

`docs/security/test-vectors/handshake-v1.json` fija una ejecución completa contra
esta especificación, con `handshake-v1.schema.json` como schema estricto.

## Enmienda A — sprint 4C.2 (QYR-0034): codificaciones X25519 no canónicas

Una coordenada `u` son 32 bytes en little-endian, y un valor mayor o igual que
el primo del cuerpo `p = 2^255 - 19` es una **segunda codificación** de un punto
que ya tenía una. Este ADR no decía nada al respecto, y el código la aceptaba sin
que nadie lo hubiera decidido.

### Decisión: se aceptan, y la aritmética las reduce

RFC 7748 §5 lo exige en la dirección de aceptar:

> implementations of X25519 … MUST mask the most significant bit in the final
> byte

El enmascarado deja el valor por debajo de `2^255`; el resto de la reducción
módulo `p` la hace la aritmética del cuerpo. Una implementación conforme al RFC
no rechaza estas codificaciones, y rechazarlas sería una desviación del estándar
con la que tropezaría cualquier otra implementación conforme.

Tres razones más, en orden de peso:

1. **No hay maleabilidad que explotar.** El hello viaja al transcript **tal
   cual**, así que dos codificaciones del mismo punto producen transcripts
   distintos y por tanto claves distintas y sesiones distintas. Ninguna puede
   hacerse pasar por la otra.
2. **El peligro real ya está cerrado.** Un punto de orden pequeño se rechaza en
   `EphemeralKeyPair::diffie_hellman` con
   `HandshakeError::NonContributorySharedSecret`.
3. **No se añade una regla sin evidencia.** Es el mismo criterio que se aplicó a
   los nombres reservados de Windows en este sprint.

### Lo que debe saber quien escriba el lado Swift o Kotlin

La auditoría externa que originó esta enmienda afirma que **libsodium y CryptoKit
rechazan** las codificaciones con `u >= p`. **Ese comportamiento no se ha
verificado en este repositorio** y no se toma como hecho: aquí no hay ninguna de
las dos bibliotecas y no existe todavía un lado Swift.

Si resulta ser cierto, la consecuencia es una divergencia entre plataformas y no
un fallo de seguridad: un peer podría completar un handshake contra el lado Rust
y no contra el lado Swift. Este proyecto rechaza en todas las plataformas lo que
rechaza en una —es la regla que gobierna las rutas del manifest—, así que en ese
caso la resolución correcta es **rechazar también en Rust**, no relajar Swift.

Queda abierto como **QYR-0034** en `BUGS_PENDING.md`, con el cierre condicionado
a medir qué hace CryptoKit de verdad. Este crate nunca **emite** una codificación
no canónica: `X25519PublicKey::from(&secret)` produce siempre la forma canónica,
así que la divergencia solo puede aparecer al recibir.

### Cumplimiento

`a_non_canonical_x25519_encoding_is_accepted_and_reduced`, en
`handshake/tests.rs`. Afirma la dirección elegida: `u` y `u + p` producen el
mismo secreto compartido, un responder acepta un hello con la forma reducida, y
los dos hellos producen transcripts distintos.

## Enmienda B — sprint 4C.2 (QYR-0035): variantes de `HandshakeError` eliminadas

**Corrige una afirmación anterior de este ADR.** El apartado de errores nombraba
`UnexpectedRole`, `InvalidEphemeralPublicKey`, `TranscriptMismatch` y
`SequenceViolation` como controles del handshake. **Nada las construía**, en
ningún punto del crate, así que este documento y
`docs/security/handshake-state-machine.md` describían cuatro comprobaciones que
no existían. Un llamante podía hacer `match` sobre ellas y concluir que el
handshake imponía algo que no imponía.

Las cuatro se eliminan. Cada una tenía un motivo concreto por el que no podía
dispararse:

| Variante | Por qué era inalcanzable |
|---|---|
| `UnexpectedRole` | Imposible por construcción: cada transición consume `self`, así que el compilador rechaza reutilizar o reordenar un estado. |
| `SequenceViolation` | Lo mismo. No hay campo `step` que pueda quedar fuera de orden. |
| `InvalidEphemeralPublicKey` | Una clave pública X25519 no tiene codificación inválida: toda cadena de 32 bytes es un punto. El peligro real, un punto de orden pequeño, es `NonContributorySharedSecret`. |
| `TranscriptMismatch` | Un transcript no se compara nunca. Se firma y se autentica con MAC, así que una discrepancia aparece como `SignatureVerificationFailed` o `FinishedVerificationFailed`. |

Se mantiene el principio que `aead/error.rs` ya enunciaba: **un error que nadie
puede provocar documenta un control que no está**. `crate::guards` lo comprueba
sola: `every_handshake_error_has_a_construction_site` analiza el enum y exige que
cada variante aparezca como `HandshakeError::X` en el fuente de producción del
crate. Volver a añadir una de las cuatro, con su brazo de `Display` para que el
build siga compilando, hace fallar esa prueba por nombre.
