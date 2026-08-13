# FASE 02 — Dart conduce una transferencia

## 1. Objetivo

**Que un test en Dart mueva un archivo real entre dos procesos, por un socket, y
lo verifique byte a byte — con el progreso llegando a Dart mientras ocurre.**

Es la primera vez en la vida de este proyecto que Dart hace algo real.

## 2. Por qué esta fase va aquí

**Depende de:** fase 01 (la superficie C).

**Y es la que convierte la fase 01 en evidencia.** Una superficie C probada sólo
desde Rust no demuestra que Dart pueda usarla: los tipos, la vida de los búferes y
el modelo de hilos son exactamente donde un puente FFI se rompe, y ninguna de esas
tres cosas la ejercita un test escrito en Rust.

## 3. Estado de partida

Reproduce los números de la fase 01 y además:

```
cd apps/qyro && flutter pub get && flutter test
```

Y lee:

- `apps/qyro/lib/` entero — es poco: splash, home, i18n, branding.
- **La prueba de FFI que ya existe**: `apps/qyro/test/ffi/qyro_native_api_test.dart`,
  la que lee `QYRO/1` desde la DLL en el job de Windows. **Ése es el patrón de
  carga de la biblioteca que ya funciona en las tres plataformas. No inventes
  otro.**
- El job `platform-builds.yml`, para ver cómo se construye y dónde se coloca
  `qyro_ffi` en cada plataforma.

## 4. La decisión: `NativeCallable.listener`

**Ya está investigada y decidida.** No abras `flutter_rust_bridge`: son 47 crates
en Windows, 59 en Android y 60 en iOS —medido con `cargo tree` el 2026-08-11—,
arrastra `tokio`, `futures`, `regex` y `backtrace`, y es un codegen cuyo propósito
es exponer el API de un crate a Dart, que es justo lo contrario de la invariante
de la fase 01.

**`dart:ffi` trae la primitiva exacta**, del SDK, cero paquetes:

> `NativeCallable.listener` — «Constructs a NativeCallable that **can be invoked
> from any thread**.»
> «The native code does not wait for a response from the callback, so **only
> functions returning `void` are supported**.»
> «The Isolate that created the callback will be kept alive until `close` is
> called.»

*(api.flutter.dev, `dart-ffi/NativeCallable`, consultada 2026-08-11.)*

Progreso de transferencia = dos enteros, retorno `void`, disparar y olvidar, desde
un hilo que Dart no creó. Encaja exactamente.

**Las cuatro reglas van en la ADR y en el código, no sólo en un comentario:**

1. **Sólo `void`.** Rust no puede leer un valor de retorno. **Para cancelar, una
   bandera atómica compartida que Rust consulta**, nunca el retorno del callback.
2. **`close()` obligatorio** o el isolate no muere nunca. Envuélvelo en
   `try/finally` en Dart.
3. **La llamada es diferida.** No pases punteros a búferes de la pila de Rust:
   cuando Dart lo mire, ya no existen. **Sólo escalares.**
4. **Llamar al `nativeFunction` después de `close()` es UB.** El apagado lo ordena
   Rust —última llamada = «completado»— y Dart cierra **al recibirla**.

**`docs/adr/ADR-0033-progress-bridge.md`, congelada antes del código**, con esas
cuatro reglas y con: qué campos lleva el callback, con qué frecuencia se emite
—**no uno por chunk de 64 KiB, eso son 128 llamadas por cada 8 MiB**—, y qué pasa
si Dart tarda más en procesar de lo que Rust tarda en emitir.

## 5. Lo que hay que construir, paso a paso

### Paso 1 — ADR-0033 congelada

**Puerta.**

### Paso 2 — El callback en el lado Rust

- Un puntero a función en la firma de apertura de sesión, más un `usize` de
  contexto opaco.
- **Aceptar `null`** — una sesión sin observador tiene que funcionar igual.
- La regla de frecuencia de la ADR, implementada y **medida**: una prueba que
  cuente cuántas llamadas se emiten para un archivo de tamaño conocido y compruebe
  que está dentro del presupuesto.
- **Y la prueba de que esa medida vería el fallo** (`R2` §1.7): fuerza una emisión
  por chunk y comprueba que la prueba lo detecta.

**Puerta.**

### Paso 3 — El lado Dart

- Carga de la biblioteca con **el patrón que ya existe**, no uno nuevo.
- Los `typedef` de `dart:ffi` para las seis operaciones de la fase 01.
- El `NativeCallable.listener`, con `close()` en un `finally`.
- **Una clase que envuelva el handle y lo cierre en `dispose()`**, para que la
  fase 05 no tenga que acordarse.

**Puerta.**

### Paso 4 — La prueba que define la fase

Un test de integración en Dart que:

1. **lanza un proceso receptor** —el binario `qyro_net_smoke` ya existe y sirve, o
   un segundo proceso Dart—;
2. abre una sesión emisora por el FFI contra ese proceso;
3. **recibe progreso por el callback** y comprueba que es monótono creciente y que
   termina en el total;
4. al terminar, **compara el archivo byte a byte**;
5. cierra todo y comprueba que no queda handle abierto.

**Y el archivo tiene que ser grande de verdad** —al menos 8 MiB, generado desde
una semilla y no guardado— para que la ventana, el go-back-N y el control de flujo
se ejerciten.

**Puerta.**

### Paso 5 — Que corra en CI, en las tres plataformas donde se pueda

- **Linux**: obligatorio, en `ci.yml`.
- **Windows**: el job de `platform-builds.yml` ya construye `qyro_ffi.dll` y corre
  un test de Dart contra ella. **Amplíalo.**
- **Android y iOS**: los workflows `android-runtime.yml` e `ios-runtime.yml` ya
  existen y ya corren cosas en emulador y simulador. **Mira si esta prueba cabe
  ahí.** Si no cabe, dilo y regístralo — no lo dejes en silencio.

**Puerta de fase.**

## 6. Las trampas concretas de esta fase

1. **El isolate que no muere.** Si olvidas `close()`, el proceso de test se cuelga
   al final sin decir por qué. Es el fallo más común de `NativeCallable.listener` y
   la documentación lo dice explícitamente.
2. **El puntero a un búfer de la pila.** La llamada es **diferida**: cuando Dart la
   procese, la pila de Rust ya cambió. Sólo escalares. Si algún día hace falta
   pasar una cadena, se copia al heap y se libera con una función propia.
3. **La avalancha de callbacks.** Uno por chunk de 64 KiB son 128 por cada 8 MiB, y
   16 000 por cada gigabyte. Cada uno encola trabajo en el event loop del isolate.
   **Mide y acota.**
4. **La prueba que pasa por no transferir nada.** Comprueba siempre `total > 0` y
   `bytes == total` al final, además del byte a byte. *Una prueba de transferencia
   que no comprueba que hubo transferencia puede estar midiendo el vacío.*
5. **La prueba de Dart que no puede fallar.** Aplica `R2` §1.7: **corrompe el
   archivo a propósito y comprueba que el test lo detecta.** Sin eso, no sabes si
   la comparación byte a byte está comparando algo.
6. **La biblioteca que no se encuentra.** Cada plataforma la coloca en un sitio
   distinto. Usa el mecanismo que ya funciona (`QYRO_FFI_LIBRARY_PATH` en el job de
   Windows) en vez de inventar uno.

## 7. Pruebas obligatorias

- `a_session_without_an_observer_still_completes` — callback `null`
- `progress_reaches_the_total_and_never_goes_backwards`
- `the_callback_budget_is_respected_for_a_known_file_size`
- `an_emission_per_chunk_would_be_visible_to_this_measurement` — la prueba de la
  prueba
- **`a_file_crosses_two_processes_driven_from_dart`** — la que define la fase
- `a_corrupted_transfer_is_detected_by_this_test` — la prueba de la prueba
- `closing_from_dart_leaves_no_handle_and_no_thread`
- `a_cancelled_transfer_from_dart_leaves_no_part_file`

## 8. Criterios de aceptación

1. ADR-0033 congelada antes del código, comprobable en el historial.
2. Las cuatro reglas de §4 implementadas, **y cada una con una prueba o un
   argumento escrito de por qué no se puede probar**.
3. **Un test en Dart mueve un archivo de ≥8 MiB entre dos procesos y lo compara
   byte a byte.**
4. **Hay evidencia de que ese test podría fallar** — la corrupción provocada lo
   detecta.
5. El progreso llega a Dart, es monótono, y termina en el total.
6. El presupuesto de callbacks está medido y acotado, con la prueba de que la
   medida vería el exceso.
7. Una sesión sin observador funciona igual.
8. Ni handle, ni hilo, ni `.qyro-part` sobrevive.
9. La prueba corre **en CI en Linux y en Windows**. Para Android e iOS: o corre, o
   está registrado por qué no.
10. **Cero dependencias externas** en Rust, **y cero paquetes nuevos de pub.dev.**
    `NativeCallable` es del SDK. Di los dos conteos con su comando.
11. Barrido con `cargo-mutants` sobre lo nuevo de Rust, con alcance declarado.
12. Las doce comprobaciones de `R2` en todas las puertas.
13. Informe `docs/reports/fase-02-dart-conduce.md` según `R5`.
14. **Los botones siguen `onPressed: null`.** Y el informe dice **si ya se cumple
    la condición que los mantiene apagados** —«que exista una transferencia real,
    cifrada y comprobada de extremo a extremo»— y qué falta para afirmarlo sin
    trampa.

## 9. Cómo tiene que quedar el resultado

```dart
final session = QyroSession.send(
  to: '192.168.1.50:9000',
  files: ['/ruta/a/foto.jpg'],
  onProgress: (done, total) => print('$done / $total'),
);
await session.run();
session.dispose();
```

Y eso mueve el archivo de verdad, cifrado, verificado, entre dos procesos.

**Y sigue sin haber producto**, porque el usuario no puede elegir el archivo, no
sabe a qué IP mandarlo, y no ve nada. Eso son las fases 03, 04 y 05.

## 10. No objetivos

- **UI.** Ni una pantalla nueva. El test es un test, no una app.
- **Selector de archivos.** Las rutas se pasan a mano.
- **Descubrimiento.** La IP se pasa a mano.
- **Emparejamiento.**
- Keystore, Keychain, empaquetado.

## 11. Qué desbloquea

La 03 y la 05, que necesitan una superficie Dart que funcione. Y es **la primera
evidencia de producto real** del proyecto: a partir de aquí el eje de producto
puede moverse de verdad.
