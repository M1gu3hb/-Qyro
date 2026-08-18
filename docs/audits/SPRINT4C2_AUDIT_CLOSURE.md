# Auditoría del sprint 4C.2 — cierre de la auditoría independiente

- Fecha UTC: 2026-08-07
- Rama: `claude/qyro-audit-closure-4c2-9a3v4j`
- Base: `claude/qyro-crypto-platform-hardening` en
  `9f79e55c54eb4e7b5df75653dae1bb106f33088f`
- Alcance: cerrar los trece hallazgos QYR-0021 … QYR-0035 de la auditoría
  externa. **Este sprint no añade funcionalidad.**

Una auditoría independiente reprodujo la evidencia del sprint 4C.1 sobre HEAD
(278 tests, clippy y fmt limpios, cuatro guardas en verde, cero advisories
aplicables) y encontró trece defectos que ese trabajo no cubría. Uno era un fallo
de seguridad real. Tres eran garantías que sobrevivían a su propio borrado. El
resto, controles sin prueba y documentación que contradecía al código.

## Regla de verificación

Una corrección cuya mutación no rompe nada no está cubierta y no cuenta como
hecha. Cada corrección de este sprint se comprobó **volviendo a aplicar la
mutación después de implementarla** y confirmando que la prueba nombrada falla.

Cinco hallazgos tenían prueba roja antes de existir la corrección. Los otros
describen controles que **ya estaban en el código y no tenían prueba**: para esos
el estado rojo no es un commit anterior, es la mutación. La tabla lo distingue
en vez de disfrazarlo.

## Tabla de mutación

| Hallazgo | Mutación aplicada | Prueba que falló | Rojo en |
|---|---|---|---|
| QYR-0021 | borrar el filtro `Cf` en `path.rs` | `unicode_format_characters_are_rejected`, `a_right_to_left_override_cannot_disguise_an_extension`, `a_zero_width_space_cannot_hide_between_a_name_and_its_extension`, `a_name_that_differs_only_by_an_invisible_character_is_rejected`, `the_decoder_refuses_a_disguised_extension_too` | `cff6a1a` (5 pruebas en rojo) |
| QYR-0022 | borrar `verify_transcript` en `handshake/mod.rs::receive_initiator_finish` | `an_unsigned_peer_cannot_present_another_identity` | solo bajo mutación |
| QYR-0023 | `verify_strict` → `verify`, con `Verifier` en ámbito | `a_non_strict_signature_is_refused` | solo bajo mutación |
| QYR-0025 (a) | borrar los prefijos `u32` BE de `update_with_length` | `the_transcript_is_what_the_specification_says_it_is` | solo bajo mutación |
| QYR-0025 (b) | `Hkdf::new(Some(salt), …)` → `Hkdf::new(Some(auth_transcript), …)` | `every_recorded_value_verifies_against_the_primitives` | solo bajo mutación |
| QYR-0026 | — (disparadores de CI; se comprueba por ejecución, no por mutación) | — | — |
| QYR-0028 | ninguna: el control no existía | `a_file_cannot_also_be_a_directory`, `the_collision_is_found_at_any_depth`, `the_decoder_refuses_an_ancestor_collision_too` | `52391a5` (3 pruebas en rojo) |
| QYR-0029 | ninguna: el nombre no estaba en la tabla | `windows_superscript_device_names_are_rejected` | `02e1e44` |
| QYR-0030 | añadir `qyro_crypto` bajo `[target.'cfg(target_os = "android")'.dependencies]` de `qyro_ffi` | `the_ffi_dependency_closure_holds_no_crypto` | solo bajo mutación |
| QYR-0031 | — (documentación; se comprueba leyendo el código, no mutando) | — | — |
| QYR-0032 (1) | borrar `computed != declared_total_bytes` en `from_sorted` | `a_declared_total_that_does_not_match_the_items_is_rejected` | solo bajo mutación |
| QYR-0032 (2) | borrar el brazo `Ordering::Less => UnsortedItems` | `items_in_descending_order_are_rejected` | solo bajo mutación |
| QYR-0032 (3) | borrar `length > limit` en `take_length_prefixed` | `an_oversize_length_prefix_is_refused_before_it_can_slice` | solo bajo mutación |
| QYR-0032 (4) | `checked_add(…).ok_or(…)?` → `wrapping_add(…)` | `two_sizes_that_wrap_the_total_are_rejected` | solo bajo mutación |
| QYR-0033 | ninguna: la guarda no leía esos archivos | `guards::no_production_path_can_panic` | `6b02db6` |
| QYR-0034 | — (decisión registrada; la prueba la afirma en la dirección elegida) | — | — |
| QYR-0035 | volver a añadir `UnexpectedRole` con su brazo de `Display` | `guards::every_handshake_error_has_a_construction_site` | solo bajo mutación |

### Lo que dijo cada mutación

Vale la pena registrar el mensaje, no solo el hecho de que falló: un fallo que
apunta al control equivocado es una prueba que pasa por otra razón.

- **QYR-0021.** Antes de la corrección, `codec::decode` devolvía
  `Ok(TransferManifest { … path: "invoice\u{202e}fdp.exe" … })`. El defecto es
  literalmente el valor aceptado.
- **QYR-0022.** Con `verify_transcript` borrado, la prueba falla con
  `Some(FinishedVerificationFailed)` donde esperaba
  `Some(SignatureVerificationFailed)`: el handshake **deriva claves** para un
  peer que no probó nada, y solo el MAC de confirmación lo detiene después.
- **QYR-0023.** Con `verify`, la aserción pasa de
  `Err(SignatureVerificationFailed)` a `Ok(())`. Eso confirma empíricamente la
  premisa del vector: es una firma que el verificador permisivo acepta.
- **QYR-0025 (a).** Falla también `the_committed_vector_is_exactly_what_regeneration_produces`,
  cuyo mensaje invita a regenerar el archivo. El mensaje de la prueba nueva dice
  lo contrario a propósito: el ADR es la especificación, así que lo que tiene que
  cambiar es el código.
- **QYR-0025 (b).** Falla con «the key schedule this crate runs disagrees with
  the primitives».
- **QYR-0030.** Falla nombrando el crate. Con la misma edición, la ventana que
  leía la prueba anterior contiene exactamente
  `qyro_core = { path = "../qyro_core" }`: la prueba vieja habría pasado.
- **QYR-0035.** Falla con «HandshakeError::UnexpectedRole is declared but
  nothing constructs it».

## Hallazgo por hallazgo

### QYR-0021 (P0) — categoría Unicode `Cf` aceptada en rutas

`RelativePath::parse` filtraba con `char::is_control()`, que es la categoría
`Cc` y nada más. La categoría `Cf` pasaba entera, se guardaba tal cual y
sobrevivía al round-trip byte a byte. `parse("invoice\u{202E}fdp.exe")` devolvía
`Ok`, y todo renderizador consciente de bidi muestra ese nombre como
`invoiceexe.pdf`.

Cerrado con una tabla de veintiún rangos transcrita de
`DerivedGeneralCategory.txt` de Unicode 16.0.0, citada en el fuente y comprobada
contra el archivo: 170 puntos de código, ninguno de más, ninguno de menos. Sin
dependencias nuevas. `U+200C` y `U+200D` se rechazan también, como decisión
explícita; el razonamiento está en la enmienda a ADR-0019.

Se eliminó además la comprobación redundante de `U+007F` en `validate_segment`:
es `Cc`, así que `is_control()` ya la había rechazado, y una comprobación que no
puede dispararse sugiere un hueco donde no lo hay.

### QYR-0022 (P1) — el iniciador no estaba autenticado por ninguna prueba

El control existía; la prueba no. Borrarlo dejaba
`cargo test --package qyro_crypto` en 124 passed, 0 failed.

### QYR-0023 (P1) — `verify_strict` sin prueba que lo distinguiera de `verify`

En ed25519-dalek 3.0.0 los dos difieren en una sola cosa: `verify_strict`
rechaza un `R` o un `A` de orden pequeño. Como `[s]B - [k]A` siempre cae en el
subgrupo de orden primo, el único punto de orden pequeño que puede igualar es la
identidad; de ahí la forma del vector.

Los bytes se derivan en vez de citarse, y el fuente dice por qué: `verify` de
este crate firma sobre su propia entrada con separación de dominio, así que
ninguna terna `(A, M)` publicada puede presentársele. Una prueba construida
sobre una lo estaría ejerciendo a ed25519-dalek, no al uso que este crate hace de
él. La clave es la de RFC 8032 §7.1 TEST 1, que ya estaba en el archivo.

### QYR-0025 (P1) — el transcript se verificaba llamándose a sí mismo

`handshake/vectors.rs` afirmaba verificar los valores registrados «contra las
primitivas» y llamaba a `base_transcript`, `auth_transcript` y `hmac_sha256`.
Ahora recalcula ambos transcripts con SHA-256 sobre concatenación literal y el
HMAC escrito desde RFC 2104, y comprueba además las dos entradas de firma
registradas contra el ADR en vez de solo pasárselas a un verificador.

También fija `Schedule::derive` contra los valores que acaba de verificar. Sin
eso el test ejecutaba HKDF a mano y no tocaba el schedule, que es por qué
reencaminar el `info` al salt lo dejaba en verde.

La prueba nueva, `the_transcript_is_what_the_specification_says_it_is`, no toca
`transcript.rs` para construir lo esperado.

### QYR-0026 (P1) — ningún workflow se disparaba en la rama de trabajo

Cuatro listaban solo `main` y dos listaban `audit/baseline-hardening`, una rama
que dejó de recibir commits cuatro sprints antes. «CI está en verde» significaba
«alguien se acordó de lanzarlo a mano».

**Se eligió `push` y no una pull request.** Un run de `pull_request` se ejecuta
sobre un commit de fusión que solo existe dentro del run, así que su ID no puede
citarse como evidencia de un commit de esta rama. Un run de `push` se ejecuta
sobre exactamente el commit empujado, que es lo que STATUS.md tiene que nombrar.
Se corrigieron de paso las dos referencias a la rama muerta.

### QYR-0028 (P2) — un archivo que es también un directorio

`validate_items` comparaba claves de colisión por igualdad. Ahora también rechaza
una clave que es prefijo de la siguiente en frontera NUL, cuando el elemento
dueño del prefijo es `File`. Formulación exacta en la enmienda a ADR-0017.

### QYR-0029 (P2) — nombres de dispositivo con superíndice

`COM¹`, `COM²`, `COM³`, `LPT¹`, `LPT²` y `LPT³` añadidos, con la página de
Microsoft Learn citada en el fuente, incluida la nota de que Windows trata los
superíndices ISO/IEC 8859-1 como dígitos dentro de un nombre de dispositivo.

`COM0`, `LPT0`, `CONIN$`, `CONOUT$` y `CLOCK$` **no** se añaden. Esa página no
los lista y no se comprobó ninguna otra fuente, así que inventar la regla
rechazaría nombres legítimos por una suposición. Queda abierto en
`BUGS_PENDING.md`, y una prueba fija que hoy se aceptan para que la respuesta no
cambie por accidente.

### QYR-0030 (P2) — la frontera FFI se comprobaba por texto

Ver la tabla de mutación. Además, los dos scripts de guarda en shell que
comprueban la misma frontera descartan ahora las líneas de comentario antes de
buscar: coincidían con el archivo entero, así que nombrar el crate en un
comentario rompía la comprobación y la única forma de mantenerla en verde era no
decir la palabra. Una tabla de dependencias específica de plataforma se sigue
detectando: no es un comentario.

### QYR-0031 (P2) — seis sitios de documentación contra el código

| Sitio | Decía | Dice |
|---|---|---|
| `qyro_manifest/lib.rs`, `path.rs` | rutas «normalized», campo `normalized` | verbatim; el campo se llama `verbatim` |
| `qyro_protocol/limits.rs`, `version.rs` | bytes de cabecera desconocidos «se saltan» | se rechazan con `UnsupportedHeaderExtension` (ADR-0018) |
| `qyro_protocol/limits.rs`, `error.rs` | QYRO/1.0 exige trailer cero | un frame sellado exige `1..=64`; `SUPPORTED_TRAILER_LEN` es la regla de los frames planos |
| `qyro_crypto/identity.rs`, `handshake/mod.rs` ×2 | `cfg(test)` | `cfg(any(test, fuzzing))` |
| `qyro_crypto/fuzzing.rs` | — | `--cfg fuzzing` es activable por `RUSTFLAGS` en todo el workspace |
| `THREAT_MODEL.md` ×3 | rutas normalizadas; rechazo de symlink; sin pánicos en producción | verbatim; symlink inexpresable por `ItemKind`; guarda sobre los doce archivos |

Cada uno queda marcado como corregido en vez de reescrito en silencio.

### QYR-0032 (P2) — cuatro controles de decode sin prueba

Los cuatro existían y ninguno tenía prueba que fallara al borrarlo. Las pruebas
nuevas construyen los bytes a mano porque la API de construcción impone los
mismos invariantes: un manifest construido con `TransferManifest::new` se rechaza
antes de poder codificarse y nunca llega a la copia del decoder.

La prueba de desbordamiento usa 1 y `u64::MAX`, que suman exactamente
`u64::MAX + 1`, con un total declarado de cero. El segundo tamaño excede por sí
solo `MAX_TOTAL_BYTES`, y **tiene que hacerlo**: el total acumulado se comprueba
*después* de la suma, así que el desbordamiento se alcanza primero. Eso es
justamente lo que hace de la suma comprobada el control bajo prueba y no del
límite.

### QYR-0033 (P2) — la guarda anti-pánico solo leía `src/aead/`

`handshake/transcript.rs` tenía un `expect` y `handshake/schedule.rs` un
`unreachable!`, ambos en ruta de producción alcanzable desde bytes de un peer.
Los dos eliminados sin añadir un error muerto:

- El transcript toma ahora los dos hellos como arrays de anchura fija y el
  prefijo de longitud es una constante, con la anchura fijada por una aserción
  de evaluación const que detiene el *build*, no el proceso.
- `finished_mac` rellena con ceros la clave de 32 bytes hasta el bloque HMAC y
  usa el constructor infalible `KeyInit::new`. El relleno es la definición de
  `K'` en RFC 2104, y una prueba fija que produce el mismo tag que la forma de
  clave variable que verifican los vectores RFC 4231.

Con ellos se fueron catorce indexaciones sin comprobar. La guarda nueva recorre
los doce archivos de producción y trae dos cosas que un `#![deny(...)]` no puede
dar: detecta un módulo al que nadie le puso el atributo, y detecta `assert!`,
que no tiene lint y termina el proceso igual que `panic!`.

### QYR-0034 (P3→registrado) — codificaciones X25519 con `u >= p`

Decisión registrada en la enmienda A a ADR-0021: se aceptan, conforme a
RFC 7748 §5. La afirmación de la auditoría sobre libsodium y CryptoKit **no se
verificó aquí** y queda abierta.

### QYR-0035 (P3) — cuatro variantes de `HandshakeError` que nada construía

Eliminadas, con el motivo de cada una escrito donde está el enum, y una guarda
que impide que vuelva a ocurrir.

## Lo que no se cerró

| Hallazgo | Estado | Motivo |
|---|---|---|
| QYR-0024 | abierto | Coste cuadrático del decoder. No objetivo declarado (§10): es del sprint 4C.3. |
| QYR-0027 | abierto | Capacidad del búfer. Mismo motivo. |
| QYR-0029 (parcial) | abierto | `COM0`, `LPT0`, `CONIN$`, `CONOUT$`, `CLOCK$` sin fuente. No se añade una regla sin evidencia. |
| QYR-0034 (parcial) | abierto | Comportamiento de libsodium/CryptoKit no verificado; se cierra cuando exista el lado Swift. |
| QYR-0036 | nuevo, abierto | `clippy::indexing_slicing` no está denegado en `qyro_protocol` ni en `qyro_manifest`, que también analizan bytes de un peer. Este sprint lo denegó en `qyro_crypto`. |
| QYR-0039 | abierto | Fuera del alcance declarado de este sprint. |

## Lo que sigue siendo verdad y no debe leerse como progreso

- **No hay transporte.** No hay sockets, ni descubrimiento, ni LAN.
- **No hay almacenamiento seguro.** Ni Keystore, ni Keychain, ni DPAPI.
- **No hay transferencia de archivos.** Nada mueve un byte a ninguna parte.
- **Los botones Enviar y Recibir siguen deshabilitados.**
- **No hay hardware físico.** Android arm64 e iOS device siguen siendo
  compile-only y así se declaran.
- **Ninguna segunda implementación ha leído los vectores.** No existe lado Swift
  ni Kotlin, así que la interoperabilidad sigue sin demostrarse.
- **`qyro_ffi` sigue sin poder ver `qyro_crypto`**, ahora comprobado sobre el
  cierre transitivo real.
