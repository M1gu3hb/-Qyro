# Paridad GUI / CLI

**Especificación:** ADR-0046 §2 y §3 · **Fase:** 21 · **2026-08-18**

> Desde la fase 13 el motor tiene **dos consumidores**. Una capacidad existe
> cuando **los dos la alcanzan**, o cuando aquí está escrito que es de uno solo y
> por qué. **Nada queda en el medio** — porque «existe a medias y nadie lo dijo»
> es exactamente el estado en el que estuvieron las cinco capacidades muertas de
> este proyecto.

**Esta tabla la comprueba `scripts/check_parity.ps1` por código de salida.** Una
celda vacía la pone en rojo. Una prosa que nadie comprueba se desincroniza en la
primera semana y sigue leyéndose como verdad; eso ya pasó aquí, en la fase 11.

## Cómo se lee una celda

- **`ruta:línea`** — la capacidad existe en esa cara y ése es su llamante.
- **`NO -- <argumento>`** — esa cara no la tiene **a propósito**, y el argumento
  está escrito. Es una respuesta completa, no un hueco.
- **Vacía** — incumplimiento. El script falla.

## Lo que este documento llegó a decir, y no era verdad

**Quince de sus citas apuntaban a nada.** `setState(() {`, `};`, `}`,
`labelText: ...`, un comentario. Trece de catorce filas. El documento decía
«**la comprueba `scripts/check_parity.ps1`**» y era cierto a medias: el script
verificaba que el archivo existiera y **tuviera esa línea**, no que la línea
dijera algo. Un guardián que comprueba la existencia y no el contenido no
protege un documento: **lo avala.**

Y el arreglo obvio —resolver cada número al símbolo más cercano hacia arriba— se
probó y **se tiró**: producía «Rechazar con motivo → `_drainReceive`» y «Cancelar
a mitad → `_sendDescriptors`». Eso es precisión falsa, que es peor que el número
viejo porque ya no se nota. Las citas de ahora están puestas a mano contra el
listado de declaraciones de los dos archivos.

**Lo que el guardián sí comprueba ahora:** que la línea citada sea algo
**nombrable**. Que corresponda a la capacidad **no es mecanizable**, y fingir que
lo es sería el mismo error otra vez.

## 2026-08-31 — las citas se movieron otra vez, y ahora hay quien lo note

**Una cita por número de línea envejece en cuanto alguien edita el archivo por
encima**, y eso es lo que pasó: la tanda que arregló QYR-0368 a QYR-0371 añadió
líneas a `flows.rs`, a `native_transfer_service.dart` y a `transfer_screens.dart`,
y **once de las catorce citas quedaron apuntando a un `}`, a un `///` o a un
comentario**. Exactamente el estado que este documento describe arriba, once días
después de arreglarlo.

Las citas están puestas otra vez, a mano, contra el listado de declaraciones. Y
para que la próxima vez no dependa de que alguien mire:
**`qyro_core::repository_contract::the_parity_table_still_points_at_code`** las
resuelve todas dentro de `cargo test --workspace`, o sea dentro de la puerta, en
cada commit. `check_parity.ps1` sigue existiendo y hace lo mismo; la diferencia
es que la guarda de Rust corre donde ya se corre todo, y la de PowerShell hay que
acordarse de llamarla.

**Y una fila cambió de contenido, no sólo de número.** «Canal óptico» decía
`NO -- [...] el lado Android que acumula frames todavía no existe`, y desde la
fase 24B sí existe: `ScanScreen`, `QyroScanner`, `ScannerChannel.kt` y `qyro_eye`.
Lo que no existía era **la puerta** — nadie construía esa pantalla (QYR-0371). Con
la puerta puesta, la celda cita el sitio que la abre.

<!-- PARIDAD-INICIO -->

| Capacidad | GUI | CLI |
|---|---|---|
| Mandar por código tecleado | `apps/qyro/lib/transfer/native_transfer_service.dart:255` | `rust/crates/qyro_cli/src/flows.rs:152` |
| Recibir y enseñar su código | `apps/qyro/lib/transfer/native_transfer_service.dart:393` | `rust/crates/qyro_cli/src/flows.rs:425` |
| Enseñar la propia huella | `apps/qyro/lib/transfer/native_transfer_service.dart:150` | `rust/crates/qyro_cli/src/flows.rs:106` |
| Ver la huella antes de aceptar | `apps/qyro/lib/transfer/transfer_screens.dart:658` | `rust/crates/qyro_cli/src/flows.rs:444` |
| Rechazar con motivo | `apps/qyro/lib/transfer/transfer_screens.dart:737` | `rust/crates/qyro_cli/src/flows.rs:501` |
| Peer con clave cambiada, rechazado por nombre | `apps/qyro/lib/transfer/native_transfer_service.dart:255` | `rust/crates/qyro_cli/src/flows.rs:657` |
| Cancelar a mitad | `apps/qyro/lib/transfer/native_transfer_service.dart:255` | `NO -- una terminal cancela con Ctrl-C, que el sistema operativo ya entrega y el proceso ya honra. Un boton de cancelar en una terminal seria una segunda forma de hacer lo que el teclado hace, y ADR-0042 dice que no se unifican las formas, solo las decisiones` |
| Peers recordados | `apps/qyro/lib/transfer/transfer_screens.dart:205` | `NO -- el CLI no tiene libreta. Recuerda las claves igual (el motor es el mismo) pero no las lista: una lista que no se puede tocar es una pantalla, y una terminal ya tiene qyro whoami para lo unico accionable` |
| Descubrimiento sin router (fase 14) | `apps/qyro/lib/transfer/native_transfer_service.dart:256` | `rust/crates/qyro_cli/src/flows.rs:696` |
| Leer un codigo de emparejamiento por la camara | `apps/qyro/lib/scanner/scan_screen.dart:29` | `NO -- la terminal DIBUJA y no lee (ADR-0044 §6). La maquina que necesita este canal es la que no tiene camara, asi que pedirle que escanee seria pedirle justo lo que no puede. Su mitad es qyro qr` |
| Descubrimiento en la GUI **de escritorio** | `NO -- la frontera C es la MISMA biblioteca que viaja dentro del APK. mdns-sd casi doblo el binario de terminal (666 624 -> 1 295 872 bytes, D9), y sacarlo por un simbolo nuevo meteria ese peso en libqyro_ffi.so por TRES ABIs, en cada telefono, para servir una capacidad que Android no necesita: alli se descubre por NsdManager, donde la eleccion de la persona ES el permiso (ADR-0035 §7). Y la pantalla no lo disimula: sin responder de la plataforma lanza QyroDiscoveryUnavailable y dice que NO PUEDE MIRAR, que no es lo mismo que «no hay nadie». La maquina que tiene esta ventana tiene tambien qyro discover en su terminal` | `rust/crates/qyro_cli/src/flows.rs:696` |
| Canal optico (fase 15) | `apps/qyro/lib/home/home_screen.dart:88` | `rust/crates/qyro_cli/src/flows.rs:794` |
| Canal serie (fase 16) | `NO -- un canal de terminal para una maquina de terminal. La GUI no lo menciona en ninguna pantalla, que es la unica forma honesta de no tenerlo` | `rust/crates/qyro_cli/src/serial.rs:165` |
| Ver QUE se ofrece antes de aceptar | `apps/qyro/lib/transfer/transfer_screens.dart:658` | `rust/crates/qyro_cli/src/flows.rs:473` |
| Comprobar la huella que promete el codigo | `apps/qyro/lib/transfer/transfer_screens.dart:516` | `rust/crates/qyro_cli/src/flows.rs:250` |
| Consejero de canal (fase 21) | `apps/qyro/lib/ffi/qyro_identity_api.dart:141` | `rust/crates/qyro_cli/src/flows.rs:1054` |

<!-- PARIDAD-FIN -->

## La fila que se anadio porque el codigo la desmentia

**Comprobar la huella que promete el codigo** (QYR-0392) no estaba en esta tabla,
y por eso nadie vio que la GUI no la tenia. La terminal la comprueba desde
QYR-0381; la pantalla sacaba la direccion del codigo y tiraba la huella, con un
parametro `expectedFingerprint` que se aceptaba y no se usaba. Un parametro
ignorado es peor que uno ausente: todo el que lo lee da por hecho que la
comprobacion ocurre.

La fila se anade **con las dos celdas llenas**, porque el arreglo entro en el
mismo commit. Lo que la fila compra es que la proxima vez que una de las dos
mitades se quede atras, esto lo diga.

## Dos filas dijeron «todavía» y las dos se cerraron llenándolas

**Ver qué se ofrece antes de aceptar** (QYR-0364) apareció el 2026-08-18
buscándole un llamante al saneado de nombres de ADR-0047 §6: la GUI enseñaba los
archivos y la terminal preguntaba «¿aceptas?» con una huella y nada más. No
estaba en esta tabla porque **nadie había mirado esa fila**, y se añadió
precisamente para que dejara de depender de que alguien mire. Cerrada el mismo
día: `Session::offered_files` existe, el receptor del CLI los dibuja, y cada
nombre pasa por `safe_terminal_name`.

La otra fue **el consejero de canal**, que el CLI alcanzaba y la GUI no. Estaba escrita como «TODAVÍA» y no como «NO», a propósito — las otras
cinco negativas son decisiones de producto, con un argumento por el que esa cara
no lo tiene y no lo va a tener; aquélla era trabajo pendiente, y llamarla
decisión habría sido convertir un pendiente en una frase que se lee como cerrada.

**Se cerró llenándola**, que es la otra salida: `qyro_advice` cruza la frontera C
—la superficie pasa de 24 a 25 símbolos, con su enmienda en ADR-0032— y
`qyro_advice_test.dart` la ejerce contra la biblioteca de verdad.

Las **5** que quedan en `NO` son decisiones, cada una con su argumento en la
celda.

**Y ese número lo cuenta una guarda, porque ya se había desincronizado.** Decía
«cinco» cuando en la tabla había **cuatro**: la fila del consejero de canal se
cerró llenándola y nadie bajó el número. Es el defecto que este documento
describe en su propia cabecera, cometido dentro de él.
`qyro_core::repository_contract::the_parity_table_agrees_with_its_own_count`
compara la cifra de esta frase con las celdas `NO --` de la tabla, dentro de
`cargo test --workspace`.
