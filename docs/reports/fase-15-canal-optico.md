# Fase 15 — el canal óptico

**Rama** `claude/qyro-cerrar-cadena-12` · **Commit del informe** `dc993d3` ·
**2026-08-18**

**Puerta ejecutada en `dc993d3`, el commit que este informe nombra**
(comprobación 16).

---

## 1. Qué hay

Un archivo sale por la pantalla como un flujo interminable de códigos QR. **Sin
red de ninguna clase**: sin cable, sin Wi-Fi, sin nada compartido. Una pantalla y
una cámara.

| Lo que ADR-0044 decidió | Estado | Dónde |
|---|---|---|
| §4 — Luby Transform escrito en el árbol, cero dependencias | **HECHO** | `rust/crates/qyro_fountain`, `3633ec0` |
| §2 — v27, nivel L, **byte crudo nunca Base64** | **HECHO** | `qyro_cli/src/optical.rs`, `0125f2e` |
| §3 — 5 FPS | **HECHO y medido**: 28 frames en 6 s = **4,7/s** | `flows.rs:606` |
| §5 — por encima de 20 MB se niega **con la estimación** | **HECHO** | `flows.rs:539` |
| §6 — el CLI dibuja, el teléfono lee | **HECHO**, y el binario no lleva decodificador | `dc993d3` |
| Receptor de CI | **HECHO, y mejor que un directorio de imágenes** | §4 |

Dos comandos nuevos: `qyro qr` (el código de este aparato, para una cámara) y
`qyro beam <archivo>` (el archivo entero, como flujo).

---

## 2. Comprobación 14 — llamante de producción, con archivo y línea

| Capacidad | Llamante de producción | Consumidor |
|---|---|---|
| `qyro_fountain::split` | `qyro_cli/src/flows.rs:584` | **CLI** |
| `qyro_fountain::encode` | `qyro_cli/src/flows.rs:607` | **CLI** |
| `qyro_fountain::encode_frame` | `qyro_cli/src/flows.rs:608` | **CLI** |
| `qyro_fountain::FRAME_HEADER_LEN` | `qyro_cli/src/flows.rs:564` | **CLI** |
| `optical::draw` | `flows.rs:494` (`qyro qr`), `flows.rs:609` (`qyro beam`) | **CLI** |
| `Vt::home` | `flows.rs:612` | **CLI** |

**Declarado, no olvidado:** `qyro_fountain::Decoder` **no tiene llamante de
producción**. Es correcto y es la decisión de ADR-0044 §6 — el receptor es el
teléfono, y un decodificador en el binario serían cientos de líneas de cámara
para una máquina sin cámara. Lo ejercita la prueba de vuelta completa, que es lo
que hay que exigirle: que exista, que funcione, y que **nadie lo anuncie como
capacidad del producto de escritorio**.

**Y una capacidad que se quedó sin llamante durante la fase, corregida en el
sitio:** `optical::footprint` dejó de usarse cuando el consejo de tamaño pasó a
medirse del dibujo. En vez de enviarla muerta, bajó a ayudante de pruebas.

---

## 3. Comprobación 15 — del gesto al byte

1. La persona escribe `qyro beam clave.pem`.
2. `flows.rs` lee el archivo. Si pasa de 20 MB **se niega y dice cuántos minutos
   habría tardado** (ADR-0044 §5): un canal que acepta en silencio un vídeo de
   dos horas no es generoso, es una trampa.
3. El payload se parte en bloques. **1 465 B es el techo, no el tamaño**: un
   archivo más pequeño que un bloque recibe un bloque de su tamaño, así que una
   clave de 4 KB dibuja un código pequeño en vez del más grande y difícil de
   escanear que ofrece el estándar.
4. Cada ronda: `qyro_fountain::encode` combina bloques elegidos por una semilla,
   `encode_frame` los serializa con 17 B de cabecera —**la forma viaja en cada
   frame**, porque quien apunta la cámara a un flujo ya en marcha no vio la
   cabecera que pasó antes.
5. `optical::draw` codifica en QR a nivel **L** y lo dibuja con medios bloques:
   dos filas de módulos por celda, porque una celda de terminal mide el doble de
   alto que de ancho y un QR a una celda por módulo sale estirado 2:1 y lo
   rechazan los lectores.
6. **Invertido a propósito**: un QR necesita oscuro sobre claro y una terminal es
   clara sobre oscuro. Se dibuja la luz como tinta. Se ve raro y se lee bien.
7. `Vt::home` devuelve el cursor arriba **sin borrar**: `ESC[2J` deja la pantalla
   en blanco entre frames y una cámara a 30 fps contra un flujo de 5 fps atrapa
   el destello y lee un frame de nada.
8. 200 ms de espera, y otra vez. **El flujo no termina nunca**, y eso es el
   diseño: no hay números de pieza que perder, y quien sostiene el teléfono para
   cuando su lado dice que ya está.
9. Al otro lado, el teléfono acumula frames hasta que `Decoder::is_complete`, y
   `finish` devuelve el archivo — **o nada**. Nunca un archivo parcial: uno casi
   correcto es lo peor que hay, porque falla el hash y nada explica por qué.

---

## 4. El receptor de CI: se hizo, y no como estaba planteado

**Planteado:** un directorio de imágenes.
**Hecho:** el mismo bucle, rasterizando en memoria, con un decodificador de QR
real que no ha visto este proyecto (`rqrr`, **sólo dev-dependency**).

Por qué así, y no es una excusa por no hacer lo otro:

- **Un fichero de fixture caduca.** Un cambio en el renderizador lo deja obsoleto
  y falla como *«se rompió el renderizador»* cuando la verdad es que la foto es
  vieja. Rasterizar en memoria no tiene esa forma de fallo.
- **No mete un decodificador de imágenes en el grafo.** `zune-jpeg` estaba
  pre-autorizada y no hizo falta: sin ficheros no hay JPEG que decodificar, y la
  trampa medida de MJPEG sin DHT no llega a existir.
- **Prueba estrictamente más.** Un directorio de imágenes prueba que *esas*
  imágenes se leen. Esto dibuja lo que dibuja `qyro beam`, en el momento, y lo
  vuelve a leer.

Las tres pruebas: que un decodificador real lee lo que dibuja la terminal; que un
archivo entero sobrevive el canal completo **con uno de cada cuatro frames
tirado**; y el control, que el escáner **no** encuentra un QR en tres líneas de
ruido — sin él, un `scan` que devolviera lo que se le pasa aprobaría las otras
dos.

---

## 5. Lo que corrigió el uso y no el diseño

Tres cosas que sólo aparecieron al ejecutarlo:

1. **El consejo de tamaño mentía.** Estimaba los módulos desde la longitud del
   payload y decía «37 columnas» para un código de 41. Un consejo que se queda
   corto es **peor que ninguno**: alguien ensancha la terminal exactamente a lo
   que dijo, el código sigue partiéndose, y la herramienta ya le mintió una vez.
   Ahora mide el dibujo que acaba de hacer.
2. **Un archivo de 51 bytes dibujaba una v27 entera** — el código más grande y
   difícil de escanear, para el payload más pequeño. Medido tras el arreglo: 37
   columnas donde antes 133.
3. **`DrawError` no se ganaba el sueldo.** El meta-guard pidió comprobar sus
   sitios de construcción y, al mirarlo, su único consumidor lo imprimía con
   `{:?}` — así que `TooLong` le llegaba a una persona como la palabra
   «TooLong». Un fallo sobre el que quien llama no puede ramificar es un mensaje.

---

## 6. Lo que costó, medido

| | `qyro.exe` (x86_64-pc-windows-msvc) |
|---|---|
| Antes del canal óptico | 1 306 624 B |
| Con la fuente, el renderizador y `qrcode` | **1 373 696 B** |

**+67 072 B (65 KB)** por un canal que funciona sin red ninguna. `rqrr` no pone
ni un byte: es dev-dependency y el binario no lleva decodificador, que es
ADR-0044 §6 cumpliéndose donde se puede medir.

Para comparar, y porque la fase 14 lo dejó anotado: `mdns-sd` cuesta **614 KB**.

---

## 7. La puerta, en `dc993d3`

| Comprobación | Resultado |
|---|---|
| `cargo test --workspace` | **711 pruebas, 0 fallos** |
| `cargo clippy --workspace --all-targets -D warnings` | 0 errores |
| `cargo audit --deny warnings` | 110 dependencias (16 de ellas sólo de prueba), 0 avisos |
| `flutter test` | 105 pruebas, 0 fallos (sin tocar en esta fase) |

Rust pasó de **679 a 711**.

---

## 8. Lo que esta fase NO promete

- **Una cámara.** Desenfoque, obturador rodante, moiré, brillo, pantalla en
  ángulo, y el decodificador del propio teléfono. Ahí es donde un canal óptico
  falla de verdad, y **no hay hardware y no se inventa**. Fase 19.
- **Que el teléfono lo lea.** La app de Android no tiene todavía el lado que
  acumula frames. El motor los produce y la prueba de vuelta completa demuestra
  que son legibles; conectarlo a la GUI no es esta fase.
- **Reanudación tras una interrupción.** ADR-0044 §5 dice que checkpoint y
  reanudación no son opcionales para sesiones largas. **No están.** Es trabajo de
  la fase 22, que es la que se ocupa de lo que la gente hace de verdad, y hasta
  entonces el límite de 20 MB es lo que impide llegar a una sesión que los
  necesite.
