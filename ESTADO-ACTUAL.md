# Estado actual — dónde se corta

**2026-08-18** · rama `claude/qyro-cerrar-cadena-12` · último commit de esta
sesión abajo.

> Este archivo dice **dónde se corta y qué es lo siguiente**, para que quien siga
> no tenga que reconstruirlo leyendo commits. Se actualiza al cerrar cada paso.

---

## 1. Lo que se cerró en esta sesión

**El gate rojo, primero.** `check_docs_consistency` estaba en rojo en `5459a64`
—*«Stale verified commit: HEAD is 11 commits ahead»*— y se arregló actualizando
el ancla de `STATUS.md` **y volviendo a correr la puerta sobre el commit
resultante**, que es la comprobación 16 aplicada a sí misma.

**Fase 21 — A MEDIAS, y aquí es donde se corta.**

| Commit | Qué |
|---|---|
| `8876334` | **ADR-0046 congelada**, en commit propio antes del código |
| `88ecbe2` | El consejero de canal en `qyro_session`, y `qyro how` que lo llama |
| `107685e` | La tabla de paridad y `scripts/check_parity.ps1` |

**Hecho:** ADR-0046 (qué significa que una capacidad existe, dónde vive la tabla,
dónde vive el consejero, y que el motor devuelve la frase y no un código). El
consejero con las cifras de `R8`, ejecutado. La tabla de doce capacidades con su
script, **vista fallar tres veces** — celda vacía, referencia rota, y fila
borrada. La tercera no fallaba: el piso era 10 sobre una tabla de 12, así que
borrar una fila pasaba en verde. Ahora es el número exacto.

**Lo que falta, con nombre:**

1. **La matriz de cuatro casillas** (§4 del documento de fase), que es la prueba
   que cierra la fase: GUI↔GUI, GUI↔CLI, CLI↔GUI, CLI↔CLI, un archivo por
   casilla verificado byte a byte. **Las dos caras están construidas y listas
   para ello**: `target/release/qyro_ffi.dll` (820 KB) y `target/release/qyro.exe`
   (1 410 KB), y el arnés que ya existe es
   `apps/qyro/test/transfer/two_process_pairing_test.dart`, que sólo necesita
   `QYRO_FFI_LIBRARY_PATH`. Para la fase 21 el otro proceso tiene que ser el
   binario `qyro` de verdad, **no el arnés de humo** — el arnés es lo que escondió
   el defecto de la identidad cinco fases seguidas. Con sus tres controles:
   receptor apagado falla por nombre, clave cambiada rechazada en las cuatro, y
   la tabla ya tiene el suyo.
2. **El consejero en la GUI.** Un símbolo nuevo en la frontera C que escriba la
   frase en un buffer prestado, como los que ya hay. Es la única fila de la tabla
   que dice «todavía» en vez de «no».

**No hay informe de fase 21 y es a propósito:** no está cerrada, y un informe que
dijera «falta esto» es exactamente lo que este taller no escribe.

---

**Fase 16 — HECHA.** Informe en `docs/reports/fase-16-canal-serie.md`, puerta
corrida en `5699fcd`.

| Commit | Qué |
|---|---|
| `4e88f37` | **ADR-0045 congelada**, en commit propio antes del código |
| `5699fcd` | `qyro_serial` (ARQ + CRC32 + Base64), los tres comandos, y el generador del receptor |

**El defecto que encontró ejecutar el script generado, antes de enviar nada:**
`BLOCK_BYTES` era 512, que no es múltiplo de tres, así que cada bloque codificaba
con relleno `=` y al concatenarlos el `=` quedaba en medio del flujo.
`certutil` lo rechaza —*«DecodeFile devolvió Datos no válidos. 0x8007000d»*— y la
transferencia habría informado de éxito con la otra máquina vacía. Ninguna prueba
interna lo veía: el decodificador de Qyro trabaja línea a línea y estaba de
acuerdo consigo mismo. **510**, y la invariante es un `const assert`.

**La puerta se puso en rojo y no por el código:** `rqrr` arrastra `lru` 0.12.5
con dos avisos de unsoundness y fija esa minor. Ignorados en `.cargo/audit.toml`
con qué son, por qué no llegan al producto y **qué los borra** — y con una guarda
que falla si `rqrr` deja de ser `dev-dependency`.

---

**Fase 15 — HECHA.** Informe en `docs/reports/fase-15-canal-optico.md`, puerta
corrida en `dc993d3`.

| Commit | Qué |
|---|---|
| `3633ec0` | `qyro_fountain`: Luby Transform, cero dependencias, generador congelado porque es formato de cable |
| `0125f2e` | `qyro qr` y `qyro beam`: medios bloques, invertido a propósito, 5 FPS |
| `dc993d3` | La vuelta completa: un decodificador real lee lo que dibuja la terminal |
| `ab947ab` | El informe |

**Lo que corrigió el uso y no el diseño:** el consejo de tamaño mentía (decía 37
columnas para un código de 41 — un consejo corto es peor que ninguno); un archivo
de 51 bytes dibujaba una v27 entera, el código más difícil de escanear para el
payload más pequeño; y `DrawError` no se ganaba el sueldo, porque su único
consumidor lo imprimía con `{:?}` y «TooLong» le llegaba a una persona como la
palabra TooLong.

**El receptor de CI se hizo, y no como estaba planteado.** No un directorio de
imágenes: rasterizando en memoria, con `rqrr` de dev-dependency. Un fixture
caduca y falla como «se rompió el renderizador»; esto dibuja lo que dibuja
`qyro beam`, en el momento, y lo vuelve a leer. `zune-jpeg` no hizo falta y la
trampa del MJPEG sin DHT no llega a existir.

**Coste medido:** +67 KB en el binario (1 306 624 → 1 373 696). `rqrr` no pone
ninguno: no viaja.

---

**Fase 14 — HECHA.** Informe en `docs/reports/fase-14-sin-router.md`, puerta
corrida en `07278ff`, el commit que el informe nombra.

| Commit | Qué |
|---|---|
| `f81c15a` | La cuenta atrás de APIPA (`qyro_session/src/link.rs`) y la trampa de `SocketAddrV6` |
| `b89a89a` | **ADR-0043 enmienda 1**, en commit propio antes del código |
| `dab9fa3` | El beacon por interfaz con `socket2`, y el puerto colapsado a una definición |
| `07278ff` | El lado Dart de `dev.qyro/discovery` y su llamante de producción |
| `f50ab2c` | El informe de la fase 14 |

**Dos hallazgos que no buscaba, los dos con cifra:**

- **D9** — `mdns-sd` casi dobla el binario: **666 624 → 1 295 872 bytes** al
  llegar `qyro find`. **+614 KB**, diez veces los 63 KB que este taller discutió
  para conservar el desenrollado de pila. El beacon propio hace lo mismo por
  **8 KB**. La ADR-0043 §7 citaba un presupuesto de 750–950 KB que el binario ya
  no cumple; la enmienda 1 lo corrige con la medida. **No se toca hoy** — lo
  decide la fase 19 con red de verdad.
- **D10** — el puerto que ADR-0041 congeló estaba escrito **dos veces y en
  ningún sitio del motor**, bajo un comentario que decía «no re-derivado: dos
  copias son dos puertos» siendo la segunda copia. Cerrado: `qyro_net::QYRO_PORT`
  es el original y una guarda lee el `.dart` y falla si se separan — **vista
  fallar a propósito** antes de darla por buena.

---

## 2. Lo siguiente, en orden

```
21 (a medias) → 22 → 17 → 18 → 19 → 20 → 23
```

- **21 — las dos caras se hablan.** ADR, consejero y tabla hechos; **falta la
  matriz de cuatro casillas y el consejero en la GUI**. Detalle arriba.
- **22 — lo que la gente hace de verdad.** Carpetas, tamaño, interrupción.

---

## 3. Lo que sigue en blanco, y sigue en blanco a propósito

- **Cero pruebas en hardware físico.** Dos procesos en `127.0.0.1` no son dos
  máquinas. Que dos aparatos se encuentren por un cable **no está verificado**.
- **`NsdManager` no está ejercitado.** Las pruebas Dart usan un `MethodChannel`
  falso: prueban el lado Dart, no Android.
- **Ninguna cámara ha leído un QR de Qyro.** La vuelta completa la hace un
  decodificador sobre píxeles perfectos. Desenfoque, obturador rodante, moiré,
  brillo y pantalla en ángulo son fase 19.
- **El teléfono no acumula frames todavía.** El motor los produce y son legibles;
  el lado Android que los junta no existe.
- **La GUI y el CLI no se han hablado nunca.** Es la fase 21 y está a medias.
- **Ningún cable serie ha llevado un byte de Qyro.** El protocolo se probó
  sobre una cola en proceso y `certutil` sobre bytes reales; los dos puertos de
  esta máquina son endpoints Bluetooth, no un par enlazado. Fase 19.
- **El canal serie no llega a la GUI.** No hay símbolo en la frontera C, y
  ninguna pantalla lo menciona.
- **La reanudación del canal óptico no existe** (D11). ADR-0044 §5 la exige para
  sesiones largas; el límite de 20 MB es lo que hoy impide llegar a una.
- **La GUI de escritorio no tiene descubrimiento.** No hay símbolo en la
  frontera C. Lo dice con una frase, no con una lista vacía.
- El binario **no arranca en Windows 7** (`api-ms-win-core-synch-l1-2-0.dll`,
  fase 17).

---

## 4. Cuatro trampas de este entorno, para no repetirlas

1. **Heredocs de bash** destrozan `\n` y `\t` antes de que Python los vea. Usa
   `chr(92)`, escribe el script con la herramienta Write, o usa Edit.
2. **`git commit -m @'...'@` en PowerShell** se rompe si el mensaje lleva
   comillas: escribe el mensaje a un archivo y usa `git commit -F`.
3. **Flutter no está en el PATH.** Está en `D:\flutter\bin`.
4. **`verify_static.ps1` exige `-Binary`**, y el binario de la tubería es
   `target/x86_64-pc-windows-msvc/release/qyro.exe` — no `target/release`, que
   se compila con otro perfil y pesa distinto.

---

## 5. La regla que más valor dio, otra vez

**Cuando una guarda te dice que estás equivocado, tiene razón más veces de las
que crees.** En esta sesión pararon tres y acertaron las tres: el registro de
`beacon.rs`, `clippy` sobre un `assert!` entre constantes que se optimiza y no
prueba nada, y —la mejor— `qyro_session_re_exports_nothing_it_does_not_own`
rechazando `pub use qyro_net::QYRO_PORT`, porque todo lo que la fachada republica
se vuelve nombrable desde `qyro_ffi` y una excepción juzgada inofensiva de una en
una es cómo llega la primera peligrosa.

Y una cuarta cosa lo dijo sin ser una guarda: **el enlazador**. Con `beacon.rs`
escrito y sin llamante el binario no cambió ni un byte. Una capacidad sin
llamante no se envía, se compila.
