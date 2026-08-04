# QYRO/1

Estado: contrato de versión implementado; framing no implementado.

## Objetivos

Binario, versionado, streaming, límites explícitos, compatibilidad futura y rechazo limpio. CBOR canónico se evaluará; no está elegido definitivamente.

## Mensajes

Discovery, Pairing, Capabilities, Offer, Accept, Reject, Manifest, DataChunk, ChunkAck, Pause, Resume, Cancel, Error, Complete, IntegrityResult y Heartbeat.

## Cabecera conceptual

Magic, versión, tipo, flags, session ID, transfer ID, stream ID, item ID, secuencia/chunk, longitud y autenticación. Todo entero debe fijar endianess y tamaño antes de congelar vectores.

## Manifest

Transfer ID, versión, fecha, emisor, conteo, bytes y por item: ruta relativa, nombre, tamaño, MIME, tipo, mtime, hash, carpeta y compresión. Rechazar rutas absolutas, .., NUL, reservados, symlinks por defecto y desbordamientos.

## Resume

Chunks adaptativos, backpressure, ACK selectivo y bitmap persistente. El ACK solo confirma datos autenticados y durables. Cierre o reconexión revalida estado.

## Óptico

Frames separados con session/transfer/epoch/symbol, parámetros FEC, payload, checksum rápido y autenticación. Duplicados y desorden son válidos; otra sesión no.

## Límites pendientes

Antes del primer decoder deben definirse tamaño máximo de frame, manifest, item count, ruta, ventanas, tiempo y memoria; cada límite requiere tests y corpus de fuzzing.
