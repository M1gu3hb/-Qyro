# QYRO/1

**Estado: el protocolo está en el cable.** Un archivo elegido con el selector del
sistema se trocea en frames QYRO/1, cada frame se sella con ChaCha20-Poly1305
bajo una clave derivada de un handshake autenticado, cruza un socket TCP, se
escribe en el destino con nombre validado y se verifica con SHA-256 antes de
entregarse. Los cuatro canales —red local, cable directo sin router, óptico y
serie— usan el mismo framing.

> La frase que estuvo aquí hasta 2026-08-31 decía, del transporte y del modo
> óptico, que el tag ya se calcula pero
> «nada pone todavía un frame en un socket».
> Dejó de ser cierta en la fase 12. Se deja escrita para que no vuelva:
> **este documento se verifica contra el código.**
>
> **Y lo que sigue siendo cierto:** nada de esto ha cruzado nunca una red de
> verdad. Está probado entre dos procesos y en CI. Ver
> `docs/testing/hardware-protocol.md`, veintiséis huecos en blanco.

## Dónde vive cada pieza

| Pieza | Crate |
|---|---|
| Encoder y decoder incremental | `rust/crates/qyro_protocol` |
| Manifiesto y validación de rutas | `rust/crates/qyro_manifest` |
| Sellado AEAD de frames | `rust/crates/qyro_crypto/src/aead` |
| Handshake e identidad | `rust/crates/qyro_crypto` |
| **Socket TCP, listener y stream de frames** | `rust/crates/qyro_net` |
| **Sesión de transferencia** | `rust/crates/qyro_transfer` |
| **Materialización en disco** | `rust/crates/qyro_fs` |
| Fachada de los dos consumidores | `rust/crates/qyro_session` |
| Canal óptico: código fuente y ojo | `rust/crates/qyro_fountain`, `rust/crates/qyro_eye` |
| Canal serie | `rust/crates/qyro_serial` |

La especificación completa está en `docs/protocols/qyro1-wire-format.md` y
`docs/protocols/manifest-format.md`; las decisiones, en ADR-0016, ADR-0017,
ADR-0018 (política de errores y estados imposibles), ADR-0019 (nombre visible
derivado), ADR-0022 (AEAD de frames), ADR-0028 (transporte) y ADR-0041 (primer
contacto: puerto, IP y quién escucha).

## Objetivos

Binario, versionado, streaming, límites explícitos, compatibilidad futura y
rechazo limpio. CBOR canónico se evaluó y se descartó frente a un formato propio
canónico y acotado; el razonamiento está en ADR-0017.

## Mensajes

Discovery, Pairing, Capabilities, Offer, Accept, Reject, Manifest, DataChunk,
ChunkAck, Pause, Resume, Cancel, Error, Complete, IntegrityResult y Heartbeat.

**`Cancel` cruza el cable de verdad** desde la fase 25: `Session::cancel()` sólo
levantaba una bandera local y el par se enteraba al vencer su reloj de 60 s.

## Cabecera

Cabecera fija de 48 bytes, big-endian, con magic, versión mayor/menor, tipo,
flags, longitud de cabecera, longitud de trailer, longitud de payload, session,
transfer, stream e item ID y secuencia. Endianness y tamaños están congelados con
tests de bytes; ver la especificación.

`session_id` son ocho bytes y su tipo es `qyro_protocol::SessionId`, el mismo que
deriva el handshake de `qyro_crypto` bajo la etiqueta `session-id`. Un único
tipo, un único ancho: nada trunca ni convierte entre establecer una sesión y
nombrarla en el cable.

## Manifest

Transfer ID, versión, fecha, emisor, conteo, bytes y por item: ruta relativa,
nombre, tamaño, MIME, tipo, mtime, hash, carpeta y compresión. Rechaza rutas
absolutas, `..`, NUL, nombres reservados, symlinks por defecto y desbordamientos.

`ItemKind::Directory` **se emite**: las carpetas vacías viajan. Llevaba años
validado en el cable y nadie lo ponía.

## Primer contacto

El emparejamiento es una cadena, `QYRO1|<ip:puerto>|<huella de 32 hex>`, y **es
también lo que codifica el QR**: no hay un segundo formato, escanear es leer
esto (ADR-0035 §2).

La huella de la cadena es una **expectativa, no una credencial**. Escanear o
teclear un código no establece confianza por sí solo: fija qué huella tiene que
salir del handshake, y si la autenticada no coincide la sesión se rechaza **sin
preguntar a nadie**. Quien escaneó ya contestó la pregunta.

> **Esto fue cierto en un consumidor de dos hasta 2026-08-31.** La terminal lo
> comprobaba (QYR-0381); el teléfono sacaba la dirección del código y tiraba la
> huella, así que escanear ataba la sesión a una dirección y a ninguna clave —y
> el teléfono es el que tiene cámara—. La frontera C no exponía esa mitad; ahora
> sí (QYR-0392, ADR-0032 enmienda 7). Se deja escrito porque una propiedad de
> seguridad que sólo la mitad del producto cumple **no se ve leyendo la
> especificación**: se ve preguntándole a cada consumidor.

El puerto es fijo, **49517**, del rango Dynamic/Private de IANA, y la razón es el
cortafuegos de Windows: el permiso se concede una vez por programa y puerto, así
que un puerto efímero devolvería el diálogo en cada sesión (ADR-0041 §3). Si está
ocupado **se dice y se ofrece elegir otro; nunca se mueve solo**, porque un
puerto que se mueve pierde el permiso y la predicción de la cadena sin avisar.

**Sólo el receptor escucha.** El emisor únicamente conecta hacia afuera, así que
un solo lado necesita permiso de cortafuegos, y es el lado donde la persona está
mirando.

## Resume

Chunks adaptativos, backpressure, ACK selectivo y bitmap persistente. El ACK sólo
confirma datos autenticados y durables. Cierre o reconexión revalida estado.

## Óptico

Frames separados con session/transfer/epoch/symbol, parámetros FEC, payload,
checksum rápido y autenticación. Duplicados y desorden son válidos; otra sesión
no. La dirección está fijada (ADR-0044 §6): **la terminal dibuja y el teléfono
lee**, porque la máquina que necesita este canal es la que no tiene cámara.

## Límites

Los límites de frame, manifest, item count y ruta están definidos y probados.
El de archivos por transferencia son **256** y la razón son los descriptores, no
el gusto: en Android el selector devuelve descriptores, no rutas, así que una
selección de unos miles no es una transferencia lenta sino un proceso agotado
(ADR-0047 §3). Se rechaza **antes de abrir nada**.

> **Y la cuenta debajo estuvo mal hasta 2026-08-31.** El techo suponía un
> descriptor por archivo y eran dos, así que 256 archivos rozaban las 512
> aperturas del CRT de Windows. Medido: 200 archivos abrían **402** de más;
> ahora abren **11**, y no crecen con el número de archivos (QYR-0391,
> ADR-0047 enmienda 1). El techo sigue siendo 256.

Quedan pendientes los de ventana y tiempo de transferencia.
