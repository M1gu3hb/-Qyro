# Auditoría del AEAD de frames — cierre del sprint 4C

Ámbito: lo que el sprint 4C entregó. Rama: `claude/qyro-aead-replay`, sobre
`cc4d7d9`.

Este documento registra defectos reales y huecos de cobertura reales, con la
prueba que los expone. No es un resumen del diseño; para eso están
`frame-encryption.md`, `nonce-lifecycle.md` y `replay-window.md`.

## Método

Cada propiedad se comprobó **borrando la corrección y comprobando que alguna
prueba falla**. Cuando ninguna falla, la propiedad no estaba cubierta, por
convincente que fuera el argumento a favor.

En este sprint el método encontró un hueco que el diseño ocultaba y una
implementación mía que estaba mal.

## Hallazgos

### H-1 · La dirección dentro de la etiqueta no estaba cubierta por nada

ADR-0022 dice: «La dirección va **dentro** de la etiqueta, así que las dos
direcciones no pueden producir la misma clave aunque partieran del mismo
secreto.»

Borré `direction.label()` de `info_for`, dejando que las dos direcciones
derivaran bajo la misma etiqueta. **Las treinta y tres pruebas siguieron
pasando**, incluida `the_two_directions_do_not_open_each_others_frames`.

El motivo es que los dos secretos de tráfico ya difieren: el schedule del
handshake los deriva bajo `initiator-to-responder` y `responder-to-initiator`.
Las pruebas extremo a extremo nunca ejercitan el caso que la afirmación cubre,
porque no pueden: no hay forma de que una sesión real ponga el mismo secreto en
las dos direcciones.

El código no estaba mal. La afirmación estaba apoyada una capa más arriba, y si
alguien tocara el schedule del handshake, nada aquí lo notaría.

La misma prueba negativa vale para el `auth_transcript` y el `session_id` dentro
de cada `info`: quitar cualquiera de los dos tampoco rompía nada, porque dos
sesiones de prueba difieren en todo a la vez.

Severidad: P2 (hueco de cobertura, no defecto). Corrección: cuatro pruebas
unitarias sobre la derivación misma —un secreto, dos direcciones, dos claves; el
mismo secreto y dirección bajo otra sesión u otro transcript, otra clave; el
prefijo de nonce no es un trozo de la clave; y las cuatro etiquetas fijadas contra
la ADR y no contra la función que las produce—. Las cuatro mutaciones ahora
fallan.

Pruebas: `the_direction_is_inside_the_label_not_only_inside_the_secret`,
`the_session_and_the_transcript_bind_every_derived_value`,
`the_nonce_prefix_is_not_a_slice_of_the_key`,
`the_derivation_labels_are_the_ones_the_adr_freezes`.

### H-2 · Tres variantes de error que nadie podía provocar

ADR-0022 congeló una lista de errores antes de que existiera el código. Al
implementarla, tres de ellos resultaron inalcanzables:

- `NotEncrypted`: un `EncryptedEnvelope` no puede existir sin el flag
  `ENCRYPTED`. Sus dos constructores lo garantizan —uno lo pone, el otro lo
  exige—, así que el paso «validar `ENCRYPTED`» del orden de apertura lo cumple el
  tipo.
- `PayloadTooLarge`: `seal` recibe un `&Frame`, y un `Frame` no puede llevar más
  de `MAX_PAYLOAD_LEN`.
- `InvalidNonceState`: era `SequenceExhausted` con otro nombre.

Un error que nadie puede provocar documenta una comprobación que no está ahí. Es
el mismo defecto que ADR-0018 registró para `TrailingBytes`.

Severidad: P3. Corrección: se eliminan del enum y la ADR se enmienda explicando
por qué. Cada variante que queda la produce alguna prueba.

### H-3 · Mi propia prueba de vectores emparejaba mal las direcciones

La primera versión de `the_recorded_frames_open_through_the_decoder_and_the_opener`
construía los dos openers a mano con `frame_crypto`, y le daba a cada uno el
secreto de la dirección que **no** lee. Ningún frame grabado abría.

El fallo fue mío y en una prueba, no en la biblioteca. Vale la pena registrarlo
porque señala la forma exacta del error que la API de producción evita: el
emparejamiento correcto lo hace `into_frame_crypto`, y la prueba ahora lo usa en
lugar de repetirlo.

Severidad: N/A (defecto de prueba, corregido antes de comprometerse).

### H-4 · El corpus de fuzzing no tenía ningún frame sellado

El smoke test de `qyro_protocol` llevaba desde el sprint 2 un comentario diciendo
que la rama `DecodedFrame::Encrypted` existía pero ningún seed la ejercitaba,
porque nada podía producir uno.

Severidad: P3. Corrección: trece semillas selladas —cuatro genuinas tomadas de los
vectores comprometidos y nueve mutaciones— reproducidas por dos smoke tests
distintos: el de framing en `qyro_protocol` y el de AEAD en
`qyro_crypto::aead::corpus`.

Ese segundo comprueba lo que un smoke de framing no puede: que ninguna mutación
pasa el AEAD, que las genuinas sí abren, y que de `open` no sale nunca un texto
claro que ningún sealer haya sellado.

## Lo que se verificó borrando la corrección

| Propiedad | Mutación aplicada | Prueba que falla |
|---|---|---|
| La ventana solo se actualiza tras verificar | `record` antes del AEAD | `a_failed_authentication_does_not_move_the_replay_window`, `tampering_with_ciphertext_or_tag_fails` |
| El sealer asigna `session_id` y `sequence` | conservar los del llamante | diez pruebas, incluida `caller_metadata_survives_and_is_authenticated` |
| La cabecera entera es AAD | AAD vacío | `every_byte_of_the_header_is_authenticated` |
| La secuencia no da la vuelta | `wrapping_add` | `an_exhausted_sequence_is_a_terminal_error` |
| La dirección está en la etiqueta | quitar `direction.label()` | `the_direction_is_inside_the_label_not_only_inside_the_secret` |
| El transcript liga la clave | quitarlo de `info` | `the_session_and_the_transcript_bind_every_derived_value` |
| El `session_id` liga la clave | quitarlo de `info` | `the_session_and_the_transcript_bind_every_derived_value` |
| El prefijo se deriva aparte | recortarlo de la clave | `the_nonce_prefix_is_not_a_slice_of_the_key` |

## Lo que sigue sin estar cubierto

- **No se ha ejecutado una campaña de fuzzing.** Lo que corre es un corpus smoke
  sobre entradas ya conocidas. Ver `parser-threats.md`.
- **No hay target de fuzzing para el opener**, y `parser-threats.md` explica por
  qué: tendría que fabricar una sesión antes del primer byte, y contra una sesión
  aleatoria casi toda entrada moriría en `WrongSession` sin llegar al AEAD.
- **No hay comprobación de canales laterales.** ChaCha20-Poly1305 en software es
  de tiempo constante por construcción y la comparación del tag la hace `subtle`
  dentro de la biblioteca, pero nada en este repositorio lo mide.
- **No hay pruebas en hardware físico.** Nada de este sprint se ha ejecutado en un
  teléfono real. Lo que hay son pruebas de host y ejecuciones de CI.
- **No hay interoperabilidad demostrada.** Los vectores existen para que otra
  implementación pueda comprobarse contra ellos; ninguna lo ha hecho todavía.
- **No hay rotación ni rekey.** Una sesión usa una clave por dirección hasta
  agotar la secuencia, y agotarla es terminal.

## Estado

El AEAD está implementado, compila en stable 1.88.0 y pasa las pruebas de host y
CI en las tres plataformas. **Qyro sigue sin transferir archivos**: no hay
sockets, ni descubrimiento, ni selector de archivos, ni persistencia de identidad.
Los botones Enviar y Recibir siguen deshabilitados.
