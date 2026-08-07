# Auditoría del sprint 4D.1 — almacenamiento seguro, primera plataforma

- Fecha UTC: 2026-08-07
- Rama: `claude/qyro-secure-storage-4d1`
- Base: `claude/qyro-resource-bounds-4c3` en `7beb671`
- Alcance: ADR-0024, el accesor de semilla, el formato del blob,
  `qyro_identity_store`, `qyro_win_dpapi`, `qyro_store_smoke` y los hallazgos
  QYR-0048 y QYR-0050 … QYR-0061

**Este es el primer sprint desde 4A que añade función.** Una identidad
sobrevive al cierre del proceso, en **una** plataforma de tres. Los otros cuatro
sprints corrigieron cosas; este cambió lo que el programa puede hacer, y por eso
la parte que más se revisó no fue el crate nuevo sino la superficie que hubo que
abrir en el crate viejo.

## Regla de verificación

Una corrección cuya mutación no rompe nada no está cubierta y no cuenta como
hecha. Cada fila de la tabla siguiente se produjo **aplicando la mutación en este
árbol y ejecutando la suite**, no recordando lo que pasó cuando se escribió.

Dos advertencias que este proyecto ya pagó una vez:

- Una mutación que no toca el camino que la prueba recorre sale verde y no
  prueba nada (sprint 4C.3, QYR-0024).
- Una edición que no se aplica y una que sí se aplica **se ven igual desde quien
  la escribe**. `str.replace` devuelve la cadena sin cambios cuando el ancla no
  coincide, y en este sprint eso produjo un commit que no cambiaba nada seguido
  de una lectura equivocada del error anterior de CI como si fuera nuevo. Todas
  las mutaciones de abajo llevan `assert` sobre el ancla.

## Tabla de mutación

Todas ejecutadas sobre `99a0f18`, host Linux, `cargo test -p <crate>`.

| # | Propiedad | Mutación aplicada | Qué falló |
|---|---|---|---|
| 1 | La entropía liga la cabecera, no solo su longitud (QYR-0052) | `entropy_for` concatena doce ceros en vez de `cabecera[0..12]` | `the_entropy_binds_the_header_and_not_only_its_length`, `the_committed_framing_vectors_match_the_primitives` |
| 2 | Ningún camino público devuelve la semilla sin declararse (QYR-0053) | añadir `pub fn leak_raw(&self) -> [u8; SEED_LEN]` que devuelve `signing_key.to_bytes()` | `guards::every_public_path_returning_key_material_is_listed` |
| 3 | Solo los crates listados relajan `forbid(unsafe_code)` (QYR-0054) | borrar `#![forbid(unsafe_code)]` de `qyro_core` | `guards::only_the_listed_crates_may_relax_forbid_unsafe` |
| 4 | `reserved` distinto de cero se rechaza | sustituir la comparación por `if false` | `a_blob_with_a_nonzero_reserved_is_refused`, `a_single_flipped_byte_is_a_typed_error` |
| 5 | La magia se comprueba antes que nada | sustituir la comparación por `if false` | `a_blob_that_is_not_ours_is_refused_before_anything_else`, `a_single_flipped_byte_is_a_typed_error` |
| 6 | `wrapped_len` debe coincidir con lo presente | sustituir la comparación por `if true` | `a_declared_length_that_disagrees_is_refused_both_ways`, `a_truncated_blob_is_refused`, `a_single_flipped_byte_is_a_typed_error` |
| 7 | `export_secret` devuelve la semilla real | devolver `[0u8; SEED_LEN]` | `an_exported_secret_is_the_seed_the_identity_was_built_from`, `an_exported_secret_rebuilds_the_same_identity`, `every_thirty_two_byte_string_is_a_usable_seed` |
| 8 | Ningún crate del producto nombra un harness (QYR-0058) | `qyro_manifest` declara `qyro_store_smoke` como dev-dependency | `check_harness_isolation.sh`, con el nombre del harness en el mensaje |
| 9 | Los vectores son la referencia, no la salida (QYR-0025) | cambiar un byte de `magic` en `storage-v1.json` | `the_committed_framing_vectors_match_the_primitives` |
| 10 | El `unsafe` está enumerado por función | añadir `unsafe { GetLastError() }` en una función no listada | `guards::the_unsafe_blocks_are_the_ones_we_listed`, nombrando `store.rs::a_function_nobody_listed` |

La 10 corre **en Linux**, aunque el crate sea de Windows. `qyro_win_dpapi` no
lleva `#![cfg(windows)]` en la raíz sino en cada módulo, precisamente para que su
guarda —que lee los archivos como texto y no necesita Windows— se ejecute en todo
CI. Una guarda que solo corre en una plataforma está apagada en la mayoría.

### Lo que no se pudo mutar aquí

Las nueve pruebas de `qyro_win_dpapi` que llaman a DPAPI **no se ejecutan en este
host**: los módulos que las contienen son `cfg(windows)`. Su evidencia es CI, run
31215102331, job `windows-crypto`, y no hay otra. Esta auditoría no presenta
ninguna mutación sobre ellas porque no se hizo ninguna.

Lo que sí hay sobre esas pruebas es mejor que una mutación inventada: **estuvieron
rojas cuatro veces por razones reales** antes de ponerse verdes, y cada fallo está
en la tabla de runs de STATUS.md con su commit.

| Run | Commit | Por qué estuvo roja |
|---|---|---|
| 31211402008 | `5d44ec8` | `LNK2019`: `Crypt32.lib` sin enlazar. `cargo check` no enlaza |
| 31211959010 | `89022c6` | el byte 20 sobrevivió al barrido: QYR-0059 |
| 31212494494 | `dd568a4` | la prueba seguía en rojo mientras respondía si la identidad cambiaba |
| 31213769557 | `1269229` | la cota «≤16 posiciones» era falsa; eran 128 |
| 31214233989 | `ec912ef` | la aserción exacta no llegó a aplicarse al archivo |

## Caminos públicos que devuelven material de clave: antes y después

Es el cambio de superficie que el sprint tenía que revisar dos veces.

**Antes (base `7beb671`).** Cero. `identity.rs` decía literalmente «there is **no
accessor for the seed or the private key**», y esa frase *era* la protección: no
había guarda, había una costumbre. Nada comprobaba que siguiera siendo cierta.

**Después del accesor (`0ff21bd`), primera forma de la guarda:**

    const PUBLIC_KEY_MATERIAL_PATHS: [&str; 2] =
        ["identity.rs::as_bytes", "identity.rs::export_secret"];

    const KEY_MATERIAL_MARKERS: [&str; 5] = [
        "IdentitySecret", "SigningKey", "SessionKey", "StaticSecret", "SEED_LEN",
    ];

Escrita **con la lista vacía y antes del accesor**, donde pasó; añadir el accesor
la puso en rojo con exactamente esos dos caminos. Hasta ahí, correcta.

**El defecto (QYR-0053).** `KEY_MATERIAL_MARKERS` es una lista de *permitidos
disfrazada de prohibidos*. `[u8; 32]` se excluyó por una observación cierta —un
fingerprint también mide treinta y dos bytes— y una conclusión falsa: que por eso
se podía ignorar. Consecuencia medida, no supuesta: un `pub fn` que devuelve la
semilla en claro **pasaba la guarda**, porque su tipo de retorno no contenía
ninguno de los cinco nombres.

**Después (forma actual):**

    const PUBLIC_KEY_MATERIAL_PATHS: [&str; 3] = [
        "aead/mod.rs::into_zeroizing_payload",
        "identity.rs::as_bytes",
        "identity.rs::export_secret",
    ];

    const PUBLIC_NON_KEY_BYTE_PATHS: [&str; 0] = [];

    const BYTE_RETURN_MARKERS: [&str; 8] = [
        "[u8; 32]", "[u8; SEED_LEN]", "[u8; PUBLIC_KEY_LEN]", "Zeroizing",
        "IdentitySecret", "SigningKey", "SessionKey", "StaticSecret",
    ];

Ahora **todo** retorno público con forma de bytes tiene que estar en una lista o
en la otra, y estar en ninguna falla. La diferencia no es de grado: la versión
anterior fallaba abierta, esta falla cerrada.

**Lo que destapó al ampliarla:** `aead/mod.rs::into_zeroizing_payload`, que
existe desde el sprint 4C.1 y devuelve el texto claro autenticado de un frame. No
es una semilla ni una clave, y sigue siendo material sensible que sale de la
crate; llevaba dos sprints sin que ninguna guarda lo viera. Se clasifica como
material de clave, que es la opción conservadora, y queda escrito para que la
próxima persona pueda discutirlo con el dato delante.

**Lo que sigue sin cubrir (QYR-0056, abierto):** la guarda razona sobre el tipo
de retorno escrito en el fuente. Un `pub fn` que devuelva `Vec<u8>` o `String`
con bytes de clave dentro no dispara ningún marcador. La corrección barata está
identificada —congelar los sitios de origen; `signing_key.to_` aparece una sola
vez en todo el crate— y **no se hizo en este sprint**, porque estaba fuera del
alcance que el prompt fijó.

## Lo que se decidió no hacer

- **No se añadió ninguna dependencia.** `windows-sys` habría traído las dos
  declaraciones ya escritas y auditadas por más gente que este repositorio; a
  cambio, once crates entran al grafo por dos funciones. ADR-0024 §1 argumenta la
  elección y **paga su precio con la prueba 1 de la tabla de arriba**: un
  `DATA_BLOB` mal declarado no sobrevive a un protect/unprotect real, y esa es la
  mitigación que se prometió a cambio de transcribir el `extern` a mano.
- **No se escribió un MAC propio.** La cabecera queda autenticada porque va
  dentro de la entropía adicional, no porque Qyro la firme. Añadir un MAC sobre
  una capa que ya autentica es inventar criptografía.
- **No se ató el blob a la máquina.** `LOCALAPPDATA` evita que viaje con un
  perfil móvil; la MasterKey sí viaja, así que copiar el archivo a mano sigue
  funcionando. Cerrarlo exige un valor propio de la máquina y estaba fuera de
  alcance.
- **No se relajó ninguna aserción para que pasara.** QYR-0059 es el caso: el
  barrido encontró posiciones que DPAPI ignora, y la respuesta fue medir cuáles
  y comprobar que la identidad no cambia, no bajar el listón. La aserción final
  fija el conjunto **exacto** —128 mutaciones, bytes 20..36— y falla si aparece
  una nueva o desaparece una.

## Dos correcciones de mediciones propias

Merecen sección aparte porque las dos empezaron siendo afirmaciones cómodas.

1. **«Ninguna posición sobrevive»** era falso. Al descubrirlo, la primera versión
   del hallazgo dijo «byte 20, bit 0», en singular, y la aserción pasó a «como
   mucho dieciséis posiciones». Las dos venían de **una muestra de uno**: la
   prueba entraba en pánico en la primera superviviente. Al recogerlas todas
   aparecieron **128**. Una cota que se elige para encajar con lo observado no es
   una propiedad, y esa habría seguido pasando mientras escondía ocho veces más
   de lo que declaraba.
2. **«Todos los crates llevan `forbid(unsafe_code)`»** era falso: eran cinco de
   siete. Lo descubrió la guarda que se escribió para vigilarlo. `qyro_ffi` y
   `qyro_crypto_smoke` **no pueden** llevarlo —`#[unsafe(no_mangle)]` es un
   atributo unsafe en edición 2024, comprobado añadiéndolo y viendo fallar la
   compilación—; `qyro_core` sí podía y no lo llevaba.

## Estado al cerrar

- 350 pruebas en el host, 0 fallos, 2 ignoradas. Las nueve de Windows **no** están
  en esa cuenta y corren solo en CI.
- `clippy --workspace --all-targets -D warnings` y `fmt --check`: limpios.
- `cargo audit`: 0 vulnerabilidades sobre 59 crates. Tres entradas nuevas, las
  tres de primera parte. **Cero dependencias externas añadidas.**
- Superficie `unsafe`: un crate, tres funciones, enumeradas por nombre.
- Persistencia: **IMPLEMENTED en Windows, NOT_IMPLEMENTED en Android y en iOS.**
  Nada probado en hardware físico.
