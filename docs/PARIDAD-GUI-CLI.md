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

<!-- PARIDAD-INICIO -->

| Capacidad | GUI | CLI |
|---|---|---|
| Mandar por código tecleado | `apps/qyro/lib/transfer/native_transfer_service.dart:227` | `rust/crates/qyro_cli/src/flows.rs:143` |
| Recibir y enseñar su código | `apps/qyro/lib/transfer/native_transfer_service.dart:353` | `rust/crates/qyro_cli/src/flows.rs:224` |
| Enseñar la propia huella | `apps/qyro/lib/transfer/native_transfer_service.dart:159` | `rust/crates/qyro_cli/src/flows.rs:102` |
| Ver la huella antes de aceptar | `apps/qyro/lib/transfer/transfer_screens.dart:201` | `rust/crates/qyro_cli/src/flows.rs:182` |
| Rechazar con motivo | `apps/qyro/lib/transfer/native_transfer_service.dart:550` | `rust/crates/qyro_cli/src/flows.rs:188` |
| Peer con clave cambiada, rechazado por nombre | `apps/qyro/lib/transfer/transfer_screens.dart:226` | `rust/crates/qyro_cli/src/flows.rs:188` |
| Cancelar a mitad | `apps/qyro/lib/transfer/native_transfer_service.dart:342` | `NO -- una terminal cancela con Ctrl-C, que el sistema operativo ya entrega y el proceso ya honra. Un boton de cancelar en una terminal seria una segunda forma de hacer lo que el teclado hace, y ADR-0042 dice que no se unifican las formas, solo las decisiones` |
| Peers recordados | `apps/qyro/lib/transfer/transfer_screens.dart:205` | `NO -- el CLI no tiene libreta. Recuerda las claves igual (el motor es el mismo) pero no las lista: una lista que no se puede tocar es una pantalla, y una terminal ya tiene qyro whoami para lo unico accionable` |
| Descubrimiento sin router (fase 14) | `apps/qyro/lib/transfer/transfer_screens.dart:116` | `rust/crates/qyro_cli/src/flows.rs:419` |
| Canal optico (fase 15) | `NO -- ADR-0044 §6: el CLI dibuja y el telefono lee. La GUI de escritorio no dibuja QR porque la maquina que los necesita es la que no tiene GUI, y el lado Android que acumula frames todavia no existe -- esta anotado en STATUS.md como hueco en blanco, no como capacidad` | `rust/crates/qyro_cli/src/flows.rs:530` |
| Canal serie (fase 16) | `NO -- un canal de terminal para una maquina de terminal. La GUI no lo menciona en ninguna pantalla, que es la unica forma honesta de no tenerlo` | `rust/crates/qyro_cli/src/serial.rs:165` |
| Ver QUE se ofrece antes de aceptar | `apps/qyro/lib/transfer/transfer_screens.dart:201` | `NO -- TODAVIA. El receptor del CLI enseña la huella y pregunta accept from this device, sin decir que archivos ni cuantos ni cuanto pesan. ADR-0036 §1 dice que nada se acepta solo, y una pregunta sin objeto es un tramite, no una decision. Bloqueado por QYR-0364: qyro_session no expone los nombres del manifiesto antes de aceptar` |
| Consejero de canal (fase 21) | `apps/qyro/lib/ffi/qyro_identity_api.dart:141` | `rust/crates/qyro_cli/src/flows.rs:552` |

<!-- PARIDAD-FIN -->

## Una fila dice «todavía», y es nueva

**Ver qué se ofrece antes de aceptar** — la GUI lo enseña, la terminal no.
Apareció el 2026-08-18 buscándole llamante al saneado de nombres de ADR-0047 §6,
y es QYR-0364. No estaba antes porque **nadie había mirado esa fila**: la tabla
la añade ahora precisamente para que deje de depender de que alguien mire.

Hubo otra durante unas horas: **el consejero de canal**, que el CLI alcanzaba y la
GUI no. Estaba escrita como «TODAVÍA» y no como «NO», a propósito — las otras
cinco negativas son decisiones de producto, con un argumento por el que esa cara
no lo tiene y no lo va a tener; aquélla era trabajo pendiente, y llamarla
decisión habría sido convertir un pendiente en una frase que se lee como cerrada.

**Se cerró llenándola**, que es la otra salida: `qyro_advice` cruza la frontera C
—la superficie pasa de 24 a 25 símbolos, con su enmienda en ADR-0032— y
`qyro_advice_test.dart` la ejerce contra la biblioteca de verdad.

Las cinco que quedan en `NO` son decisiones, cada una con su argumento en la
celda.
