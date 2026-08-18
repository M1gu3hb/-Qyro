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
| Consejero de canal (fase 21) | `NO -- TODAVIA no cruza la frontera C. El motor lo tiene y el CLI lo llama; darselo a la GUI es un simbolo nuevo que escribe la frase en un buffer prestado, y esta abierto en ESTADO-ACTUAL.md con esa descripcion. Se anota como incompleto en vez de como decision, porque la GUI SI deberia tenerlo` | `rust/crates/qyro_cli/src/flows.rs:552` |

<!-- PARIDAD-FIN -->

## La única fila que dice «todavía»

**El consejero de canal**, y está escrito así a propósito. Las otras cinco
negativas son decisiones: hay un argumento de producto por el que esa cara no lo
tiene y no lo va a tener. Ésta no — la GUI **debería** tenerlo, y no lo tiene
todavía.

Llamarlo decisión sería exactamente lo que ADR-0046 §2 prohíbe: convertir un
trabajo pendiente en una frase que se lee como cerrada. Queda como incumplimiento
con nombre, y `ESTADO-ACTUAL.md` dice dónde se corta.
