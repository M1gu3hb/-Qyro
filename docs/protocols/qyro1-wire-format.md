# Formato de wire QYRO/1

Especificación derivada de `docs/adr/ADR-0016-qyro1-wire-framing.md`.
Implementación: `rust/crates/qyro_protocol`. Contratos: `tests/wire_contract.rs`.

Todos los enteros son **big-endian**. Ningún campo del wire usa `usize`.

## Frame

    +------------------+-------------------+---------------------+
    | header (48 B)    | payload (0..1MiB) | trailer (0 B en 1.0)|
    +------------------+-------------------+---------------------+

## Cabecera

| Offset | Bytes | Campo | Tipo | Notas |
|---|---|---|---|---|
| 0 | 4 | `magic` | `[u8;4]` | `QYRO` = `0x51 0x59 0x52 0x4F` |
| 4 | 1 | `version_major` | `u8` | 1. Distinto → rechazo |
| 5 | 1 | `version_minor` | `u8` | 0. Mayor se acepta |
| 6 | 1 | `message_type` | `u8` | ver tabla; 0 reservado |
| 7 | 1 | `flags` | `u8` | bits 4–7 deben ser 0 |
| 8 | 2 | `header_len` | `u16` | **debe ser exactamente 48** |
| 10 | 1 | `trailer_len` | `u8` | 0 sin `ENCRYPTED`; el tamaño exacto del tag con él |
| 11 | 1 | `reserved` | `u8` | debe ser 0 |
| 12 | 4 | `payload_len` | `u32` | ≤ 1 MiB |
| 16 | 8 | `session_id` | `u64` | opaco |
| 24 | 8 | `transfer_id` | `u64` | opaco |
| 32 | 4 | `stream_id` | `u32` | opaco |
| 36 | 4 | `item_id` | `u32` | opaco |
| 40 | 8 | `sequence` | `u64` | monotónico por stream |

## Tipos de mensaje

Valores congelados; cambiarlos exige un major nuevo.

| Valor | Mensaje | Valor | Mensaje |
|---|---|---|---|
| 1 | Hello | 10 | ChunkAck |
| 2 | Capabilities | 11 | Pause |
| 3 | Pairing | 12 | Resume |
| 4 | TransferOffer | 13 | Cancel |
| 5 | TransferAccept | 14 | Complete |
| 6 | TransferReject | 15 | IntegrityResult |
| 7 | Manifest | 16 | Error |
| 8 | ItemStart | 17 | Heartbeat |
| 9 | DataChunk | | |

El valor 0 está reservado, así que un buffer a ceros nunca decodifica como
mensaje legítimo.

## Flags

| Bit | Nombre |
|---|---|
| 0 | `END_OF_ITEM` |
| 1 | `END_OF_TRANSFER` |
| 2 | `ENCRYPTED` |
| 3 | `COMPRESSED` |

Los bits 4–7 están reservados y deben ser 0. Un flag desconocido se **rechaza**,
no se ignora: puede cambiar cómo debe leerse el payload.

Los flags se dividen en dos grupos (ADR-0018):

- **De transporte** (bits 0–1): ajustables desde la API pública.
- **Protegidos** (bits 2–3): no ajustables. `COMPRESSED` se rechaza con
  `UnsupportedFlag` hasta que exista compresión. `ENCRYPTED` solo lo activa el
  sellado, que produce el tag en la misma operación; declararlo sin trailer da
  `EncryptedWithoutTrailer`.

## Límites

| Constante | Valor |
|---|---|
| `HEADER_LEN` | 48 |
| `MAX_HEADER_LEN` | 1024 |
| `MAX_PAYLOAD_LEN` | 1 MiB |
| `MAX_TRAILER_LEN` | 64 |
| `MAX_FRAME_LEN` | 1024 + 1 MiB + 64 |
| `MAX_BUFFER_LEN` | `MAX_FRAME_LEN` |

## Orden de validación

Es la propiedad de seguridad central del crate. `FrameHeader::decode` valida en
este orden, **antes** de que el llamante sepa cuántos bytes esperar y antes de
reservar nada:

1. longitud mínima disponible para la cabecera fija;
2. `magic`;
3. `version_major`;
4. `header_len` dentro de rango;
5. `trailer_len` = 0;
6. byte reservado = 0;
7. `payload_len` ≤ `MAX_PAYLOAD_LEN`;
8. `flags`;
9. `message_type`;
10. tamaño total ≤ `MAX_FRAME_LEN`.

Un `payload_len` de `0xFFFFFFFF` se rechaza en el paso 7 y nunca se convierte en
una reserva de 4 GiB.

## Decoder incremental

`FrameDecoder` acumula bytes bajo un techo duro y entrega frames completos.
Acepta header partido, payload partido, varios frames en un buffer y bytes
sobrantes.

Tras un error de framing el decoder queda **envenenado**: el stream perdió la
sincronización y no hay forma de distinguir cabecera de payload, así que devuelve
el mismo error tipado hasta `reset()` en lugar de adivinar.

## Errores estructurales frente a eventos semánticos

ADR-0018 separa dos categorías, y el decoder las trata distinto.

**Estructurales** — envenenan el stream, solo `reset()` explícito recupera:
`InvalidMagic`, `UnsupportedMajorVersion`, `InvalidHeaderLength`,
`UnsupportedHeaderExtension`, `PayloadTooLarge`, `FrameTooLarge`,
`InvalidFlags`, `UnsupportedFlag`, `EncryptedWithoutTrailer`,
`AuthenticationTrailerInvalid`, `BufferLimitExceeded`.

**Semánticos delimitados** — mantienen la sincronización: un `message_type`
desconocido consume su frame completo y se devuelve como
`DecodedFrame::Unsupported`, con el valor del tipo, la longitud del payload y el
tamaño total. **No expone el payload**: bytes cuyo significado se desconoce no se
convierten en algo procesable.

## Compatibilidad futura

- Una versión menor puede añadir tipos de mensaje. **No** puede extender la
  cabecera: QYRO/1.0 rechaza `header_len != 48` con
  `UnsupportedHeaderExtension`, porque saltar bytes que no conserva rompería la
  reserialización byte-exacta y dejaría al AEAD sin poder autenticarlos.
- Las extensiones llegarán en una versión que las preserve, las serialice y las
  incluya en los datos asociados.
- El trailer de autenticación se habilita con `EncryptedEnvelope`, que no puede
  construirse sin aportar un tag. Su plantilla es un `&Frame`, no un
  `&FrameHeader`: un `Frame` no puede construirse alrededor de una cabecera
  protegida, así que el tipo del parámetro *es* la prueba de que la plantilla
  está en claro. Sin eso, un sobre podía envolverse a sí mismo por segunda vez.
- `Frame::from_parts` rechaza con `ProtectedHeaderNotPlain` cualquier cabecera
  con un flag protegido o un trailer declarado. Un `Frame` solo guarda payload:
  codificar esa cabecera emitiría menos bytes de los que ella misma promete y
  dejaría al decodificador del par esperando un trailer que nunca llega.

## Reserialización byte-exacta

Para todo frame aceptado, `decode → encode` produce exactamente los mismos
bytes. Es precondición de usar la cabecera completa como datos asociados del
AEAD: si un byte de cabecera no se conservara, el tag se calcularía sobre algo
distinto de lo que viajó.
