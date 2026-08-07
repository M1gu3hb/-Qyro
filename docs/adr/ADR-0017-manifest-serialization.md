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

## Enmienda — sprint 4C.2 (QYR-0028): colisión ancestro/descendiente

**Corrige una afirmación anterior de este ADR.** El apartado de invariantes de
colección decía que `PortableCollisionKey` impide que dos rutas se materialicen
sobre el mismo archivo. Solo impedía una de las dos formas.

La clave pliega la ruta completa y une los segmentos con NUL, y `validate_items`
comparaba esas cadenas por **igualdad**. La igualdad detecta `a/b` contra `A/B`.
No detecta que `a` y `a/b` son el mismo nombre a distinta profundidad, porque
`"a"` y `"a\0b"` son sencillamente dos cadenas distintas.

Un receptor que materialice ese manifest tiene que crear `a` como archivo y `a`
como directorio. Lo que haga en segundo lugar falla o sustituye a lo primero,
después de haber aceptado la transferencia y sin forma de informar de qué
elemento se perdió.

### Formulación exacta

Tras el orden canónico de las claves plegadas:

> Una clave `K1` que es prefijo propio de la siguiente clave `K2` **en una
> frontera NUL** —es decir, `K2 = K1 || 0x00 || resto`— es un ancestro de `K2`.
> Si el elemento al que pertenece `K1` es de tipo `File`, el manifest se
> rechaza con `ManifestError::FileIsAlsoADirectory`.

Tres detalles que la regla necesita, y por qué:

- **La frontera NUL es la regla, no un detalle.** `"report"` es prefijo de
  `"reports\0page.txt"` como cadena y ancestro de nada como ruta. Una prueba de
  prefijo en crudo rechazaría dos archivos sin relación.
- **La adyacencia basta.** NUL es el byte más bajo y ninguna ruta válida puede
  contenerlo, así que todo descendiente de `a` ordena inmediatamente después de
  `a` y antes de cualquier otra clave que empiece por `a`. Una pasada por
  `windows(2)` sobre las claves ordenadas ve todos los pares posibles.
- **Solo un ancestro `File` es conflicto.** Un item `Directory` con hijos es la
  forma normal de un árbol. Rechazarlo convertiría todo manifest con carpetas en
  un error, que es el modo de fallo por exceso que este crate ya cometió una vez
  con la tabla de plegado de diacríticos.

### Cumplimiento

`rust/crates/qyro_manifest/tests/ancestor_collision_contract.rs`. Incluye los
dos casos que impiden una corrección demasiado agresiva —un directorio con hijos,
y `a` junto a `ab`— y un caso construido byte a byte para el decoder, porque la
API de construcción rechaza el manifest antes de poder codificarlo.
