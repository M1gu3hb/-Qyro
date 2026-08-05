# Ciclo de vida del nonce

Especificación: `docs/adr/ADR-0022-qyro1-frame-aead.md`. Implementación:
`rust/crates/qyro_crypto/src/aead/mod.rs`.

## La regla

```
nonce = nonce_prefix (4 bytes, derivado) || sequence (u64, big-endian)
```

Doce bytes. El prefijo es fijo para una sesión y una dirección; la secuencia
empieza en 0 y sube de uno en uno.

El nonce **no viaja**. El receptor lo reconstruye con su propio prefijo y la
secuencia que sí va en la cabecera. Un nonce no es secreto —conocerlo no ayuda a
nadie— pero repetirlo es fatal, y esa es la única propiedad que importa aquí.

## Por qué repetir un nonce es fatal

ChaCha20 es un cifrador de flujo: cifra haciendo XOR del texto claro con un
keystream determinado por (clave, nonce). Dos frames bajo el mismo par producen
el mismo keystream, así que el XOR de los dos ciphertexts **es** el XOR de los dos
textos claros. No hace falta la clave para leerlo.

Todo lo que sigue existe por esa frase.

## Quién asigna la secuencia

El sealer, siempre. `seal` toma un `&Frame`, ignora la secuencia y el
`session_id` que el llamante hubiera puesto, y escribe los suyos.

No hay API para elegir una secuencia. No es que esté desaconsejado: no existe.

## La secuencia no da la vuelta

El contador es `Option<u64>`. `checked_add(1)` devuelve `None` al llegar al
final, y a partir de ahí `seal` responde `SequenceExhausted` para siempre.

Es un `Option` y no un `u64` con un flag aparte porque «agotado» debe ser un
estado que el tipo sostiene, no una condición que alguien tenga que acordarse de
comprobar. El estado no se recupera: la sesión se acabó.

Comprobado invirtiendo el fix —cambiando `checked_add` por `wrapping_add`— y
viendo fallar `an_exhausted_sequence_is_a_terminal_error`.

## Descartar un frame no libera su secuencia

El contador avanza cuando `seal` **produce** un frame, no cuando alguien lo
envía. Si el llamante tira el `SealedFrame` a la basura, el siguiente usa la
secuencia siguiente y ese nonce queda quemado.

Es deliberado y es la opción segura. Un contador que retrocediera al descartar
tendría que saber si el frame llegó a salir, y no puede saberlo: para cuando
alguien decide descartarlo, los bytes pueden estar ya en un búfer del sistema
operativo.

## Las dos direcciones comparten el espacio de secuencias

Ambas empiezan en 0. No comparten prefijo ni clave, así que no comparten nonces:
un nonce completo es prefijo más secuencia, y los prefijos se derivan bajo
etiquetas distintas.

El test `no_nonce_is_ever_produced_twice` sella 256 frames en una dirección,
comprueba que los 256 nonces son distintos, y comprueba además que el primer
nonce de la otra dirección no está entre ellos.

## Un solo sealer por dirección

`into_frame_crypto` consume el estado establecido. No hay forma de derivar dos
sealers de la misma dirección y arrancar dos contadores desde cero, porque el
valor del que se derivan deja de existir en el momento en que se deriva el
primero.

`FrameSealer` tampoco es `Clone`, por lo mismo.
