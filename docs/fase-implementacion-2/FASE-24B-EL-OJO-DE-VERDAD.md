# FASE 24B — El ojo, de verdad: el teléfono mira

> La 24 construyó el decodificador. **Falta el aparato.** Verificado sobre
> `c4252a5`: cero Kotlin de cámara, cero `androidx.camera`, cero permiso `CAMERA`,
> cero pantalla de escaneo.
>
> **Calibre bajo** salvo donde se diga (`03-EL-CAMINO-AL-99` §3). Es una pantalla
> y un canal de plataforma, no la frontera criptográfica.

---

## 1. La decisión, ya tomada. No la vuelvas a deliberar.

`R10` §3 ofrecía tres puentes. **Se toma el que no añade `unsafe`:**

```
CameraX ImageAnalysis (Kotlin, hilo propio)
   │  extrae SÓLO el plano Y a un ByteArray compacto  ← el de-padding aquí
   ▼
MethodChannel "dev.qyro/scanner"  (mismo patrón que dev.qyro/file_picker)
   ▼
Dart  →  qyro_buffer_alloc (ADR-0038, ya existe)  →  dart:ffi
   ▼
qyro_eye  (ya escrito, 570 líneas, 15 pruebas)  →  fountain LT (ya existe)
```

**Cero `unsafe` nuevo en Rust. Cero JNI. Cero excepción nueva a
`forbid(unsafe_code)`.** El aplazamiento de ADR-0048 enmienda 1 **sigue en pie
para el cruce de copia cero** — y deja de bloquear el aparato, porque este camino
no lo necesita.

**El presupuesto, para que se mida en vez de discutirse:** luma de 1280×720 son
**921 600 bytes por frame**. A 5 fps son **4,6 MB/s** por el channel. Kotlin manda
**sólo Y**, no los tres planos, así que es un tercio de lo que `R10` §3 medía para
el package `camera`.

**Y la regla que decide si esto basta:** monta el camino, **mide los fps que
sostiene de verdad**, y escríbelo. Si sostiene ≥5, está hecho y el JNI no hace
falta nunca. Si no llega, **entonces** el cruce de copia cero tiene su argumento
medido y no una intuición — y eso es lo que ADR-0048 enmienda 1 estaba esperando.

---

## 2. Entregables

**Enmienda a ADR-0048** (calibre medio, porque cambia una decisión congelada):
el puente es MethodChannel, con el presupuesto de §1 y la condición que
reabriría el JNI.

**Kotlin — `QyroScannerChannel.kt`**, canal `dev.qyro/scanner`, siguiendo el
patrón que `FilePickerChannel.kt` y `DiscoveryChannel.kt` ya establecieron:

- `androidx.camera:camera-core` + `camera-camera2` + `camera-lifecycle` **1.6.1**.
  Jetpack, **cero Play Services**, y sus AAR **no declaran ni un permiso**.
- `ImageAnalysis` con **`ResolutionSelector` pidiendo ≥1280×720**. **Nunca el
  default de 640×480** — `R10` §8 T1: da 3,07 px/módulo para un v27, el suelo
  exacto de `rqrr`, sin margen.
- `OUTPUT_IMAGE_FORMAT_YUV_420_888`, `STRATEGY_KEEP_ONLY_LATEST`, y un
  `Executor` de un solo hilo que **no** es el main.
- **El de-padding va aquí**: `rowStride` puede ser mayor que el ancho, y
  `buffer.capacity()` puede ser `rowStride*(h-1)+w`, **no** `rowStride*h`. Leer de
  más es un crash, no una degradación (`R10` §8 T10).
- `imageProxy.close()` **en un `finally`**, o el preview se para (`R10` §8 T5).
- **No rotes el buffer.** Los finder patterns ya resuelven la orientación y rotar
  cuesta 10–15 ms por frame. Rota el preview, nunca lo que va a Rust.
- Permiso con `Activity.requestPermissions()` — **no `androidx.activity`**, porque
  `FlutterActivity extends Activity` pelado.
- `FLAG_KEEP_SCREEN_ON` mientras escanea, `clearFlags` al salir. No es un wake
  lock: cero permisos.

**Manifest — cuatro renglones exactos** (`R10` §6):

```xml
<uses-permission android:name="android.permission.CAMERA" />
<uses-feature android:name="android.hardware.camera.any"       android:required="false" />
<uses-feature android:name="android.hardware.camera"           android:required="false" />
<uses-feature android:name="android.hardware.camera.autofocus" android:required="false" />
```

Las tres `uses-feature` **no son decoración**: el permiso `CAMERA` implica
`android.hardware.camera` y `autofocus` con `required=true`, y eso sería mentir
sobre la tablet barata sin autofocus.

**Y la prueba del manifest sube a DOS permisos exactos** —
`CHANGE_WIFI_MULTICAST_STATE` + `CAMERA`. **Al número exacto. No la relajes a
«≥1»**, que es como una prueba deja de comprobar.

**Dart — la pantalla de escaneo**, con:
- el preview por `Texture` (`TextureRegistry.createSurfaceProducer()` del propio
  embedding de Flutter, **cero dependencias**),
- **progreso real**: símbolos recibidos sobre los necesarios, y el tiempo que
  falta **recalculado con la tasa medida**, no con la estimada,
- **qué hacer cuando no hay cámara**: decirlo y ofrecer los otros canales, no
  fallar,
- **y el aviso de píxeles por módulo**: si la geometría dice que está por debajo
  del suelo, decirle a la persona que acerque el teléfono en vez de dejarla
  mirando una pantalla que no avanza. `qyro_eye::pixels_per_module` ya existe.

**El símbolo del ojo cruzando la frontera C**, con su enmienda a ADR-0032
(calibre alto — es la frontera C).

**La fila de la tabla de paridad**, llena: canal óptico, cara GUI.

---

## 3. Lo que NO se hace aquí, y está decidido

- **No `jni`, no `jni-sys`, no vtable a mano.** ADR-0048 enmienda 1 sigue en pie.
- **No el package `camera` de pub.dev**: inyecta permiso de **micrófono** y de
  almacenamiento en el manifest, y mueve los tres planos por el channel.
- **No `qr_flutter`**: sin publicar desde 2023 y anclado a la major anterior.
- **No un lector en el CLI de escritorio.** `R9` lo midió y lo descartó: la máquina
  que necesita el canal óptico es la que **no tiene cámara**.

---

## 4. La prueba que cierra la fase

**En CI, sin cámara:** la vuelta completa que la fase 24 ya tiene
(`qyro_eye::round_trip`), extendida para que **el mismo código** que corre en el
teléfono corra en CI — frames como PNG, leídos en orden aleatorio, **tirando uno de
cada cuatro**, y aun así reconstruye. Con sus tres controles: al 60 % falla **por
nombre y en tiempo acotado**, un frame corrompido se rechaza por checksum, y con el
fountain neutralizado la prueba del 25 % **falla**.

**Y una medición nueva, que es la que decide el diseño:** los **fps sostenidos por
el channel** con frames de 921 600 bytes. Escríbela en el informe con el número.

**Lo que NO se puede probar aquí, y va al protocolo de hardware en blanco:**

> Un archivo de texto de 100 KB, de la pantalla del PC a la cámara del teléfono.
> **Cronometrar y anotar el throughput real en KB/s** y compararlo con los
> 6–10 KB/s de `R8` §4. Y **medir píxeles por módulo en el aparato**: si no llega a
> 4, bajar el emisor a v20–v22 — una constante en el CLI, más frames, y el fountain
> lo absorbe.

---

## 5. La puerta

Las dieciocho, con `scripts/gate.ps1`. Y la **14 se aplica igual aunque el calibre
sea bajo**: el ojo tiene que tener **un flujo de usuario detrás**, no un preflight.
El preflight de `qyro beam` es un llamante honesto y **no es el que esta fase
necesita**: el de esta fase es una persona tocando *Escanear*.

En la 15, la cadena entera: **«la persona toca Escanear» → permiso → CameraX
entrega un frame → luma compacta → channel → `qyro_buffer_alloc` → `qyro_eye` → el
fountain → el archivo en disco → el hash coincide.** Sin saltos.
