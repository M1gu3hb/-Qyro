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

## 0.ter — FASE 22, ABIERTA. Aquí se corta.

**ADR-0047 congelada** (`b5f5e97`), con los cinco números que la fase pedía. Dos
salieron de mirar en vez de suponer:

- **El desbordamiento de 4 GiB no existe.** `done` y `total` son `u64` en el
  motor y en la frontera C; el único `u32` es `item`, que vale siempre cero. La
  aritmética está bien; **la evidencia con archivos grandes sigue faltando**, y
  son dos cosas distintas.
- **`request_resume` tiene cero llamantes de producción** — sólo un test, sin
  símbolo C ni bandera de CLI. Habría sido el noveno caso. **ADR-0047 §5 la
  retira de la v1.x**, con argumento aritmético y dejando el número de mensaje
  reservado.

**Lo siguiente, en orden:**

1. **Ejecutar la retirada de §5.** Marcar `#[cfg(test)]` o borrar
   `request_resume` y el manejo de `MessageType::Resume` en
   `qyro_transfer/src/session.rs`, **y quitarla de todos los documentos que la
   mencionan** — una capacidad retirada que sigue anunciada es la misma mentira
   que una muerta.
2. **Los cinco escenarios** de `FASE-22 §4`, cada uno con su control. **El quinto
   deja de aplicar** si la reanudación se retira: se sustituye por comprobar que
   **cancelar deja el destino limpio**, sin `.qyro-part`.
3. ~~El saneado de nombres para terminal~~ **HECHO**, y con él **QYR-0364**: el
   receptor del CLI preguntaba «¿aceptas?» con una huella y nada más, mientras
   la GUI enseñaba los archivos desde siempre. `Session::offered_files` existe,
   el receptor los dibuja, y cada nombre pasa por `safe_terminal_name`.

4. ~~El techo de 256 archivos~~ **HECHO** (`3520b14`). `TooManyFiles` con código
   propio `-14` en la frontera, rechazo antes del primer descriptor, y su control:
   el techo exacto **no** se rechaza, porque un `>=` mal escrito movería el límite
   real a 255 sin que nadie lo notara.

**Queda de la fase 22: cuatro de los cinco escenarios de `FASE-22 §4`.** Todos
viven en `apps/qyro/test/transfer/gui_cli_matrix_test.dart`, que ya tiene el
arnés montado —biblioteca, binario, y el `tearDown` que espera a que el puerto
se suelte— así que cada uno es una casilla más:

1. ~~Carpeta con subcarpetas y una vacía~~ **HECHO** (`a932ec2`). Árbol comparado
   entrada por entrada; la carpeta vacía no viaja y está afirmado.
2. **200 archivos.** Con el techo en 256, el escenario es que 200 pasen y que
   257 se nieguen por número — la segunda mitad ya tiene prueba unitaria, falta
   la de extremo a extremo.
3. **Un archivo > 4 GiB**, esparcido para no gastar disco. El control: el
   progreso del último frame **no es menor** que el del anterior. La aritmética
   ya se comprobó (`done`/`total` son `u64`, ADR-0047 §2.1); **falta la
   evidencia**, que es otra cosa.
4. **Disco lleno a mitad.** El destino no queda con ningún `.qyro-part`, y su
   contra-prueba: dejar uno a propósito y exigir que el mismo listado lo vea.
5. **Cancelar a mitad** — sustituye al escenario de reanudación, que ADR-0047 §5
   retiró. El destino tiene que quedar limpio.

---

## 0. LA RELEASE — retractada en público, y a medio rehacer

**Hecho hoy, y está vivo en
<https://github.com/M1gu3hb/-Qyro/releases/tag/v1.0.0>:**

- **Retractación pública** encabezando las notas, con los dos P0 explicados por su
  nombre, qué los causó, y **por qué no los detectó nada** — cada pieza verde y la
  cadena rota. El título dice `RETRACTADO: estos binarios no pueden enviar`.
- **`qyro-cli-windows-x64-QYR-0361-arreglado.zip` subido**, con su `LEEME.txt`,
  `SHA-256 b78199c147d93255…`. Verificado **antes** de subirlo: dos copias con
  huellas distintas, 20 000 bytes que cruzan, hash idéntico en destino.

**Lo que falta, y está dicho también en las notas públicas:**

1. **El APK — y hay un bloqueo con nombre.** `app-release.apk` sigue siendo el de
   antes y **no lleva el arreglo de QYR-0362**: la aplicación sigue sin poder
   enviar.

   **No se puede reconstruir en esta máquina y no es falta de tiempo:**
   `flutter doctor` encuentra el SDK de Android 36.0.0 pero **las licencias no
   están aceptadas** (`Android license status unknown`). Aceptarlas es aceptar un
   acuerdo legal en nombre del dueño, y eso no lo hace el implementador —
   **lo tiene que hacer una persona**, con `flutter doctor --android-licenses`, o
   hacerlo el CI con sus propias credenciales.

   Hasta entonces el hueco se queda en blanco y **está dicho en las notas
   públicas de la Release**, no sólo aquí.
2. **`qyro-windows-x64.zip`**, el paquete completo con la GUI de escritorio,
   tampoco está rehecho.

No se borró nada ni se despublicó: la nota se queda aunque el fallo esté
corregido, porque quien descargó aquello merece saber qué tenía en las manos.

---

**Fase 21 — HECHA.** Informe en `docs/reports/fase-21-las-dos-caras.md`, puerta
corrida en `52fa4d5`.

**Las cuatro casillas de la matriz pasan**, con el binario `qyro` de verdad al
otro lado y comparación byte a byte, más los dos controles. Tabla de paridad con
su script —vista fallar tres veces— y el consejero de canal en las dos caras
(`qyro_advice`, 24 → 25 símbolos con enmienda en ADR-0032).

**Lo que encontró vale más que lo que construyó:** tres defectos, los tres de la
misma forma —dos mitades probadas y el medio jamás recorrido— y ninguno lo vio
leer código.

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
(retractar la Release) → 22 → 17 → 18 → 19 → 20 → 23
```

- **22 — lo que la gente hace de verdad.** Carpetas, tamaño, interrupción.
  **Es lo siguiente**, después de la Release.

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
