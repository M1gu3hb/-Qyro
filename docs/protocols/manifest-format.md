# Formato del manifest

Especificación derivada de `docs/adr/ADR-0017-manifest-serialization.md`.
Implementación: `rust/crates/qyro_manifest`. Contratos:
`tests/manifest_contract.rs`.

Codificación binaria propia, canónica y acotada. Todos los enteros son
**big-endian**.

## Cabecera

| Offset | Bytes | Campo | Tipo |
|---|---|---|---|
| 0 | 4 | `magic` = `QYRM` | `[u8;4]` |
| 4 | 2 | `version` = 2 | `u16` |
| 6 | 8 | `transfer_id` | `u64` |
| 14 | 8 | `created_unix_seconds` | `i64` |
| 22 | 8 | `total_bytes` | `u64` |
| 30 | 4 | `item_count` | `u32` |

A partir del offset 34 siguen `item_count` items consecutivos.

## Item

En orden fijo, sin claves ni etiquetas:

1. `item_id` (`u32`)
2. `kind` (`u8`): 1 = File, 2 = Directory
3. `path`: `u32` de longitud + bytes UTF-8
4. `size` (`u64`)
5. `mime_type`: byte de presencia (0/1); si 1, `u32` + bytes
6. `modified_unix_seconds`: byte de presencia (0/1); si 1, `i64`
7. `hash_algorithm` (`u8`): 0 = None, 1 = SHA-256, 2 = BLAKE3
8. `digest`: exactamente los bytes que exige el algoritmo (0 o 32)
9. `compression` (`u8`): 0 = None

El nombre visible **no viaja**. Se deriva de la ruta (ADR-0019), así que no puede
discrepar de dónde caerán los bytes: `factura.pdf.exe` no puede presentarse como
`factura.pdf`.

Invariantes por tipo:

- **File**: digest obligatorio, incluso para 0 bytes. `MissingFileHash` si falta.
- **Directory**: `size = 0`, algoritmo `None`, digest vacío.

## Reglas de canonicidad

Un manifest lógico tiene exactamente una representación en bytes, así que puede
autenticarse sin normalización previa.

- Campos en orden fijo y tamaño explícito.
- Los bytes de presencia solo admiten `0` o `1`; cualquier otro valor es error.
- Los items van ordenados por su ruta normalizada. Un manifest desordenado se
  **rechaza**, no se reordena: reordenar cambiaría los bytes firmados.
- Rutas duplicadas e identificadores duplicados son error.
- Sobrar bytes al final es error.

## Rutas relativas

`RelativePath` es el único camino por el que bytes de un peer se convierten en
algo con forma de ruta. **Rechaza en lugar de sanear**: reescribir una ruta
hostil suele producir otra ruta hostil. Las reglas de Unix y Windows se aplican
en todas las plataformas, así que un manifest se acepta o se rechaza igual en
cualquier receptor.

Se rechaza:

| Caso | Ejemplo | Motivo |
|---|---|---|
| Travesía | `../evil` | sale del directorio de destino |
| Segmento `.` | `./a` | dos grafías del mismo lugar |
| Segmento vacío | `a//b`, `a/` | separador duplicado o final |
| Absoluta Unix | `/etc/passwd` | destino fuera de control |
| Prefijo de unidad | `C:/Windows` | ruta absoluta en Windows |
| UNC | `//server/share` | destino remoto |
| Barra invertida | `a\b` | separador en Windows, nombre válido en Unix |
| NUL | `a\0b` | trunca la ruta en APIs estilo C |
| Carácter de control | `a\nb` | nombres ilegibles o engañosos |
| Nombre reservado | `CON`, `COM1.txt` | Windows los resuelve como dispositivos |
| Punto/espacio final | `evil.`, `evil ` | Windows los elimina y provoca colisiones |
| Longitud excesiva | >1024 total, >255 por segmento | |
| Anidamiento excesivo | >64 segmentos | |
| UTF-8 inválido | | |
| Carácter ilegal en Windows | `a<b`, `a:b`, `a?b` | `< > : " \| ? *` no son válidos en NTFS |
| DEL | `a\u{7F}b` | carácter de control |

## Colisiones portables

Dos rutas distintas en Linux pueden ser el mismo archivo en Windows o macOS.
`PortableCollisionKey` las detecta y el manifest **rechaza** el par en lugar de
aceptar ambos y sobrescribir uno en silencio tras aceptar la transferencia.

Se pliegan: mayúsculas/minúsculas ASCII y marcas combinantes sobre Latin-1, de
modo que `Foto.jpg`/`foto.jpg`, `A/B.txt`/`a/b.TXT` y las grafías NFC/NFD de
`mañana.txt` colisionan. La escritura original nunca se altera.

Límite conocido: dos rutas que difieran solo por una marca combinante fuera del
rango Latin-1 se consideran distintas. Se documenta en
`docs/security/parser-threats.md` en lugar de ocultarse.

Se acepta Unicode, emoji, espacios internos y nombres que solo *parecen*
reservados (`CONsole.txt`, `COM10.txt`).

## Límites

| Constante | Valor |
|---|---|
| `MAX_ITEMS` | 100 000 |
| `MAX_TOTAL_BYTES` | 1 TiB |
| `MAX_PATH_LEN` | 1024 |
| `MAX_SEGMENT_LEN` | 255 |
| `MAX_PATH_SEGMENTS` | 64 |
| `MAX_NAME_LEN` | 255 |
| `MAX_MIME_LEN` | 128 |
| `MAX_HASH_LEN` | 64 |
| `MAX_ENCODED_LEN` | 8 MiB |

`codec::encoded_len` calcula el tamaño exacto con aritmética `checked` **antes**
de reservar nada, así que el límite se aplica por adelantado y no a mitad de un
buffer que ya lo superó. Una prueba fija que coincide con `encode().len()`.

El conteo de items se valida contra `MAX_ITEMS` antes de reservar el vector, y
después contra los bytes que quedan: un conteo que los bytes restantes no podrían
satisfacer se rechaza antes de `Vec::with_capacity`. La suma de tamaños usa
aritmética `checked`, así que items diseñados para desbordar `u64` producen un
error en lugar de un total pequeño y creíble.

## Fuera de alcance

El crate no toca el sistema de archivos: no abre, no consulta y no crea nada.
Produce un valor validado y deja la escritura a una capa que reciba un
directorio raíz.
