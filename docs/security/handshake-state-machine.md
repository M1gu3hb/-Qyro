# Máquina de estados del handshake

Implementación: `rust/crates/qyro_crypto/src/handshake`. Decisión: ADR-0021.

## Los estados

```
Iniciador
  InitiatorStart
    │  send_hello()                       → InitiatorHello (100 B)
    ▼
  InitiatorAwaitResponder
    │  receive_responder_hello(bytes)     → InitiatorFinish (99 B)
    ▼
  InitiatorAwaitResponderFinish
    │  receive_responder_finish(bytes)
    ▼
  EstablishedInitiator

Respondedor
  ResponderStart
    │  receive_initiator_hello_from_system(bytes)  → ResponderHello (164 B)
    ▼
  ResponderAwaitInitiatorFinish
    │  receive_initiator_finish(bytes)
    ▼
  ResponderFinishPending          ← tiene la sesión, no puede usarla
    │  encoded_finish()                   → ResponderFinish (35 B)
    │  confirm_sent()
    ▼
  EstablishedResponder
```

## Los estados se consumen

Cada transición toma `self` por valor. No es cosmético: un handshake con estado
mutable y un campo `step` permite procesar dos veces el mismo mensaje, o
procesarlos en otro orden, y esos son precisamente los errores que un revisor no
ve leyendo el camino feliz. Aquí el compilador los rechaza.

Ningún estado implementa `Clone` ni `Copy`.

## `ResponderFinishPending`

El estado que este documento existe para explicar.

`receive_initiator_finish` devolvía antes **los bytes y una sesión establecida a
la vez**. En ese instante el respondedor ha autenticado al iniciador y ha
derivado todas las claves, pero el peer **no ha visto** el mensaje que cierra el
handshake y puede no verlo nunca: el proceso puede morir, el socket puede
cerrarse, el paquete puede perderse.

Todo lo demás en la API dice que una sesión establecida es una sesión que se
puede usar. Un respondedor que empieza a usar claves ahí está hablando con
alguien que todavía no cree que el handshake terminó.

El estado intermedio ofrece exactamente una cosa: los bytes que hay que enviar.
**Ninguna clave es alcanzable desde él**, a propósito: el punto del estado es que
los secretos existen y siguen fuera de alcance.

`confirm_sent()` consume `self`, así que una sesión se establece una vez y una
segunda confirmación no se puede escribir.

### Cuándo llamarlo

Solo cuando el transporte informe de que los bytes se entregaron de verdad.
**Todavía no hay transporte**, así que nada puede llamarlo de verdad: las
pruebas confirman la entrega a mano, y esa es exactamente la costura que ocupará
un transporte.

Descartar el estado sin confirmar destruye los secretos, que es el resultado
correcto para un handshake cuyo último mensaje nunca se entregó.

## Orden de las comprobaciones

El orden importa y está fijado por los tests.

**Iniciador, al recibir `ResponderHello`:**

1. longitud, versión, suite y tipo;
2. identidad del peer —malformada, o de orden bajo, se rechaza aquí—;
3. **firma sobre el transcript**;
4. intercambio X25519, que rechaza un secreto no contributorio;
5. firma propia, transcript autenticado, derivación, MAC.

El intercambio va **después** de verificar la firma. Un intercambio hecho antes
produciría un secreto derivado de una clave no autenticada, y algo acabaría
tentado de usarlo.

**Respondedor, al recibir `InitiatorHello`:** el hello del iniciador no lleva
firma, así que la comprobación de contributoriedad es la primera puerta
criptográfica.

## Errores

Cada variante nombra una regla distinta, ninguna revela material de clave, y
**algo construye cada una**. Una variante que nada produce documenta un control
que no existe.

`UnsupportedHandshakeVersion`, `UnsupportedCryptoSuite`, `UnexpectedMessage`,
`InvalidState`, `InvalidMessageLength`, `InvalidPublicIdentity`,
`WeakPublicIdentity`, `NonContributorySharedSecret`,
`SignatureVerificationFailed`, `FinishedVerificationFailed`,
`EntropyUnavailable`, `KeyDerivationFailed`, `TrailingBytes`.

**Corregido en el sprint 4C.2 (QYR-0035).** Esta lista incluía
`UnexpectedRole`, `InvalidEphemeralPublicKey`, `TranscriptMismatch` y
`SequenceViolation`. Ninguna se construía en ninguna parte, así que este
documento afirmaba cuatro controles que el código no tenía. Fueron eliminadas.

Cada una no podía dispararse por una razón concreta. La confusión de rol y el
desorden de mensajes son imposibles por construcción: cada transición consume
`self`, de modo que el compilador rechaza reutilizar o reordenar un estado. Una
clave pública X25519 no tiene codificación inválida —toda cadena de 32 bytes es
un punto— y el peligro real, un punto de orden pequeño, se reporta como
`NonContributorySharedSecret`. Un transcript nunca se compara: se firma y se
autentica con MAC, así que una discrepancia aparece como
`SignatureVerificationFailed` o `FinishedVerificationFailed`.

`crate::guards` lo mantiene cierto: una variante sin sitio de construcción en
todo el crate hace fallar la suite. Registrado como enmienda a ADR-0021.

`SignatureVerificationFailed` y `FinishedVerificationFailed` son distintas a
propósito: la primera dice que el peer no probó su identidad; la segunda, que la
probó pero derivó claves distintas. Son fallos con causas diferentes.

Firmar con un dominio no disponible responde `InvalidState`, no
`SignatureVerificationFailed`: ese fallo es **local**, y reportarlo como un fallo
de verificación culparía al peer de un defecto propio. Ocurrió exactamente eso
mientras el dominio `HandshakeTranscript` seguía reservado, y convirtió una
corrección de una línea en una búsqueda por el transcript.
