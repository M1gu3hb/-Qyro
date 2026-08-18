# Fase 12 — Cerrar la cadena, y las tres cosas que estaban escritas y no se podían llamar

**Base:** `d575ac8`. **Rama:** `claude/qyro-cerrar-cadena-12`.

---

## 1. Objetivo

> **Que un archivo viaje entre dos aparatos.** Hasta que eso ocurra, Qyro es un
> motor excelente dentro de una aplicación que no funciona.

Y publicar la Release, autorizada por el propietario, **después** de corregir lo
que la documentación afirmaba de más.

---

## 2. Qué se encontró — el mismo defecto, tres veces

La fase esperaba uno. Aparecieron tres, y son **la misma forma exacta**:
capacidad escrita, probada en Rust, y **sin un llamante que la alcance desde el
producto**.

| Ficha | Qué prometía | Qué hacía |
|---|---|---|
| **QYR-0322** P0 | El receptor enseña su código | `ownPairingString()` devolvía `null` **siempre**. El campo se leía y nunca se escribía |
| **QYR-0356** P0 | Pulsar Recibir y esperar | **Congelaba la aplicación entera** — la sesión corría en el isolate que dibuja |
| **QYR-0357** P0 | «Entregado» | Dejaba un `.qyro-part`. `Session::finish()` **no tenía llamante** |

Las tres juntas: **dos aparatos con Qyro instalado no podían completar una sola
transferencia**, y la aplicación decía que sí.

### Por qué sobrevivieron, y es una sola razón

**Ninguna prueba había puesto nunca un receptor de Dart frente a un emisor
real.** `qyro_session_transfer_test.dart` prueba Dart-como-**emisor** contra
`qyro_net_smoke serve`, que es un receptor de Rust y sí llama a `finish`. La
mitad receptora de la frontera nunca se ejercitó de extremo a extremo.

Y las pantallas se probaban contra un `FakeService` cuyo `ownPairingString()`
devuelve el literal que el propio test escribe al lado. **Una prueba así no puede
distinguir un valor medido de una constante.**

---

## 3. Cómo se cerró

**ADR-0041 disuelve QYR-0322 en vez de contestarla.** La ficha pedía partir
`bind` de `accept`. No hizo falta: **si el puerto se conoce de antemano no hay
nada que preguntarle al socket**, así que la cadena se compone antes de ligar.

`qyroDefaultPort = 49517`, del rango Dynamic/Private que IANA nunca asigna. Fijo
y no efímero, y la razón que decide **no es la comodidad**: el firewall de
Windows concede el permiso de inbound **una vez por programa y puerto**, y una
red sin gateway cae en perfil Public (R8 §9). Fijo = se autoriza una vez.
Efímero = el diálogo vuelve en cada sesión.

Si el puerto está ocupado **se dice, no se mueve**: un puerto que se reubica solo
pierde en silencio las dos propiedades por las que se eligió fijo.

**Todas las IP candidatas con el nombre de su interfaz**, no una adivinada. Se
excluye loopback y link-local —el zone-id es local al nodo y no viaja (RFC
4007)—. **No** se excluyen los adaptadores virtuales: filtrarlos pide una lista
de nombres por sistema operativo, la clase de heurística que este proyecto ya ha
pagado dos veces.

Enumerar es `NetworkInterface.list()` de `dart:io`, así que **esta parte añadió
cero símbolos**. El único símbolo nuevo es `qyro_session_finish` (QYR-0357).

---

## 4. Comprobación 14 — el llamante de producción

| Capacidad | Símbolo | Llamante de producción | Archivo:línea |
|---|---|---|---|
| Componer el código de este aparato | — (`NetworkInterface.list`) | `NativeTransferService.listenCandidates` | `native_transfer_service.dart:186` |
| Enseñarlo antes de ligar | — | `_ReceiveScreenState._loadCandidates` | `transfer_screens.dart:412` |
| Registrar dónde se escucha | — | `NativeTransferService.receive` | `native_transfer_service.dart:376` |
| Parsear el código del otro | `qyro_pairing_parse` | `NativeTransferService.addressOfPairingString` | `native_transfer_service.dart:155` |
| Huella de este aparato | `qyro_identity_fingerprint` | `NativeTransferService.ownFingerprint` | `native_transfer_service.dart:101` |
| **Materializar lo recibido** | **`qyro_session_finish`** | **worker de `receive`** | **`native_transfer_service.dart:487`** |
| Ligar en un puerto conocido | `qyro_session_open_receiver_blocking` | `_ReceiveScreenState._listen` | `transfer_screens.dart:470` |

**Filas con «ninguno», dichas y no escondidas:**

| Capacidad | Símbolo | Llamante |
|---|---|---|
| Descubrimiento automático | **ninguno en la superficie C** | **ninguno** — `DiscoveryChannel.kt` registrado, ningún Dart abre `dev.qyro/discovery` |
| Dirección local de una sesión viva | `qyro_session_local_address` | **ninguno de producción.** Sigue sin usarse: la cadena se compone del puerto fijo, no del socket |
| Historial | — | **ninguno.** `qyro_fs::history` graba y ningún símbolo lee |

El descubrimiento **se declara fuera de la v1.x** en los cuatro sitios que lo
anunciaban. La fase 14 lo conecta. `qyro_session_local_address` se queda sin
llamante **a propósito y dicho**: ADR-0041 lo hace innecesario hoy, y la fase 14
lo necesitará de verdad.

---

## 5. Comprobación 15 — la cadena entera, desde el gesto

**Recibir, desde el dedo hasta el socket ligado:**

1. Una persona toca **Recibir** → `TransferHome` monta `ReceiveScreen`.
2. `initState` → `_loadCandidates()` → `service.listenCandidates()`.
3. Enumera interfaces con `NetworkInterface.list(includeLoopback: false,
   includeLinkLocal: false, type: IPv4)` y pide la huella a
   `qyro_identity_fingerprint`.
4. Compone `QYRO1|<ip>:49517|<huella>` **por cada interfaz** y la pantalla las
   dibuja con su nombre. **Nada se ha ligado todavía.**
5. La persona toca **Recibir ahora** → `_listen()` → `service.receive(bind:
   '0.0.0.0:49517')`.
6. `receive` escribe `_listeningAddress` y **después** lanza el worker.
7. El worker llama a `qyro_session_open_receiver_blocking`, que liga y espera.

**Enviar, desde el código tecleado hasta el byte en disco:**

1. La persona teclea el código en la pantalla de peers y pulsa **Usar este
   código** → `_resolve()` → `addressOfPairingString` → `qyro_pairing_parse`.
2. Elige archivos → `pickFiles()` → SAF en Android, `IFileOpenDialog` en Windows.
3. **Enviar** → `send()` → `Isolate.run` → `qyro_session_open_sender_blocking`.
4. Handshake autenticado; el receptor despierta de `accept`.
5. El worker del receptor da un paso, lee la huella y el veredicto, y **emite la
   oferta al isolate que tiene una persona delante**.
6. La persona acepta → el booleano vuelve por el puerto → el worker sigue.
7. `step_blocking` en bucle; los bytes van a `<nombre>.qyro-part`.
8. Al terminar, **`qyro_session_finish`** verifica el digest y renombra a
   `<nombre>`. **Sin este paso el archivo no llega, y era exactamente lo que
   faltaba.**

Sin saltos. Cada eslabón tiene su archivo y su línea en §4.

---

## 6. Resultado — **CUMPLIDO**

```
QyroConnecting -> QyroAwaitingDecision -> QyroMoving x7 -> QyroDelivered
destino: in\payload.bin          (antes: in\payload.bin.qyro-part)
```

Dos procesos de verdad, `NativeTransferService` de receptor, 256 KiB comparados
**byte a byte** —más fuerte que un digest, que sólo puede no darse cuenta— y el
segundo proceso recibiendo **sólo** el código que el primero publicó.

Con control de falsabilidad: mandar donde nadie escucha falla por nombre y con un
final distinto, o la prueba no distinguiría «funcionó» de «no llegó a intentarlo».

---

## 7. La puerta — 2026-08-17

| # | Comprobación | Exit |
|---|---|---|
| 1 | `cargo test --workspace` | 0 |
| 2 | `cargo fmt --all --check` | 0 |
| 3 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| 4–8 | `flutter analyze`, `flutter test`, `dart format` | 0 — **106 pasadas, 0 saltadas** |
| 9–12 | `check_docs_consistency` en Bash **y** PowerShell | 0 y 0 |
| 13 | `cargo clippy --target aarch64-linux-android` | 0 |
| **14** | **El llamante de producción** | §4, con sus tres filas de «ninguno» |
| **15** | **La cadena desde el gesto** | §5, sin saltos |

---

## 8. Lo que NO debe leerse como progreso

**Sigue sin ejecutarse en hardware físico.** Los veinte escenarios siguen en
blanco. Esta fase movió un archivo entre dos procesos del mismo ordenador, que es
lo que faltaba y no es lo mismo que dos aparatos en una Wi-Fi.

**Tres P0 en una fase que esperaba uno.** El producto llevaba una etiqueta `v1.0`
mientras no podía completar una transferencia. La lección no es que se
arreglaran: es que **cuatro fases se cerraron en verde por encima**.

**La comprobación 14 encontró la tercera en diez minutos.** `KeystoreWrapper`,
`qyro_session_local_address`, `Session::finish`. Preguntar «¿quién llama a esto?»
es más barato que cualquier prueba, y las tres veces habría bastado.

**La regla que sale de aquí, y vale más que los tres arreglos:** cuando una
operación existe en los dos lados —emitir y recibir, sellar y abrir, envolver y
desenvolver— **las dos mitades necesitan su prueba de extremo a extremo, con el
producto en cada papel.**

---

## 9. Ledger y Release

- `BUGS_PENDING.md`: **157 fichas, 0 abiertas.** Tres nuevas — QYR-0355, 0356,
  0357 — y QYR-0322 cerrada **respondiendo a la pregunta que hacía**, con la
  comprobación de su condición escrita en el cierre.
- IDs siguientes desde **QYR-0358 en adelante**.
- **Release publicada** en `v1.0.0`, marcada **pre-release**, con el APK firmado,
  el ZIP de Windows y sus SHA-256. La advertencia va **arriba del todo**: no
  aprobado, nada ejecutado en hardware, y la identidad de Android sin Keystore.
  Cero archivos sensibles: `git ls-files` no encuentra ni un `.jks`, ni un
  `key.properties`, ni una clave privada.
- Siguiente: **fase 13**, el binario de terminal.
