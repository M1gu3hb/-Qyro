# ADR-0029 — Identificadores autenticados de la cabecera

- Estado: aceptada
- Fecha: 2026-08-11
- Alcance: QYRO/1, `qyro_protocol` y el contrato de consumo tras el AEAD
- Hallazgos: QYR-0068, QYR-0102 y QYR-0103

## Contexto

La cabecera fija reserva `transfer_id`, `stream_id` e `item_id` en los offsets
24–39. ADR-0022 hace de los 48 bytes completos los datos asociados del AEAD,
pero ADR-0016 sólo los llama opacos y estables: no decide cómo los rellena un
emisor ni qué significa recibir valores desconocidos.

QYR-0068 registró que no había API pública para rellenarlos. La auditoría previa
a esta decisión encontró que esa premisa no coincide con el código base:
`Frame::with_identifiers` y `FrameHeader::with_identifiers` son públicas desde
`cc38554`, pruebas de integración externas las llaman y
`qyro_crypto_smoke` construye frames con valores no cero. QYR-0102 registra esa
contradicción. El hueco real no es una función ausente, sino una superficie
pública sin decisión canónica y evidencia específica.

## Decisión

### API pública mínima

Se congelan las dos vías aditivas que ya existen:

```rust
Frame::with_identifiers(
    self,
    session_id: SessionId,
    transfer_id: u64,
    stream_id: u32,
    item_id: u32,
) -> Frame

FrameHeader::with_identifiers(
    self,
    session_id: SessionId,
    transfer_id: u64,
    stream_id: u32,
    item_id: u32,
) -> FrameHeader
```

`Frame` cubre el constructor ordinario; `FrameHeader` cubre el constructor de
bajo nivel que luego pasa por `Frame::from_parts`. Añadir un tercer constructor,
setters por campo o un `FrameIdentifiers` duplicaría una capacidad pública que
ya está desplegada. Retirar una de las dos sería un cambio incompatible, no una
reducción aditiva. El sealer sigue sustituyendo `session_id` y `sequence` por
los de la sesión criptográfica; conserva los otros tres campos.

### Valores válidos y el cero

Los rangos completos de `u64`/`u32`, incluido cero, son válidos en la capa de
framing. Cero significa «sin ámbito asignado por la capa superior», no
«transferencia número cero». Esta elección conserva los frames existentes que
`Frame::new` construye con ceros y permite mensajes previos al establecimiento
de una transferencia.

Que un valor sea representable no obliga a un receptor a aceptarlo en su estado
actual. Tras autenticar el frame, la capa que enruta transferencias exige un
`transfer_id` activo y, para mensajes ligados a un item, un `item_id` presente
en el manifest autenticado de esa transferencia.

### Qué autentica el AEAD

Los tres identificadores están en los 48 bytes que el sealer entrega como AAD.
Alterar un bit de `transfer_id`, `stream_id` o `item_id` en vuelo hace fallar el
tag. Eso garantiza que el valor recibido es el valor que autenticó el emisor;
no garantiza que sea verdadero, único, autorizado, conocido por el receptor ni
coherente con un manifest.

### Rechazo por el receptor

El chequeo ocurre después de `FrameOpener::open`, nunca sobre metadata todavía
no autenticada. La capa receptora debe responder con errores tipados de routing,
equivalentes a `UnexpectedTransferId { found }` y
`UnknownItemId { transfer_id, item_id }`. No son `Io`, no se convierten en un
fallo de framing y no se adivina otra transferencia o item. La implementación
concreta corresponde a la capa de red/transferencia que consume el frame y no
se añade a `qyro_protocol`, que no conoce el conjunto de transfers activos ni
el manifest. Tampoco se reutiliza `FrameError::InvalidIdentifier`: esa variante
no tiene ningún sitio de construcción, y ADR-0029 confirma que framing acepta
todos los valores. QYR-0103 conserva ese problema público para resolverlo con
su compatibilidad explícita, no como efecto secundario de routing.

### El formato no cambia

La cabecera sigue midiendo exactamente 48 bytes. No cambia ningún offset, ancho,
endianness, valor de tipo ni regla de decodificación. Esta decisión congela la
semántica y la API de campos que ya estaban en el wire; no crea una versión
nueva del formato.

## Alternativas descartadas

- Hacer que cero sea inválido: rompería todos los frames no asignados que la
  API pública ya construye y mezclaría framing con estado de routing.
- Añadir `FrameIdentifiers`: agruparía mejor los tres campos, pero dejaría dos
  formas públicas equivalentes o exigiría retirar una API existente.
- Setters independientes: permiten estados parciales sin aportar capacidad.
- Eliminar ahora el `item_id` duplicado del cuerpo de `DataChunk`: pertenece a
  `qyro_transfer`, que esta fase sólo puede leer, y exige coordinar una revisión
  del contrato de ADR-0026.

## Evidencia exigida

- `identifiers_survive_a_seal_and_open_round_trip` fija que los tres valores del
  llamante sobreviven al wire y al AEAD.
- `altering_an_identifier_in_flight_breaks_the_tag` altera específicamente
  `transfer_id` y exige `AuthenticationFailed`.
- `the_forty_eight_byte_layout_is_unchanged` compara contra un vector literal
  de 48 bytes, no contra una longitud calculada por el mismo código.

## Lo que esta decisión no promete

- No asigna identificadores ni garantiza unicidad global.
- No convierte los identificadores en autorización o identidad del peer.
- No implementa sockets, multiplexación ni el routing receptor.
- No elimina la duplicación de `item_id` en el cuerpo de `DataChunk`.
- No cambia la protección de replay, que sigue gobernada por `sequence`.
- No aporta evidencia de hardware físico.

## Enmienda 2026-08-11 — retirar el rechazo de framing inalcanzable

Fase 9 instaló en `qyro_protocol` la misma guarda de sitios de construcción que
ya protegía otros errores. Falló exactamente con
`FrameError::InvalidIdentifier`: ningún constructor ni byte puede producirlo.

Se retiran esa variante y su tipo auxiliar `IdentifierField`. La decisión es
compatible con el contrato semántico de esta ADR por tres razones:

1. `FrameError` es `#[non_exhaustive]`; un consumidor ya debe conservar una rama
   comodín y no puede hacer un `match` exhaustivo que dependa de la variante.
2. Construir manualmente el error no demuestra que framing lo produzca y no es
   una capacidad que deba conservarse como promesa falsa.
3. El rango completo, incluido cero, sigue siendo válido en framing. El rechazo
   de IDs desconocidos ocurre después del AEAD en routing y necesita estado que
   `qyro_protocol` deliberadamente no tiene.

No se reutiliza la variante para hacer real algo en la capa equivocada y no se
la exime de la guarda: ambas salidas perpetuarían una API que afirma un control
inexistente. El wire, los offsets, los builders y los errores tipados futuros de
routing permanecen sin cambios.
