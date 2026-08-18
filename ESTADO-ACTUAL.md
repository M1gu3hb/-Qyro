# Estado actual — aqui se corta

**2026-08-19** · **rama unica: `main`** · **se acaba el contexto.**

## Lo siguiente, y por donde

**22 → 17 → 18 → 19 → 20 → 23.** En la 22, **QYR-0365 va primero** y esta sesion
la dejo mas acotada:

- **Descartado que `pump` serialice por elemento.** `is_drained()` es
  `next_to_send >= chunks_total`, asi que un archivo de un trozo se drena en el
  mismo envio y el bucle pasa al siguiente sin salir. Veinte archivos salen en
  **una** llamada, y `WINDOW_CHUNKS` es 16.
- **Queda un sospechoso:** `Session::advance` maneja un frame por llamada, y su
  lectura cuesta un `READ_TIMEOUT` **solo cuando el socket esta vacio** — porque
  `read_frame` vacia primero el decodificador.
- **Un candidato probado y descartado hoy, con su mecanismo:** escribir en el mismo
paso lo que `pump` acaba de producir parecia gratis y no lo es. Una prueba lo
tumbo — escribir a un par que ya cerro provoca un **RST**, y el RST **descarta el
bufer de recepcion con el frame de rechazo dentro**, asi que el emisor termina en
«no llegue» en vez de «me dijeron que no». De mejor esfuerzo tampoco vale: el
daño no es el error de escritura, es el RST. **Cualquier arreglo tiene que no
escribir a un par que pueda haber terminado la conversacion**, y hoy el emisor no
puede saberlo antes de leer.

**La medida que lo cierra, y es una:** por lado, cuantas veces entra `advance`,
  cuantas lecturas vencen, y **que frame estaba en vuelo cuando vencio**. Sin ese
  tercer dato los otros dos no distinguen «el par no ha contestado» de «contesto
  y nadie leyo».

**La fase 24 quedo cerrada** con su informe; lo que le falta —el cruce JNI— esta
cerrado con argumento a la fase 19 en ADR-0048 enmienda 1.

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
