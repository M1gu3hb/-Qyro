# ADR-0018: Errores estructurales frente a eventos semánticos

- Estado: aceptada
- Fecha: 2026-08-05
- Enmienda: ADR-0016
- Implementa: `rust/crates/qyro_protocol`

## Contexto

ADR-0016 afirmaba que un `message_type` desconocido es **recuperable**, porque
las longitudes se validan antes de resolver el tipo y el receptor conoce el
tamaño exacto del frame.

El código no cumplía esa promesa. `FrameDecoder::next_frame` envenenaba el stream
ante **cualquier** error de `FrameHeader::decode`, incluido
`UnknownMessageType`. En la práctica, un peer que hablase una versión menor con
un mensaje nuevo mataba la conexión en lugar de recibir un `Error`. La
documentación describía compatibilidad futura que la implementación no tenía.

También había dos promesas huecas más:

- `header_len > 48` se aceptaba y los bytes de extensión se **descartaban**, así
  que `decode → encode` no era byte-exacto y una futura capa AEAD no podría
  autenticar lo que nunca se conservó.
- `ENCRYPTED` y `COMPRESSED` podían activarse con `with_flags` aunque no
  existiera ni cifrado ni compresión, permitiendo construir un frame que miente
  sobre su propio contenido.

## Decisión

Se separan explícitamente dos categorías, y el decoder las trata distinto.

### Fallos estructurales — envenenan el stream

Se ha perdido la confianza en el framing: ya no se puede distinguir cabecera de
payload, así que resincronizar sería adivinar.

- `InvalidMagic`
- `UnsupportedMajorVersion`
- `InvalidHeaderLength`
- `UnsupportedHeaderExtension`
- `PayloadTooLarge`
- `FrameTooLarge`
- `InvalidFlags` (bit reservado o byte reservado distinto de cero)
- `UnsupportedFlag`
- `EncryptedWithoutTrailer`
- `AuthenticationTrailerInvalid`
- `BufferLimitExceeded`

El decoder queda envenenado y **solo** `reset()` explícito lo recupera. Empujar
más bytes no limpia el fallo.

### Eventos semánticos delimitados — mantienen la sincronización

El framing es válido y el tamaño total es conocido. El frame se consume entero y
se devuelve un valor tipado.

- `message_type` desconocido en un frame con longitudes válidas.

`next_frame` pasa a devolver `Option<DecodedFrame>`:

```rust
pub enum DecodedFrame {
    Message(Frame),
    Unsupported(UnsupportedFrame),
}
```

`UnsupportedFrame` expone el valor numérico del tipo, la longitud del payload y
el tamaño total, lo suficiente para responder `Error` sin volver a parsear. **No
expone el payload como algo procesable**: bytes que no se saben interpretar no se
convierten en un mensaje.

La capa superior decide: responder `Error`, ignorar el frame o cerrar la sesión.

## Extensiones de cabecera en QYRO/1.0

Se elige la **opción recomendada**: `header_len` debe ser exactamente 48.
Cualquier otro valor dentro de rango produce `UnsupportedHeaderExtension`.

Motivo: la compatibilidad que se anunciaba era ficticia. Saltar bytes que no se
conservan rompe tres cosas a la vez — la reserialización deja de ser byte-exacta,
el AEAD no puede autenticar lo descartado, y el receptor acepta datos cuyo
significado desconoce. Es peor que rechazar.

Las extensiones se introducirán en una versión que las preserve, las serialice,
las incluya en los datos asociados y tenga límites y tests. Hasta entonces, `1.0`
declara que no las soporta y lo demuestra rechazándolas.

`MAX_HEADER_LEN` se conserva como cota superior de validación: un `header_len`
absurdo se distingue de uno meramente extendido, y el error lo refleja.

## Estados imposibles

`FrameHeader` pasa a tener campos privados con constructores validados. No existe
un camino público que produzca una cabecera que el propio decoder rechazaría.

Los flags se dividen:

- **De transporte**, públicamente ajustables: `END_OF_ITEM`, `END_OF_TRANSFER`.
- **Protegidos**, no ajustables desde la API pública: `ENCRYPTED`, `COMPRESSED`.

`Frame::with_flags` devuelve `Result` y rechaza los protegidos.

- `COMPRESSED` se rechaza en el decoder con `UnsupportedFlag` hasta que exista
  compresión real.
- `ENCRYPTED` solo es válido acompañado de un trailer del tamaño exacto del tag.
  Un frame que declara `ENCRYPTED` sin trailer produce `EncryptedWithoutTrailer`;
  un trailer sin `ENCRYPTED` produce `AuthenticationTrailerInvalid`.

El único camino que puede activar `ENCRYPTED` y fijar `trailer_len` es el sellado
en `qyro_crypto`, que cifra el payload y calcula el tag en la misma operación. Ni
la UI ni el transporte pueden fabricar «un frame cifrado» a mano.

## Consecuencias

- Un peer con una versión menor más nueva ya no mata la conexión: recibe una
  respuesta y la sesión continúa.
- `decode → encode` es byte-exacto para **todo** frame aceptado, lo que es
  precondición de que la cabecera sirva como datos asociados del AEAD.
- La superficie pública ya no permite construir frames inválidos.
- Se rompe compatibilidad con lo que ADR-0016 anunciaba sobre extensiones. El
  formato no se ha publicado y `MAX_PAYLOAD_LEN`, el layout y los valores de
  mensaje no cambian, así que el impacto real es nulo.
