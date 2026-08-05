# Cifrado autenticado de frames

Especificación: `docs/adr/ADR-0022-qyro1-frame-aead.md`. Implementación:
`rust/crates/qyro_crypto/src/aead/`. Vectores:
`docs/security/test-vectors/aead-v1.json`.

Este documento explica **por qué** el diseño es el que es. Los valores concretos
están en la ADR, y donde los dos discrepen manda la ADR.

## Qué garantiza y qué no

Un `AuthenticatedFrame` afirma tres cosas, todas comprobadas antes de existir:

1. el texto claro es el que selló el par de esta sesión, en esta dirección;
2. ninguno de los 48 bytes de cabecera fue alterado;
3. esa secuencia no se había aceptado antes en esta dirección.

No afirma nada más. En particular **no** afirma que el par sea de confianza:
sostener la identidad de alguien y confiar en ella son pasos distintos, y el
segundo todavía no existe. Tampoco afirma nada sobre orden de entrega: la
ventana acepta reordenamiento, así que el frame 7 puede llegar antes que el 5.

Y no hay transporte. Nada mueve estos frames a ninguna parte. Qyro sigue sin
transferir archivos.

## Por qué ChaCha20-Poly1305

Constante en software, en cualquier CPU, sin depender de instrucciones de
hardware. AES-GCM es igual de sólido pero necesita AES-NI para ser rápido *y*
resistente a canales laterales a la vez; sin ella, una implementación en software
o pierde velocidad o pierde el tiempo constante. Qyro corre en teléfonos de gama
baja, así que la alternativa no es hipotética.

No XChaCha20-Poly1305: su nonce de 24 bytes existe para poder elegirlo al azar sin
temer colisiones, y aquí el nonce es un contador. La ventaja desaparece y queda
solo un nonce que no cabe en el diseño.

## Los tres tipos

`EncryptedEnvelope` vive en `qyro_protocol` y **no afirma nada**. Es la forma en
cable de un frame cifrado: cabecera, ciphertext y unos bytes que alguien llamó
tag. Cualquiera puede construir uno con cualquier cosa. Su documentación lo dice
desde que se llamaba `SealedFrame` y no debía.

`SealedFrame` y `AuthenticatedFrame` viven en `qyro_crypto::aead` y sí afirman.
Tienen constructor privado y no hay otra forma de obtenerlos que `seal` y `open`.
Esa asimetría es el diseño entero: el tipo que puedes fabricar no promete nada, y
el tipo que promete no puedes fabricarlo.

## Los datos asociados son la cabecera entera

Los 48 bytes, sin excepciones ni campos elegidos a mano. Elegir campos significa
que algún día alguien añade uno y no lo añade a la lista.

Esto solo es correcto porque reserializar una cabecera decodificada devuelve los
mismos bytes (ADR-0018). Si un byte no se conservara, el tag se calcularía sobre
algo distinto de lo que viaja, y la comprobación no comprobaría nada.

El test que lo cubre no enumera campos: recorre los 48 bytes, voltea un bit en
cada uno y exige que ninguno produzca un frame autenticado. Un campo que nadie
recordó nombrar queda cubierto igual.

## Quién decide qué

El llamante elige tipo de mensaje, flags de transporte, los tres identificadores
de enrutado y el texto claro. El sealer elige `session_id`, `sequence`, nonce y
tag, y sobrescribe los dos primeros si el llamante los puso.

No es una comodidad de API. Un llamante que pudiera elegir la secuencia podría
repetir un nonce, y repetir un nonce en un cifrador de flujo revela el XOR de los
dos textos claros. La única forma de que eso no pase es que no haya API para
hacerlo.

## Derivación

Los secretos de tráfico del handshake **no** son claves AEAD. Cada dirección
expande el suyo con HKDF-SHA256 —solo la fase `Expand`, porque el secreto ya es
uniforme— bajo una etiqueta que contiene la dirección, y con el `auth_transcript`
y el `session_id` dentro de cada `info`.

Tres propiedades, y las tres necesitaron test propio:

- **La dirección va en la etiqueta.** Borrarla no rompía ninguna prueba
  extremo a extremo, porque los dos secretos de tráfico ya difieren: el schedule
  del handshake los deriva bajo etiquetas propias. La afirmación de la ADR es más
  fuerte que eso —las dos direcciones no pueden coincidir *aunque partieran del
  mismo secreto*— y solo un test sobre la derivación misma la sostiene.
- **El transcript y el `session_id` van en cada `info`.** Dos sesiones derivan
  claves distintas aunque un fallo futuro repitiera un secreto de tráfico.
- **El prefijo de nonce no es un trozo de la clave.** Expansiones separadas bajo
  etiquetas separadas. Un prefijo recortado de la clave filtraría cuatro bytes de
  clave en cada nonce, y un nonce no es secreto.

## Verificar antes de descifrar

`decrypt_inout_detached` calcula Poly1305 sobre el ciphertext, compara en tiempo
constante y **solo entonces** aplica el keystream. El búfer no se toca si el tag
no coincide.

Eso lo hace la biblioteca, no este repositorio: aquí no hay ningún orden de
verificación escrito a mano que alguien pueda invertir en un refactor.

## Lo que no hay

Rotación de claves, rekey y renegociación. Una sesión usa una clave por dirección
hasta agotar la secuencia, y agotarla es terminal. Con frames de 1 MiB, agotar un
contador de 64 bits requiere más datos de los que cabe transferir; no es un límite
que se alcance, es un límite que existe para no tener que dar la vuelta.
