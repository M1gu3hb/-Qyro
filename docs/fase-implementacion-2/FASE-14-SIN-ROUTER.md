# FASE 14 — Que se encuentren sin router

> El nivel 2 de la escalera de canales de `R7` §R7.4. Un cable entre dos máquinas, o
> dos máquinas en una red sin DHCP, y Qyro funciona igual.

---

## 1. Por qué existe esta fase

La escena de `R7` §2 es un PC viejo. Puede que tenga tarjeta de red y nada más: sin
router, sin DHCP, sin Wi-Fi que funcione. **Un cable Ethernet entre las dos máquinas
debería bastar, y hoy no basta**, porque Qyro asume que hay una red configurada.

Y hay una cosa que hay que decirle al usuario **antes** de proponerle nada lento:

> Si la máquina vieja tiene tarjeta de red, **un cable directo es entre 10 y 10 000
> veces más rápido que el QR o el serie.** Comprobar eso primero es parte del
> producto, no una nota.

---

## 2. La decisión que hay que congelar

`docs/adr/ADR-00XX-enlace-directo.md`. Decide:

1. **El orden de intentos**, y que sea observable: qué se prueba primero, cuánto se
   espera, y qué se le enseña al usuario mientras tanto.
2. **IPv6 link-local como transporte interno, nunca como cosa que el usuario teclee.**
   Está siempre presente sin espera (RFC 4291: *«All interfaces are required to have
   at least one Link-Local unicast address»*), pero el zone-id **es local al nodo** y
   no viaja (RFC 4007). Un código de emparejamiento con `%3` dentro es un código que
   sólo funciona en la máquina que lo generó.
3. **El presupuesto de espera de APIPA.** `R8` §8: Windows tiene 169.254/16
   habilitado por defecto, pero el cliente DHCP intenta y falla primero, y en la
   práctica son **decenas de segundos**. La interfaz debe **tolerar ~60 s de "sin
   dirección", decir qué está esperando, y reintentar** — no fallar ni quedarse muda.
4. **Quién escucha.** `R8` §9: el firewall bloquea inbound por defecto, un enlace sin
   gateway es «red no identificada» ⇒ perfil **Public**, el diálogo de permiso es un
   *MAY* y no un *MUST*, y con `AllowLocalPolicyMerge=false` por GPO ni una regla de
   admin sobrevive. **El protocolo debe permitir que un solo lado escuche.** Decide
   cuál por defecto y cómo se invierte.

---

## 3. Entregables

### 3.1 — Descubrimiento propio, dentro del binario

**Windows no ofrece un responder mDNS usable.** La especificación oficial de
resolución de nombres de Windows ([MS-WPO]) lista DNS, NetBIOS/WINS, LLMNR y PNRP —
**mDNS no está**. Y musl no tiene NSS. **Conclusión: se implementa dentro.**

- mDNS/DNS-SD sobre **224.0.0.251** y **FF02::FB**, UDP **5353** (RFC 6762 / 6763).
  Funciona sin router: es multicast link-local puro.
- **Respaldo simultáneo, no alternativo:** broadcast a **255.255.255.255** (RFC 1122:
  *«will be received by every host on the connected physical network»*), broadcast de
  subred, y **ff02::1**.
- **Por cada interfaz enumerada**, no «la que elija el sistema». `std` no expone
  `IPV6_MULTICAST_IF` y `join_multicast_v6(.., 0)` deja elegir al SO, que **elige mal**
  cuando hay Wi-Fi + Ethernet + adaptadores de VPN/Docker/Hyper-V. Usa `socket2`
  (pre-autorizado, `R8` §7).
- Cada 1–2 s. **Desduplica por huella criptográfica, nunca por IP** — la IP cambia
  cuando aparece o desaparece un DHCP y el mismo aparato se vería como dos.
- **`set_multicast_ttl_v4` por defecto es 1** y así se queda: los paquetes no salen
  del enlace local. No lo toques.

### 3.2 — Lo que ya existe y hay que conectar, no reescribir

`qyro_net::discovery` ya tiene `MdnsDiscovery` bajo `cfg(windows)`, `PeerEndpoint`,
`fingerprint_to_txt` y `fingerprint_from_txt`, escrito en la fase 04b y **con cero
consumidores**. `DiscoveryChannel.kt` ya existe en Android con `NsdManager` +
`FLAG_SHOW_PICKER`, registrado en el canal `dev.qyro/discovery`, **y ningún Dart lo
abre**.

**Exponlos y conéctalos. No los reescribas.** Y aplica la comprobación 14 a los dos:
tras esta fase, cada uno debe tener un llamante de producción con archivo y línea.

### 3.3 — La trampa del multicast que no da error

Cualquier cosa que no sea `NsdManager` en Android necesita
`WifiManager.MulticastLock`: **el stack Wi-Fi filtra el multicast por debajo del
socket** y `join_multicast_v4` **tiene éxito sin recibir nada y sin error**. Está
anotado desde la fase 04 y sigue siendo cierto.

### 3.4 — El asistente del cable

Cuando el descubrimiento no encuentra nada, Qyro debe **guiar**, no rendirse:

1. «¿Los dos aparatos están en la misma red?» → si no, «conecta un cable entre los
   dos».
2. Esperar la dirección link-local **diciendo que la espera es normal y por qué**.
3. Si a los 60 s no hay enlace: **«prueba un cable cruzado»** — Auto-MDI-X está en
   IEEE 802.3 cláusula 40.4.4, que es la de **1000BASE-T**; una NIC vieja de sólo
   10/100 puede no tenerlo, y ésa es exactamente la NIC de la máquina de la escena.
4. Si hay enlace pero no hay peer: **el código de emparejamiento tecleado**, que
   funciona aunque el descubrimiento esté filtrado.

### 3.5 — Wi-Fi sin router: **no se promete**

`R8` §8. `netsh wlan set hostednetwork` es de Windows 7, está deprecado de facto y
**depende del driver**; Wi-Fi Direct es WinRT y Windows 10+; Mobile Hotspot está
construido alrededor de compartir una conexión y pide admin.

**Qyro promete «cable directo o red compartida». Nunca «Wi-Fi sin router».** Si
algún documento del repositorio lo promete, corrígelo en esta fase.

---

## 4. La prueba que cierra la fase

> **Dos procesos, cada uno atado a una interfaz distinta de una red aislada sin
> DHCP y sin router, se encuentran solos y transfieren un archivo verificado.**

En CI eso se monta con **dos namespaces de red conectados por un `veth`, sin
servidor DHCP y sin ruta por defecto**. Es reproducible por código de salida y no
necesita hardware.

**Controles, los dos obligatorios:**
- Con el descubrimiento **deshabilitado**, la misma prueba **falla** con un error
  nombrado — así se sabe que el descubrimiento es lo que la hizo pasar.
- Con **dos** peers anunciando, el desduplicado por huella **no los colapsa** en uno.

**Y una prueba de la trampa de Rust** (`R8` §8): que
`"[fe80::1%eth0]:9000".parse::<SocketAddrV6>()` **falla**, y que el camino de Qyro
—`if_nametoindex` y después el índice decimal— **funciona**. Es un renglón y ahorra
un día.

---

## 5. La puerta

Quince comprobaciones. En la 15, la cadena completa es:
**«la persona conecta un cable» → APIPA/link-local → anuncio mDNS/broadcast →
el otro lo ve → aparece en la lista con su huella → transferencia».** Sin saltos.

---

## 6. Lo que NO hay que hacer

- **No implementes Wi-Fi Direct.** Es WinRT, es Windows 10+, mata Windows 7, y el
  peer del otro lado tiene que hablarlo también.
- **No añadas un servidor DHCP.** Link-local existe exactamente para no tenerlo.
- **No resuelvas nombres.** Ni `.local` del sistema, ni DNS, nunca (`R8` §6).
- **No metas NAT traversal ni relay.** `R7` §R7.2: cero terceros. Eso no cambia.
