# FASE 01 — El FFI del motor

## 1. Objetivo

**Que Dart pueda pedirle a Rust que envíe o reciba archivos por un socket, y que
las claves privadas sigan sin poder llegar a Dart.**

Al terminar esta fase existe una superficie C con la que un llamante abre una
sesión emisora o receptora, consulta su progreso, la cancela y la cierra. Nada de
UI todavía.

## 2. Por qué esta fase va primera

Hoy `qyro_ffi` expone **exactamente dos funciones**:
`qyro_protocol_version_ptr` y `qyro_protocol_version_len`. Depende únicamente de
`qyro_core`. **Todo lo construido en siete meses —motor, disco, red, confianza—
es inalcanzable desde la aplicación.**

Es el cuello de botella único: las fases 02, 03, 05 y 07 dependen de ésta y
ninguna otra la desbloquea.

**Depende de:** nada. El árbol de `R6` basta.

## 3. Estado de partida — reproduce esto antes de escribir una línea

```
cargo test --workspace          # 527 passed, 0 failed, 2 ignored
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
grep -c '^\[\[package\]\]' Cargo.lock    # 63
bash scripts/check_docs_consistency.sh
```

Y lee, enteros, antes de decidir nada:

- `rust/crates/qyro_ffi/src/` — los 60 lines que hay.
- La prueba de cierre transitivo del FFI: **la que consulta a `cargo metadata` y
  falla si `qyro_ffi` puede alcanzar `qyro_crypto`**. Encuéntrala y entiéndela
  antes de tocarla.
- `rust/crates/qyro_net/src/lib.rs` y `listener.rs` — la superficie que vas a
  envolver.
- `rust/crates/qyro_transfer/src/session.rs` — `Sender`, `Receiver`,
  `Receiver::manifest()`.
- `rust/crates/qyro_fs/src/io.rs` — `FileSource`, `FileSink`.
- ADR-0026 y ADR-0028.

## 4. La decisión central, y no la resuelvas por inercia

**`qyro_ffi` no puede alcanzar `qyro_crypto`.** No es una convención: hay una
prueba que le pregunta al compilador por el cierre transitivo de dependencias y
falla si alguien conecta los dos. El motivo está escrito desde el principio del
proyecto: *si Dart nunca puede pedir una clave, no hay forma de que una clave se
escape por ahí. La seguridad no depende de que nadie escriba mal el código;
depende de que el camino no exista.*

**Y `qyro_net` sí depende de `qyro_crypto`**, porque el handshake vive ahí. Así
que conectar `qyro_ffi` a `qyro_net` **rompe esa guarda**.

Tres salidas. **Evalúalas de verdad, elige una, y escribe qué pierdes:**

**(a) Precisar la guarda.** Cambiarla de «no puede alcanzar el crate» a «no puede
alcanzar los tipos que llevan material de clave». Más exacta y **más frágil**: el
día que alguien añada un tipo nuevo con una clave dentro, la guarda no lo sabe
salvo que la lista se mantenga. Si eliges ésta, **la guarda nueva tiene que verse
fallar** con un intento real de exponer material de clave, y la lista de tipos
tiene que auto-caducar como hacen las excepciones de
`MINIMUM_GUARD_SET_EXCEPTIONS`.

**(b) Un crate intermedio `qyro_session`.** Lo único que `qyro_ffi` ve. Expone
operaciones —abrir, avanzar, cancelar, cerrar— y **ningún tipo que contenga una
clave**. La guarda original se conserva intacta y se le añade una segunda:
`qyro_session` no reexporta nada de `qyro_crypto`. Cuesta un crate.

**(c) No cruzar la red esta fase.** Sólo motor y disco por el FFI; la red se queda
detrás. Deja el producto a medias y obliga a rehacer la superficie en la fase 02.

**Mi lectura, que no es una orden:** (b) es la que conserva la propiedad que este
proyecto trata como sagrada sin debilitarla, y el coste —un crate de primera
parte, cero dependencias— es el que este proyecto paga sin pestañear. (c) es
tiempo perdido. (a) sólo si demuestras que la guarda nueva es tan difícil de
romper por accidente como la vieja.

**Lo que elijas va en `docs/adr/ADR-0032-engine-ffi.md`, congelada antes del
código, y es comprobable en el historial.**

## 5. Lo demás que la ADR-0032 tiene que congelar

**5.1 — El modelo de objetos.** Rust no puede devolver structs con vida propia a
Dart. Lo estándar son **handles opacos**: un entero que identifica una sesión viva
en una tabla del lado Rust. Decide:

- entero o puntero, y por qué;
- **cómo se garantiza que un handle no se confunda con otro** — un contador que
  empieza en 1 y no reutiliza índices, o un tag en los bits altos;
- **qué pasa si Dart pierde el handle sin cerrarlo** — ¿fuga hasta el final del
  proceso, o hay un barrido?;
- **qué pasa con un handle de una sesión ya cerrada**: tiene que ser un error
  tipado, nunca un pánico y nunca un acceso a memoria liberada.

**5.2 — Los errores.** Rust tiene `Result`; C no. Códigos de error enteros,
parámetros de salida, o un `last_error` por hilo. Decide, y responde a esto: **un
error ocurre en un hilo de Rust que Dart no creó — ¿cómo se entera Dart?** Si la
respuesta es «en la siguiente llamada», dilo.

**5.3 — Cadenas y búferes.** Quién asigna, quién libera. Y **qué pasa si Dart
libera dos veces**. La regla que este proyecto ya usa en su lado nativo: quien
asigna, libera; y hay una función `qyro_free_*` por cada cosa que Rust devuelve.

**5.4 — Hilos.** El motor corre en un hilo de Rust. Dart llama desde su isolate.
Decide qué llamadas son seguras desde cualquier hilo y cuáles no, y **dilo en el
nombre o en la firma**, no sólo en un comentario.

**5.5 — Pánico.** Un `panic!` que cruza una frontera `extern "C"` es **undefined
behaviour**. Toda función `extern "C"` tiene que atrapar el pánico
(`std::panic::catch_unwind`) y convertirlo en un código de error. **Esto no es
opcional y no se decide: se hace.** Y hay una prueba que lo demuestra con un
pánico provocado a propósito.

**5.6 — Lo que esta decisión no promete.** Sección obligatoria.

## 6. Lo que hay que construir, paso a paso

### Paso 1 — ADR-0032 congelada

Antes de una línea de código. Commit propio.

**Puerta.**

### Paso 2 — La estructura elegida en §4

Si es (b): el crate `qyro_session`, con su `guards.rs` y el conjunto mínimo, y la
guarda nueva de que no reexporta nada de `qyro_crypto`, **vista fallar**.

Si es (a): la guarda precisada, **vista fallar** con un intento real de exponer
material de clave.

**Puerta.** Con una comprobación específica: **la prueba de cierre transitivo, sea
cual sea su forma nueva, tiene que fallar cuando la violas a propósito.** Si no
falla, no has construido una guarda: has construido un comentario.

### Paso 3 — La tabla de handles

- Creación, consulta y destrucción, con la política de §5.1.
- Pruebas: **doble cierre**, **handle inválido**, **handle de otra sesión**,
  **handle cero**. Los cuatro devuelven error tipado.
- Y la prueba de §5.5: un pánico provocado dentro de una función `extern "C"` sale
  como código de error.

**Puerta.**

### Paso 4 — La superficie mínima

Seis operaciones, y **ni una más de las que la fase 02 vaya a usar**:

| Operación | Qué recibe | Qué devuelve |
|---|---|---|
| Abrir sesión **emisora** | dirección `ip:puerto`, lista de rutas, directorio raíz | handle o error |
| Abrir sesión **receptora** | puerto, directorio destino | handle o error |
| **Avanzar** la sesión | handle | estado: en curso / terminada / rechazada / error |
| Consultar **progreso** | handle | bytes transferidos, bytes totales, item actual |
| **Cancelar** | handle | ok o error |
| **Cerrar** | handle | ok o error |

**El progreso se consulta en esta fase**; el callback empujado es la fase 02.
Hacerlo así separa dos cosas que fallan distinto.

**Puerta.**

### Paso 5 — El barrido y las guardas

- `cargo-mutants` sobre lo nuevo, con `--timeout 90`.
- `guards.rs` con el conjunto mínimo en cualquier crate nuevo.
- Y comprueba que `every_workspace_crate_has_the_minimum_structural_guards_or_an_exact_exception`
  sigue en verde: **si creaste un crate, o le pones sus guardas o le pones una
  excepción con la razón escrita**.

**Puerta de fase.**

## 7. Las trampas concretas de esta fase

1. **El pánico que cruza la frontera.** Es UB, no un crash limpio. `catch_unwind`
   en cada `extern "C"`, con prueba.
2. **El doble cierre.** Si Dart cierra un handle dos veces y la tabla no lo
   detecta, en el mejor caso es un error y en el peor es un uso después de
   liberar. Prueba obligatoria.
3. **La guarda relajada por comodidad.** Si al final la prueba de cierre
   transitivo «no aplica» o se le añade un `allow`, has borrado la propiedad más
   antigua del proyecto. **La guarda nueva tiene que verse fallar.**
4. **La superficie que crece.** Cada función `extern "C"` es superficie que hay
   que auditar para siempre. Seis operaciones. Si necesitas la séptima, escribe
   por qué.
5. **`qyro_net` en Windows (QYR-0078).** Está abierta y puede que el job
   `rust-windows` ya la haya cerrado sin que nadie lo mirara. **Compruébalo y
   cierra o actualiza la ficha.**

## 8. Pruebas obligatorias

- `a_double_close_is_an_error_and_not_a_crash`
- `an_invalid_handle_is_refused_by_name`
- `a_panic_inside_the_c_boundary_becomes_an_error_code`
- `the_ffi_cannot_reach_key_material` — la guarda de §4, en su forma nueva
- `a_sender_and_a_receiver_opened_through_the_ffi_move_a_file` — dos procesos
  reales, conducido desde la API C, archivo comparado **byte a byte**
- `a_cancelled_session_leaves_no_part_file`
- `closing_a_session_leaves_no_thread_and_no_descriptor` — con su prueba de que la
  medida vería la fuga (`R2` §1.7)

## 9. Criterios de aceptación

1. ADR-0032 congelada antes del primer commit de código, comprobable en el
   historial.
2. **La decisión de §4 tomada, argumentada, implementada, y la guarda resultante
   vista fallar.**
3. Las seis operaciones existen y no hay una séptima sin justificar.
4. Los cuatro casos de handle inválido devuelven error tipado.
5. Un pánico dentro de una función `extern "C"` sale como código de error, con
   prueba que lo provoca.
6. **Un archivo cruza dos procesos reales conducido desde la API C**, comparado
   byte a byte.
7. Ni hilo ni descriptor ni `.qyro-part` sobrevive a una sesión cerrada o
   cancelada.
8. **Cero dependencias externas.** Di el conteo de `Cargo.lock` con el comando.
9. Barrido con `cargo-mutants` sobre lo nuevo, con alcance declarado, **sin
   supervivientes de las tres familias de riesgo sin ficha**.
10. QYR-0078 cerrada o actualizada con evidencia.
11. Las doce comprobaciones de `R2` en todas las puertas.
12. Los workflows en verde sobre el commit final, con la tabla de runs exhaustiva.
13. Informe `docs/reports/fase-01-ffi-del-motor.md` según `R5`.
14. **Los botones siguen `onPressed: null`.**

## 10. Cómo tiene que quedar el resultado

Un programa en C —o el test de integración que hace de programa en C— puede:

```
handle = qyro_send_open("127.0.0.1:9000", ["/tmp/a.bin"], "/tmp/origen");
while (qyro_step(handle) == QYRO_IN_PROGRESS) { qyro_progress(handle, &done, &total); }
qyro_close(handle);
```

…y en la otra punta un proceso receptor escribe el archivo verificado. **Y la
prueba de cierre transitivo sigue impidiendo que ese programa pueda pedir una
clave.**

## 11. No objetivos

- **Callbacks de progreso empujados.** Es la fase 02. Aquí se consulta.
- **Dart.** Ni una línea. La fase 01 se prueba desde Rust.
- **UI, selector de archivos, descubrimiento.**
- **Emparejamiento por el FFI.** La confianza existe en Rust; exponerla es la
  fase 04.
- Keystore, Keychain, empaquetado.

## 12. Qué desbloquea

**Todo.** Las fases 02, 03, 05 y 07 dependen de esta superficie. Si queda mal
diseñada, se rehace cuatro veces.
