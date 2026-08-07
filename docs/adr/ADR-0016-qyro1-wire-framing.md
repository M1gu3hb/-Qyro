# ADR-0016: Framing binario de QYRO/1

- Estado: aceptada
- Fecha: 2026-08-05
- Implementa: `rust/crates/qyro_protocol`
- Especificación derivada: `docs/protocols/qyro1-wire-format.md`

## Contexto

`PROTOCOL.md` describía la cabecera de forma conceptual y dejaba explícito que
«todo entero debe fijar endianness y tamaño antes de congelar vectores» y que los
límites debían definirse «antes del primer decoder». Este ADR congela esas
decisiones para que el codec pueda escribirse con vectores estables.

El decoder es la primera superficie que procesa bytes de un peer no confiable, de
modo que las decisiones se toman priorizando el rechazo limpio y la ausencia de
reservas de memoria proporcionales a datos hostiles, por encima de la compacidad.

## Decisiones

### Endianness

Big-endian (orden de red) para todos los enteros multibyte. Es la convención de
protocolos de red, se lee sin ambigüedad en un hexdump y evita discrepancias
entre plataformas little-endian. El coste en ARM/x86 es un `bswap`, irrelevante
frente al I/O.

### Cabecera de longitud fija

48 bytes, campos alineados a su tamaño natural:

| Offset | Bytes | Campo | Tipo |
|---|---|---|---|
| 0 | 4 | `magic` = `QYRO` (`0x51 0x59 0x52 0x4F`) | `[u8; 4]` |
| 4 | 1 | `version_major` | `u8` |
| 5 | 1 | `version_minor` | `u8` |
| 6 | 1 | `message_type` | `u8` |
| 7 | 1 | `flags` | `u8` |
| 8 | 2 | `header_len` | `u16` |
| 10 | 1 | `trailer_len` | `u8` |
| 11 | 1 | `reserved` (debe ser 0) | `u8` |
| 12 | 4 | `payload_len` | `u32` |
| 16 | 8 | `session_id` | `u64` |
| 24 | 8 | `transfer_id` | `u64` |
| 32 | 4 | `stream_id` | `u32` |
| 36 | 4 | `item_id` | `u32` |
| 40 | 8 | `sequence` | `u64` |

Se eligió longitud fija con campo `header_len` explícito en lugar de TLV. Una
cabecera fija se valida en una sola comprobación de longitud y no admite
cabeceras degeneradas ni bucles de parsing.

`header_len` deja la puerta abierta a crecer, pero **no saltando bytes**: ver la
enmienda al final de este documento. QYRO/1.0 acepta exactamente 48.

Ningún campo del wire usa `usize`: su tamaño depende de la plataforma y
convertiría el formato en dependiente del host.

### Identificadores

`session_id` y `transfer_id` son `u64`; `stream_id` e `item_id` son `u32`;
`sequence` es `u64`. Son opacos en esta capa: el protocolo no les asigna
significado ni asume que sean secuenciales, solo que son estables dentro de una
sesión. `item_id` a 32 bits es coherente con el límite de items del manifest
(ADR-0017), y `sequence` a 64 bits no puede desbordar en ninguna transferencia
realista.

### Trailer de autenticación

`trailer_len` lleva el tag AEAD. La regla original —«en QYRO/1.0 **debe ser 0**»—
era correcta mientras nadie calculaba tags y dejó de serlo con ADR-0022; ver la
enmienda al final.

El límite es 64 bytes, suficiente para los tags de las primitivas candidatas de
`SECURITY.md`. ChaCha20-Poly1305, que es la que se eligió, produce 16.

### Versionado

- `version_major != 1` → `UnsupportedMajorVersion`. Un major distinto puede
  cambiar el layout, así que no se intenta interpretar nada más.
- `version_minor` mayor que el soportado **se acepta**. Una versión menor puede
  añadir tipos de mensaje; **no** puede extender la cabecera. Ver la enmienda.
- `header_len` distinto de 48 se rechaza. `InvalidHeaderLength` fuera del rango
  `[48, MAX_HEADER_LEN]`, `UnsupportedHeaderExtension` dentro de él.

### Mensajes desconocidos

Un `message_type` fuera del conjunto conocido produce
`UnknownMessageType(value)`. Se prefiere el error tipado a una variante
`Unknown(u8)` silenciosa porque la capa de aplicación no debe poder confundir un
mensaje que no entiende con uno procesable.

El rechazo es **determinista y recuperable**: `payload_len` y `header_len` se
validan *antes* de resolver el tipo, así que el receptor conoce el tamaño exacto
del frame y puede descartarlo y continuar, o responder `Error`. Añadir mensajes
en el futuro es, por tanto, un incremento de versión menor.

### Flags

`u8`. Bits definidos en 1.0:

| Bit | Nombre | Significado |
|---|---|---|
| 0 | `END_OF_ITEM` | último frame del item |
| 1 | `END_OF_TRANSFER` | último frame de la transferencia |
| 2 | `ENCRYPTED` | payload protegido por la capa de contenido |
| 3 | `COMPRESSED` | payload comprimido |

Los bits 4–7 están reservados y **deben ser 0**; en caso contrario,
`InvalidFlags`. Se rechaza en vez de ignorar porque un flag que el receptor no
entiende puede cambiar el significado del payload, y procesarlo como si no
estuviera es precisamente el fallo que causa vulnerabilidades de interpretación
divergente.

### Límites

Constantes públicas y documentadas:

| Constante | Valor | Motivo |
|---|---|---|
| `MAX_HEADER_LEN` | 1024 | margen de crecimiento acotado |
| `MAX_PAYLOAD_LEN` | 1 MiB | chunk máximo; acota la memoria por frame |
| `MAX_TRAILER_LEN` | 64 | tags AEAD |
| `MAX_FRAME_LEN` | suma de los tres | cota de un frame completo |
| `MAX_BUFFER_LEN` | `MAX_FRAME_LEN` | cota del decoder incremental |

### Estrategia de streaming y de memoria

El decoder acumula en un buffer propio acotado por `MAX_BUFFER_LEN` y expone
`push` / `next_frame`, porque **una lectura de red no equivale a un frame**:
puede traer medio header, varios frames, o un frame partido en cualquier byte.

La regla que gobierna el diseño: `payload_len` se valida contra
`MAX_PAYLOAD_LEN` en cuanto los 48 bytes de cabecera están disponibles, **antes**
de reservar nada. Un `payload_len` de `0xFFFFFFFF` produce `PayloadTooLarge` sin
que el proceso reserve 4 GiB ni espere indefinidamente más datos. `push` rechaza
con `BufferLimitExceeded` en lugar de crecer sin límite.

Es decir: la memoria que un peer puede hacer reservar está acotada por constantes
de compilación, no por lo que ese peer declare.

## Alternativas descartadas

- **TLV para toda la cabecera**: más flexible, pero obliga a un bucle de parsing
  sobre datos no confiables y admite representaciones múltiples del mismo frame,
  lo que complica la canonicidad y el futuro cálculo de MAC.
- **Longitudes varint**: ahorran pocos bytes frente a un chunk de 1 MiB y añaden
  una ruta de decodificación con casos límite propios.
- **Little-endian**: coincide con las plataformas objetivo, pero rompe la
  convención de red y dificulta la lectura de trazas.

## Consecuencias

- Los valores numéricos de `MessageType` y el layout quedan congelados y con
  tests que los fijan; cambiarlos exige un major nuevo.
- El crate no añade ninguna dependencia externa, así que la superficie de parsing
  auditable es solo código propio.
- `MAX_PAYLOAD_LEN` de 1 MiB fija el techo del chunk. Si el hito de transporte
  demuestra que conviene otro tamaño, cambiarlo es una constante y un ajuste de
  vectores, no un cambio de formato.

## Enmienda (ADR-0018, sprint 2; registrada aquí en el sprint 4C.1)

Dos reglas de arriba dejaron de ser ciertas y este documento siguió afirmándolas
durante cuatro sprints. Una especificación canónica que contradice al código no
es una especificación: es una segunda fuente de verdad, y la peor de las dos.

### `header_len` distinto de 48 se rechaza

La versión original decía que un peer antiguo «salta los bytes que no entiende»
y que `header_len` mayor que 48 «se acepta hasta `MAX_HEADER_LEN`».

No puede hacerse. Saltar bytes que no se conservan rompe la reserialización
byte-exacta, y la reserialización byte-exacta es la precondición de usar la
cabecera completa como datos asociados del AEAD (ADR-0022): un byte de cabecera
que no sobrevive al round trip es un byte sobre el que el tag no dice nada.

QYRO/1.0 acepta exactamente 48 bytes de cabecera y responde
`UnsupportedHeaderExtension` a cualquier otro valor dentro del rango. Las
extensiones llegarán en una versión que sepa preservarlas, serializarlas y
meterlas en los datos asociados.

### `trailer_len` de 16 es válido, y solo con `ENCRYPTED`

La versión original exigía trailer cero en QYRO/1.0, con un argumento correcto en
su momento: aceptar un trailer que nadie verifica es aceptar bytes sin
autenticar. Desde ADR-0022 alguien lo verifica.

La regla vigente:

- sin `ENCRYPTED`, `trailer_len` debe ser 0;
- con `ENCRYPTED`, debe ser distinto de 0 y no mayor que `MAX_TRAILER_LEN`, y el
  AEAD rechaza cualquier valor que no sea exactamente 16 con `InvalidTagLength`;
- `ENCRYPTED` sin trailer es `EncryptedWithoutTrailer`, porque es un frame que
  afirma una protección que no lleva.

Nada de esto cambia el layout. La cabecera de 48 bytes de la tabla de arriba es
la misma que se congeló en el sprint 2.

## Enmienda — sprint 4C.3 (QYR-0024): la cota de coste, no solo la de memoria

**Corrige una omisión de este ADR.** El apartado de límites acota lo que un peer
puede hacer *reservar* y no dice nada de lo que puede hacer *trabajar*. Con esa
cota sola, el decoder era cuadrático y cumplía el ADR.

`FrameDecoder::next_frame` reclamaba cada frame entregado con
`drain(..total)`, que memmovea todo lo que queda detrás. Llenar el búfer de
frames mínimos y drenarlo cuesta Θ(n²/48): 21 868 heartbeats, 1 049 664 bytes
empujados, **11 476 501 344 bytes movidos**. Es tráfico perfectamente válido —
ningún frame mal formado, ningún error, nada que un limitador basado en validez
pueda ver—, y el peer elige el tamaño del frame, así que elige el cuadrado.

### La propiedad, en una frase comprobable

> Entre que un byte entra al búfer del decoder y sale de él como parte de un
> frame, puede copiarse **un número acotado de veces**, independiente de cuántos
> bytes haya en el búfer. La constante es de una copia por byte de media, y el
> presupuesto que las pruebas afirman es de dos.

Es comprobable porque se cuenta, no se cronometra: `bytes_moved` está
instrumentado bajo `cfg(test)` en el único sitio que mueve bytes. Un reloj de
pared en un runner compartido mide el runner, y no dice qué se rompió.

### Cómo se sostiene

Un frame entregado solo avanza un cursor de lectura. El espacio se reclama en
`compact`, y solo en dos casos: cuando los bytes entrantes no cabrían bajo el
techo, o cuando al menos la mitad del búfer está ya consumida. La segunda
condición es la amortización — una compactación mueve como mucho la mitad del
búfer y no puede repetirse hasta que se haya consumido otro tanto — y la primera
es lo que mantiene la cota de memoria que este ADR ya tenía.

La reserva se dobla y se recorta al techo. Doblar es lo que hace un `push`
amortizado O(1); recortar es lo que impide que la capacidad llegue a 2 097 152
frente a un `MAX_BUFFER_LEN` de 1 049 664, que es lo que hacía (QYR-0027).
`reserve_exact` a secas habría sido peor que el defecto original: con pushes de
un byte reasigna en cada byte.

### Medidas

| Forma | Antes | Después |
|---|---|---|
| Llenar `MAX_BUFFER_LEN` de frames mínimos y drenar | 11 476 501 344 bytes movidos | 0 |
| Backlog de 4 096 frames con 50 000 llegadas | ~9,8 GB (50 000 × 196 608) | 2 359 296 sobre 2 596 608 empujados |

El cero del primer caso no es una optimización perfecta: es que esa forma no
necesita compactar en ningún momento. Por eso la segunda existe, y es la que
corre la compactación de verdad.

### Lo que no cambia

Ni un byte del cable. El layout de 48 bytes, los límites declarados, el
envenenamiento estructural y `reset` como única salida son los mismos. Ninguna
prueba de contrato existente necesitó edición, que es la evidencia de que esto
es coste y no comportamiento.
