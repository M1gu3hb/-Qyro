# ADR-0043 — El enlace directo: encontrarse sin router

- **Estado:** **congelada** antes de una sola línea de código.
- **Fecha:** 2026-08-17
- **Fase:** 14
- **Depende de:** ADR-0028 (transporte), ADR-0035 (descubrimiento), ADR-0041
  (primer contacto), ADR-0042 (CLI).
- **Gobernada por:** `R7` §R7.4, nivel 2 de la escalera de canales.

---

## 1. Lo primero que hay que decirle a la persona

> **Si esa máquina tiene tarjeta de red, un cable directo es entre 10 y 10 000
> veces más rápido que el QR o el serie.**

`R8` §5.4. Eso **se pregunta antes** de proponer un canal lento, y va en la
interfaz, no en una nota al pie. Un producto que ofrece el QR a alguien que tiene
un cable en el cajón le está costando horas.

---

## 2. Decisión 1 — el orden de intentos, y es observable

| # | Se prueba | Cuánto se espera | Qué ve la persona |
|---|---|---|---|
| 1 | La dirección que ya hay (LAN normal) | inmediato | «buscando en la red» |
| 2 | **IPv6 link-local**, por cada interfaz | inmediato | «probando el cable» |
| 3 | IPv4 link-local (APIPA) | **hasta 60 s** | «esperando dirección… 23 s» |

**El paso 2 va antes que el 3 y ésa es la decisión.** RFC 4291: *«All interfaces
are required to have at least one Link-Local unicast address»* — **siempre está,
sin espera**. APIPA no: el cliente DHCP intenta y falla primero, y `R8` §8 mide
decenas de segundos.

**Nunca se falla en silencio ni se queda mudo.** La cuenta atrás se enseña, y a
los 60 s el mensaje dice qué probar —«si no enlaza, prueba un cable cruzado»
(`R8` §8: Auto-MDI-X es cláusula de 1000BASE-T y una NIC de sólo 10/100 puede no
tenerlo)—, no «error».

---

## 3. Decisión 2 — el zone-id **nunca** sale del aparato

IPv6 link-local es el transporte más limpio y **su dirección no se puede
teclear**: el zone-id es local al nodo (RFC 4007), así que `fe80::1%3` significa
otra interfaz en la máquina de al lado.

- Se usa **dentro**, resuelto con `if_nametoindex`.
- **Nunca** entra en un código de emparejamiento, ni se enseña.
- Trampa verificada en `R8` §8: `"[fe80::1%eth0]:9000".parse()` **falla** —
  `SocketAddrV6` sólo acepta el scope-id como entero decimal.

Lo que la persona teclea sigue siendo `QYRO1|<ipv4>:49517|<huella>` (ADR-0041).

---

## 4. Decisión 3 — un solo lado escucha, y es el receptor

`R8` §9. El firewall bloquea inbound por defecto; un enlace sin gateway es «red
no identificada» ⇒ perfil **Public**; el diálogo de permiso es un *MAY* y con
`AllowLocalPolicyMerge=false` ni una regla de admin sobrevive.

**El receptor escucha; el emisor sólo conecta hacia afuera.** Se invierte
pulsando Recibir en el otro aparato — a mano, nunca solo. Un producto que
«prueba al revés» duplica las máquinas que necesitan permiso.

---

## 5. Decisión 4 — el descubrimiento se implementa dentro, y `socket2` entra

**Windows no ofrece un responder mDNS usable**: [MS-WPO] lista DNS, NetBIOS/WINS,
LLMNR y PNRP — **mDNS no está**. Y musl no tiene NSS. Se implementa en el árbol.

- mDNS/DNS-SD sobre **224.0.0.251** y **FF02::FB**, UDP **5353**. Multicast
  link-local puro: funciona sin router.
- **Respaldo simultáneo, no alternativo**: broadcast a `255.255.255.255`, el de
  subred, y `ff02::1`. Los tres a la vez, cada 1–2 s.
- **Por cada interfaz enumerada.** `std` no expone `IPV6_MULTICAST_IF`, y
  `join_multicast_v6(.., 0)` deja elegir al sistema, que **elige mal** con Wi-Fi
  + Ethernet + VPN + Hyper-V. **`socket2` 0.5** (pre-autorizado, `R8` §7).
- **Se desduplica por huella, nunca por IP.** Una IP cambia cuando aparece o
  desaparece un DHCP, y el mismo aparato se vería como dos.
- `set_multicast_ttl_v4` se queda en 1: los paquetes no salen del enlace.

### Lo que se conecta en vez de reescribirse

`qyro_net::discovery` ya tiene `MdnsDiscovery`, `PeerEndpoint` y las dos
funciones de TXT, escritos en la fase 04b **con cero consumidores**.
`DiscoveryChannel.kt` ya existe con `NsdManager` + `FLAG_SHOW_PICKER` y **ningún
Dart abre su canal**.

**Se exponen y se conectan.** Al cerrar esta fase los dos tienen que aparecer en
la tabla de la comprobación 14 con archivo y línea, **por consumidor** — y el
CLI es el consumidor que los estrena, porque es donde el caso de uso vive.

---

## 6. Lo que NO se promete

**Wi-Fi sin router, nunca.** `R8` §8: `netsh wlan set hostednetwork` está
deprecado y depende del driver — la mayoría de los WDI de Windows 10/11 reportan
«Hosted network supported: No». Wi-Fi Direct es WinRT y Windows 10+. Mobile
Hotspot pide admin.

**Se promete «cable directo o red compartida».** Nada más.

---

## 7. Alternativas descartadas

**`mdns-sd` en todos los targets.** Ya está en el grafo de Windows y es buena,
pero arrastra 16 paquetes a un binario que apunta a 750–950 KB y no da control
por interfaz, que es justo lo que falla con varias NIC.

**Esperar a APIPA antes de probar IPv6 link-local.** Sesenta segundos de espera
por delante de un transporte que ya estaba listo.
