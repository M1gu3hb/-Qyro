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
| Mandar por código tecleado | `apps/qyro/lib/transfer/native_transfer_service.dart:241` | `rust/crates/qyro_cli/src/flows.rs:152` |
| Recibir y enseñar su código | `apps/qyro/lib/transfer/native_transfer_service.dart:379` | `rust/crates/qyro_cli/src/flows.rs:326` |
| Enseñar la propia huella | `apps/qyro/lib/transfer/native_transfer_service.dart:124` | `rust/crates/qyro_cli/src/flows.rs:106` |
| Ver la huella antes de aceptar | `apps/qyro/lib/transfer/transfer_screens.dart:613` | `rust/crates/qyro_cli/src/flows.rs:345` |
| Rechazar con motivo | `apps/qyro/lib/transfer/transfer_screens.dart:640` | `rust/crates/qyro_cli/src/flows.rs:384` |
| Peer con clave cambiada, rechazado por nombre | `apps/qyro/lib/transfer/native_transfer_service.dart:241` | `rust/crates/qyro_cli/src/flows.rs:505` |
| Cancelar a mitad | `apps/qyro/lib/transfer/native_transfer_service.dart:241` | `NO -- una terminal cancela con Ctrl-C, que el sistema operativo ya entrega y el proceso ya honra. Un boton de cancelar en una terminal seria una segunda forma de hacer lo que el teclado hace, y ADR-0042 dice que no se unifican las formas, solo las decisiones` |
| Peers recordados | `apps/qyro/lib/transfer/transfer_screens.dart:231` | `NO -- el CLI no tiene libreta. Recuerda las claves igual (el motor es el mismo) pero no las lista: una lista que no se puede tocar es una pantalla, y una terminal ya tiene qyro whoami para lo unico accionable` |
| Descubrimiento sin router (fase 14) | `apps/qyro/lib/transfer/native_transfer_service.dart:205` | `rust/crates/qyro_cli/src/flows.rs:544` |
| Canal optico (fase 15) | `apps/qyro/lib/home/home_screen.dart:70` | `rust/crates/qyro_cli/src/flows.rs:642` |
| Canal serie (fase 16) | `NO -- un canal de terminal para una maquina de terminal. La GUI no lo menciona en ninguna pantalla, que es la unica forma honesta de no tenerlo` | `rust/crates/qyro_cli/src/serial.rs:165` |
| Ver QUE se ofrece antes de aceptar | `apps/qyro/lib/transfer/transfer_screens.dart:613` | `rust/crates/qyro_cli/src/flows.rs:358` |
| Consejero de canal (fase 21) | `apps/qyro/lib/ffi/qyro_identity_api.dart:141` | `rust/crates/qyro_cli/src/flows.rs:902` |

<!-- PARIDAD-FIN -->

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

Las cinco que quedan en `NO` son decisiones, cada una con su argumento en la
celda.
