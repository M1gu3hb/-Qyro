# Fase 14 — que se encuentren sin router

**Rama** `claude/qyro-cerrar-cadena-12` · **Commit del informe** `07278ff` ·
**2026-08-18**

**Puerta ejecutada en `07278ff`, el commit que este informe nombra**
(comprobación 16). Se corrió *después* del último commit de código, no antes.

---

## 1. Qué prometía la fase y qué hay

ADR-0043, congelada antes del código, más su **enmienda 1** en commit propio
(`b89a89a`) antes de tocar `qyro_net`.

| Lo prometido | Estado | Dónde |
|---|---|---|
| Cuenta atrás de APIPA, contada en voz alta | **HECHO** | `qyro_session/src/link.rs`, `f81c15a` |
| Multicast **por interfaz** con `socket2` | **HECHO** | `qyro_net/src/beacon.rs`, `dab9fa3` |
| Multicast y broadcast **simultáneos**, no en cadena | **HECHO** | `announcement_targets()` |
| Lado Dart de `dev.qyro/discovery` | **HECHO** | `apps/qyro/lib/discovery/`, `07278ff` |
| Desduplicar por huella, nunca por IP | **HECHO** | en los tres sitios: Rust, Dart, y la pantalla |
| Prueba en CI con dos espacios de nombres `veth` | **NO HECHO** | §5, cerrado con argumento |

---

## 2. Comprobación 14 — llamante de producción, con archivo y línea

**Por consumidor**, que es como esta comprobación encuentra lo que encuentra.

| Capacidad | Llamante de producción | Consumidor |
|---|---|---|
| `qyro_net::BeaconSwarm::bind_all` | `qyro_session/src/discovery.rs:139` | motor (los dos) |
| `BeaconSwarm::announce_and_collect` | `qyro_session/src/discovery.rs:151` | motor (los dos) |
| `qyro_net::MdnsDiscovery::start` | `qyro_session/src/discovery.rs:111` | motor (los dos) |
| `qyro_session::browse` | `qyro_cli/src/flows.rs:419` | **CLI** |
| `qyro_session::wait_for_link` | `qyro_cli/src/flows.rs:384` | **CLI** |
| `qyro_net::QYRO_PORT` | `qyro_cli/src/flows.rs:34`, `qyro_session/src/discovery.rs:156` | ambos |
| `dev.qyro/discovery` (canal) | `apps/qyro/lib/transfer/transfer_screens.dart:116, 145, 150` | **GUI** |

**Un hueco declarado, no olvidado:** `qyro_session::browse` **no cruza la
frontera C**, así que la GUI de escritorio no tiene descubrimiento. No devuelve
una lista vacía: `QyroNoDiscovery` lo dice con una frase. En Windows el
consumidor con descubrimiento es el CLI.

### 2.1 — La comprobación 14 llegando por el enlazador

Lo más útil que salió de esta fase no lo encontró una tabla, lo encontró una
medida. Con `beacon.rs` escrito, probado y **sin llamante**, el binario pesaba
**exactamente lo mismo** que sin él:

| Estado | `qyro.exe` (x86_64-pc-windows-msvc) |
|---|---|
| Antes de `socket2` | 1 298 432 B |
| `socket2` dentro, nada llama a `Beacon` | **1 298 432 B** |
| `qyro_session::browse` llamando al enjambre | 1 306 624 B |

Cero bytes de diferencia porque el enlazador descartó el módulo entero. **Una
capacidad sin llamante no se envía: se compila.** Es el mismo defecto que este
proyecto lleva encontrando cinco veces, y esta vez lo dijo una cifra.

---

## 3. Comprobación 15 — del gesto al byte

**Escena:** dos máquinas unidas por un cable, sin router y sin DHCP.

1. La persona escribe `qyro find` → `qyro_cli/src/main.rs` despacha a
   `flows::find`.
2. `flows.rs:384` llama a `wait_for_link`, que **cuenta en voz alta** cada
   segundo: *«waiting for a network address … 12s — this is normal on a direct
   cable»*. `R8` §8 midió que APIPA tarda decenas de segundos porque el cliente
   DHCP tiene que fracasar primero.
3. A los 60 s sin dirección, `LinkState::StillNothing` — **consejo, no error**:
   prueba un cable cruzado, y si comparten red el código tecleado funciona igual.
   Auto-MDI-X vive en la cláusula 40.4.4 de IEEE 802.3, que es la de
   **1000BASE-T**: una NIC de 10/100 puede no tenerlo, y es justo la NIC de la
   máquina para la que se hizo esto.
4. Con dirección (`169.254.x.x` cuenta: es lo que produce un cable directo),
   `flows.rs:419` llama a `qyro_session::browse`.
5. `discovery.rs:111` pregunta al responder de la plataforma y `discovery.rs:139`
   levanta **un socket por interfaz**. `set_multicast_if_v4` nombra la interfaz,
   porque `std` no puede y el sistema elige mal con Wi-Fi + Ethernet + VPN +
   Hyper-V.
6. Cada interfaz anuncia **su propia dirección**: `QYRO1|169.254.7.3:49517|<huella>`
   por el cable, `QYRO1|192.168.1.9:49517|<huella>` por el Wi-Fi. Anunciar el
   mismo texto en todas sería darle al otro lado una dirección que no rutea.
7. Multicast a `224.0.0.251` **y** broadcast a `255.255.255.255`, los dos cada
   ronda, los dos en el puerto 5353 — una sola concesión del cortafuegos.
8. Lo que llega lo parsea `PairingEndpoint::parse`, **el mismo parser que lee un
   código tecleado**. Se descarta lo propio por huella, no por dirección.
9. La pantalla —o el CLI— ofrece el código. **No marca.** La comprobación de
   confianza que sigue es la misma que pasa un código escrito a mano: un aparato
   que se anunció no ha demostrado nada.
10. A partir de ahí, la cadena de la fase 12 sin cambios: handshake, manifiesto,
    `DataChunk` sellados, y el archivo materializado por `Session::finish`.

**Verificado ejecutando** en esta máquina: `qyro find` imprime
`address: 192.168.100.136` y busca 3 s.
**No verificado:** que dos máquinas se vean por un cable. Eso es la fase 19 y el
hueco sigue en blanco.

---

## 4. Lo que encontró esta fase y no buscaba

### 4.1 — `mdns-sd` casi dobla el binario (D9)

Midiendo el coste de `socket2` salió una cifra que nadie había mirado:
**666 624 → 1 295 872 bytes** entre `458d4bd` y `3ecebed`, el commit que trajo
`qyro find`. **+614 KB por una dependencia**, en un producto cuyo argumento es un
binario portátil. Diez veces los 63 KB que este taller discutió a fondo para
conservar el desenrollado de pila. El beacon propio hace lo mismo por **8 KB**.

Y la ADR-0043 §7 descartaba `mdns-sd` en todos los targets citando un binario
«que apunta a 750–950 KB» — **un presupuesto que el binario ya no cumple**. La
enmienda 1 lo corrige con la medida.

**No se toca hoy.** El descubrimiento funciona y quitarlo sin sustituto probado
es cambiar el producto por una cifra. Lo decide la fase 19 con red de verdad.

### 4.2 — El puerto de ADR-0041 estaba escrito dos veces (D10)

`qyro_cli::DEFAULT_PORT = 49_517` y `qyroDefaultPort = 49517` en Dart, y **nada
en el motor**. El comentario del lado Rust decía, literalmente, *«**No
re-derivado**: dos copias de un número de puerto son dos puertos el día que una
cambie»* — siendo la segunda copia. El beacon iba a escribir la tercera.

Cerrado en el sitio: `qyro_net::QYRO_PORT` es el original, el CLI lo reenvía, y
`the_two_consumers_agree_on_the_port` lee el `.dart` y falla si se separan.
**Vista fallar a propósito** con 49518 antes de darla por buena.

### 4.3 — Dos guardas me contradijeron y las dos tenían razón

- `every_production_file_is_listed` paró `beacon.rs` igual que había parado
  `link.rs` una hora antes. Registrar, no rodear.
- `qyro_session_re_exports_nothing_it_does_not_own` rechazó
  `pub use qyro_net::QYRO_PORT`. **Tenía razón**: todo lo que la fachada
  republica se vuelve nombrable desde `qyro_ffi`, y una excepción juzgada
  inofensiva de una en una es cómo llega la primera peligrosa. Un `const` que
  reenvía un `u16` no expone ningún tipo ajeno y sigue teniendo una definición.
- Y `clippy` tuvo razón sobre `assert!(CONST < CONST)`: se optimiza y no prueba
  nada. Ahora es `const _: () = assert!(...)`, que no se puede optimizar — no
  compila.

### 4.4 — Un defecto que encontró la prueba antes que ningún llamante

`QyroNoDiscovery.advertise` y `.browse` estaban escritos `=> throw ...` y
lanzaban **síncronamente** desde métodos declarados `Future`. Un caller con
`.catchError` —la forma que esta interfaz invita a usar— no lo habría visto
nunca, y la excepción habría salido como error no capturado en otro sitio.

---

## 5. La prueba de CI con dos espacios `veth` — cerrada con argumento

**No se hace, y esto es la decisión, no un aplazamiento.**

La prueba planteada era: dos espacios de nombres de red unidos por un `veth`, un
Qyro en cada uno, y comprobar que se encuentran. Tres razones para cerrarla:

1. **Mide lo que ya está medido y no lo que falta.** Un `veth` es un enlace
   perfecto: sin pérdida, sin filtrado de multicast, con las dos interfaces
   bajo el mismo kernel. Lo que rompe el descubrimiento en la vida real es
   exactamente lo que un `veth` no tiene — el switch que descarta multicast, el
   aislamiento de clientes del router, la pila Wi-Fi que filtra por debajo del
   socket. Una prueba verde ahí **no autoriza a decir que funciona en una red**.
2. **El runner de CI de este proyecto es Windows** para los targets que importan,
   y los espacios de nombres de red son de Linux. La prueba viviría en el único
   target donde `mdns-sd` **no** se compila (`cfg(windows)`), así que ejercitaría
   sólo la mitad del código.
3. **Lo que sí prueba, ya está probado sin red**: que lo compuesto es lo que el
   parser acepta (`what_a_beacon_announces_is_what_the_other_side_parses`), que
   los dos objetivos comparten puerto, que la desduplicación no colapsa dos
   aparatos, y que el payload cabe en el buffer.

**Lo que la sustituye está con fecha:** la fase 19, con dos máquinas y un cable
de verdad. Escribir una prueba de CI que dé verde sobre un enlace perfecto y
llamar a eso «descubrimiento verificado» sería exactamente la clase de evidencia
que este proyecto tiene prohibido inventar.

---

## 5.bis. CORRECCIÓN (2026-08-18) — la puerta de este informe **no** era verde

Este informe dijo «puerta en verde» y **sólo se había compilado en Windows**.

`dab9fa3` —el commit del beacon de esta misma fase— insertó los `pub use` **entre
un `#[cfg(windows)]` y el elemento que guardaba**, así que el atributo se pegó al
bloque nuevo. Fuera de Windows el beacon desapareció y `MdnsDiscovery` se exportó
sin existir, y `qyro_session::discovery` dejó de compilar. **El repositorio no
compilaba en Linux desde esta fase**, y se llevó por delante 193 ejecuciones de
CI.

**Lo que decían las tablas de abajo era cierto en Windows y falso como
afirmación.** Se corrige aquí en vez de reescribirse arriba: el informe es
histórico y lo que se escribió se escribió.

Arreglado moviendo el atributo a su sitio, con la decisión congelada en
**ADR-0043 enmienda 2** y una comprobación nueva que lo impide en adelante:

> **Comprobación 17** — ninguna afirmación de «puerta en verde» vale sin
> `cargo check --workspace --all-targets` contra Linux, por código de salida.

Verificado tras el arreglo: `cargo check --workspace --all-targets --target
x86_64-unknown-linux-gnu` **sale 0**.

---

## 6. La puerta, en `07278ff`

| Comprobación | Resultado |
|---|---|
| `cargo test --workspace` | **679 pruebas, 0 fallos** (55 binarios) |
| `flutter test` | **105 pruebas, 0 fallos** |
| `cargo clippy --workspace --all-targets -D warnings` | 0 errores |
| `cargo audit --deny warnings` | 92 dependencias, 0 avisos |
| `dart analyze` | sin incidencias |
| `check_docs_consistency` | OK |
| `check_repo_portability` | OK |
| `check_harness_isolation` | OK |
| `check_crypto_platform_evidence` | OK |
| `verify_static` | sin runtime de C; **nota** de Windows 8 mínimo (fase 17, ya fichada) |

Rust pasó de **664 a 679** pruebas; Dart de 92 a **105**.

---

## 7. Lo que esta fase NO promete

- Que dos máquinas se encuentren por un cable. **No hay hardware y no se
  inventa.**
- Que `NsdManager` funcione. Las pruebas Dart usan un `MethodChannel` falso:
  prueban el lado Dart, no Android.
- Descubrimiento en la GUI de escritorio. No hay símbolo en la frontera C y se
  dice con una frase, no con una lista vacía.
- Que el beacon atraviese un switch que descarta multicast. Por eso el broadcast
  dispara en la misma ronda, y por eso el código tecleado sigue siendo el camino
  que funciona siempre.
