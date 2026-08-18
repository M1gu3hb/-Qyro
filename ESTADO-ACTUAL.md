# Estado actual — la fase 24B, y lo que le falta medir

**2026-08-19** · **rama unica: `main`**

## La fase 19 esta lista, y es del propietario

`docs/testing/hardware-protocol.md` tenia veinte escenarios y **ninguno de los
tres canales nuevos**. Ahora tiene la seccion F: cable directo, canal optico,
canal serie, y la maquina que no puede instalar nada. **36 huecos, todos en
blanco.**

Cada uno trae el comando exacto. Y tres piden el numero que falta:

- **F1:** cuantos segundos tarda APIPA de verdad. Es la primera vez que se
  mediria fuera de `R8`.
- **F2:** **los fps que sostiene el telefono.** Es la medida que ADR-0048 §4 dejo
  en blanco: si son >=5, el puente esta hecho para siempre; si no, el JNI de
  copia cero tiene su argumento medido.
- **F4:** si arranca en un Windows 7 de verdad. ADR-0049 dice que **no esta
  confirmado en `msvc`**.

**No se ejecuto ninguno**, y eso es lo correcto: hace falta hardware, y un
escenario sin marcar no es un aprobado.

---

## Fase 20 — el arranque resuelto, y la decision de firma SIN tomar

- **`qyro send --self`** manda el propio binario. Es la respuesta al arranque:
  una vez hay un Qyro corriendo, se lleva a si mismo a la siguiente maquina --
  800 KB, ochenta segundos por serie. Con su control: sin `--self`, una ruta
  sigue haciendo falta, porque un `--self` que se aplicara siempre convertiria
  `qyro send informe.pdf` en `qyro send qyro.exe` en silencio.
- **`docs/release/DECISION-DE-FIRMA.md`**: los numeros y las consecuencias
  ordenados para decidir en cinco minutos. **NO decidida** — cuesta dinero.

**Lo que el implementador si dice**, y esta escrito ahi: el caso de uso empuja
hacia no firmar, porque la maquina que Qyro existe para servir recibe el archivo
por USB o por el propio Qyro, y en las dos rutas **el certificado no cambia
nada**. Firmar compra sobre todo la primera impresion de quien descarga en una
maquina normal, que es otro publico.

**Hechos tambien:** `BUILD-INFO.txt` en el artefacto de Windows —con el sha256 y
**NO FIRMADO en mayusculas**— y `docs/release/INSTALAR.md`, que son cinco pasos y
el segundo es el USB.

**Queda de la 20, y esta dicho:** la pagina de la Release **no se toco** —la
redaccion esta lista para copiar en `DECISION-DE-FIRMA.md` §6, y publicar es una
accion hacia fuera que ya lleva dos correcciones esta semana— y el
`BUILD-INFO.txt` **solo esta en el artefacto de Windows**; el job de musl tiene su
propio `upload-artifact` sin tocar.

---

## Fase 18 — la verdad, y dos frases que eran falsas

- **La Release prometia cifrado sin decir por que canal.** Es cierto por la red;
  el QR y el serie degradado **no cifran nada** — el fountain codifica, que no es
  lo mismo. Corregido a «**Por la red**...», con las excepciones nombradas.
- **`THREAT_MODEL.md` describia un canal de cuatro.** §4.bis, nueva: el optico es
  **difusion, no punto a punto** y no puede haber handshake; el serie degradado
  no autentica nada **y un cable es el canal mas privado de los cuatro**; y una
  direccion nunca es una identidad (RFC 3927 §5).
- **Deuda:** D1 y D6 cerradas. D2 ya lo estaba. Quedan cinco, **y ninguna es una
  afirmacion falsa**.

**D6 se gano el sueldo al primer intento**: tres enlaces rotos, uno de ellos
publico apuntando a un item privado. Y **entro sola en la puerta** — `gate.ps1`
lee `ci.yml`, asi que paso de 5 comandos a 6 sin tocar el script.

---

## Fase 17 — cerrada, con el binario en CI y no aqui

ADR-0049 congelada. Job `win7-builds.yml` con `-Z build-std` y los cuatro
targets, y `check_win7_imports.ps1` **con su control**: el binario normal DEBE
fallar la comprobacion, o el `[PASS]` del otro no vale nada. Visto fallar con las
tres entradas que tiene que rechazar.

**No se compilo un binario de win7 aqui**: `-Z build-std` necesita nightly y
`rust-src`, ~1,5 GB en el disco de sistema de esta maquina, que va justo. Lo
compila el runner y sube el binario y su tabla de imports.

**Y por eso ADR-0049 §3 deja la confirmacion sobre `msvc` como PENDIENTE.** `R8`
§10 midio sobre `-gnu`; el codigo de `std` es el mismo y eso es un argumento, no
una medida. Hasta que ese `dumpbin` corra, **este proyecto no afirma que
Windows 7 funcione.**

---

## QYR-0365: la medida desmiente el diagnostico

`rust/crates/qyro_session/tests/qyr_0365_measurement.rs`, con los contadores
`Session::step_tally` que la ficha pedia por su nombre. Veinte archivos de 64
bytes, dos sesiones de verdad sobre loopback:

```
  emisor:   22 pasos, 0 lecturas vencidas
  receptor: 43 pasos, 0 lecturas vencidas
  tiempo:   0.06 s   ->  0,003 s por archivo
```

**Tres milisegundos, no 1,2 segundos. Y cero esperas en los dos lados.**

La ficha decia —y yo lo repeti— que el bucle de sesion serializaba. **No
serializa.** Y `set_nodelay(true)` ya estaba desde ADR-0028, asi que Nagle
tampoco era.

**Donde queda:** los 75/1 salieron de `gui_cli_matrix_test.dart`, que es la GUI
contra el CLI — Dart conduciendo el motor por la frontera C. Esta medida es Rust
contra Rust. La diferencia entre 3 ms y 1 200 ms **esta en el lado Dart o en el
cruce**, no en el motor.

**La siguiente medida, y es una:** cronometrar por iteracion el bucle de
`native_transfer_service.dart` — `stepBlocking`, `progress`, `peerFingerprint` y
el `yield`. Si el motor hace 22 pasos en 60 ms, el coste esta entre esas cuatro
llamadas.

**No busques en `qyro_transfer` ni en `Session::advance`.** Ya esta medido y esta
limpio.

---

## El telefono ya puede mirar

`R7` prometia cuatro canales y habia tres y medio: `qyro beam` dibujaba QR y
nadie los leia. **El puente esta montado, y sin JNI:**

`ScannerChannel.kt` saca **solo el plano Y** con CameraX a **1280x720** →
`dev.qyro/scanner` → Dart → `qyro_buffer_alloc` → `qyro_scanner_look` →
`qyro_eye`. **Cero `unsafe` nuevo, cero excepcion nueva, cero paquetes de
pub.dev.**

Las tres que no se negociaban, hechas: `ResolutionSelector` pidiendo >=1280x720 ·
el de-padding fila a fila, porque `buffer.capacity()` puede ser
`rowStride*(h-1)+w` y leer de mas revienta · y la prueba del manifest en **dos
permisos exactos**, no «>=1».

## Lo que falta medir, y es una sola cifra

**Los fps que sostiene el aparato.** 921 600 bytes por frame a 720p; a 5 fps son
4,6 MB/s por un MethodChannel y otra copia por FFI. `QyroScanner.framesPerSecond`
existe para escribir ese numero. **Si sostiene >=5, hecho para siempre; si no,
entonces el cruce de copia cero por JNI tiene su argumento medido.**

**No hay aparato**, asi que el hueco esta en blanco. Fase 19.

## Lo que encontro una guarda

`promised_capabilities_test` prohibia el icono de escaner **porque no habia
camara**, y su propia razon decia como terminaba: «o se va la promesa, o llega la
camara con su plugin, su permiso, su ADR y su fila en el modelo de amenazas».
Llegaron cuatro y **faltaba la fila**. Ahora esta, y la guarda **no se debilito**:
dejo de prohibir el icono y pasa a exigir las cuatro piezas si aparece.

Y el changelog de dependencias canto que `rqrr` metia el crate `image` entero
—con `moxcms` y `pxfm`— en la biblioteca que Dart carga en el telefono.
`default-features = false` y fuera quince paquetes.

---

## El cruce JNI, cerrado con argumento a la fase 19

**No se escribe sin aparato.** Serian la **segunda** excepcion a
`forbid(unsafe_code)` de este taller —la primera costo una ADR entera— y un slot
equivocado en la vtable de JNI no da error de compilacion: da un salto a una
funcion arbitraria, y el sintoma es un proceso muerto sin traza en el aparato de
otra persona. Ninguna prueba de aqui puede tocar una `JNIEnv`.

Lo que falta es **exactamente un transporte de pixeles**, y su forma ya esta
fijada por `Eye::look(&[u8], usize, usize)`. Informe en
`docs/reports/fase-24-el-ojo.md`; decision en ADR-0048 enmienda 1.

## El hueco, en blanco

> **`R10` §8 T1 manda medir píxeles por módulo en el aparato real antes de
> escribir nada más. NO HAY APARATO.**

Lo que sí se hizo: **reproducir la aritmética** de forma independiente y dejarla
en código con prueba. Salen los dos números de `R10` idénticos — **3,07
px/módulo a 640×480 y 4,60 a 1280×720** para una v27. La decisión de ADR-0048 §3
es pedir ≥1280×720 y quedarse en v27, con la palanca escrita.

**Falta el glue de Kotlin y JNI**, que es la parte que no se puede ejercitar aquí.

## La comprobación 18, que ya cazó dos cosas

`scripts/gate.ps1` **lee `ci.yml`** y corre sus comandos más el objetivo de
Linux. Hoy cazó un `#![cfg(test)]` duplicado **antes** de empujar — el mismo error
de forma que en la fase 15, esta vez detenido por la puerta y no por CI.

---

## Los cinco arreglos

1. **`ptr_arg` en Linux** — `collect_mdns` del stub de no-Windows pedía
   `&mut Vec<FoundPeer>` sin añadir nada. Ahora `&mut [FoundPeer]`.
2. **Los cuatro enlaces de la Release daban 404** — apuntaban a la rama borrada.
   Reapuntados a `blob/main/`, y **comprobados los cuatro con `curl`: 200**.
3. **`ci.yml` decía «No `paths:` filter, deliberately»** diez líneas debajo del
   bloque `paths:` que lo desmiente.
4. **El registro de fichas tenía tres defectos, y el tercero lo encontró una
   guarda cuando yo creía haber terminado**: dos `- Estado:` en QYR-0088 y
   QYR-0089, **QYR-0089 duplicada entera** al principio del archivo, y ninguna
   cabecera. **167 fichas, 1 abierta** — antes decía «155, 0».
5. **`STATUS.md` daba un número de pruebas y son dos.** Windows **753**, medido
   hoy aquí; Linux, lo que diga CI — esta máquina compila y lintea para Linux
   pero **no ejecuta sus binarios**, y el último publicado (750) es anterior a
   los cambios de hoy. Se cita como la medida anterior, no como la actual.

**De regalo:** la prueba del enlace simbólico fallaba en cualquier consola sin
`SeCreateSymbolicLinkPrivilege` (error 1314) — indistinguible de «el resolvedor
deja pasar un enlace». Ahora **dice en voz alta que no se ejecutó**, porque
saltada no es pasada, y en `windows-latest` sigue corriendo de verdad.

## Lo siguiente

**24 → 22 → 17 → 18 → 19 → 20 → 23.** La 24 es la última capacidad que falta:
`qyro beam` dibuja QR desde la fase 15 y **nadie los lee**. `R10` ya decidió la
arquitectura y **lo primero no es código: medir píxeles por módulo en el aparato
real** (`R10` §8 T1 — 640×480 da 3,07 px/módulo, el suelo exacto de `rqrr`).

---

## 0.P0 — EL REPOSITORIO NO COMPILABA EN LINUX. Arreglado.

`qyro_net/src/lib.rs`: `dab9fa3` metió los `pub use` del beacon **entre un
`#[cfg(windows)]` y el elemento que guardaba**. El atributo se pegó al bloque
nuevo, así que fuera de Windows el beacon desapareció y `MdnsDiscovery` se
exportó sin existir. 193 ejecuciones de CI en rojo.

**Decisión congelada en ADR-0043 enmienda 2:** el beacon **es multiplataforma y
no lleva `cfg`** —sólo usa `std`, `socket2` e `if-addrs`, y es la implementación
que la §5 exige para las plataformas sin responder de mDNS—; sólo
`MdnsDiscovery` es de Windows.

**Comprobación 17, que sale de aquí:** ninguna «puerta en verde» sin
`cargo check --workspace --all-targets` contra Linux por código de salida. En
esta máquina: `rustup target add x86_64-unknown-linux-gnu` y
`--target x86_64-unknown-linux-gnu`; `check` no enlaza, no hace falta enlazador
cruzado. **Medido tras el arreglo: sale 0.**

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
2. **200 archivos — QYR-0365, causa localizada y es peor de lo que parecía.**
   Bisecado: 10 y 50 entregan; 100 y 150 «fallan» — **pero los archivos llegan
   todos**. `IDLE_TIMEOUT` es 60 s y el corte cae entre 49,4 s y 80,3 s: es un
   reloj, no un recuento. Y debajo está el defecto real: **~1,2 s por archivo de
   64 bytes**, lineal. **No subas `IDLE_TIMEOUT`** — escondería esto y
   convertiría el fallo en veinte minutos de espera.

   **Culpé al disco y lo medí, y estaba mal.** `sync_all` cuesta **4,9 ms extra
   por archivo** en esta máquina (6,5 frente a 1,6): el 0,4 % de los 1 200 ms.
   Descartado con números.

   **Comprobado: es el reloj de lectura.** Con `READ_TIMEOUT` a 25 ms en vez de
   250, los mismos 50 archivos pasan de **49,4 s a 6,5 s** — 7,5× con el mismo
   código. La constante se revirtió: es el latido de ADR-0028 §4.1, no un botón.

   **El arreglo no es bajarla** —multiplicaría por diez los despertares de un
   hilo ocioso en máquinas viejas— sino que el bucle deje de necesitar varias
   lecturas vencidas por elemento.

   **El mecanismo ya está localizado en el código:**
   `qyro_session/src/session.rs:717` hace **un `read_frame()` por `step()`**, y
   su `Ok(None)` es «venció» — 250 ms gastados. Los dos lados hacen `step` en
   bucle, así que una ida y vuelta por elemento significa que ambos se turnan
   para esperar el reloj.

   **Medido cuál de los dos lados espera: el EMISOR.** Con 20 archivos,
   `emisor=75 receptor=1` lecturas vencidas — ~3,75 por archivo a 250 ms son
   ~0,94 s de los 1,24. El receptor no espera: trabaja y contesta.

   **El arreglo queda acotado a uno:** que el emisor no consuma un
   `READ_TIMEOUT` entero cuando todavía tiene trabajo que poner en el cable.
   `qyro_transfer` ya tiene ventana (`WINDOW_CHUNKS`, `chunks_in_flight()`), o
   sea que el protocolo ya está pensado para varias cosas en vuelo — es el bucle
   de sesión el que lo serializa. Las otras dos opciones quedan descartadas como
   primera medida y está escrito por qué.

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
