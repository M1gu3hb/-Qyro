# QYRO/1

Estado: framing binario implementado y probado; handshake autenticado y cifrado
autenticado de frames implementados en memoria en `qyro_crypto`. Transporte y
modo óptico no implementados: el tag ya se calcula, pero nada pone todavía un
frame en un socket.

El encoder y el decoder incremental viven en `rust/crates/qyro_protocol`, el
manifest en `rust/crates/qyro_manifest` y el sellado en
`rust/crates/qyro_crypto/src/aead`. La especificación completa está en
`docs/protocols/qyro1-wire-format.md` y `docs/protocols/manifest-format.md`;
las decisiones, en ADR-0016, ADR-0017, ADR-0018 (política de errores y estados
imposibles), ADR-0019 (nombre visible derivado) y ADR-0022 (AEAD de frames).

## Objetivos

Binario, versionado, streaming, límites explícitos, compatibilidad futura y
rechazo limpio. CBOR canónico se evaluó y se descartó frente a un formato propio
canónico y acotado; el razonamiento está en ADR-0017.

## Mensajes

Discovery, Pairing, Capabilities, Offer, Accept, Reject, Manifest, DataChunk, ChunkAck, Pause, Resume, Cancel, Error, Complete, IntegrityResult y Heartbeat.

## Cabecera conceptual

Cabecera fija de 48 bytes, big-endian, con magic, versión mayor/menor, tipo,
flags, longitud de cabecera, longitud de trailer, longitud de payload, session,
transfer, stream e item ID y secuencia. Endianness y tamaños están congelados
con tests de bytes; ver la especificación.

`session_id` son ocho bytes y su tipo es `qyro_protocol::SessionId`, el mismo que
deriva el handshake de `qyro_crypto` bajo la etiqueta `session-id`. Un único
tipo, un único ancho: nada trunca ni convierte entre establecer una sesión y
nombrarla en el cable.

## Manifest

Transfer ID, versión, fecha, emisor, conteo, bytes y por item: ruta relativa, nombre, tamaño, MIME, tipo, mtime, hash, carpeta y compresión. Rechazar rutas absolutas, .., NUL, reservados, symlinks por defecto y desbordamientos.

## Resume

Chunks adaptativos, backpressure, ACK selectivo y bitmap persistente. El ACK solo confirma datos autenticados y durables. Cierre o reconexión revalida estado.

## Óptico

Frames separados con session/transfer/epoch/symbol, parámetros FEC, payload, checksum rápido y autenticación. Duplicados y desorden son válidos; otra sesión no.

## Límites pendientes

Los límites de frame, manifest, item count y ruta están definidos y probados.
Quedan pendientes los de ventana, tiempo y memoria de transferencia, que
dependen del transporte todavía no implementado.
