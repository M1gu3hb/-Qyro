# ADR-0017: Codificación canónica del manifest

- Estado: aceptada
- Fecha: 2026-08-05
- Implementa: `rust/crates/qyro_manifest`
- Especificación derivada: `docs/protocols/manifest-format.md`

## Contexto

El manifest describe qué se va a transferir: rutas, tamaños, tipos y hashes. Lo
produce un peer y lo consume el otro **antes** de escribir nada en disco, así que
su decoder es, junto al framing, la superficie de ataque principal.

Dos exigencias condicionan el formato:

1. **Canonicidad.** El manifest se autenticará y se incluirá en el cálculo de
   integridad final. Si un mismo manifest lógico admite varias representaciones
   en bytes, la firma deja de ser inequívoca y aparecen ataques de
   interpretación divergente entre emisor y receptor.
2. **Límites antes de reservar.** Igual que en ADR-0016, un conteo o una longitud
   declarados por el peer no pueden traducirse en una reserva proporcional.

## Alternativas evaluadas

### CBOR canónico (por ejemplo `ciborium`)

- A favor: estándar (RFC 8949), tiene perfil canónico definido, interoperable.
- En contra: arrastra `serde` y su maquinaria de derive; el perfil canónico es
  una convención que hay que verificar, no algo que el tipo garantice; los mapas
  admiten claves repetidas o desordenadas que cada implementación trata distinto;
  acotar la reserva por longitud declarada exige envolver el decoder.
- Licencia: Apache-2.0/MIT, aceptable.

### `postcard`

- A favor: compacto, `no_std`, pensado para embebidos, determinista para un
  esquema dado.
- En contra: también depende de `serde`; su formato está definido por el esquema
  Rust, así que la compatibilidad entre versiones depende de no tocar el orden de
  los campos de las structs, algo que ningún test evidente protege; los varint
  `usize` complican fijar vectores estables entre plataformas.
- Licencia: Apache-2.0/MIT, aceptable.

### Formato binario propio

- A favor: canónico por construcción (campos en orden fijo, longitudes
  explícitas, sin claves ni opcionales representables de dos maneras); cada
  reserva se puede acotar exactamente donde se lee la longitud; reutiliza las
  primitivas big-endian de `qyro_protocol`, así que el proyecto tiene **un solo**
  estilo de parsing que auditar; mantiene el workspace sin dependencias externas.
- En contra: hay que escribir y probar el decoder, y hoy no interopera con
  herramientas de terceros.

## Decisión

Se elige el **formato binario propio, canónico y acotado**.

El argumento decisivo es la canonicidad por construcción unida al control de
reservas. Con CBOR o postcard, la canonicidad y los límites se consiguen
*envolviendo* una librería de propósito general; el envoltorio pasa a ser el
componente crítico y hay que auditarlo igualmente, además de la librería. Con un
formato propio, la propiedad la da el layout: no existe forma de codificar el
mismo manifest de dos maneras, así que no hace falta un paso de normalización
antes de firmar.

Que el workspace siga sin dependencias externas no es el motivo principal, pero
es un beneficio real: reduce la superficie que `cargo audit` debe vigilar y evita
introducir código de parsing de terceros en la ruta que procesa datos hostiles.

Se acepta explícitamente el coste: no hay interoperabilidad con herramientas
externas hoy. Qyro habla solo con Qyro, así que ese coste no se paga todavía. Si
en el futuro hiciera falta exportar manifests a terceros, se añadiría una
conversión de salida, sin tocar el formato autenticado.

## Reglas de canonicidad

- Enteros big-endian y de tamaño fijo, como en ADR-0016.
- Campos en orden fijo; no hay claves ni etiquetas.
- Los opcionales se codifican con un byte de presencia (`0x00`/`0x01`); cualquier
  otro valor es un error. Un campo ausente no puede representarse además como
  presente-y-vacío.
- Las cadenas son UTF-8 con longitud `u32` previa y se validan al decodificar.
- Los items se ordenan por su ruta normalizada; un manifest desordenado o con
  rutas duplicadas se rechaza en lugar de reordenarse en silencio, porque
  reordenar cambiaría los bytes que se firmaron.
- No hay relleno ni bytes reservados libres: sobrar bytes al final es un error.

## Límites

| Constante | Valor | Motivo |
|---|---|---|
| `MAX_ITEMS` | 100 000 | acota el conteo antes de reservar el vector |
| `MAX_TOTAL_BYTES` | 1 TiB | tamaño declarado de la transferencia |
| `MAX_PATH_LEN` | 1024 | ruta relativa completa |
| `MAX_SEGMENT_LEN` | 255 | coincide con el límite habitual de los sistemas de archivos |
| `MAX_PATH_SEGMENTS` | 64 | profundidad de anidamiento |
| `MAX_NAME_LEN` | 255 | nombre visible |
| `MAX_MIME_LEN` | 128 | tipo MIME |
| `MAX_HASH_LEN` | 64 | digest más largo previsto (SHA-512) |
| `MAX_ENCODED_LEN` | 8 MiB | manifest serializado completo |

El conteo de items se valida contra `MAX_ITEMS` **antes** de reservar el vector,
y la suma de tamaños usa aritmética `checked` para que un desbordamiento sea un
error y no un total pequeño y falso.

## Consecuencias

- El manifest tiene una representación única, lista para autenticar sin
  normalización previa.
- `qyro_manifest` no añade dependencias; `cargo audit` sigue sin superficie
  externa que vigilar en la ruta de parsing.
- Cambiar el orden o el tipo de un campo es un cambio de formato y exige subir
  la versión del manifest, protegida por tests de bytes congelados.
