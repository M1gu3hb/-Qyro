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

## 0. LO PRIMERO DE LA SIGUIENTE SESIÓN — **las dos caras tenían el envío roto**

**Ninguna de las dos había enviado nunca un archivo.** Las dos se encontraron el
mismo día, por la misma razón: la fase 21 pone una cara contra la otra.

- **QYR-0361 (P0, arreglado y verificado, `9274393`).** `qyro send` pasaba a
  `open_sender` el nombre pelado con `root` = el directorio padre, y
  `strip_prefix` falla siempre. Todo envío devolvía `BadArgument`. **Verificado
  ejecutando**: dos copias del binario, huellas distintas, 5 000 bytes que
  aterrizan.
- **QYR-0362 (P0, arreglado, evidencia parcial).** `NativeTransferService.send`
  escribía `port.sendPort` **dentro** del closure de `Isolate.run`, lo que hace
  capturar el `ReceivePort` — no enviable. Todo envío moría con «object is
  unsendable» antes de mover un byte. Arreglado sacando `sendPort` fuera.
  **Falta ver el archivo aterrizar**: la casilla GUI→CLI falla ahora en «nothing
  was materialised», que es **otra capa y está sin diagnosticar**.

**Y hay una Release publicada con las dos.** Se retracta y se republica, como con
`2c01de0`. No se hizo aquí porque tocar una Release publicada necesita contexto de
sobra y la evidencia de la matriz completa.

**«nothing was materialised» YA ESTÁ DIAGNOSTICADO** — falta confirmarlo y
arreglarlo, que es lo primero.

`NativeTransferService._commonRoot` parte la ruta por `Platform.pathSeparator`
(`\` en Windows) y quita el último trozo. La prueba construye la ruta con
`'${source.path}/payload.bin'` — **barra normal** — así que el último trozo es
`out/payload.bin` entero, la raíz sale siendo el *abuelo*, y el nombre relativo
que viaja es `out/payload.bin`. El archivo aterriza en
`destination/out/payload.bin` y la prueba mira `destination/payload.bin`.

**Dos cosas que hacer, y son distintas:**

1. **La prueba mezcla separadores.** Usar `Platform.pathSeparator` o
   `path.join`. Eso sólo arregla la prueba.
2. **`_commonRoot` es frágil ante separadores mezclados**, y en Windows una ruta
   con barras normales es perfectamente válida — la acepta todo el API de
   Win32. Un archivo elegido por el selector del sistema no las traerá, pero
   uno que llegue por argumento, por arrastrar-y-soltar o por una prueba, sí.
   **Decidir si eso es defecto o límite documentado**, y si es defecto, normalizar
   antes de partir. No se decidió aquí por falta de contexto, y adivinarlo sin
   comprobarlo sería inventar.

---

## 0.bis — la Release rota (contexto de arriba)

**QYR-0361, P0, arreglado en `9274393`.** `qyro send` **no ha movido nunca un
byte**: pasaba a `open_sender` el nombre pelado del archivo con `root` = el
directorio padre, y `strip_prefix` falla siempre, así que toda invocación
devolvía `BadArgument` impreso como *«could not connect»*. Desde la fase 13, el
día que se escribió, **y hay una Release publicada con él**.

**Se retracta y se republica**, como se hizo con `2c01de0`. No se hizo aquí
porque tocar una Release publicada no se hace con el contexto justo, y porque el
arreglo tiene que ir con la evidencia de la matriz completa.

Lo encontró **la fase 21 haciendo su trabajo**: no lo vio leer código, lo vio
poner una cara contra la otra por primera vez. Es la **sexta** costura que este
proyecto envía sin que ninguna prueba la cruzara.

---

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

1. **La matriz de cuatro casillas**, en
   `apps/qyro/test/transfer/gui_cli_matrix_test.dart`. **Escrita y a medio
   pasar.** Se corre con `QYRO_FFI_LIBRARY_PATH` y `QYRO_CLI_PATH` puestos.
   - **CLI→CLI: verificado a mano y funciona** tras el arreglo del P0 — dos
     copias del binario, huellas distintas, 5 000 bytes que llegan.
   - **CLI→GUI: PASA.** Era el P0 QYR-0361 quien lo bloqueaba, no el
     cortafuegos.
   - **CLI→CLI: PASA**, con dos copias del binario y huellas distintas.
   - **GUI→CLI y GUI→GUI: escritas y fallando** en «nothing was materialised»,
     ya pasado el arreglo de QYR-0362. Es lo primero de la siguiente sesión.
   - Los dos controles escritos —«nadie escuchando» y «huella que no coincide»—
     **pasan los dos**.
2. ~~El consejero en la GUI~~ — **HECHO** en `3758be3`: `qyro_advice` cruza la
   frontera (24 → 25 símbolos, con enmienda en ADR-0032) y cinco pruebas Dart lo
   ejercen contra la biblioteca de verdad. La tabla de paridad ya no tiene
   ninguna fila que diga «todavía».

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
