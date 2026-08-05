# Ventana de replay

Especificación: `docs/adr/ADR-0022-qyro1-frame-aead.md`. Implementación:
`rust/crates/qyro_crypto/src/aead/replay.rs`.

## Qué es

Ventana fija de 1024 secuencias: la mayor aceptada hasta ahora, más un bitmap de
1024 bits (`[u64; 16]`) donde el bit `n` marca `mayor - n`.

Responde una sola pregunta —*¿se aceptó ya esta secuencia en esta dirección?*— y
la responde en **dos pasos separados a propósito**.

## Los dos pasos

`check` decide y no modifica nada. `record` confirma. El AEAD corre entre los
dos.

```
1. longitud del tag
2. SessionId
3. window.check(sequence)      <- no modifica nada
4. AEAD: verificar y descifrar
5. window.record(sequence)     <- solo si el paso 4 pasó
```

Esa separación es la propiedad de seguridad entera. Si la ventana se actualizara
al leer la secuencia, cualquiera —sin clave, sin haber participado en el
handshake, solo capaz de poner bytes en el cable— podría enviar
`sequence = u64::MAX - 1` con dieciséis bytes al azar como tag y dejar la sesión
incapaz de aceptar nada más. Un ataque de denegación de servicio que no cuesta
nada montar.

Comprobado invirtiendo el orden: mover `record` por delante del AEAD hace fallar
`a_failed_authentication_does_not_move_the_replay_window` y
`tampering_with_ciphertext_or_tag_fails`.

Por lo mismo, un frame que nombra otra sesión se rechaza **antes** del paso 3: el
tráfico de una sesión ajena no puede costarle nada a esta.

## Por qué un bitmap y no un contador

Las redes reordenan. Una ventana que solo aceptara orden estricto rompería
transferencias legítimas, así que se acepta el desorden dentro de las últimas
1024 secuencias y se rechaza lo que caiga por detrás.

Lo que cae por detrás se rechaza porque ya **no se puede distinguir** de un
replay, no porque se sepa que lo es. `SequenceTooOld` dice eso y no otra cosa.

## `highest_seen` es un `Option`

La secuencia 0 es un primer frame legítimo. Un centinela de 0 haría que «ventana
vacía» y «ya acepté el frame 0» fueran el mismo estado, y el segundo frame 0 —el
replay— pasaría o el primero se rechazaría; en cualquier caso, mal.

## Saltos hacia delante

Un salto grande es pérdida de paquetes, no un ataque, y se acepta. Lo que no
puede pasar es que el desplazamiento deje bits antiguos en su sitio: marcarían
como aceptadas secuencias que nunca llegaron, y los frames reales del par
parecerían replays. Cuando el salto supera la ventana, el bitmap se pone a cero
entero.

`shifting_carries_bits_across_word_boundaries` pasea un bit por cada palabra del
bitmap con ocho desplazamientos distintos —1, 63, 64, 65, 127, 128, 512, 1023—,
que es donde un bitmap escrito a mano se equivoca.

## Una ventana por dirección

Cada `FrameOpener` tiene la suya. Reproducir un frame en una dirección no toca la
otra, aunque ambas usen la secuencia 0.

`FrameOpener` no es `Clone`: dos openers tendrían dos ventanas, y un frame
rechazado por uno sería aceptado por el otro.

## Límites

La ventana protege dentro de una sesión. No hay nada que impida a un atacante
reproducir un handshake completo, porque no hay transporte todavía y por tanto no
hay nada a lo que reproducirlo. Cuando exista, ese es un problema distinto y
tendrá su propio análisis.
