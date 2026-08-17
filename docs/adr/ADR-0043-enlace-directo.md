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

---

## 8. Enmienda 1 (2026-08-18) — dos cosas medidas al implementar la §5

Esta enmienda no cambia ninguna decisión. Añade una dependencia que la §5 daba
por supuesta sin nombrarla, y **corrige una cifra que esta misma ADR escribió mal
porque nadie la había medido.**

### 8.1 — `if-addrs` pasa a dependencia declarada de `qyro_net`

La §5 dice «por cada interfaz enumerada» y pre-autoriza `socket2` para *nombrar*
la interfaz. **Enumerarlas es la otra mitad y `std` no la tiene.** El truco de la
tabla de rutas que usa el CLI —conectar un UDP a una dirección de documentación y
preguntar la local— devuelve **una** dirección: la de la ruta por defecto, que es
justo la que el sistema ya prefiere y justo la equivocada cuando el cable directo
no es la ruta por defecto.

`if-addrs` 0.15 ya estaba en el grafo auditado a través de `mdns-sd` en Windows.
Esto la promueve a declarada y la añade en los demás targets. MIT. **El cierre de
dependencias no cambió en este host** (77 antes, 77 después), que es la
comprobación de que no es un paquete nuevo aquí.

### 8.2 — La cifra de la §7 está desmentida por la medida

La §7 descarta `mdns-sd` en todos los targets diciendo que «arrastra 16 paquetes
a un binario que **apunta a 750–950 KB**». Medido con
`cargo build --locked --release -p qyro_cli --target x86_64-pc-windows-msvc`, el
mismo comando de `cli-builds.yml`:

| Commit / estado | `qyro.exe` |
|---|---|
| `458d4bd` — CLI sin `find` | **666 624 B** |
| `3ecebed` — llega `qyro find` y con él `mdns-sd` | **1 295 872 B** |
| `socket2` añadido, nada llamando a `Beacon` | 1 298 432 B |
| El beacon con llamante de producción | **1 306 624 B** |

**El binario ya está en 1 276 KB, por encima del techo que esta ADR se puso.** Lo
gastó `mdns-sd`: **+614 KB**, diez veces los 63 KB que este taller discutió para
conservar el desenrollado de pila. El beacon propio hace el mismo trabajo por
**8 KB**.

Las dos filas del medio son la comprobación 14 llegando por el enlazador: con el
módulo escrito y sin llamante el binario **no cambió ni un byte**, porque el
enlazador lo descartó entero. Una capacidad sin llamante no se envía, se compila.

**Qué se hace con esto: nada todavía, y a propósito.** El descubrimiento por mDNS
funciona hoy y quitarlo sin sustituto probado es cambiar el producto por una
cifra. Lo que cambia es que la §7 ya no puede citar un presupuesto que el binario
no cumple: **la fase 19, con red de verdad, decide si el beacon basta solo.** Si
basta, `mdns-sd` se va y con él 614 KB. Anotado como **D9** en
`deuda-de-calidad.md`; **D10** es el puerto duplicado que esta misma
implementación destapó al ir a escribir la tercera copia.
