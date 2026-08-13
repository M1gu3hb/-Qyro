# FASE 04 — El descubrimiento y el emparejamiento en red

## 1. Objetivo

**Que dos aparatos en la misma red se encuentren solos, y que una persona pueda
confirmar que el que ve es el que quiere.**

## 2. Por qué esta fase va aquí

**Depende de:** fases 01, 02 y 03.

Es la última pieza técnica antes de la UI. Y contiene la única parte del proyecto
que **no puede hacerse en Rust**: iOS y Android están cerrando activamente el
acceso a la red local desde sockets crudos, y hay que pasar por sus APIs.

## 3. La decisión, ya investigada, y es la más importante del plan

**El descubrimiento NO se hace en Rust en móvil. Se hace en Kotlin y en Swift,
detrás de un trait.**

### 3.1 — iOS: bloqueante duro

El entitlement `com.apple.developer.networking.multicast` dice, textualmente:

> «Your app must have this entitlement to send or receive IP multicast or
> broadcast on iOS. … **This entitlement requires permission from Apple before you
> can use it in your app.**»

Es un formulario tras autenticación con Apple ID: **una revisión humana**. Y
TN3179 confirma que el gate está *«deep in the networking stack, and thus they
apply to all networking APIs. This includes Network framework, **BSD Sockets**,
URLSession»* — **un socket de Rust no lo esquiva**. Broadcast UDP tampoco.

**El camino sin entitlement:** `NWBrowser`/`NWListener` de Network.framework, con
`NSBonjourServices = ["_qyro._tcp"]` y `NSLocalNetworkUsageDescription` en el
`Info.plist`. El multicast real lo hace `mDNSResponder`, el daemon del sistema.
**Cero entitlements especiales, cero revisión manual.**

### 3.2 — Android: bloqueante con fecha

«Local Network Protections» dice:

> «Apps will be affected if they access the user's local network using: **Direct
> or library use of raw sockets on local network addresses, for example,
> Multicast DNS (mDNS)** … These restrictions are implemented deep in the
> networking stack, and thus they apply to **all** networking APIs.»

Conexión TCP saliente, **aceptar TCP entrante**, UDP unicast/multicast/broadcast:
todo requiere permiso. En Android 16 es opt-in; **en Android 17 (target SDK 37) es
`ACCESS_LOCAL_NETWORK`, dangerous y bloqueado por defecto**.

**La escapatoria, y es excelente:** `NsdManager` con
`DiscoveryRequest.FLAG_SHOW_PICKER`:

> «Once the user selects a service, the app is granted permission to communicate
> with that specific device… **This grant persists across reboots.** …
> Connections to IP addresses obtained this way **don't require the
> `ACCESS_LOCAL_NETWORK` permission**.»

Descubrimiento **y** conexión TCP autorizada, con **cero permisos runtime**.

**Y un modo de fallo silencioso que hay que conocer:** `WifiManager.MulticastLock`.
El stack Wi-Fi **filtra los paquetes multicast por debajo del socket**. Un
`UdpSocket` de Rust hará `join_multicast_v4` con éxito y **no recibirá nada, sin
error**. El lock sólo se adquiere desde Java/Kotlin.

### 3.3 — Windows

Ahí sí es terreno de Rust. **`mdns-sd 0.20.3`**: puro Rust, **sin runtime async**
—usa un hilo propio y canales, que encaja con el motor síncrono—, Apache-2.0 OR
MIT, **14 dependencias**, limpio en `cargo audit`, 1,6 M descargas/90 d, cobertura
de RFC 6762/6763 documentada sección por sección.

**Es la única dependencia externa que este plan contempla, y sólo bajo
`#[cfg(windows)]`.** El core de Rust conserva cero dependencias en los tres
targets móviles.

Descartados, medidos: `simple-mdns` (10 deps pero 2 564 descargas/90 d frente a
1,6 M); `libmdns` (29 deps, arrastra `tokio`, y sólo responde);
**`searchlight` (86 deps, último release 2023-09-26, con `RUSTSEC-2024-0421`
confirmado por `cargo audit`)**; `zeroconf` y `astro-dnssd` (no son Rust puro, y en
Android no existe Avahi ni `dns_sd.h`).

**Y escribirlo a mano no sale a cuenta:** `std::net::UdpSocket` **no expone**
`SO_REUSEADDR`, `SO_REUSEPORT`, `SO_BINDTODEVICE` ni `IP_MULTICAST_IF`, ni enumera
interfaces. RFC 6762 §15.1 dice que un responder **SHOULD** usar
`SO_REUSEPORT`/`SO_REUSEADDR` para convivir con el resolver del sistema en el
5353; sin ellos el bind falla con `EADDRINUSE` en cualquier máquina con Avahi o
mDNSResponder. Harían falta `socket2` y ~2 000 líneas. El ahorro neto sería ~9
crates.

### 3.4 — El fallback, y va PRIMERO

**IP:puerto manual, y código QR sobre esa misma cadena.** Cero dependencias en
Rust; en Dart, un paquete de QR o dibujarlo a mano.

**Es lo único que funciona en el 100 % de los escenarios**: aislamiento de cliente
en el router, redes corporativas que filtran multicast, el usuario que deniega el
permiso, y el emulador.

**Constrúyelo antes que el descubrimiento automático**, no después. Así la fase 05
puede empezar sin esperar a tres integraciones nativas, y siempre hay un camino
que funciona cuando el bonito falla.

## 4. La confianza: ya está construida, hay que conectarla

`qyro_identity_store::known_peers` ya existe, de ADR-0031: `TrustVerdict`,
`HumanFingerprint` con `to_grouped_hex()`, `decide_trust`, `seal_known_peers` /
`open_known_peers`, y `PeerCandidate`.

**Esta fase la expone por el FFI y la usa.** No la reescribas. Lee la ADR-0031 y
el crate antes de decidir nada.

**Y responde a la pregunta que la ADR dejó abierta:** ¿se completa el handshake y
se pregunta después, o se corta antes? Los dos tienen coste — preguntar después
significa que ya derivaste claves con un desconocido; cortar antes significa que
no puedes enseñar la huella del otro. **Decide con el razonamiento escrito.**

## 5. Lo que hay que construir, paso a paso

### Paso 1 — ADR-0035, congelada

`docs/adr/ADR-0035-discovery-and-pairing.md`:

- El trait `PeerDiscovery` en el core: `advertise`, `browse` →
  `Vec<PeerEndpoint>`, con `PeerEndpoint = (IpAddr, u16, huella)`. **Cero
  dependencias.**
- El nombre del servicio: `_qyro._tcp`, y qué va en el registro TXT. **Ojo: lo que
  pongas ahí lo ve toda la red.** La huella pública sí; el nombre del usuario,
  piénsalo.
- El momento de la decisión de confianza (§4).
- Qué se guarda y cuándo en el almacén de peers.
- La política de `mdns-sd` sólo bajo `cfg(windows)`, con la justificación de §3.3.
- **Lo que no promete:** no hay NAT, no hay internet, no hay reconexión
  automática, y **nada probado en una red real** hasta la fase 07.

**Puerta.**

### Paso 2 — El fallback manual y el QR

- Superficie FFI: conectar a `ip:puerto` **ya existe** desde la fase 01.
- El QR codifica `ip:puerto` **más la huella**, para que escanear sea también
  emparejar.
- **Prueba:** dos procesos, uno anuncia su cadena, el otro la usa, transferencia
  completa.

**Puerta.**

### Paso 3 — La confianza por el FFI

- Consultar el veredicto de un peer, listar conocidos, marcar uno como conocido,
  y **olvidar** uno.
- **`a_known_peer_whose_key_changed_is_refused_by_name`** tiene que seguir pasando
  desde el otro lado del FFI. Es el caso que importa: en SSH eso es un aviso a
  gritos y aquí también.
- La huella se expone **ya formateada** (`to_grouped_hex`), para que la UI no
  invente su propio formato.

**Puerta.**

### Paso 4 — Windows con `mdns-sd`

- Bajo `#[cfg(windows)]`, detrás del trait.
- **`Cargo.lock` sube: dilo, con el conteo exacto y el diff.** Y `cargo audit` en
  verde.
- Prueba en el runner de Windows: anunciar y encontrarse a sí mismo.

**Puerta.**

### Paso 5 — Android con `NsdManager`

- Kotlin, por platform channel, con `FLAG_SHOW_PICKER`.
- **Y el `MulticastLock` si acabas usando cualquier cosa que no sea `NsdManager`.**
- **Comprueba el manifiesto**: si aparece `ACCESS_LOCAL_NETWORK`, has perdido la
  ventaja del picker. Si tiene que aparecer, di por qué.
- Prueba en emulador, con lo que el emulador permita, y **di qué no permite**.

**Puerta.**

### Paso 6 — iOS con `NWBrowser`

- Swift, `NWBrowser` y `NWListener`.
- `Info.plist`: `NSBonjourServices` y `NSLocalNetworkUsageDescription`.
- **Comprueba que NO añadiste el entitlement de multicast.** Si lo necesitas, has
  elegido mal el camino.
- Prueba en simulador, y **di qué no cubre un simulador**: el permiso de red local
  se comporta distinto en un aparato real, y eso es la fase 07.

**Puerta de fase.**

## 6. Las trampas concretas

1. **El socket de Rust que hace `join_multicast_v4` y no recibe nada, sin error**,
   porque falta el `MulticastLock` de Android. Silencioso.
2. **El entitlement de Apple.** Si tu diseño lo necesita, has metido una revisión
   humana en el camino crítico. Rediséñalo.
3. **Aislamiento de cliente.** Muchos routers domésticos y casi todas las Wi-Fi
   públicas impiden que dos clientes se vean. **No es un bug tuyo y no se puede
   arreglar** — por eso el fallback manual va primero.
4. **Confiar en quien firma.** El handshake prueba posesión de una clave, no
   identidad. Sin `decide_trust` no hay seguridad real.
5. **La huella corta.** La ADR-0031 ya fijó su longitud; **no la acortes para que
   quepa mejor en la UI.** Si la UI no cabe, cambia la UI.
6. **El TXT que filtra.** Lo que anuncies lo ve toda la red, incluida la
   cafetería.

## 7. Pruebas obligatorias

- `a_manual_endpoint_string_round_trips_through_a_qr_payload`
- `two_processes_connected_by_a_manual_endpoint_transfer_a_file`
- `a_known_peer_whose_key_changed_is_refused_by_name` — **a través del FFI**
- `forgetting_a_peer_makes_it_new_again_and_not_trusted`
- `the_fingerprint_shown_by_the_ffi_matches_the_one_the_store_computed` — **dos
  caminos distintos, no la misma llamada dos veces**
- Windows: `an_advertised_service_is_found_by_a_browser_on_the_same_host`
- Android, emulador: lo que el emulador permita, con lo que no permita escrito
- iOS, simulador: ídem
- `the_manifest_does_not_declare_access_local_network` (o el argumento escrito)
- `the_entitlements_do_not_include_multicast`

## 8. Criterios de aceptación

1. ADR-0035 congelada antes del código.
2. **El fallback manual + QR funciona y está probado entre dos procesos**, antes de
   que exista descubrimiento automático.
3. La confianza se consulta por el FFI, y el caso de clave cambiada se refuta
   **por nombre**.
4. La huella la formatea el core, no la UI.
5. `mdns-sd` sólo bajo `cfg(windows)`, con el conteo de `Cargo.lock` antes y
   después, `cargo audit` en verde, y la justificación escrita en la ADR.
6. **Cero dependencias externas en los targets de Android e iOS.** Compruébalo con
   `cargo tree --target aarch64-linux-android` y `--target aarch64-apple-ios`, y
   pega la salida.
7. Android usa `NsdManager` con picker y **no declara `ACCESS_LOCAL_NETWORK`**, o
   está justificado.
8. iOS usa `NWBrowser` y **no pide el entitlement de multicast**.
9. Los tres caminos probados donde se pueda, **y lo que no se pudo probar está
   escrito con su motivo**.
10. Barrido con `cargo-mutants`, alcance declarado. `R2` en todas las puertas.
    Informe según `R5`.
11. **Los botones siguen `onPressed: null`.** Ésta es la última fase en que eso es
    cierto.

## 9. Cómo tiene que quedar el resultado

Dos procesos en la misma máquina, o dos aparatos si los hay: uno se anuncia, el
otro lo encuentra, **muestra su huella**, y tras marcarlo como conocido le
transfiere un archivo. Y si el descubrimiento falla, **la cadena manual o el QR
hacen lo mismo**.

## 10. No objetivos

- **UI.** La fase 05.
- NAT, internet, reconexión automática, TLS.
- Keystore, Keychain, empaquetado.
- **Hardware físico.** La fase 07. Aquí todo es emulador, simulador y runners.

## 11. Qué desbloquea

La fase 05, que es el producto. A partir de aquí no queda ninguna incógnita
técnica grande: lo que falta es interfaz, plataformas y empaquetado.
