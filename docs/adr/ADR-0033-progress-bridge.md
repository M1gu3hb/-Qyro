# ADR-0033 — El puente de progreso, y por qué su presupuesto es una constante

- **Estado:** aceptada
- **Fecha:** 2026-08-13
- **Fase:** 02, paso 1
- **Sustituye a:** nada. Extiende ADR-0032 §6 y ensancha dos de sus seis
  operaciones, que es la razón de que esto sea una ADR y no un cambio.

---

## Contexto

La fase 01 dejó seis operaciones `extern "C"` y ninguna forma de que Dart se
entere de nada mientras una transferencia ocurre. `qyro_session_progress` existe
y funciona, pero es **sondeo**: Dart tendría que preguntar en un temporizador,
y un temporizador es o demasiado lento para la barra o demasiado rápido para el
event loop.

La fase 02 §4 decide la primitiva y la investigación ya está hecha:
`NativeCallable.listener` de `dart:ffi`, del SDK, cero paquetes. Lo que esta ADR
decide es **la forma exacta del puente**, congelada antes del código.

`flutter_rust_bridge` está descartado y no se reabre: 47–60 crates medidos con
`cargo tree` el 2026-08-11, con `tokio`, `regex` y `backtrace`, y su propósito
—exponer el API de un crate a Dart— es lo contrario de la invariante que
ADR-0032 §2 defiende.

---

## 1. La medición que decide la frecuencia

Es el número que gobierna todo lo demás, así que va primero.

El motor mueve **chunks de 64 KiB**. Una emisión por chunk da:

| Archivo | Emisiones a una por chunk |
|---|---|
| 8 MiB | **128** |
| 100 MiB | **1 600** |
| 1 GiB | **16 384** |

Cada llamada de un `NativeCallable.listener` **encola un mensaje en el event loop
del isolate que lo creó**. Dieciséis mil mensajes para mover un gigabyte no es
una barra de progreso: es una inundación del event loop que compite con el
propio dibujado de la barra.

**Y crece con el tamaño del archivo**, que es la propiedad inaceptable. La v1.0
tiene que mover un archivo de más de un gigabyte por una Wi-Fi doméstica.

**Decisión: el número de emisiones está acotado por una constante, no por el
tamaño.**

---

## 2. Decisión: la firma

```c
/* Cero campos que necesiten liberarse. Cero punteros. Retorno void. */
typedef void (*qyro_progress_fn)(uintptr_t context,
                                 uint64_t  done,
                                 uint64_t  total,
                                 uint32_t  item);
```

Y las dos operaciones de apertura ganan **dos** parámetros, al final:

```c
int32_t qyro_session_open_sender_blocking(
    const uint8_t *address, uintptr_t address_len,
    const uint8_t *root,    uintptr_t root_len,
    const uint8_t *paths,   uintptr_t paths_len,
    qyro_progress_fn on_progress,   /* nullable */
    uintptr_t        context,
    uint64_t        *out_handle);

int32_t qyro_session_open_receiver_blocking(
    const uint8_t *bind,        uintptr_t bind_len,
    const uint8_t *destination, uintptr_t destination_len,
    qyro_progress_fn on_progress,   /* nullable */
    uintptr_t        context,
    uint64_t        *out_handle);
```

**Los tres campos son exactamente los de `qyro_session_progress`.** Deliberado:
una superficie con dos formas distintas de decir «progreso» invita a que una de
las dos se quede atrás, y ya hay dos defectos abiertos sobre esos campos
(QYR-0317, QYR-0318). Una forma, un sitio donde arreglarlos.

**Al final de la lista de parámetros, y no en medio.** Un parámetro insertado en
medio de una firma `extern "C"` compila en Dart sin decir nada y pasa los
argumentos corridos de sitio.

### Por qué un `uintptr_t` de contexto y no el handle

El handle todavía no existe cuando se llama a la apertura: **la apertura es quien
lo crea**. Una emisión durante el handshake no podría llevarlo. El contexto lo
pone Dart, Rust no lo interpreta jamás, y sólo lo devuelve.

---

## 3. Las cuatro reglas, y cómo las cumple la forma

La fase 02 §4 pide que las cuatro vivan en la ADR y en el código, no en un
comentario. Tres de las cuatro las cumple la **forma**, que es más fuerte que
cumplirlas por disciplina.

| # | Regla | Cómo se cumple |
|---|---|---|
| 1 | **Sólo `void`.** Para cancelar, una bandera atómica, nunca el retorno | Por construcción: el `typedef` devuelve `void`, así que no hay valor que leer. La cancelación ya existe y ya es correcta — `qyro_session_cancel` levanta un `AtomicBool` y **no toma el candado de la sesión**, porque tomarlo haría que cancelar esperase al paso que intenta interrumpir (ADR-0032 §7) |
| 2 | **`close()` obligatorio** o el isolate no muere | No se puede cumplir desde Rust: es una obligación de Dart. Va en `try/finally`, y la clase que envuelve el handle la cumple en `dispose()`. **Esta es la única de las cuatro que descansa en disciplina**, y se dice |
| 3 | **La llamada es diferida: sólo escalares** | Por construcción: los cuatro parámetros son enteros. No hay puntero que pueda apuntar a una pila de Rust que ya no existe cuando Dart lo mire |
| 4 | **Llamar al `nativeFunction` después de `close()` es UB** | Por ordenación, §5 |

---

## 4. La frecuencia, congelada

```
PROGRESS_TARGET_EMISSIONS = 100
PROGRESS_MIN_STEP         = 256 KiB
paso = max(PROGRESS_MIN_STEP, total / PROGRESS_TARGET_EMISSIONS)
```

**Se emite cuando `done - último_emitido >= paso`.** Más:

- **una emisión de apertura**, con `done = 0` y el `total` ya conocido, para que
  la barra sepa su escala antes del primer byte;
- **una emisión terminal**, siempre, cuando la sesión alcanza un estado terminal,
  aunque no toque paso. **Sin ella la barra se queda en el 99 %**, que es el
  fallo visible más común de este patrón.

**Cota: 102 llamadas por sesión, sea cual sea el tamaño.**

| Archivo | Paso | Emisiones | Frente a una por chunk |
|---|---|---|---|
| 2 MiB | 256 KiB | ~10 | 32 |
| 8 MiB | 256 KiB | ~34 | 128 |
| 100 MiB | 1 MiB | ~102 | 1 600 |
| 1 GiB | 10,7 MiB | ~102 | 16 384 |

Por debajo de 25 MiB manda `PROGRESS_MIN_STEP` y hay **menos** de 100
emisiones; por encima manda la fracción y hay **exactamente** 100 más las dos.
Las dos ramas están acotadas y ninguna crece con el archivo.

**Por qué un umbral de bytes y no de tiempo.** Un umbral de tiempo es
indeterminista, y la fase 02 §5 exige *«una prueba que cuente cuántas llamadas se
emiten para un archivo de tamaño conocido y compruebe que está dentro del
presupuesto»*. Una prueba sobre un reloj no cuenta nada reproducible: pasaría en
una máquina y fallaría en la de al lado. Un umbral de bytes da el **mismo** número
en las dos.

### La prueba de que la medida vería el exceso

`R2` §1.7, y no es opcional: la prueba del presupuesto viene con
**`an_emission_per_chunk_would_be_visible_to_this_measurement`**, que fuerza el
paso a un chunk y comprueba que el contador lo detecta.

Y la forma tiene que distinguir un contador medido de una constante, que es la
sexta trampa de `R1` §5: **dos tamaños y una desigualdad estricta.** Un archivo
grande tiene que emitir **estrictamente más** que uno pequeño por debajo del
codo, porque una implementación que emitiera siempre 102 —o siempre 2— satisface
cualquier cota superior y no está midiendo nada.

---

## 5. Contrapresión, y el apagado

**Dart no puede ir más lento que Rust de forma no acotada, porque el emisor está
acotado.** Peor caso, 102 mensajes de tres enteros por sesión, y como mucho
cuatro sesiones vivas (`MAX_ESTABLISHED_SESSIONS`): **408 mensajes**. Eso no es
contrapresión, es una cola. Ésa es la respuesta a la pregunta de la fase 02 §4, y
es una respuesta que sólo vale **porque** §4 acota.

**No hay descarte, ni compactación, ni «quédate con el último».** Un mecanismo de
descarte podría tirar la emisión terminal, que es la única que no se puede
perder.

**El apagado lo ordena Rust y Dart cierra al recibirlo** (fase 02 §4, regla 4):

1. Rust emite la emisión terminal **desde dentro** de la llamada a
   `qyro_session_step_blocking` que devuelve un estado terminal.
2. Esa llamada retorna. **A partir de ese retorno, Rust no vuelve a invocar el
   puntero para ese handle.**
3. Dart, al ver el estado terminal, hace `close()` del `NativeCallable` y
   `qyro_session_close` del handle.

**La ordenación es lo que hace la regla 4 comprobable:** toda emisión ocurre
dentro de una llamada que Dart hizo y que todavía no ha vuelto, así que Dart no
puede estar cerrando el callback mientras Rust lo invoca — estaría bloqueado en
`step`. Un hilo de fondo que emitiera por su cuenta no tendría esa garantía, y es
la razón de que **no lo haya**.

---

## 6. Desde qué hilo

**Desde el que llame a `qyro_session_step_blocking`**, que es el que Dart pone en
un isolate ayudante. Es lo que hace falta `NativeCallable.listener` en vez de
`isolateLocal`: la documentación del SDK dice *«can be invoked from any thread»*,
y aunque hoy sea siempre el hilo que Dart eligió, atarlo a eso sería atarse a un
detalle que la fase 05 puede cambiar.

**No se crea ningún hilo en Rust para esto.** ADR-0028 congela `std::net` con
hilos y sin async, y un hilo emisor propio rompería la ordenación de §5.

---

## 7. Lo que esta decisión NO promete

- **No promete que la barra sea suave.** 102 emisiones sobre un gigabyte son una
  cada 10 MiB; en una Wi-Fi lenta eso puede ser una actualización cada varios
  segundos. Es una decisión, no un descuido: la alternativa es inundar el event
  loop. Si la fase 05 demuestra con una medición que hace falta más, se sube
  `PROGRESS_TARGET_EMISSIONS` y se vuelve a medir.
- **No promete progreso en el lado receptor.** QYR-0317 está abierta: el receptor
  no asigna `done` en absoluto. El puente lo transportará en cuanto exista; hoy
  transportaría ceros.
- **No promete que `item` sea útil.** QYR-0318 está abierta: no se asigna nunca.
  El campo cruza porque la forma tiene que ser la definitiva, no porque hoy diga
  algo.
- **No promete nada sobre hardware físico.** Ni un aparato ha ejecutado esto.
- **No promete que Dart cierre.** La regla 2 es disciplina de Dart y no hay forma
  de que Rust la imponga. Lo que sí se puede es que el test lo compruebe, y
  `closing_from_dart_leaves_no_handle_and_no_thread` es esa prueba.

---

## Alternativas descartadas

| Alternativa | Por qué no |
|---|---|
| **Sondeo con `qyro_session_progress` desde un temporizador en Dart** | Ya es posible y no hace falta ADR. Se descarta porque el temporizador es o demasiado lento para la barra o demasiado rápido para el event loop, y porque no puede saber cuándo la sesión terminó sin sondear también el estado |
| **Una emisión por chunk** | §1. Crece con el archivo: 16 384 llamadas por gigabyte |
| **Umbral de tiempo** | Indeterminista, y la fase exige contar emisiones para un tamaño conocido |
| **Cola con descarte, «quédate con el último»** | Puede tirar la emisión terminal, que es justo la que no se puede perder |
| **Un hilo emisor en Rust** | Rompe la ordenación de §5, que es lo que hace comprobable la regla 4, y contradice ADR-0028 |
| **Pasar una cadena con el nombre del archivo** | Un puntero en una llamada diferida apunta a memoria que ya no existe. Si algún día hace falta, se copia al heap y se libera con una función propia — y eso es otra ADR |
| **Un séptimo `extern "C"` para registrar el callback después de abrir** | Deja una ventana entre abrir y registrar en la que las emisiones se pierden, y un estado más que representar: sesión abierta sin observador todavía |
