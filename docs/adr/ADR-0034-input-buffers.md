# ADR-0034 — Quién asigna los búferes que Dart pasa

- **Estado:** aceptada
- **Fecha:** 2026-08-14
- **Fase:** 02, paso 3
- **Enmienda:** ADR-0032 §6, primera frase. La segunda mitad de esa frase deja de
  ser cierta y esta ADR dice por qué y con qué se sustituye.

---

## Contexto

ADR-0032 §6 congeló: **«Dart posee todo lo que pasa; Rust posee todo lo que
devuelve. Nunca mezclado.»** La entrada son pares `(*const u8, usize)`, y Rust
copia al entrar sin retener el puntero más allá de la llamada.

Al escribir el lado Dart aparece el problema que esa frase no anticipó:

> **Dart no puede asignar memoria nativa.** `dart:ffi` no trae asignador. El
> `malloc`/`calloc` que todo el mundo usa vive en `package:ffi`, que es un
> paquete de pub.dev.

Y el criterio de aceptación 10 de la fase 02 dice, literalmente, **«cero paquetes
nuevos de pub.dev»**. `pubspec.lock` tiene hoy 39 paquetes y **`ffi` no está entre
ellos**; añadirlo sería exactamente el paquete nuevo que el criterio prohíbe.

La propia ADR-0032 §6 vio venir que aquí faltaba algo: *«La fase 02 necesitará un
liberador de verdad y eso lleva su propia cláusula de ADR, no llega de refilón.»*
Esto es esa cláusula, y resulta que hace falta para la **entrada**, no sólo para
la salida.

---

## 1. Las tres salidas, medidas

| Salida | Coste | Veredicto |
|---|---|---|
| **`package:ffi`** | Un paquete nuevo de pub.dev | **No.** Criterio 10. Y es una dependencia para una función de doce líneas |
| **`@Native(isLeaf: true)` + `TypedData.address`** | Cero paquetes, cero superficie nueva | **No, y por una razón de plataforma, no de gusto.** `TypedData.address` es exactamente para esto —un puntero válido sólo durante una llamada *leaf*, que es justo lo que ADR-0032 §6 promete al copiar al entrar—, pero `@Native` sin `assetId` resuelve el símbolo **en el proceso**, y `DynamicLibrary.process()` no está soportado en Windows. Los *native assets* de Flutter que darían `assetId` siguen siendo experimentales. Una solución que funciona en Android y no en Windows no sirve para un producto de tres plataformas |
| **Dos funciones en `qyro_ffi`** | Ensancha una superficie congelada, y por eso está aquí | **Sí** |

---

## 2. Decisión

```c
/* Devuelve `len` bytes escribibles, o NULL si `len` es 0 o la asignación falla.
   La memoria es de Rust de principio a fin. */
uint8_t *qyro_buffer_alloc(uintptr_t len);

/* Libera lo que devolvió qyro_buffer_alloc. `len` DEBE ser el mismo que se pidió.
   NULL es un no-op. */
void qyro_buffer_free(uint8_t *ptr, uintptr_t len);
```

**La regla de ADR-0032 §6 se reescribe así, y es más simple que la anterior:**

> **Rust posee todos los búferes que cruzan, en las dos direcciones. Dart nunca
> posee memoria nativa: la pide prestada para llenarla y la devuelve.**

No es un debilitamiento, es la eliminación de una clase entera de error. La frase
original repartía la propiedad en dos y dejaba a Dart la mitad que Dart **no
puede** cumplir sin una dependencia. Un solo dueño no puede confundirse consigo
mismo.

### El contrato, y qué lo hace comprobable

- **`len` debe coincidir.** Un `Vec<u8>` se reconstruye con capacidad igual a la
  longitud, así que liberar con otro `len` es comportamiento indefinido. Es la
  única obligación real que se le pide a Dart, y la clase que la envuelve la
  cumple guardando la longitud junto al puntero, no pidiéndosela al llamante.
- **`len == 0` devuelve `NULL`,** y `NULL` se libera sin efecto. Una lista de
  archivos vacía y un búfer vacío dejan de ser casos especiales en el lado Dart.
- **La asignación puede fallar y lo dice devolviendo `NULL`.** No hay `abort` por
  falta de memoria: `Vec::try_reserve`, no `vec![0; len]`.
- **Ninguna de las dos entra en pánico**, así que ninguna necesita la frontera de
  ADR-0032 §8 — y aun así la llevan, porque una excepción a la forma es una
  excepción que alguien tiene que recordar.

---

## 3. Por qué esto no reabre lo que ADR-0032 cerró

**No toca la invariante de nombrabilidad.** Las dos funciones viven en
`qyro_ffi`, no nombran ningún tipo de `qyro_session` ni de `qyro_crypto`, y el
grafo de dependencias no cambia en un solo paquete. La guarda
`the_ffi_names_exactly_two_crates` sigue diciendo lo mismo.

**No introduce propiedad compartida.** En ningún instante hay dos dueños: Rust
asigna, Dart escribe dentro de un búfer prestado, Rust lee durante la llamada y
Rust libera. La regla «Rust copia al entrar y no retiene el puntero del llamante»
de ADR-0032 §6 sigue valiendo palabra por palabra.

**Sube el conteo de símbolos `extern "C"` de ocho a diez**, y eso se dice en vez
de que aparezca en un diff. La fase 01 declaró ocho: dos de versión de protocolo
y seis de sesión.

---

## 4. Lo que esta decisión NO promete

- **No promete que sea rápido.** Cada apertura de sesión asigna y libera tres o
  cuatro búferes pequeños. Si algún día eso importa, la respuesta es una arena
  por sesión, y sería otra ADR.
- **No promete proteger de un `len` equivocado.** No se puede desde C. Lo que sí
  se hace es que ningún código de Dart escrito en este repositorio tenga que
  acordarse: la longitud viaja pegada al puntero dentro de la clase que los
  envuelve, y el `finally` que libera es el mismo que asigna.
- **No promete nada sobre la salida.** El liberador que ADR-0032 §6 anticipaba
  para lo que Rust devuelve **sigue sin existir**, porque sigue sin haber nada
  que devolver que necesite liberarse. Cuando lo haya, será otra cláusula.
- **No se ha ejecutado en ninguna plataforma al escribir esto.** Está congelada
  antes del código, que es el punto.
