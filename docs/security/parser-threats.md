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

Existen tres targets `cargo-fuzz` en `rust/fuzz/fuzz_targets`:

- `frame_decoder`: bytes arbitrarios, con el primer byte eligiendo el tamaño de
  fragmento, de modo que la partición también se fuzzea.
- `manifest_decoder`: cualquier manifest aceptado debe llevar solo rutas seguras
  y volver a codificarse en los mismos bytes.
- `relative_path`: el parser no puede entrar en pánico ni reescribir su entrada.

`rust/fuzz` es un workspace aparte porque `libfuzzer-sys` exige nightly, mientras
el proyecto compila en stable 1.88.0.

**No se ha ejecutado una campaña de fuzzing.** Lo que CI ejecuta es un *corpus
smoke*: los 65 archivos de `rust/fuzz/corpus` se reproducen contra las mismas
aserciones que hacen los targets, en el caso del framing a cuatro tamaños de
fragmento distintos. Eso es una defensa contra regresiones sobre entradas ya
conocidas; no dice nada sobre entradas que nadie ha imaginado todavía.

Para ejecutar una campaña real:

    rustup toolchain install nightly
    cargo install cargo-fuzz
    cd rust/fuzz
    cargo +nightly fuzz run frame_decoder -- -max_total_time=300
    cargo +nightly fuzz run manifest_decoder -- -max_total_time=300
    cargo +nightly fuzz run relative_path -- -max_total_time=300

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

- Sin campaña de fuzzing, la cobertura sobre entradas imprevistas es desconocida.
- Los límites son constantes de compilación elegidas por criterio, no medidas.
  `MAX_PAYLOAD_LEN` de 1 MiB deberá revisarse cuando exista transporte real.
- El manifest todavía no se autentica: la canonicidad está lista para firmar,
  pero la firma llega en el hito de identidad y cifrado.
- Nada de esto protege contra un peer legítimo que envíe contenido malicioso;
  eso corresponde a la confirmación del receptor y a la integridad final.
- **`qyro_protocol` no hace criptografía.** `EncryptedEnvelope` define la forma
  de un frame cifrado y expone la cabecera completa como datos asociados, pero
  ningún AEAD la consume: los bytes que llama «tag» no los verifica nadie. La
  identidad Ed25519 vive en `qyro_crypto` desde el sprint 4A; el AEAD sigue sin
  existir.
