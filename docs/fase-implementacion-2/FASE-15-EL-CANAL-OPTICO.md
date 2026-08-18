# FASE 15 — El canal óptico: QR animado

> El nivel 3 de la escalera. **No hay red de ninguna clase.** Una pantalla enseña, una
> cámara mira, y el archivo cruza el aire sin tocar un solo cable.
>
> **Lee `R8` §§1–4 entero antes de escribir una línea. Los números están medidos y
> hay uno publicado que está mal, y explicado por qué.**

---

## 1. Por qué existe, y qué NO va a resolver

Existe porque el propietario lo pidió con esta frase:

> «El objetivo también es que no es necesario que los dispositivos estén literalmente
> conectados. Para eso también es lo de los QRs.»

Y **no** va a resolver mandar fotos ni vídeo. Los números de `R8` §4, con el
escenario realista de 8 KB/s:

| | |
|---|---|
| Clave, seed, certificado | < 1 s |
| Config, script, `.env` | < 7 s |
| Texto o código comprimido, 1 MB | 2 min |
| PDF de 5 MB | 11 min |
| **Una foto JPEG** | **6–17 min** |
| **Un minuto de vídeo 1080p** | **2–5 h** |

**Esto es un canal de 1–10 KB/s. Es un módem de los noventa con pasos extra.** Y aun
así es el único canal que funciona cuando no hay absolutamente nada más — que es
exactamente el caso para el que se pidió.

**La honestidad va en la interfaz, no en la documentación.** Ver §4.

---

## 2. La decisión que hay que congelar

`docs/adr/ADR-00XX-canal-optico.md`. **Todo lo de abajo ya está decidido por la
investigación; la ADR lo registra con su fuente, no lo vuelve a discutir.**

| Decisión | Valor | Fuente |
|---|---|---|
| Versión de QR | **20–27**, adaptativa | BBQr: v27 «a good sweet spot»; qr-backup: v10 por webcam |
| Corrección de errores | **L** | BBQr: «only showing them on a perfect LCD screen» |
| Codificación | **modo byte crudo** | Base64 cuesta +33 %, Bytewords +37,5 %. Controlas ambos extremos |
| Payload por frame | **858–1 465 B** | `R8` §1 |
| Frame rate | **5 FPS** por defecto, 3–10 ajustable | divan 6–7, BBQr 4, Sparrow 5 |
| Fountain code | **Luby Transform, en el árbol** | RaptorQ tiene IPR de Qualcomm (#2554, 2015-03-19) |
| Techo por defecto | **20 MB**, se niega y explica | `R8` §4 |

Lo que la ADR sí tiene que decidir de verdad:

1. **LT propio o `ur` (BC-UR).** LT propio son ~300 líneas y cero dependencias. `ur`
   0.5.2 (MIT) da interoperabilidad con el ecosistema airgap desplegado —Sparrow,
   Keystone, Passport— a cambio de 4 dependencias y **+37,5 % de overhead por
   Bytewords**. **Elige, escribe por qué en una línea, y sigue.**
2. **El formato del header por frame.** Necesita: identificador de transferencia,
   número de símbolos K, tamaño total, semilla del símbolo, y checksum. Y **una
   versión**, para que un Qyro futuro pueda cambiarlo sin ambigüedad.
3. **Cómo se autentica.** Esto es lo importante y no es obvio: el canal óptico **no
   tiene handshake bidireccional** — la pantalla no ve a la cámara. Decide:
   - o el payload va **cifrado con una clave derivada de un código corto** que el
     usuario teclea o que va en un QR estático inicial,
   - o va **firmado** por la identidad del emisor y el receptor comprueba la huella,
   - o las dos. **Escribe qué garantiza y qué no**, y llévalo a `THREAT_MODEL.md` en
     la fase 18. Un canal sin handshake tiene adversarios distintos: una cámara más
     en la habitación, una grabación de pantalla, un hombro.
4. **Checkpoint y reanudación.** `R8` §4: en una sesión de horas la probabilidad de
   fallo se acerca a 1 —salvapantallas, notificación, batería, throttling térmico—.
   **No es opcional.** Decide cómo se persiste el estado parcial y cómo se reanuda.

---

## 3. Entregables

1. **La ADR de §2, congelada en su propio commit.**
2. **`qyro_optical`** (o dentro de `qyro_transfer`, decide tú): el LT fountain, el
   troceado, el header, el ensamblado y la verificación. **Con `#![forbid(unsafe_code)]`.**
3. **El emisor en el CLI.** `qyro send <archivo> --optical`:
   - Estima **antes de empezar** y **enseña la estimación** (§4).
   - Renderiza los QR en la terminal con **half-block** (`R8` §11): `█ ▀ ▄` y
     espacio, emitidos como **bytes OEM 0xDB/0xDF/0xDC** cuando la code page es
     437/850, UTF-8 cuando es 65001, y ASCII `##`/`  ` como último recurso. **Nunca
     Braille ni quadrant blocks — no existen en cp437.**
   - `--qr-png <dir>` como salida alternativa, para pantallas donde la terminal no
     dé el ancho.
   - **Bucle infinito hasta que el receptor confirme o el usuario pare.** Es un
     fountain: no hay «se acabó la vuelta», hay «ya tengo suficiente».
4. **El receptor.** Aquí hay que decidir el alcance y **decidirlo tú**:
   - **Mínimo honesto:** el receptor es la **aplicación de Android**, que ya es un
     aparato con cámara. Eso requiere una pantalla nueva en Flutter y un decodificador
     — y `R7` §R7.1 dice que alcanzar máquinas manda, así que esto solo no basta.
   - **Lo que de verdad hace falta:** `qyro recv --optical` en el CLI, leyendo de una
     **webcam** con `rqrr` (pre-autorizado) y de **un vídeo o una carpeta de imágenes**
     como entrada alternativa. La entrada por archivo es la que se puede probar en CI
     y la que hace la fase verificable.
   - **Empieza por la entrada de archivo/vídeo**, que cierra la fase con evidencia
     real, y añade la webcam después. Si la webcam no cierra en esta fase, **regístralo
     y sigue** — no es un P0.
5. **La estimación, y el techo.** Ver §4.

---

## 4. La honestidad va en la pantalla

**Antes de mostrar el primer QR**, Qyro dice:

```
Archivo:      vacaciones.jpg
Tamaño:       4,2 MB  (ya comprimido: JPEG, no se puede reducir)
Canal:        óptico (QR)
Velocidad:    ~8 KB/s
Tiempo:       unos 9 minutos

Un cable de red entre los dos aparatos tardaría menos de un segundo.
¿Seguimos igual?  [s/N]
```

Reglas, y son de producto:

- **Estima con el número medido, no con el techo.** El techo teórico es 43 KB/s y
  nadie lo sostiene.
- **Di si el archivo ya está comprimido.** JPEG, PNG, MP4, ZIP: ganancia 0. Texto,
  código, logs: 3–5× con zstd, y entonces **comprime y dilo**.
- **Por encima de 20 MB, negarse por defecto**, explicar por qué, y ofrecer `--force`
  con la estimación repetida.
- **Ofrece el canal rápido si existe.** Si hay red o hay cable, proponerlo primero es
  parte de resolver el problema del usuario, no una interrupción.
- **Progreso real**: símbolos recibidos sobre K necesarios, y el tiempo que falta
  **recalculado con la tasa medida**, no con la estimada.

---

## 5. La prueba que cierra la fase

> **Un archivo entra por un extremo, sale por el otro, y el SHA-256 coincide — sin
> que ningún socket se abra en ningún momento.**

En CI, sin cámara, eso se monta así y es completamente verificable:
el emisor escribe los frames como **PNG a un directorio**; el receptor los lee **en
orden aleatorio**, **descartando el 20 % al azar con una semilla fija**, y aun así
reconstruye. Eso prueba las dos propiedades que importan: que el fountain funciona y
que el orden no importa.

**Tres controles, obligatorios:**
1. Descartando el **60 %**, la reconstrucción **falla** con un error nombrado, no se
   cuelga para siempre. (Un fountain sin límite de paciencia es un programa colgado.)
2. **Un frame corrompido** —un bit cambiado— se **rechaza por checksum** y no
   envenena el ensamblado.
3. Con el fountain **neutralizado** (símbolos secuenciales sin XOR), la prueba del
   20 % descartado **falla**. Sin esto no se sabe si el fountain hizo algo.

**Y la medida de throughput, escrita:** frames por segundo × bytes por frame,
medidos sobre la ejecución de CI, comparados con los 6–10 KB/s de `R8` §4. Si sale
muy por encima, **la prueba no está midiendo lo que crees** —probablemente no cuenta
la adquisición—, que es exactamente el error del post de divan documentado en `R8`.

---

## 6. La puerta

Quince comprobaciones. Y en la 14, el llamante de producción del decodificador tiene
que ser un flujo de usuario, no un test. **`MdnsDiscovery` es la lección: un módulo
perfecto sin llamante es un módulo que no existe.**

---

## 7. Lo que NO hay que hacer

- **No inventes un throughput.** Los de `R8` §4 están medidos y uno publicado está
  mal; si mides otro, escribe cómo lo mediste.
- **No uses versión 40.** Ni «por si acaso». Tres proyectos en producción convergen
  en no hacerlo.
- **No uses Base64.** Controlas los dos extremos.
- **No copies BBQr en lo de exigir las N piezas.** Su propia spec lo dice: *«All 'N'
  QR codes must be scanned, there is no way to skip one»*. Eso es lo que el fountain
  arregla.
- **No prometas fotos ni vídeo.** En ningún documento, en ninguna pantalla, en
  ninguna Release.
- **No añadas audio.** `R8` §5.3: ggwave son 12 B/s reales y 1 MB son 17 horas.
  Queda registrado como descartado, con su número.
