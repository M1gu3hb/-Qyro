# R10 — El ojo: cámara en Android sin terceros

> Complemento de `R8` y `R9`. Mismo contrato: **medido, citado contra AOSP y los
> POM reales, fechado. No lo vuelvas a investigar.** Investigación del 2026-08-18.
>
> Esto cierra la única pregunta que quedaba abierta del objetivo de `R7`: **el
> canal óptico tiene emisor y no tiene ojo.** El CLI dibuja QR desde la fase 15 y
> nadie los lee. Sin esto, el nivel 3 de la escalera de canales no existe.

---

## 1. La arquitectura, decidida

**Kotlin captura · JNI cruza una vez · Rust decodifica.**

```
CameraX ImageAnalysis  ──►  external fun nativeFeed(ByteBuffer, w, h, rowStride)
   (Kotlin, hilo propio)         │
                                 │  GetDirectBufferAddress  ← CERO copias
                                 ▼
                         rqrr::prepare_from_greyscale  ──►  fountain LT (ya existe)
                                 │
                                 ▼
                         MethodChannel: sólo el progreso, nunca píxeles
```

**Coste estimado:** Kotlin ~180–260 líneas · Rust ~120–180 (de ellas **~25 de
`unsafe` nuevo**) · Dart ~120–180. **Total ~450–650.**

---

## 2. Por qué NO el package `camera` de pub.dev

Es BSD-3-Clause y no lleva Play Services — pero **su manifest inyecta tres
permisos**, verificado en el AAR:

```xml
<uses-permission android:name="android.permission.CAMERA" />
<uses-permission android:name="android.permission.RECORD_AUDIO" />
<uses-permission android:name="android.permission.WRITE_EXTERNAL_STORAGE" .../>
<uses-feature android:name="android.hardware.camera.any" />   <!-- sin required=false -->
```
https://raw.githubusercontent.com/flutter/packages/main/packages/camera/camera_android_camerax/android/src/main/AndroidManifest.xml

Una aplicación cuya primera promesa es «sin terceros y sin permisos de
almacenamiento» acabaría **pidiendo micrófono**. Y su `Analyzer` está implementado
**en Dart**, así que por cada frame hay **≥3 round-trips por channel y cruzan los
tres planos** (Y+U+V, 1,5× los bytes) aunque sólo necesites la luma.

**CameraX a mano no declara ni un permiso**: verificado descomprimiendo
`camera-core-1.6.1.aar` — su manifest sólo tiene `<uses-sdk>`. **El permiso lo
pones tú, y sólo el que necesitas.**

---

## 3. Dependencias nuevas, exactas

| Dónde | Qué | Licencia |
|---|---|---|
| Gradle | `androidx.camera:camera-core`, `camera-camera2`, `camera-lifecycle` **1.6.1** | Apache-2.0 + BSD-3-Clause (libyuv vendorizado) |
| Gradle, transitivas | ~15 AAR de AndroidX/Kotlin | Apache-2.0. **Ninguna de Play Services** |
| Cargo | **cero crates nuevos en el lock** | — |
| pub.dev | **cero packages nuevos** | — |

CameraX **es Jetpack**, no Play Services: *«CameraX is a Jetpack library… supports
devices running Android 5.0 (API level 21) and higher»*
(https://developer.android.com/media/camera/camerax). El único artefacto del grupo
con GMS es `camera-feature-combination-query-play-services`, que es alpha y **no se
usa**.

### `rqrr` pasa de dev a normal, y eso desbloquea algo

Hoy `qyro_cli/Cargo.toml:51` tiene `rqrr = "0.9"` como dev-dependency. Súbelo a
**0.10.1 con `default-features = false`** (si no, arrastra `image`).

**Y borra los dos ignores de `.cargo/audit.toml`.** Su propio comentario dice:
*«Delete these two lines when either happens: rqrr releases a version that takes
lru 0.13 or later, or anything makes rqrr a normal dependency»*. **Las dos
condiciones se cumplen a la vez**: `rqrr 0.10.1` depende de `lru ^0.16`, muy por
encima de la 0.12.5 afectada.

El guard `qyro_cli/src/guards.rs:108` afirma `!normal.contains("rqrr")` y **va a
saltar. Tiene razón**: reescríbelo para que afirme lo nuevo —que `rqrr` está en
normal **y** que `lru` es ≥0.16—, no para relajarlo.

Cero crates nuevos en el lock: `g2p`, `g2gen`, `g2poly`, `lru` y `hashbrown` ya
están como dev-deps; lo que cambia es que pasan al grafo enviado. Para
`THIRD_PARTY_NOTICES.md`: `rqrr` **(MIT OR Apache-2.0) AND ISC**, `g2p`/`g2gen`/
`g2poly` MIT/Apache-2.0, `lru` MIT.

---

## 4. La luma: garantía, no suposición

AOSP, javadoc de `ImageFormat.YUV_420_888`, cita literal:

> *«The order of planes … is guaranteed such that **plane #0 is always Y** … The
> Y-plane is guaranteed not to be interleaved with the U/V planes (**in
> particular, pixel stride is always 1 in `yPlane.getPixelStride()`**).»*

Y por contraste, `YUV_422_888` dice que ahí el pixel stride **sí** puede ser >1 —
lo que confirma que en 420 es un contrato del formato.
https://raw.githubusercontent.com/aosp-mirror/platform_frameworks_base/master/graphics/java/android/graphics/ImageFormat.java

**El buffer es directo por contrato**, así que `GetDirectBufferAddress` da el
puntero al buffer real de la cámara: *«the buffer returned will always have
`isDirect` return true, so the underlying data could be mapped as a pointer in JNI
without doing any copies»* (`Image.java`).

**Y no hace falta buffer intermedio**, porque `rqrr` toma un closure:

```rust
let img = rqrr::PreparedImage::prepare_from_greyscale(w, h, |x, y| {
    unsafe { *y_ptr.add(y * row_stride + x) }   // pixel_stride == 1, garantizado
});
```

El patrón **ya existe en el árbol**: `qyro_cli/src/round_trip.rs:89`. Y el decode
por bytes crudos es `grid.decode_to(&mut bytes)` — no `String`, porque los frames
del fountain son bytes, no UTF-8.

**Trampa que revienta:** `buffer.capacity()` puede ser `rowStride*(h-1) + w`, **no**
`rowStride*h` — *«the stride after the last row may not be mapped into the
buffer»* (`Image.java`). Un bucle que lea `rowStride*h` bytes casca.

---

## 5. JNI sin el crate `jni`

El **único** servicio de `JNIEnv` que hace falta es `GetDirectBufferAddress`, y su
posición en la vtable la fija la especificación JNI: **slot 230** de 237, contado
sobre el `jni.h` oficial
(https://raw.githubusercontent.com/openjdk/jdk/master/src/java.base/share/native/include/jni.h);
mismo orden en el `jni.h` de Android.

Son **~25 líneas de `unsafe`** y cero crates:

```rust
#[repr(C)]
struct JniNativeInterface {
    _reserved: [*const core::ffi::c_void; 230],
    get_direct_buffer_address: unsafe extern "system" fn(
        *mut *const JniNativeInterface, *mut core::ffi::c_void) -> *mut core::ffi::c_void,
}
```

Alternativa si no quieres firmar esas 25 líneas: `jni-sys 0.4.1` (MIT OR
Apache-2.0, 1 dependencia). **El crate `jni` completo queda descartado** — arrastra
`combine`, `log`, `thiserror`, `simd_cesu8` y más.

**Nota para la ADR**, porque a primera vista parece prohibido: el aviso de ART en
Android 16 sobre *«internal structures (such as non-SDK interfaces)»* **no aplica**
— la vtable de `JNIEnv` es la ABI pública de la especificación JNI, no una
estructura interna de ART. Escríbelo, o alguien lo reabrirá.

`System.loadLibrary("qyro_ffi")` **sigue siendo necesario desde Kotlin** aunque
`dart:ffi` ya haya hecho `dlopen`: el `dlopen` de Dart no alimenta la tabla de
nativos de la JVM. Es una línea en un `init {}`.

---

## 6. Permisos: exactamente cuatro líneas

```xml
<uses-permission android:name="android.permission.CAMERA" />
<uses-feature android:name="android.hardware.camera.any"       android:required="false" />
<uses-feature android:name="android.hardware.camera"           android:required="false" />
<uses-feature android:name="android.hardware.camera.autofocus" android:required="false" />
```

Las tres `uses-feature` **no son decoración**. AOSP: *«This will automatically
enforce the `uses-feature` manifest element for **all** camera features»*, y la
tabla 2 de la doc oficial dice que `CAMERA` implica `android.hardware.camera` **y**
`android.hardware.camera.autofocus`, con `required=true`. Declararlas
explícitamente con `required="false"` es lo que impide **mentir sobre la tablet
barata sin autofocus**.
https://developer.android.com/guide/topics/manifest/uses-feature-element

**El runtime no toca Play Services.** Y aquí la elección está forzada, a favor:
`FlutterActivity extends Activity` **pelado, no `ComponentActivity`**, así que
`registerForActivityResult` no existe sin añadir `androidx.activity`. Con
`minSdk 24`, `Activity.requestPermissions()` + `onRequestPermissionsResult()` son
API de plataforma. **Cero dependencias, ~20 líneas en `MainActivity.kt`.**

**Consecuencia en CI:** la prueba que afirma «el manifest declara exactamente un
permiso» pasa a **dos** — `CHANGE_WIFI_MULTICAST_STATE` + `CAMERA`. **Actualiza la
aserción al número exacto; no la relajes a «≥1».**

---

## 7. La dirección inversa: el teléfono dibuja

**Expón el `qrcode 0.14.1` de Rust por la FFI que ya existe.** No añadas
`qr_flutter`: lleva **sin publicar desde 2023-05-14** y está anclado a la major
anterior de `qr`.

Cuatro razones, todas verificables: `qrcode` es MIT OR Apache-2.0 y su única
dependencia (`image`) es opcional y ya está desactivada en el CLI · ya está en el
`Cargo.lock` · **un solo encoder para las dos caras** —dos implementaciones
distintas alimentando el mismo fountain decoder es un bug que sólo aparece cruzando
aparatos— · y la ABI es trivial: `qyro_optical_frame(seq, out, out_len) -> i32`
devolviendo el bitmap de 125×125, que encaja en el patrón
`qyro_buffer_alloc`/`free` de ADR-0038.

Dibujar 125×125 rectángulos a 5–8 fps es trivial para un `CustomPainter`. **El
cuello de botella no está aquí.**

---

## 8. Las diez trampas, con su fuente

**T1 — LA GRANDE: 640×480 no decodifica un v27.** CameraX: *«If not set,
**resolution of 640x480** will be selected to use in priority»*
(`ImageAnalysis.java`). ADR-0044: **v27 = 125 módulos** + 4 de quiet zone por lado =
133. Con el QR al 85 % del alto: a 640×480 → **3,07 px/módulo**; a 1280×720 →
**4,6**. `rqrr` necesita ~3 como suelo absoluto y 4–5 para ser fiable.
**640×480 es exactamente el borde del precipicio.**
→ `ResolutionSelector` pidiendo **≥1280×720**, y **medir px/módulo en el aparato
real antes de escribir nada más**. Si no llega, **plan B: bajar el emisor a
v20–v22** — una constante en el CLI, más frames, el fountain lo absorbe.

**T2 — Rotación: no rotes nada.** *«CameraX does not perform an internal rotation
of the data»*. **Y no hace falta**: los tres finder patterns son el mecanismo de
orientación del propio formato y `rqrr` resuelve la perspectiva desde ellos; un QR
a 90° o a 37° decodifica igual. `setOutputImageRotationEnabled(true)` cuesta
*«about 10-15ms for 640x480 image on a mid-range device»*. **Rota sólo el preview,
nunca el buffer.**

**T3 — Autofocus a 30 cm sobre pantalla plana.**
`LENS_INFO_MINIMUM_FOCUS_DISTANCE` *«If the lens is fixed-focus, this will be 0»* y
es **opcional, puede ser `null`**. En la tablet barata puede no enfocar nunca a esa
distancia. Usa `CONTROL_AF_MODE_CONTINUOUS_PICTURE`, **nunca `MACRO`** (que *«does
not move unless the autofocus trigger action is called»*). Y pon una guía de
distancia en pantalla.

**T4 — Glare y rolling shutter.** La pantalla es una fuente de luz, no un objeto
reflectante: el AE la mide sobreexpuesta. El rolling shutter contra el refresco del
panel produce bandas que rompen módulos. Mitigaciones por coste: bajar el brillo
del portátil al 60–70 % (gratis y suele bastar) · quiet zone generosa ·
`CONTROL_AE_TARGET_FPS_RANGE` fijo y bajo · bajar el emisor de 5 a 3 fps. **El
fountain existe justamente para que perder frames no sea perder la transferencia.**

**T5 — `close()` o se para el preview.** *«If the images are not closed then it may
**block further images from being produced (causing the preview to stall)**»*. El
default `STRATEGY_KEEP_ONLY_LATEST` **es el que quieres**; `STRATEGY_BLOCK_PRODUCER`
*«may also stop producing images for other use cases, such as Preview»*.
**`close()` en un `finally`, sin excepciones.**

**T6 — Nunca decodifiques en el hilo de UI.** `setAnalyzer(Executor, Analyzer)` con
un `newSingleThreadExecutor()`. Si el decode tarda más que el frame,
`KEEP_ONLY_LATEST` tira frames — degradación limpia, no jank.

**T7 — La pantalla se apaga a los 30 s** y una transferencia son minutos.
`FLAG_KEEP_SCREEN_ON` al entrar, `clearFlags` al salir. **No es un wake lock: cero
permisos.**

**T8 — Android 14/15/16: menos de lo que temes.** La regla de foreground service
types aplica **sólo si accedes a la cámara desde background**: *«you cannot create
a `camera` foreground service while your app is in the background»*. **Con una
Activity visible no hace falta ni el service ni `FOREGROUND_SERVICE_CAMERA`** — y
T7 garantiza que está visible. Android 15 y 16 no documentan cambios de cámara
relevantes.

**T9 — Los guards del repo van a saltar y tienen razón.** Ver §3.

**T10 — `capacity() != rowStride*height`.** Ver §4. Es un crash, no una
degradación.
