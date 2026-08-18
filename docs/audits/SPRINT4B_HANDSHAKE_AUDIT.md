# Auditoría del handshake — cierre del sprint 4B

Ámbito: lo que el sprint 4B entregó y lo que el sprint 4B.1 corrigió sobre ello.
Rama del cierre: `claude/qyro-handshake-closure`, sobre `436bdc4`.

Este documento registra defectos reales con la prueba que los expone. No es un
resumen del diseño; para eso está `authenticated-handshake.md`.

## Método

Cada hallazgo se comprobó **borrando la corrección y comprobando que alguna
prueba falla**. Cuando ninguna falla, la propiedad no estaba cubierta, por
convincente que fuera el argumento a favor. Ese método encontró en el sprint 4B
que el enlace de la firma del iniciador no aportaba nada, y en este sprint que
dos de las cuatro correcciones no eran las que parecían.

## Hallazgos

### H-1 · El identificador de sesión no encajaba consigo mismo

`qyro_protocol` guardaba `session_id` como un `u64` desnudo. `qyro_crypto`
derivaba 32 bytes bajo la etiqueta `session-id`. No había ninguna conversión
entre ambos.

Nada fallaba **todavía**, porque nada conectaba los dos crates. El defecto es lo
que habría pasado después: el primer código que pusiera un identificador de
handshake en un frame habría tenido que inventar un truncamiento —¿los primeros
ocho bytes? ¿los últimos?— y esa decisión sobre un formato congelado la habría
tomado quien conectara el transporte, en un call site, sin ADR.

Corrección: `qyro_protocol::SessionId`, ocho bytes, un solo tipo, usado por los
dos lados. HKDF-Expand se llama pidiendo ocho bytes, que es una derivación
completa y no una clave recortada.

Severidad: P1. Prueba: `session_id_contract.rs`,
`the_handshake_derives_the_wire_session_identifier`.

### H-2 · El respondedor quedaba establecido antes de entregar su último mensaje

`receive_initiator_finish` devolvía los bytes de `ResponderFinish` **y** un
`EstablishedResponder`. En ese punto el peer no ha visto el mensaje que cierra el
handshake y puede no verlo nunca.

Severidad: P1. No es explotable por un atacante de red; es una forma de que el
código propio use una sesión que el otro lado no cree que exista.

Corrección: `ResponderFinishPending`, que ofrece los bytes y ninguna clave, y
`confirm_sent()` que lo consume. Prueba:
`the_responder_is_not_established_until_it_confirms_delivery`.

### H-3 · Las claves de sesión estaban en la API pública

`SessionKey` se exportaba desde la raíz del crate y ambos estados establecidos
entregaban referencias con `sending_key()` y `receiving_key()`.

Severidad: P2. Nada fuera del crate tiene uso para un secreto de tráfico crudo, y
cada poseedor adicional es otro sitio que debe acertar con la zeroización, el
logging y la serialización.

Corrección: tipo privado del crate, accesores eliminados, y `into_secrets` como
única costura interna. Prueba:
`no_session_key_handle_is_reachable_from_the_public_api`.

### H-4 · La entropía efímera podía fabricarse

El adaptador RNG respondía a cualquier lectura posterior a la primera, y a
cualquier lectura de más, **rellenando ceros y devolviendo éxito**. El comentario
que lo defendía decía que un handshake con claves obviamente muertas es mejor
que uno que reutiliza entropía.

La premisa es falsa. Un secreto X25519 de ceros se *clampea* a un escalar válido
y completa un handshake perfectamente normal que no contiene entropía. Es
exactamente el resultado que `EntropyUnavailable` existe para impedir.

Severidad: **P0**, condicionada: el camino no era alcanzable con la versión
fijada de `x25519-dalek`, que hace exactamente una lectura de 32 bytes. Era una
trampa esperando una actualización de dependencia, no un fallo activo.

Al corregirlo apareció algo que el brief no anticipaba: **el adaptador no se
podía arreglar**. `EphemeralSecret::random_from_rng` exige un `CryptoRng`, cuyo
`fill_bytes` es infalible; ningún adaptador que lo alimente puede informar de
agotamiento. El fallback estaba forzado por la forma del trait, no por descuido.

Corrección: construir el secreto directamente desde bytes con
`StaticSecret::from`, envuelto en un tipo propio que no es `Clone` y cuyo
`diffie_hellman` consume `self`, recuperando la garantía de un solo uso que el
nombre `StaticSecret` cede. Eso **elimina** el modo de fallo en vez de
gestionarlo. Prueba: `no_code_path_can_substitute_bytes_for_entropy`.

## Correcciones a este propio análisis

Dos cosas que se creyeron y resultaron falsas al comprobarlas:

1. La primera versión de la regla de evidencia de plataforma en
   `check_docs_consistency` marcaba «iOS/Android hardware físico: **NO**» como
   una afirmación positiva sin run id. La causa: la búsqueda insensible a
   mayúsculas de `SI` encontraba el «si» dentro de «fí**si**co». La regla ahora
   exige `YES` como palabra completa.
2. La primera versión de la prueba de entropía escaneaba el texto de `mod.rs`
   buscando `random_from_rng` y fallaba contra el propio comentario que explica
   por qué ese constructor se rechaza. La prueba ahora filtra los comentarios
   antes de mirar.

Ninguna de las dos era un defecto del producto, y las dos habrían quedado como
reglas que parecen estrictas y no lo son.

## Lo que sigue sin verificarse

- **Auditoría criptográfica independiente**: no existe. Los KAT prueban que las
  primitivas cumplen sus RFC; no prueban que la *composición* sea correcta.
- **Campaña real de fuzzing**: hay corpus y smoke, no exploración.
- **Segunda implementación**: los vectores existen precisamente para que alguien
  escriba una en Swift o Kotlin y descubra las ambigüedades que quedan. Hasta que
  eso pase, «formato definido sin ambigüedad» es una intención, no un hecho
  comprobado.
- **Hardware físico**: nada de esto se ha ejecutado fuera de host, emulador y
  simulador.
