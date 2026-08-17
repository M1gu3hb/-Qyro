# ADR-0044 — El canal óptico: QR animado, pantalla a cámara

- **Estado:** **congelada** antes de una sola línea de código.
- **Fecha:** 2026-08-17
- **Fase:** 15
- **Depende de:** ADR-0042 (CLI), ADR-0043 (enlace directo).
- **Gobernada por:** `R7` §R7.4 nivel 3 — *«no es necesario que los dispositivos
  estén literalmente conectados. Para eso también es lo de los QRs.»*

> **Todas las cifras de aquí salen de `R8`, que está medido con fuente y fecha.
> No se vuelven a investigar y no se inventa ningún throughput.**

---

## 1. Cuándo existe este canal, y cuándo no debe ofrecerse

**Nivel 3.** Se llega aquí cuando no hay red **ni cable**. Antes de proponerlo,
el producto pregunta por lo de arriba, porque `R8` §5.4 es contundente: si esa
máquina tiene lector de CD, disquetera, PCMCIA o tarjeta de red, **cualquiera de
las cuatro es entre 10 y 10 000 veces más rápida**. Un CD-R mueve 700 MB en cinco
minutos; este canal tardaría dieciséis horas.

---

## 2. Decisión 1 — **versión 27, nivel L, modo byte crudo**

`R8` §1. Tres proyectos reales convergen: BBQr señala **v27 como «a good sweet
spot»** y advierte contra v40; qr-backup usa v10 por resolución de webcam;
Sparrow manda fragmentos de 400 B.

| | |
|---|---|
| Versión | **27** — 125×125 módulos, **1 465 B** en nivel L |
| Corrección | **L**, y no más |
| Codificación | **byte crudo**, nunca Base64 |

**Por qué no v40.** 177 módulos + quiet zone = 185 de ancho; un decodificador
fiable quiere 3–4 px por módulo ⇒ ~648 px sólo para el código. A 1080p con el QR
ocupando el 60 % del alto se está en el límite, sin margen para desenfoque,
reflejo, moiré ni pulso. A v27 el mismo presupuesto da **4,9 px/módulo**.

**Por qué L.** Una pantalla no es papel. La corrección de errores del QR protege
contra suciedad y arrugas que aquí no existen; **lo que se pierde son frames
enteros**, y de eso protege el fountain code, no el nivel EC.

**Por qué byte crudo.** Base64 cuesta +33 %, los bytewords de BC-UR +37,5 %.
Controlamos los dos extremos, así que el modo byte cuesta **0 %**.

---

## 3. Decisión 2 — **5 FPS por defecto**, ajustable 3–10

`R8` §2. El limitante no es el ancho de banda: **son los frames perdidos**.
Pantalla y cámara no están sincronizadas y cualquier frame que caiga a caballo de
una transición es basura, así que la pantalla va muy por debajo de
`fps_cámara / 2`.

Tres fuentes independientes: txqr mide **6–7** óptimo en un barrido automatizado,
BBQr/Coldcard recomienda **4**, Sparrow usa **5** en código. Se empieza en 5 y se
acelera sólo si el receptor confirma que no pierde.

---

## 4. Decisión 3 — **Luby Transform, escrito en el árbol**

`R8` §3, y la razón que decide es legal, no técnica.

| | Luby Transform | RaptorQ |
|---|---|---|
| Implementar | **200–400 líneas** | miles |
| Overhead | 5–15 % | 0,02 % |
| Patentes | **ninguna viva** | **Qualcomm, IPR #2554** |

El compromiso de Qualcomm es **unilateral y condicionado** —exige implementar el
RFC completo y lleva cláusula de reciprocidad—, no una licencia limpia. Para un
proyecto cuyo punto entero es no depender de terceros, eso no entra.

**Y BBQr enseña el error que no hay que copiar:** sin fountain exige escanear las
N piezas —*«there is no way to skip one»*—, así que un frame perdido en el 90 %
obliga a empezar de nuevo. Con LT, el receptor sigue recogiendo hasta que puede
decodificar.

Se elige LT propio y no `ur` 0.5.2 en una línea: **300 líneas sin dependencias
pesan menos que cuatro crates en un binario que apunta a 750–950 KB**, y no hace
falta interoperar con carteras de criptomonedas.

---

## 5. Decisión 4 — **por encima de 20 MB se niega, y dice cuánto tardaría**

`R8` §4, y esto va **en la interfaz**, no en una nota al pie.

| Payload | Tiempo a 8 KB/s |
|---|---|
| Clave o certificado (≤4 KB) | < 1 s |
| Config o `.env` (≤50 KB) | < 7 s |
| Documento de texto (1 MB) | **2 min** |
| **Una foto (3–8 MB)** | **6–17 min** |
| Un minuto de vídeo | **2–5 h** |

Y las correcciones obligatorias antes de enseñar una estimación:

- **JPEG, PNG, MP4, ZIP: ganancia por compresión = 0.** Ya están comprimidos.
- **Texto, código, logs: gzip da 3–5×.** Ahí este canal brilla.
- **Una sesión desatendida de 3 h falla con probabilidad cercana a 1** —
  salvapantallas, notificación, batería, throttling térmico. **Checkpoint y
  reanudación no son opcionales.**

**Regla de producto:** por encima de **20 MB** se niega por defecto y explica por
qué con la estimación. `--force` existe y avisa. Un canal que acepta en silencio
un vídeo de 2 horas no es generoso: es una trampa.

---

## 6. Decisión 5 — **el receptor de QR es la GUI; el emisor es los dos**

La cámara vive donde hay cámara. El CLI **dibuja** el QR en la terminal —half
blocks `▀`/`▄`, que `R8` §11 verificó existen en cp437 como 0xDF/0xDC, y **nunca
Braille ni quadrant blocks, que no existen ahí**— y la GUI de Android **lee**.

Ésa es la escena real: el PC viejo enseña, el teléfono escucha. El PC viejo no
tiene cámara y el teléfono sí, y construir el lector para la terminal sería
construirlo para una máquina que no puede usarlo.

**La técnica de half-block divide la altura del QR por dos**, y es lo que hace
que un v27 quepa en una consola de 25 líneas. Se emiten los bytes OEM crudos
cuando la code page es 437/850, UTF-8 cuando es 65001, y ASCII `##` cuando no se
puede saber. **Nunca se llama a `chcp` por el usuario.**

---

## 7. Lo que esta ADR NO decide

- **El canal serie.** Fase 16, y es la respuesta a «la máquina no puede leer un
  QR porque no tiene cámara». Son hermanos, no alternativas.
- **Leer QR desde el CLI.** No se hace: ver §6.

---

## 8. Alternativas descartadas

**BC-UR con el crate `ur`.** MIT, correcto, y trae cuatro dependencias y un
formato de texto que cuesta +37,5 % en bits. Interoperar con el ecosistema
airgap de carteras no es un requisito de este producto.

**RaptorQ.** Gravado. Ver §4.

**v40 para ir más rápido.** El techo teórico sube y la tasa real **baja**, porque
los frames ilegibles se descartan enteros. Optimizar el número equivocado.
