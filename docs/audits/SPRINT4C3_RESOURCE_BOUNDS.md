# Auditoría del sprint 4C.3 — cotas de recursos antes de que exista un consumidor

- Fecha UTC: 2026-08-07
- Rama: `claude/qyro-resource-bounds-4c3`
- Base: `claude/qyro-audit-closure-4c2-9a3v4j` en
  `8725ab7cc2e5da9a2412f96e7bbc33a7ae57707b`
- Alcance: QYR-0024, QYR-0027, QYR-0036 y QYR-0040 … QYR-0044.
  **Este sprint no añade funcionalidad.**

Nada consume todavía `qyro_protocol` ni `qyro_manifest`. Eso es exactamente
cuándo conviene corregir un coste cuadrático: cuando el único perjudicado es una
prueba, y no una transferencia a medias.

## Regla de verificación

Una corrección cuya mutación no rompe nada no está cubierta y no cuenta como
hecha. Cada corrección se comprobó volviendo a aplicar la mutación **después**
de implementarla.

## Tabla de mutación

| Hallazgo | Mutación aplicada | Prueba que falló | Rojo en |
|---|---|---|---|
| QYR-0024 | reclamar el espacio tras cada frame (el `drain` anterior) | `draining_a_full_buffer_copies_a_bounded_number_of_bytes`, `the_cost_of_draining_does_not_grow_with_the_buffer`, `a_socket_loop_with_a_backlog_stays_bounded`, `a_buffer_filled_one_byte_at_a_time_still_yields_its_frames` | `9c4a1a2` |
| QYR-0027 | devolver el crecimiento geométrico de `Vec` | `the_buffer_never_reserves_more_than_its_limit` | `9c4a1a2` |
| QYR-0036 (protocol) | reintroducir `.expect(` en `session.rs` | `guards::no_production_path_can_panic` | solo bajo mutación |
| QYR-0036 (manifest) | reintroducir `.expect(` en `limits.rs` | `guards::no_production_path_can_panic` | solo bajo mutación |
| QYR-0038 | borrar la comprobación de `MAX_HASH_LEN` | `a_digest_longer_than_the_hash_limit_is_rejected` | solo bajo mutación |
| QYR-0040 | escribir un nombre de rama literal en `ci.yml` | `check_docs_consistency`, regla «Workflow branch trigger» | `5008fef` |
| QYR-0042 | quitar `#[cfg(test)]` de `mod schema;` | `guards::every_test_only_module_is_actually_gated` | solo bajo mutación |
| QYR-0043 | citar un `QYR-00xx` sin entrada en el registro | `check_docs_consistency`, regla «Finding ledger» | `a1b61c4` |
| QYR-0044 | borrar los prefijos `u32`-BE del transcript | el consejo cambia a «Do not regenerate» | solo bajo mutación |
| QYR-0041 | — (una fecha; se comprueba leyendo la fuente, no mutando) | — | — |

### Lo que dijo cada mutación

- **QYR-0024.** Reponer el reclamo por frame devuelve exactamente los números de
  partida: 11 476 501 344 bytes en el llenado-drenado, 9 830 400 000 en el bucle
  con backlog, y 44 684 640 contra 65 520 empujados ya en el techo más pequeño.
  La primera versión de esta mutación salió **verde** porque solo parcheó la
  rama `Unknown` del decoder y un heartbeat es un tipo conocido: una mutación
  que no toca el camino que la prueba recorre no prueba nada, y queda anotada
  aquí porque distinguir eso es el trabajo.
- **QYR-0027.** «capacity reached 2097152 at 1048577 buffered bytes, past the
  MAX_BUFFER_LEN of 1049664».
- **QYR-0038.** Sin la comprobación, un digest de 65 bytes cae en
  `InvalidHashLength` en vez de `FieldTooLong`: el límite deja de existir y el
  error pasa a hablar del algoritmo.
- **QYR-0042.** «src/schema.rs is compiled into a release build and no guard
  covers it».
- **QYR-0044.** «**and this build no longer computes the transcript ADR-0021
  specifies**. Do not regenerate».

## Coste, antes y después

Contado, no cronometrado. `bytes_moved` está instrumentado bajo `cfg(test)` en
el único sitio que mueve bytes; un reloj de pared en un runner compartido mide
el runner y no dice qué se rompió.

| Forma | Antes | Después |
|---|---|---|
| `MAX_BUFFER_LEN` de frames mínimos, drenado entero (21 868 frames, 1 049 664 B) | 11 476 501 344 B movidos | **0** |
| Backlog de 4 096 frames con 50 000 llegadas (2 596 608 B empujados) | 9 830 400 000 B | **2 359 296 B** |
| Techo de 65 536 B, drenado entero (65 520 B empujados) | 44 684 640 B | **0** |

El cero no es una optimización perfecta: llenar y luego drenar **nunca necesita
compactar**, porque no llega nada nuevo mientras se drena. Esa forma sola no
demuestra que la compactación esté amortizada, y por eso existe la segunda fila,
que es la que un transporte produce de verdad y la única donde `compact` corre.
Allí el resultado es 0,91 copias por byte empujado.

**Dónde el nuevo drenado es más lento.** En la forma que antes no compactaba
nada — un solo frame grande que se consume entero — el nuevo código hace una
compactación de coste cero (`read == len`, no queda cola) que el anterior no
hacía. Es una llamada, no un memmove. No se ha encontrado ninguna forma en la
que el nuevo drenado mueva más bytes que el anterior; el peor caso del nuevo es
una copia por byte y el mejor caso del anterior ya era ese.

## Indexados sin comprobar

| Crate | Infracciones | En qué se convirtieron |
|---|---|---|
| `qyro_protocol` | **33** (29 `header.rs`, 3 `envelope.rs`, 1 `frame.rs`) | 21 desaparecieron al estrechar `parse` a `&[u8; HEADER_LEN]`: indexar un array con índice constante es demostrablemente correcto. 8 `try_into().expect(...)` pasaron a `field::<OFFSET, WIDTH>`, con la cota comprobada en tiempo de compilación. 2 conversiones de longitud con `expect` pasan a devolver `PayloadTooLarge`, el mismo error que levanta la comprobación que las precede. 2 expresiones de rebanado pasan a `split_at_checked` |
| `qyro_manifest` | **22** (18 `model.rs`, 2 `codec.rs`, 2 `path.rs`) | `windows(2)` + `window[0]`/`window[1]` pasa a `zip`, que entrega el par como par. `items[index - 1]` pasa a un predecesor arrastrado por el bucle. `items[window[0].1]` pasa a `get(..).is_some_and(..)`. `take_exact` pasa a `get` con `checked_add`. `take_u8` pasa a un patrón de rebanada. `has_drive_prefix` pasa a `[drive, b':', ..]` |

Ninguna se silenció con `allow` salvo los módulos de prueba dentro de cada
crate, que pueden afirmar e indexar libremente.

La guarda encontró además algo que ningún lint cubre: un `debug_assert_eq!` al
final de `codec::encode`, duplicando un invariante que
`encoded_len_matches_the_bytes_actually_produced` ya fija en todos los perfiles.
Eliminado.

## Ninguna prueba de contrato existente necesitó edición

El cambio del decoder toca **un solo archivo**, `decoder.rs`. Los 11 contratos
de `wire_contract.rs`, los de `forward_compatibility.rs`, los de
`plain_encrypted_boundary.rs`, los property tests y el corpus smoke pasan sin
tocarlos. Esa es la evidencia de que lo que cambió fue el coste y no el
comportamiento.

## Lo que no se cerró

| Hallazgo | Estado | Motivo |
|---|---|---|
| QYR-0029 (parcial) | abierto | `COM0`, `LPT0`, `CONIN$`, `CONOUT$`, `CLOCK$` sin fuente primaria. No objetivo declarado. |
| QYR-0034 (parcial) | abierto | Verificación de libsodium/CryptoKit. Se cierra cuando exista el lado Swift. |
| QYR-0039 | abierto, **sin descripción** | Su contenido no está en este repositorio. Registrado como referencia colgante resuelta y hallazgo desconocido, con la acción concreta escrita. |
| QYR-0003 | abierto | Aviso de `actions/checkout`, de sprints anteriores. |
| QYR-0001, QYR-0004, QYR-0005 … | abiertos | Anteriores a este sprint y fuera de su alcance. |

## Lo que sigue siendo verdad y no debe leerse como progreso

- **No hay transporte.** El decoder que este sprint hizo lineal no recibe nada:
  no hay sockets, ni descubrimiento, ni LAN.
- **No hay almacenamiento seguro.** Ni Keystore, ni Keychain, ni DPAPI.
- **No hay transferencia de archivos.** Nada mueve un byte a ninguna parte.
- **Los botones Enviar y Recibir siguen deshabilitados.**
- **No hay hardware físico.** Android arm64 e iOS device siguen siendo
  compile-only.
- **Ninguna segunda implementación ha leído los vectores.**
