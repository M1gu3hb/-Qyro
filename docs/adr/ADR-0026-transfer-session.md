# ADR-0026 — `TransferSession`: mover una transferencia completa

- Estado: **congelada** antes de escribir código del motor.
- Fecha: 2026-08-08
- Sprint: 5A
- Usa, sin modificar: ADR-0016 (framing), ADR-0017/0019 (manifest), ADR-0021
  (handshake), ADR-0022 (AEAD de frames).

## Contexto

Todas las piezas existen y ninguna se ha usado con otra. Este es el primer
sprint que las conecta, y por eso lo que decide no es criptografía nueva —no hay
ninguna— sino **quién habla cuándo, con qué cuerpo, y qué pasa cuando no**.

Alcance estricto: dos extremos en el mismo proceso, intercambiando `Vec<u8>`.
**No hay sockets y no hay disco.** La fuente de bytes es memoria y el destino es
un búfer. Que eso sea poco es intencionado: si la máquina de estados está mal, es
mucho más barato descubrirlo sin un socket delante.

## 1. Qué mensaje lleva qué

Big-endian, como todo lo demás. Los tipos ya existen en `MessageType` y esta ADR
sólo define el **cuerpo** de los que 5A usa. Los que no aparecen aquí —`Hello`,
`Capabilities`, `Pairing`, `TransferReject`, `Error`, `Heartbeat`— siguen sin
cuerpo definido y **el motor los rechaza por tipo**, que es distinto de
ignorarlos.

### `TransferOffer` (4)

    offset  bytes  campo          valor
    0       4      item_count     u32, número de elementos del manifest
    4       8      total_bytes    u64, suma de tamaños
    12      4      chunk_size     u32, el que el emisor va a usar
    16      4      window_chunks  u32, la ventana que el emisor promete respetar

El emisor **declara** su tamaño de chunk y su ventana; no los negocia. El
receptor los acepta o rechaza enteros. Negociar es una máquina de estados más y
no compra nada mientras haya una sola implementación.

### `TransferAccept` (5)

    offset  bytes  campo          valor
    0       4      window_chunks  u32, la ventana que el receptor concede

**El receptor puede conceder menos que lo pedido, nunca más.** Un valor mayor que
el ofrecido es un error tipado, no un valor que se recorta en silencio: recortar
en silencio es cómo dos extremos acaban creyendo cosas distintas sobre el mismo
número.

### `Manifest` (7)

    offset  bytes  campo          valor
    0       N      encoded        el manifest canónico de ADR-0017/0019

Sin envolver. El manifest ya tiene su propia codificación y su propia validación,
y volver a envolverla sería un segundo formato que puede discrepar del primero.

### `ItemStart` (8)

    offset  bytes  campo          valor
    0       4      item_id        u32, el de la entrada del manifest
    4       8      item_bytes     u64, tamaño declarado

`item_bytes` está en el manifest y se repite aquí a propósito: el receptor
comprueba que coinciden. Un `ItemStart` que declara otro tamaño es un error
tipado antes de que llegue un solo byte de contenido.

### `DataChunk` (9)

    offset  bytes  campo          valor
    0       4      item_id        u32
    4       4      chunk_index    u32, base 0, dentro del elemento
    8       N      content        los bytes

`item_id` viaja en cada chunk aunque `ItemStart` ya lo dijo. Cuesta cuatro bytes
por chunk y elimina toda una clase de fallo: un chunk que llega mientras el
receptor cree estar en otro elemento se rechaza por lo que dice, no por lo que se
supone del orden.

### `ChunkAck` (10)

    offset  bytes  campo          valor
    0       4      item_id        u32
    4       4      through_index  u32, acumulativo: todo hasta aquí, inclusive

**Acumulativo, no selectivo.** Ver §3.

### `Pause` (11), `Resume` (12), `Cancel` (13)

    offset  bytes  campo          valor
    0       1      reason         u8, 0 = petición del usuario

Cuerpo mínimo con un byte reservado para razón. Un mensaje de control sin cuerpo
no se puede extender sin cambiar su longitud, y una longitud que cambia es un
cambio de formato.

### `Complete` (14)

    offset  bytes  campo          valor
    0       8      total_bytes    u64, lo que el emisor cree haber mandado

### `IntegrityResult` (15)

    offset  bytes  campo          valor
    0       4      item_count     u32
    4       4×N    item_ids       u32 por elemento, en orden de manifest
    …       1×N    verdicts       u8 por elemento: 0 = ok, 1 = digest distinto,
                                  2 = tamaño distinto, 3 = incompleto

**Un veredicto por elemento, no uno global.** «Falló algo» no le sirve a nadie
que tenga que decidir qué reintentar.

## 2. Tamaño de chunk: 64 KiB

`MAX_PAYLOAD_LEN` es 1 MiB y es el techo del **payload sellado**, no del
contenido. Un `DataChunk` de contenido `C` ocupa `8 + C` de payload claro, y el
sellado añade el tag de Poly1305. El techo real para `C` es
`MAX_PAYLOAD_LEN - 8 - TAG_LEN`.

Se fija **65 536 bytes**, con margen de sobra, y por tres razones:

1. **La memoria en vuelo es `ventana × chunk`** (§6). Con 64 KiB y ventana 16 son
   1 MiB por dirección, que es una cota que cabe en un teléfono sin pensarlo.
2. Un chunk cerca del techo haría que la cota de memoria dependiera del techo del
   protocolo, y entonces subir `MAX_PAYLOAD_LEN` cambiaría el consumo del motor
   sin que nadie lo pidiera.
3. 64 KiB es suficientemente grande para que el coste por frame —cabecera,
   sellado, ACK— sea ruido frente al contenido.

**No es una decisión medida.** No hay red, así que no hay nada contra qué medir
un tamaño de chunk; medirlo pertenece al sprint que tenga transporte. Lo que esta
ADR fija es una cota segura y la razón, no un óptimo.

## 3. ACK acumulativo, ventana de 16 chunks

**Acumulativo.** `through_index` significa «tengo todo hasta aquí». Es la opción
que hace imposible el estado que más duele: con ACK selectivo, emisor y receptor
mantienen dos conjuntos que pueden divergir, y reconciliarlos es un protocolo
propio. Con acumulativo hay un solo número por elemento en cada lado.

Lo que cuesta: un chunk perdido bloquea el avance de la ventana hasta que se
retransmite. Con una ventana de 16 y sin red, es un coste teórico; con red, es la
decisión que 5B o el sprint de LAN tendrá que revisar **con una medición
delante**. Queda dicho aquí para que se revise a propósito y no por sorpresa.

**Cuándo se manda:** el receptor emite `ChunkAck` cuando ha entregado
`window/2` chunks contiguos desde el último ACK, o cuando recibe un chunk que no
es el que esperaba —lo que le dice al emisor enseguida que hay un hueco—, o al
cerrar un elemento. No hay temporizador: **no hay reloj en este sprint** y un
temporizador sin transporte mediría el planificador de pruebas.

**Ventana: 16 chunks.** Con 64 KiB son 1 MiB en vuelo por dirección. El número
sale de la cota de memoria que se quiere, no al revés, y esa es la relación
correcta: se elige cuánta memoria puede consumir un extremo y de ahí sale la
ventana.

## 4. Qué pasa cuando algo va mal

| Situación | Qué hace el motor |
|---|---|
| El chunk no autentica | `FrameOpener` ya falla. El motor **no lo reintenta ni lo cuenta**: un frame que no autentica no tiene remitente conocido, así que no puede probar nada sobre el estado. La sesión se envenena. |
| Chunk fuera de orden | Se descarta el contenido y se manda `ChunkAck` con el último índice contiguo. No se buferiza fuera de orden: buferizar es una cota de memoria más que gobernar, y con ACK acumulativo no compra nada. |
| ACK de algo que no se mandó | `AckAheadOfSender` y la sesión se envenena. Un receptor que confirma lo que no existe no es un receptor con retraso: es otro programa. |
| `Complete` antes de tiempo | `CompleteBeforeAllItems`, tipado. El receptor **no** cierra ni verifica: verificar lo incompleto produciría un `IntegrityResult` que parece un veredicto. |
| `Cancel` con datos en vuelo | Los dos lados pasan a `Cancelled`, que es terminal. Los chunks en vuelo que lleguen después se descartan por estado, no por contenido. |
| Chunk de un elemento ya cerrado | Rechazado por `item_id`, no por índice. |
| Mensaje sin cuerpo definido en 5A | `UnexpectedMessage { got }`, por tipo. |

**Envenenar** es lo que hace `FrameSealer` desde el sprint 4C.1 y aquí significa
lo mismo: tras un error la sesión no vuelve a aceptar nada. Un motor que se
recupera de un estado que no entiende es un motor que sigue en un estado que no
entiende.

## 5. Dos numeraciones, y por qué

Hay dos contadores y **no** se pueden unificar:

- La **secuencia del frame** la asigna `FrameSealer`, va en la cabecera, entra en
  el nonce y el llamante **no la puede elegir** (ADR-0022). Cuenta *frames de esa
  dirección*, incluidos ACK, control y todo lo demás.
- El **`chunk_index`** cuenta chunks *dentro de un elemento*, empieza en 0 en
  cada elemento y lo elige el motor.

Unificarlos exigiría que el motor eligiera la secuencia del frame, que es
exactamente lo que ADR-0022 prohíbe para que un nonce no se repita nunca. Que
sean dos números es la consecuencia de esa decisión, no un descuido.

Corolario que el motor **debe** respetar: la ventana de replay de 1024 es del
opener y cuenta secuencias de frame. Una retransmisión de un chunk es un **frame
nuevo** con una secuencia nueva sellada de nuevo — nunca el reenvío de los mismos
bytes sellados, que el opener rechazaría como replay, correctamente.

## 6. El límite de memoria, y quién lo impone

**El emisor** lo impone, porque es el único que puede: no manda el chunk
`n + window` hasta tener ACK de `n`.

- En vuelo por dirección: **`window × chunk_size` = 16 × 64 KiB = 1 MiB**.
- El receptor no buferiza fuera de orden (§4), así que su consumo es un chunk
  más el estado del elemento en curso.
- El decoder ya tiene su propia cota, `MAX_BUFFER_LEN` (ADR-0016), y sigue siendo
  suya. Esta ADR no la toca.

**Lo que esto significa para un archivo grande**: el emisor toma su contenido de
un `&[u8]` o de un iterador y sólo sostiene la ventana. El contenido completo
nunca tiene por qué estar en RAM **en el motor**; que el llamante de 5A le pase
un slice entero es una propiedad del llamante de pruebas, no del motor, y la
prueba de §10.8 mide lo que el motor sostiene, con un contador instrumentado bajo
`cfg(test)` como el `bytes_moved` del decoder.

## Alternativas descartadas

- **ACK selectivo.** §3.
- **Negociar el tamaño de chunk.** Una máquina de estados más sin beneficio con
  una sola implementación.
- **Buferizar fuera de orden en el receptor.** Otra cota de memoria que gobernar,
  y con ACK acumulativo no aporta.
- **Reutilizar la secuencia del frame como índice de chunk.** §5; rompería el
  nonce.
- **Reenviar los mismos bytes sellados al retransmitir.** El opener lo rechazaría
  como replay, y tendría razón.
- **Un `IntegrityResult` global.** No sirve para decidir qué reintentar.

## No objetivos

Sockets, red, descubrimiento, filesystem, temporales, reanudación entre sesiones,
UI, FFI, SQLite, emparejamiento, y cualquier medición de rendimiento: sin
transporte no hay nada que medir.

## Lo que esta decisión no promete

- **No mueve archivos.** Mueve búferes entre dos valores del mismo proceso.
- **No sobrevive al cierre del proceso.** La pausa y la reanudación de 5A son
  dentro de una sesión viva.
- **No está medida.** El tamaño de chunk y la ventana son cotas argumentadas, no
  óptimos observados, y no habrá con qué medirlos hasta que exista transporte.
