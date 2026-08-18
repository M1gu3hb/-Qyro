# FASE 24 — El ojo: el teléfono lee lo que la terminal dibuja

> **La última capacidad que le falta al objetivo de `R7`.** Todo lo demás que
> queda es robustez, plataforma, verdad y empaquetado. Esto es funcionalidad, y
> sin ello el nivel 3 de la escalera de canales **no existe**.
>
> Lee `R10` entero antes de escribir una línea. Está medido y citado contra AOSP.

---

## 1. Por qué existe

`R7` §R7.4 promete cuatro canales. Hoy hay tres y medio:

| Nivel | Canal | Estado |
|---|---|---|
| 1 | TCP sobre la red que ya existe | **hecho** |
| 2 | TCP sobre enlace directo sin router | **hecho** (fase 14) |
| 3 | **Óptico: QR animado, pantalla → cámara** | **medio**: `qyro beam` dibuja desde la fase 15 y **nadie lee** |
| 4 | Serie RS-232 | **hecho** en código (fase 16), sin cable real |

`qyro_fountain` codifica y decodifica, `qyro qr` y `qyro beam` dibujan, y hay una
prueba de ida y vuelta en la que **un decodificador de terceros lee lo que la
terminal pinta**. Lo que no hay es **un aparato con cámara ejecutando ese
decodificador**.

Es, además, **la novena capacidad de la misma forma** que este proyecto lleva
persiguiendo: escrita, probada, y sin llegar a una persona. La diferencia es que
esta vez está declarada — ADR-0044 §6 lo dijo desde el principio: *«el CLI dibuja,
el teléfono lee»*. Esta fase construye la mitad que faltaba.

---

## 2. La decisión que hay que congelar

`docs/adr/ADR-00XX-el-ojo.md`, antes de una línea de código. **La arquitectura ya
está decidida por `R10` §1 — la ADR la registra con su fuente, no la rediscute:**

**Kotlin captura con CameraX · JNI cruza una vez con cero copias · Rust decodifica
con `rqrr` y alimenta el fountain que ya existe.**

Lo que la ADR sí tiene que decidir:

1. **El `unsafe` nuevo.** Son ~25 líneas de vtable de `JNIEnv` más un deref por
   píxel. `ADR-0024 §1` estableció que `qyro_win_dpapi` es *el único crate del
   producto que relaja `forbid(unsafe_code)`*. Esto sería **la segunda excepción** —
   o va confinada en `qyro_ffi`, que ya la tiene. **Decide dónde vive y escríbelo.**
   Y escribe también por qué el aviso de ART de Android 16 sobre «internal
   structures» **no aplica** (`R10` §5), o alguien lo reabrirá en tres semanas.
2. **`jni-sys` o la vtable a mano.** ~25 líneas y cero crates, contra un crate de
   1 dependencia. Elige, una línea de argumento, y sigue.
3. **Qué cruza de vuelta a Dart.** Sólo progreso: bytes recibidos, frames vistos,
   símbolos que faltan. **Nunca píxeles.**
4. **Qué pasa cuando no hay cámara**, y no es un detalle: la pantalla tiene que
   decir «este aparato no tiene cámara» y ofrecer los otros canales, no fallar.
5. **El umbral de píxeles por módulo** bajo el cual el escáner avisa a la persona
   de que acerque el teléfono, en vez de dejarla mirando una pantalla que no
   avanza. Ver `R10` §8 T1.

---

## 3. Entregables

1. **La ADR de §2, congelada en su propio commit.**
2. **`rqrr` de dev a normal**, subido a **0.10.1 con `default-features = false`**;
   **los dos ignores de `.cargo/audit.toml` borrados** porque su propia condición
   se cumple; y el guard de `qyro_cli/src/guards.rs:108` reescrito para afirmar lo
   nuevo, no para relajarse (`R10` §3).
3. **El símbolo del decodificador**, en la superficie C con su enmienda a ADR-0032.
4. **`QyroScannerChannel.kt`**: `System.loadLibrary`, permiso con
   `Activity.requestPermissions()` —**no** `androidx.activity`, porque
   `FlutterActivity extends Activity` pelado—, CameraX con `camera-core` +
   `camera-camera2` + `camera-lifecycle` **1.6.1**, `ImageAnalysis` con
   **`ResolutionSelector` forzando ≥1280×720** (nunca el default de 640×480),
   `OUTPUT_IMAGE_FORMAT_YUV_420_888`, `STRATEGY_KEEP_ONLY_LATEST`, y un
   `Executor` de un solo hilo que **no** es el main.
5. **El preview sin dependencias**: `TextureRegistry.createSurfaceProducer()` del
   propio embedding de Flutter, y `FLAG_KEEP_SCREEN_ON` mientras escanea.
6. **Los cuatro renglones del manifest** (`R10` §6), y **la prueba del manifest
   actualizada al número exacto: dos permisos.** No la relajes a «≥1».
7. **La pantalla de escaneo en Flutter**, con progreso real —símbolos recibidos
   sobre los necesarios— y el tiempo que falta **recalculado con la tasa medida**,
   no con la estimada.
8. **La otra dirección**: `qyro_optical_frame` expuesto por la FFI para que la GUI
   también pueda **dibujar**, con el mismo `qrcode 0.14.1` que usa el CLI. Un solo
   encoder para las dos caras (`R10` §7). Y la fila correspondiente de la tabla de
   paridad, llena.

---

## 4. Lo primero que hay que hacer, y no es código

**Mide píxeles por módulo en el aparato más barato al que apuntas.**

`R10` §8 T1: a 640×480 son **3,07 px/módulo** —el suelo teórico exacto de `rqrr`,
sin margen para glare, rolling shutter ni un autofocus que puede no enfocar a 30 cm—
y a 1280×720 son **4,6**.

Si no llegas a 4, **la solución barata no es subir la resolución hasta que el
decode no quepa en el presupuesto de frame: es bajar el emisor a v20–v22.** Es una
constante en el CLI, salen más frames, y el fountain lo absorbe. Escríbelo como
enmienda a ADR-0044 con el número que hayas medido.

---

## 5. La prueba que cierra la fase

**Sin cámara, en CI, y aun así real** — el emisor escribe los frames como PNG a un
directorio y el decodificador los lee con `rqrr` **en orden aleatorio, tirando el
20 % con semilla fija**, y aun así reconstruye. Esa prueba **ya existe**
(`qyro_cli/src/round_trip.rs`): extiéndela al camino nuevo, para que sea el mismo
código el que corre en CI y en el teléfono.

**Y la prueba que sólo puede hacer una persona**, que va al protocolo de hardware
como escenario nuevo y **en blanco**:

> Un archivo de texto de 100 KB, de la pantalla del PC a la cámara del teléfono.
> **Cronometrar y anotar el throughput real en KB/s**, y compararlo con los
> 6–10 KB/s de `R8` §4. Es la medición que valida o refuta todo el diseño.

**Tres controles, obligatorios:**
1. Tirando el **60 %** de los frames, falla con un error **nombrado** y en tiempo
   acotado. **No se cuelga.** Un fountain sin límite de paciencia es un programa
   colgado.
2. **Un frame corrompido** se rechaza por checksum y no envenena el ensamblado.
3. Con el fountain **neutralizado** (símbolos secuenciales sin XOR), la prueba del
   20 % descartado **falla**. Sin esto no se sabe si el fountain hizo algo.

---

## 6. La puerta

Dieciséis comprobaciones, más la 17 (`cargo check --workspace` en Linux). Y en la
14 —el llamante de producción con archivo y línea— **el decodificador tiene que
tener un flujo de usuario detrás, no un test**. `MdnsDiscovery` es la lección: un
módulo perfecto sin llamante es un módulo que no existe.

En la 15, la cadena entera: **«la persona toca Escanear» → permiso → CameraX
entrega un frame → JNI → `rqrr` → el fountain → el archivo en disco → el hash
coincide.** Sin saltos.

---

## 7. Lo que NO hay que hacer

- **No uses el package `camera` de pub.dev.** `R10` §2: inyecta permiso de
  micrófono y de almacenamiento, y mueve los tres planos por el channel.
- **No añadas `qr_flutter`.** Sin publicar desde 2023, y anclado a la major
  anterior de `qr`.
- **No rotes el buffer.** `R10` §8 T2: los finder patterns ya resuelven la
  orientación, y rotar cuesta 10–15 ms por frame.
- **No decodifiques en el hilo de UI.**
- **No relajes la prueba del manifest.** Súbela a dos, exacto.
- **No inventes el throughput del canal óptico.** Está en `R8` §4 y el número real
  lo dará el teléfono, no tú.
