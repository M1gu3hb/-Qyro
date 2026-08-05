# Amenazas sobre los parsers

Cubre `qyro_protocol` y `qyro_manifest`, las dos primeras superficies que
procesan bytes de un peer no confiable. Complementa `THREAT_MODEL.md`.

## Por qué estos dos crates

Antes de que exista transporte, cifrado o escritura en disco, algo tiene que
convertir bytes ajenos en estructuras. Si esa conversión puede agotar memoria,
entrar en pánico o producir una ruta que escape del directorio de destino, nada
de lo que venga después importa.

Ambos crates son **sin dependencias externas**, así que toda la lógica de parsing
es auditable en este repositorio y `cargo audit` no tiene superficie de terceros
que vigilar en esa ruta.

## Amenazas y controles

| Amenaza | Control | Prueba |
|---|---|---|
| Longitud hostil agota memoria | `payload_len` se valida contra `MAX_PAYLOAD_LEN` antes de reservar | `hostile_payload_length_is_rejected_without_a_proportional_reservation` |
| Conteo hostil de items | `MAX_ITEMS`, y después contra los bytes restantes antes de `with_capacity` | `a_hostile_item_count_is_rejected_without_reserving_for_it`, `an_item_count_larger_than_the_remaining_bytes_is_rejected_early` |
| Buffer sin límite | `FrameDecoder` acota con `MAX_BUFFER_LEN` y no crece al rechazar | `buffer_limit_is_enforced_and_leaves_the_buffer_untouched` |
| Espera indefinida | Las longitudes se validan antes de decidir cuántos bytes faltan | `payload_truncated_at_every_byte_waits_instead_of_yielding` |
| Desincronización de framing | El decoder se envenena y no adivina | `a_framing_error_poisons_the_stream_until_reset` |
| Path traversal | `RelativePath` rechaza `..`, `.`, absolutas, prefijos de unidad, UNC | `traversal_is_rejected`, `a_manifest_carrying_a_hostile_path_cannot_be_decoded` |
| Divergencia entre plataformas | Reglas Unix y Windows aplicadas en todas | `backslash_paths_are_rejected_as_ambiguous`, `windows_reserved_device_names_are_rejected` |
| Truncamiento por NUL | Se rechaza cualquier NUL en la ruta | `nul_bytes_are_rejected` |
| Colisión por normalización de Windows | Punto o espacio final rechazados | `trailing_dot_or_space_is_rejected` |
| Desbordamiento de totales | Suma con aritmética `checked` | `summing_item_sizes_cannot_wrap_into_a_believable_total` |
| Interpretación divergente | Flags reservados y bytes de presencia no canónicos se rechazan | `unknown_flag_bits_are_rejected_rather_than_ignored`, `a_non_canonical_option_tag_is_rejected` |
| Contrabando de datos | Bytes sobrantes son error | `trailing_bytes_are_rejected` |
| Pánico por entrada arbitraria | Property tests y corpus | `arbitrary_bytes_never_panic*` |
| Nombre visible engañoso | el nombre se deriva de la ruta; no viaja aparte | `an_executable_cannot_be_presented_as_a_document` |
| Archivo sin integridad final | digest obligatorio en el constructor | `every_file_needs_a_final_digest_including_an_empty_one` |
| Nombre irrealizable en Windows | caracteres ilegales rechazados en todas las plataformas | `windows_illegal_characters_are_rejected_on_every_platform` |
| Sobrescritura por plegado del FS | `PortableCollisionKey` | `case_only_differences_collide_portably`, `composed_and_decomposed_unicode_collide` |
| Reserva antes de validar el tamaño | `encoded_len` con aritmética checked | `encoded_len_matches_the_bytes_actually_produced` |
| Desincronización por mensaje nuevo | evento delimitado en vez de veneno | `an_unknown_message_type_does_not_desynchronise_the_stream` |
| Frame que miente sobre su protección | `ENCRYPTED` solo por sellado, con tag | `a_plain_frame_cannot_claim_to_be_encrypted` |
| Extensión de cabecera no autenticable | rechazo explícito en 1.0 | `qyro1_rejects_a_header_extension_it_cannot_preserve` |

## Estado del fuzzing

Existen seis targets `cargo-fuzz` en `rust/fuzz/fuzz_targets`:

- `frame_decoder`: bytes arbitrarios, con el primer byte eligiendo el tamaño de
  fragmento, de modo que la partición también se fuzzea.
- `encrypted_envelope`: lo que salga como sobre cifrado debe volver a codificarse
  en los bytes de los que se decodificó, porque la cabecera completa son los
  datos asociados y una cabecera que no sobrevive al round trip autentica algo
  distinto de lo que viajó.
- `frame_opener`: muta un frame sellado real bajo una sesión fija y comprueba que
  solo sale texto claro de frames que autentican, y que un fallo de autenticación
  no mueve la ventana de replay.
- `replay_window`: recorre transiciones arbitrarias y sostiene los dos
  invariantes: `check` no muta, y una secuencia aceptada no vuelve a aceptarse.
- `manifest_decoder`: cualquier manifest aceptado debe llevar solo rutas seguras
  y volver a codificarse en los mismos bytes.
- `relative_path`: el parser no puede entrar en pánico ni reescribir su entrada.

`rust/fuzz` es un workspace aparte porque `libfuzzer-sys` exige nightly, mientras
el proyecto compila en stable 1.88.0.

### Los targets no compilaban

Hasta el sprint 4C.1, **ninguno de los tres targets originales podía construirse**
y el recetario de esta sección no podía funcionar. Dos causas encadenadas:

1. `rust/fuzz/Cargo.toml` decía «excluded from the main workspace» y nada lo
   excluía: el manifest raíz ni lo listaba ni lo excluía, y el paquete no
   declaraba `[workspace]` propio. Cargo respondía «current package believes it's
   in a workspace when it's not» y no llegaba a compilar nada.
2. Detrás de eso, `frame_decoder` seguía usando `frame.header().payload_len` como
   campo público y una `next_frame()` que devolvía un frame en lugar de un
   `DecodedFrame`. La API cambió en el sprint 2 y el target se quedó atrás.

Nada lo detectó porque lo único que CI ejecutaba sobre estos archivos era
`rustfmt --check`, que no necesita tipos para pasar. La frase «el corpus smoke
reproduce las mismas aserciones que hacen los targets» era cierta solo porque los
smoke tests las reimplementaban.

La sesión determinista que `frame_opener` necesita vive en
`qyro_crypto::fuzzing`, que existe **solo bajo `--cfg fuzzing`**. No es una
feature: las features de Cargo son aditivas y cualquier crate del grafo puede
encenderlas para todos, así que una feature pública `test-vectors` estaría a una
línea de meter un constructor determinista en un build de release. `--cfg
fuzzing` lo pone cargo-fuzz en la línea de órdenes para una compilación.

CI sigue ejecutando además un *corpus smoke* en cada run: los 94 archivos de
`rust/fuzz/corpus` se reproducen contra las mismas aserciones, en el caso del
framing a cuatro tamaños de fragmento distintos. Es una defensa contra
regresiones sobre entradas ya conocidas, y es lo que corre en stable en cada
commit; la campaña de fuzzing corre aparte y semanalmente.

Desde el sprint 4C el corpus de `frame_decoder` incluye trece semillas selladas:
cuatro frames genuinos tomados de `aead-v1.json` y nueve mutaciones —cabecera
truncada, tag ausente, tag truncado, tag alterado, secuencia alterada, sesión
ajena, `ENCRYPTED` sin trailer declarado, trailer sobredimensionado y dos frames
concatenados—. Se reproducen dos veces: el smoke de `qyro_protocol` comprueba que
ninguna rompe el framing, y el de `qyro_crypto` (`src/aead/corpus.rs`, dentro del
crate porque necesita los constructores deterministas) comprueba la capa de
arriba: que ninguna mutación pasa el AEAD, que las genuinas sí abren, y que nada
sale de `open` que un sealer no haya sellado.

Ya existe un target `cargo-fuzz` para el opener, que en el sprint 4C no existía
por una razón que resultó tener solución: una sesión aleatoria haría que casi toda
entrada muriese en `WrongSession` antes de llegar al AEAD. La sesión fija de
`qyro_crypto::fuzzing` resuelve eso sin poner un constructor determinista en la
API pública.

Para ejecutar una campaña real, **desde la raíz del repositorio**:

    rustup toolchain install nightly
    cargo install cargo-fuzz --locked --version 0.13.1
    cargo +nightly fuzz run --fuzz-dir rust/fuzz frame_decoder \
        -- -max_total_time=300 -print_final_stats=1

`--fuzz-dir` no es opcional. Sin él, cargo-fuzz busca `<raíz>/fuzz`, encuentra el
manifest del workspace y responde «could not read the manifest file:
.../fuzz/Cargo.toml», que no dice nada sobre el problema real —que este proyecto
guarda su crate de fuzzing bajo `rust/`—. El recetario anterior de esta sección
omitía la opción, y como tampoco se podía construir nada, nadie lo notó.

`.github/workflows/crypto-fuzz.yml` ejecuta los seis targets semanalmente y bajo
demanda, con un job por target y sin `fail-fast`, para que un crash en uno no
oculte si los demás también fallan. Imprime las estadísticas finales de libFuzzer
en el log, de modo que «se fuzzeó» sea un número de ejecuciones y no una
afirmación.

Cualquier hallazgo debe añadirse al corpus antes de corregirse, para que el smoke
lo cubra a partir de entonces.

## Plegado de colisiones

`PortableCollisionKey` aplica normalización NFC real
(`unicode-normalization`) y después `str::to_lowercase`, por segmento. No está
limitado a ASCII ni a Latin-1: pliega marcas combinantes fuera de ese rango
(`ḍ` frente a `d` + U+0323), singletons como U+2126 OHM SIGN, y el plegado de
mayúsculas de griego y cirílico.

Este apartado se titulaba «Límite conocido» y describía una tabla escrita a mano
que plegaba solo ASCII y Latin-1, junto con la decisión de no añadir
`unicode-normalization` todavía. Esa decisión se revirtió en el sprint 4A —la
tabla además sobre-plegaba, tratando `ano.txt` y `año.txt` como el mismo
archivo— pero el texto se quedó atrás y siguió declarando un hueco que ya no
existía.

Riesgo que sí permanece: el plegado responde a la pregunta «¿pueden estas dos
rutas ser el mismo archivo en Windows o macOS?», no a «¿se parecen?». Dos nombres
visualmente confundibles con puntos de código distintos (homoglifos) son
deliberadamente rutas distintas, y la confirmación del receptor es lo que
protege ahí, no el manifest.

## Riesgo residual

- La campaña de fuzzing es acotada: dos minutos por target, semanal. La cobertura
  sobre entradas imprevistas sigue siendo desconocida más allá de eso.
- Los límites son constantes de compilación elegidas por criterio, no medidas.
  `MAX_PAYLOAD_LEN` de 1 MiB deberá revisarse cuando exista transporte real.
- El manifest todavía no se autentica: la canonicidad está lista para firmar,
  pero la firma llega en el hito de identidad y cifrado.
- Nada de esto protege contra un peer legítimo que envíe contenido malicioso;
  eso corresponde a la confirmación del receptor y a la integridad final.
- **`qyro_protocol` sigue sin hacer criptografía**, y eso es deliberado.
  `EncryptedEnvelope` define la forma de un frame cifrado y expone la cabecera
  completa como datos asociados; quien verifica el tag es `qyro_crypto::aead`
  desde el sprint 4C. Un `EncryptedEnvelope` decodificado por sí solo no
  demuestra nada: los bytes que llama «tag» no los ha comprobado nadie hasta
  que un `FrameOpener` con la clave de sesión los consume. Esta frase decía
  antes que «el AEAD sigue sin existir», lo cual dejó de ser cierto en 4C.
