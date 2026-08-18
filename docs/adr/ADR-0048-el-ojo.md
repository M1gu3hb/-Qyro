# ADR-0048 — El ojo: el teléfono lee los QR que el CLI dibuja

- **Estado:** **congelada** antes de una sola línea de código.
- **Fecha:** 2026-08-19
- **Fase:** 24
- **Depende de:** ADR-0044 (canal óptico), ADR-0039 (iOS fuera), ADR-0046 (dos consumidores).
- **Gobernada por:** `R7` §R7.4 nivel 3, y `R10`, medida contra AOSP.

---

## 1. Qué falta, exactamente

`R7` promete cuatro canales. Hay **tres y medio**: `qyro beam` dibuja QR desde la
fase 15 y **nadie los lee**. El emisor existe, el códec fountain existe, la
prueba de vuelta completa demuestra que lo dibujado es legible — **por un
decodificador, sobre píxeles perfectos.** Falta el ojo.

---

## 2. Decisión 1 — la arquitectura, que `R10` ya midió

| Capa | Qué | Por qué |
|---|---|---|
| Captura | **CameraX `ImageAnalysis`** (Jetpack) | Cero Play Services, cero packages de pub.dev |
| Cruce | **JNI a mano, `GetDirectBufferAddress`** | Una copia menos por frame, y a 5 fps son megabytes |
| Decodificación | **`rqrr`**, ya en el árbol | Es el que la fase 15 usó para probar el emisor |
| Reensamblado | **`qyro_fountain`**, ya en el árbol | Sin cambios: el ojo produce `Frame` y el decodificador ya sabe |

**Cero crates nuevos en el lock y cero packages de pub.dev.** `rqrr` sube de
dev-dependency a normal, que es la única línea que cambia en un `Cargo.toml`.

---

## 3. Decisión 2 — **v27 se queda, y la palanca queda escrita**

`R10` §8 T1 es la trampa grande: CameraX, si no se le pide otra cosa, elige
**640×480**, y ahí un v27 da **3,07 px/módulo** — el suelo exacto de `rqrr`, sin
margen.

Aritmética reproducida de forma independiente el 2026-08-19, con el QR al 85 %
del alto del frame:

| Versión | Módulos + quiet | 640×480 | 1280×720 | 1920×1080 |
|---|---|---|---|---|
| v20 | 105 | 3,89 | 5,83 | 8,74 |
| v22 | 113 | 3,61 | 5,42 | 8,12 |
| **v27** | **133** | **3,07** | **4,60** | **6,90** |
| v40 | 185 | 2,21 | 3,31 | 4,96 |

Los dos números que `R10` cita —3,07 y 4,60— salen idénticos, así que el modelo
no es una suposición heredada.

**La decisión: se pide `≥1280×720` con `ResolutionSelector` y el emisor se queda
en v27.** A 720p da **4,60**, dentro de la banda fiable de 4–5, y 720p para
`ImageAnalysis` lo soporta cualquier aparato que pueda correr esta aplicación —
el 640×480 es el **default cuando no se pide nada**, no un techo.

Bajar a v22 costaría **~40 % más frames a todo el mundo** para protegerse de un
aparato que ignore la petición. Eso es pagar siempre por un caso que no está
medido.

**La palanca existe y es una constante**, con su aritmética en una prueba: el
tamaño de bloque del emisor. Si el aparato de verdad entrega 640×480, se baja y
el fountain absorbe los frames de más. **No se toca hoy porque no hay medida, y
elegir sin medir en la dirección cara es tan malo como elegir sin medir en la
barata.**

---

## 4. Lo que esta ADR **no** puede decidir, y se queda en blanco

> **`R10` §8 T1 manda medir píxeles por módulo en el aparato real antes de
> escribir nada más. NO HAY APARATO.**

Es una de las cuatro excepciones que el implementador no cruza. **El hueco se
queda en blanco**: la aritmética de la §3 es aritmética, no una medida, y no se
presenta como otra cosa.

Lo que sí se puede hacer sin aparato, y es lo que la fase hace:

1. **La aritmética, en código y con prueba**, para que la palanca de la §3 sea
   real y sus números estén comprobados en vez de escritos en prosa.
2. **El camino de decodificación en Rust**, que la fase 15 ya demostró
   ejercitable sin cámara: píxeles → `rqrr` → `decode_frame` → fountain.
3. **El glue de Kotlin y JNI**, que **no** se puede ejercitar aquí y se declara
   como tal.

**Lo que NO se hará:** dar por buena una cadena que ninguna cámara ha recorrido.
El informe de esta fase dirá qué se ejecutó y qué no, y la línea entre las dos
cosas no se mueve.

---

## 5. Decisión 3 — el ojo vive detrás de una interfaz, no dentro de la pantalla

El receptor óptico se expone como un tipo con tres operaciones —empezar, tragar
un frame de luma, y decir si ya está— **sin saber de dónde vienen los píxeles**.

Es lo que hace que la fase 15 y la 24 compartan prueba: el arnés de
`round_trip.rs` rasteriza lo que dibuja `qyro beam` y se lo da al mismo tipo que
CameraX alimentará. **Una cámara es una fuente de píxeles más**, y la única que
no se puede ejercitar aquí.

---

## 6. Lo que NO se promete

- **Que un teléfono lea un QR de Qyro.** Ninguno lo ha hecho. Fase 19.
- **iOS.** ADR-0039 lo aparcó y esto no lo reabre.
- **Que el ojo funcione a 640×480.** Se pide 720p; si un aparato lo ignora, la
  palanca de la §3 es la respuesta y **está sin medir**.
- **Enfoque a 30 cm sobre una pantalla plana** (`R10` §8 T3): en un aparato de
  lente fija puede no ocurrir nunca, y eso no lo arregla el software.

---

## 7. Enmienda 1 (2026-08-19) — el cruce JNI espera a la fase 19, y por qué

La §2 decidió **`GetDirectBufferAddress` a mano**, siguiendo `R10` §5: el único
servicio de `JNIEnv` que hace falta, en el **slot 230** de la vtable, ~25 líneas y
cero crates. Esa decisión **no cambia**. Lo que se decide aquí es *cuándo* se
escribe, porque al llegar salieron dos cosas que la §2 no pesó:

### 7.1 — Son la segunda excepción a `forbid(unsafe_code)`

Este taller ha concedido **una** en toda su historia: `qyro_win_dpapi`, y le costó
una ADR entera (ADR-0024 §1). Veinticinco líneas de aritmética sobre desplazamientos
de una vtable no son veinticinco líneas cualesquiera: **un slot equivocado no da un
error de compilación, da un salto a una función arbitraria**, y el síntoma es un
proceso muerto sin traza en el aparato de otra persona.

### 7.2 — Y no hay forma de ejercitarlas aquí

Un `GetDirectBufferAddress` mal indexado se descubre **ejecutándolo**. No hay
aparato, no hay emulador con cámara en esta máquina, y ninguna prueba de este
repositorio puede tocar una `JNIEnv`. Escribirlas ahora sería añadir la única
clase de código que este proyecto trata con más cuidado —`unsafe`— con la única
clase de evidencia que este proyecto prohíbe: ninguna.

### 7.3 — La decisión

> **El cruce JNI y el `ImageAnalysis.Analyzer` de Kotlin se escriben en la fase
> 19, con el aparato delante.** No antes.

**Lo que eso NO significa:** que el ojo esté a medias. `qyro_eye` está entero,
tiene llamante de producción en `qyro beam`, y la cadena
*dibujar → rasterizar → decodificar → fountain → archivo* está probada de punta a
punta con uno de cada cuatro frames tirado. **Lo que falta es exactamente un
transporte de píxeles**, y su forma ya está fijada por la firma de
`Eye::look(&[u8], usize, usize)` — un plano de luma, que es literalmente el plano 0
de un `ImageProxy` en `YUV_420_888`.

**Lo que sí significa:** que `R7` sigue prometiendo cuatro canales y hay tres y
medio, y que el medio que falta **no es medio ojo, es el cable entre el ojo y la
cámara.** Escrito así para que nadie lea esta fase como terminada.

**La alternativa descartada, con su motivo:** `jni-sys 0.4.1` evitaría el `unsafe`
propio a cambio de un crate nuevo en el lock. No se descarta *por el crate* —
se descarta porque **tampoco se puede ejercitar sin aparato**, así que cambia una
incertidumbre por otra y encima añade una dependencia. Cuando haya aparato, esa
comparación se hace con las dos cosas ejecutándose, que es la única forma de
hacerla bien.
