# R9 — La cámara, y la lección del contexto

> Complemento de `R8`. Mismo contrato: **está medido, compilado y fechado. No lo
> vuelvas a investigar.** Investigación del 2026-08-17.

---

## 1. Primero, una regla que estaba mal escrita, y es mía

El prompt de la sesión del 2026-08-17 decía: *«Sólo se para por un P0 que no puedas
arreglar. Si se acaba el contexto, reescribes ESTADO-ACTUAL.md, commiteas y sigues.»*

**Eso está mal y produjo un bucle.** Al final de aquella sesión el agente se quedó
sin contexto, dijo por qué no podía escribir tres fases más, y algo le exigió
continuar ocho veces seguidas. Gastó en ese intercambio el contexto que le quedaba,
y **tuvo razón en negarse**: la única forma de aparentar cumplimiento era emitir
módulos que no compilan e informes afirmando que funcionan. Eso es fabricar
evidencia, que es la única cosa prohibida sin excepción en este proyecto.

**La regla corregida, y es la que rige:**

> **Quedarse sin contexto es un motivo legítimo de parada, igual que un P0.**
> La forma de parar es **un solo mensaje**: reescribe `ESTADO-ACTUAL.md` diciendo
> exactamente dónde se corta, commitea, empuja, y di en una frase que se acabó el
> contexto. **No lo repitas, no lo argumentes, no lo negocies.** Un párrafo y
> fuera.
>
> Lo que sigue prohibido es parar **por ordenado**: terminar una fase con contexto
> de sobra y anunciar «lo siguiente es X». Ahí se abre X y se empieza.

---

## 2. La cámara: se puede, y aun así no se hace

**Pregunta:** ¿puede un binario Rust estático, sin una sola dependencia que compile
C, leer QR animados de una pantalla con una webcam, en Windows y en Linux?

**Respuesta medida: sí, técnicamente.** Compilado y verificado, no deducido:

| Plataforma | Ruta | Estado |
|---|---|---|
| Linux | **`linuxvideo 0.3.5`** (0BSD) — 5 crates, **cero build scripts**, ELF `static-pie` sobre musl | ✅ probado |
| Windows | **`windows 0.62.2`** de Microsoft (MIT OR Apache-2.0) vía Media Foundation | ✅ compila contra `x86_64-win7-windows-msvc` con `-Z build-std` |

`windows-link` enlaza con **`raw-dylib`**: rustc genera la import library él mismo,
así que **no hace falta ni un `.lib` del Windows SDK**. Eso es lo que hace viable el
target Tier 3. `MFEnumDeviceSources` y `MFCreateSourceReaderFromMediaSource` están
documentadas por Microsoft como disponibles **desde Windows 7**.

### Y aun así, la decisión de ADR-0044 §6 se mantiene

El coste real de Windows son **400–700 líneas de COM `unsafe`** — una **segunda
excepción** a `forbid(unsafe_code)` frente a la única que ADR-0024 §1 concedió, y
aquélla eran dos `extern "system"`, no un backend entero. Más las **ediciones N/KN
de Windows**, donde Media Foundation sencillamente no existe
(https://support.microsoft.com/en-us/topic/media-feature-pack-list-for-windows-n-editions-c1c6fffa-d052-8338-7a79-a4bb980a700a).
Más hardware Windows 7 real que probar, en un proyecto donde nada ha corrido nunca
en hardware.

Y todo eso **para una máquina que, por la premisa del canal, no tiene cámara.**
ADR-0044 §6 ya lo resolvió con el mejor argumento: *«el PC viejo enseña, el teléfono
escucha… construir el lector para la terminal sería construirlo para una máquina que
no puede usarlo.»*

**Se mantiene: el CLI dibuja, el teléfono lee.** Si algún día hace falta un receptor
de escritorio, **se empieza por Linux** con `linuxvideo` **vendorizado dentro del
repositorio** — es 0BSD, se copia sin atribución ni condiciones, deja de ser una
dependencia, y **no obliga a relajar `forbid(unsafe_code)` en ningún crate del
producto**. Windows se decide después, con un aparato delante.

---

## 3. Lo que hay que saber igualmente, porque afecta a la fase 15

### 3.1 — YUYV es el formato ideal, y la conversión es gratis

El kernel documenta que en `V4L2_PIX_FMT_YUYV` *«Byte 0 contains Y'0, Byte 1
contains Cb0, Byte 2 contains Y'1…»* — **los bytes pares son luma**
(https://www.kernel.org/doc/html/latest/userspace-api/media/v4l/pixfmt-packed-yuv.html).

Y `rqrr 0.10.1` expone justo el punto de entrada que hace falta
(`src/prepare.rs:526`):

```rust
pub fn prepare_from_greyscale<F>(w: usize, h: usize, mut fill: F) -> Self
```

```rust
let img = rqrr::PreparedImage::prepare_from_greyscale(w, h, |x, y| buf[(y * w + x) * 2]);
```

**Coste: una lectura con stride 2. Cero aritmética, cero buffer RGB.** El
submuestreo 4:2:2 es sólo de croma, así que **no se pierde ni un módulo del QR**.

### 3.2 — `rqrr` ya hace el preprocesado. No metas `image` ni `imageproc`

| Necesidad | ¿lo hace `rqrr`? | Dónde |
|---|---|---|
| Binarización adaptativa | **Sí** | `src/prepare.rs:189` — media móvil por fila, ventana `w/8`, sesgo 5 % |
| Corrección de perspectiva | **Sí** | `src/geometry.rs:4` `Perspective([f64;8])` |
| Reed-Solomon | **Sí** | `src/decode.rs` sobre `g2p` |
| Glare, moiré, motion blur | **No** — y ninguna librería lo hace. Es UX: ángulo, brillo, distancia |

Verificado: `rqrr` con `default-features = false` **no arrastra `image`**. El árbol
es `g2p`, `lru`, `hashbrown`.

### 3.3 — La trampa del MJPEG sin DHT, que cuesta un día si no se sabe

`linuxvideo` lo documenta: los frames MJPG de UVC **no llevan el segmento `DHT`**.
Los dos decodificadores puros (`zune-jpeg 0.5.15`, `jpeg-decoder 0.3.2`) traen la
tabla de respaldo, pero **la activan sólo al ver un APP0 `AVI1` que V4L2 no emite**:

- `zune-jpeg/src/decoder.rs:583` → `if &buffer == b"AVI1\0" { self.is_mjpeg = true }`
- `jpeg-decoder/src/decoder.rs:552` → `AppData::Avi1 => self.is_mjpeg = true`

Sin eso falla con `"scan makes use of unset dc huffman table"`. **Mitigación:**
buscar `0xFFC4` antes del `SOS` (`0xFFDA`) y, si falta, insertar el DHT con las
tablas de **ITU-T T.81 Anexo K.3.3** — que están literales en los dos crates, ambos
MIT/Apache. **~60 líneas.** Presupuéstalo; no lo descubras en hardware.

### 3.4 — Resolución mínima, y por qué 640×480 no sirve

ADR-0044 fija v27 = 125 módulos + quiet zone ≈ 133. A 4,9 px/módulo hacen falta
**~650 px sólo para el QR**. **640×480 en YUYV no llega; 1280×720 sí** — y ahí YUYV
roza el techo de USB 2.0 (≈55 MB/s a 30 fps), así que muchas cámaras sólo lo ofrecen
a 10 fps. **Es exactamente donde MJPEG vuelve a hacer falta**, y con él el §3.3.

---

## 4. Crates: lo que está descalificado, y por qué

Verificado inspeccionando el `.crate` publicado, no el README.

| Crate | Problema |
|---|---|
| `nokhwa` **con default features** | arrastra `mozjpeg-sys` → **`cc` + `nasm-rs`**, y licencia **IJG** |
| `v4l` / `v4l2-sys-mit` / `v4l2r` | **bindgen** obligatorio; `v4l` con `default-features = false` **ni compila** |
| `libv4l` | **LGPL-2.1**, y `.so` dinámica |
| `escapi` | `cc::Build::new().cpp(true)`. Muerto desde 2019 |
| `openpnp-capture-sys` | `cmake` + `cc` + `bindgen`, enlaza `stdc++` |
| `zxing-cpp` | `cmake::Config::new("core")`, 389 ficheros C/C++ |
| `bardecoder` | exige `image 0.24` (dos majors atrás) y `newtype_derive` de **2016** |
| `eye-hal` | **no resuelve hoy**: todas las `mozjpeg-sys 0.10.x` están *yanked* |
| `uvc` | bindgen + enlaza `libuvc` (C) |

**Lo que sí pasa:** `linuxvideo 0.3.5` (0BSD), `uoctl 1.0.1` (0BSD), `windows 0.62.2`
(MIT/Apache-2.0), `rqrr 0.10.1`, `zune-jpeg 0.5.15`, `jpeg-decoder 0.3.2` **sin
`rayon`**. Ni una GPL, LGPL ni MPL en todo el árbol.

---

## 5. Leer un vídeo en vez de una cámara: **descartado, y es peor**

Demultiplexar MP4 en Rust puro se puede (`mp4 0.14.0`, `re_mp4 0.5.1`, ambos MIT).
**Decodificar H.264 no.**

- `openh264-sys2`: feature default `source = ["cc","walkdir","nasm-rs"]` → compila
  C++. La alternativa carga una DLL de Cisco en runtime → **deja de ser un binario
  único**.
- `oxideav-h264`: su propia descripción dice *«no decode/encode functionality yet»*.
- `rusty_h264`: publicado el **2026-08-13**, cuatro días antes de esta investigación,
  proveedor único. **No se apuesta un release a eso.**

**Leer MP4 en Rust puro es estrictamente más difícil que capturar de la cámara.**
Y `ffmpeg` externo no rompe la licencia —invocar no es enlazar— pero rompe la
promesa: en Windows no viene instalado, y una feature de nivel 3 no justifica una
carga de soporte permanente.

**Lo que sí vale como entrada del receptor, y cierra la fase 15 en CI: un directorio
de imágenes PNG/JPEG.** ~100 líneas, todo `safe`, y es lo que permite probar el
fountain descartando frames a propósito.

---

## 6. Comparativas de decodificadores: el hallazgo es que no hay

**No existe ninguna comparativa publicada de `rqrr` vs `bardecoder` vs ZXing.** Si
hace falta el dato, hay que medirlo. Lo más cercano usa quirc —el ancestro directo de
`rqrr`— como proxy: BoofCV (2019-03-19,
https://boofcv.org/index.php?title=Performance%3AQrCode) sobre >1 000 QR etiquetados
con categorías `monitor`, `glare`, `perspective`, `blurred`, concluye que en
velocidad *«BoofCV is the fastest library by a large margin followed by Quirc»*.

**Y esas cifras no se aplican directamente aquí.** Ese corpus es de fotos
adversariales: papel arrugado, dañado, a contraluz. El escenario de Qyro es el más
benigno posible —LCD plano, autoiluminado, contraste máximo, geometría estable— y,
lo decisivo, **flujo continuo con fountain code encima**. No hace falta que un frame
concreto decodifique: hace falta que decodifiquen suficientes. **Un 30 % de acierto
por frame a 15 fps sigue dando 4,5 frames útiles por segundo.**

**Decisión: `rqrr` 0.10.1. Segundo si falla en hardware: `quircs` 0.10.3.**
`rxing 0.9.2` (Apache-2.0, Rust puro, sin `build.rs`) es la reserva si hace falta un
algoritmo *distinto* al de quirc — pero exige podar sus features agresivamente.
