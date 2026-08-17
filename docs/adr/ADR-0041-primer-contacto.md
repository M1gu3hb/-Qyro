# ADR-0041 — El primer contacto: qué puerto, qué IP, y quién escucha

- **Estado:** **congelada** antes de una sola línea de código.
- **Fecha:** 2026-08-17
- **Fase:** 12
- **Cierra:** QYR-0322, reabierta como P0.
- **Depende de:** ADR-0028 (transporte), ADR-0032 (FFI), ADR-0035 (emparejamiento),
  ADR-0036 (interfaz), ADR-0040 (identidad).
- **Gobernada por:** `R7` §6 — *¿esto acerca el día en que alguien mete un archivo
  en un PC viejo tecleando un comando?* Sí: sin primer contacto no hay ningún día.

---

## 1. El problema, en una frase

**La mitad de la huella del código de emparejamiento existe y funciona desde la
fase 11. La mitad de la dirección nunca se escribió.**

`ownPairingString()` compone `QYRO1|<dirección>|<huella>`. `ownFingerprint()`
llama a `qyro_identity_fingerprint`, que existe y tiene llamante de producción.
`_listeningAddress` se lee y **no se asigna en ninguna parte del árbol**. Así que
la función devuelve `null` siempre, la pantalla enseña «sin conexión, así que no
hay código que mostrar», y la otra pantalla pide teclear ese código. Bucle
cerrado.

---

## 2. Por qué esto **no** exige separar `bind` de `accept`

`Session::open_receiver` liga y acepta dentro de la misma llamada y no vuelve
hasta que un peer se conecta. Ésa es la forma que QYR-0322 describía, y la
solución que la ficha proponía —un `Bound` que sepa su dirección y del que salga
una `Session` al aceptar— es correcta.

**Y aquí no hace falta**, por una razón que hace desaparecer el problema en vez
de resolverlo: **si el puerto se conoce de antemano, no hay nada que preguntarle
al socket.** La cadena se puede componer antes de ligar, no sólo antes de
aceptar.

**La separación se hace en la fase 14**, que la necesita de verdad: el enlace
directo sin router tiene que ligar por interfaz y reintentar mientras APIPA
tarda sus decenas de segundos (`R8` §8), y ahí sí hace falta un objeto ligado que
sobreviva entre intentos. Hacerlo hoy sería construir la API pública del crate
frontera para un caso que aún no se ha escrito.

**Una línea de por qué, como pide la regla:** se elige el puerto fijo porque
elimina la pregunta en lugar de contestarla, y porque la respuesta larga la
necesita la fase 14 con requisitos que hoy no se conocen del todo.

---

## 3. Decisión 1 — **el puerto es fijo: 49517**

`QYRO_DEFAULT_PORT = 49517`.

**Por qué en 49152–65535.** Es el rango **Dynamic/Private** de IANA, del que la
propia IANA dice que **nunca asigna** un servicio registrado. Un número de ahí no
puede colisionar por registro con nada; sólo por coincidencia con otro programa
que también eligió al azar.
Fuente: <https://www.iana.org/assignments/service-names-port-numbers/service-names-port-numbers.xhtml>

**Por qué fijo y no efímero, y ésta es la razón que decide.** `R8` §9: el
firewall de Windows bloquea todo el inbound por defecto, y una red sin gateway
—exactamente el cable directo de la fase 14— se clasifica como perfil **Public**,
el más restrictivo. El permiso se concede **una vez, por programa y puerto**.

> Con un puerto **fijo**, la persona autoriza Qyro **una vez** y no vuelve a ver
> el diálogo. Con un puerto **efímero**, cada sesión pide un puerto distinto y el
> diálogo **vuelve cada vez** — en la máquina donde menos ganas hay de verlo.

Un puerto efímero además hace la cadena impredecible, así que habría que ligar
antes de mostrarla, que es el problema que §2 acaba de disolver.

### Si el puerto está ocupado: **se dice, no se mueve**

Nada de «siguiente libre en silencio». Un puerto que se mueve solo pierde las dos
propiedades por las que se eligió fijo —el permiso de firewall y la cadena
predecible— y las pierde sin avisar, que es peor que fallar.

Qyro dice qué puerto está ocupado y ofrece elegir otro. La cadena de
emparejamiento **siempre lleva el puerto dentro**, así que un puerto elegido a
mano sigue funcionando: lo que se pierde es la comodidad, no la función.

---

## 4. Decisión 2 — **qué IP va en la cadena: todas las candidatas**

Un aparato tiene varias direcciones. Adivinar produce un código que no funciona y
que no dice por qué.

**Se enumeran las interfaces y se enseñan todas las candidatas**, cada una con el
nombre de su interfaz al lado, para que una persona reconozca la suya.

**Qué se excluye, y sólo esto:**

| Se excluye | Por qué |
|---|---|
| Loopback (`127.0.0.0/8`, `::1`) | Un código con loopback sólo funciona contra uno mismo |
| IPv6 link-local (`fe80::/10`) | Lleva zone-id, y el zone-id **es local al nodo**: la zona del emisor no viaja (RFC 4007). Meterlo en un código que se teclea en otra máquina es meter un dato que allí significa otra cosa. **La fase 14 lo usará, con su índice de interfaz y sin enseñárselo a nadie** |

**Qué NO se excluye, y es deliberado:** las direcciones de adaptadores virtuales
—Hyper-V, VirtualBox, WSL, VPN—. Filtrarlas exige una lista de nombres de
adaptador por sistema operativo, que es exactamente la clase de heurística que
envejece mal y que este proyecto ya ha pagado dos veces. **Se enseñan todas con
su nombre y decide la persona**, que sabe a qué red está conectada y el programa
no. Si hay una sola candidata, no hay nada que decidir.

*(`R8` §8 pide además no resolver nombres nunca: la enumeración devuelve IP
literales y la conexión va a IP literales. Aquí se cumple por construcción.)*

**Enumerar es de Dart, no del FFI.** `NetworkInterface.list()` de `dart:io`
existe en Android y en Windows, no pide permiso, y devuelve nombre e IP. Añadir
un símbolo C para algo que la plataforma ya da sería ensanchar la frontera de
seguridad sin comprar nada — y la frontera es la superficie que ADR-0032 cuenta
símbolo a símbolo.

**Consecuencia medible: esta fase añade cero símbolos al FFI.** Sigue en
veintitrés.

---

## 5. Decisión 3 — **quién escucha y quién conecta**

**Sólo el receptor escucha. El emisor únicamente conecta hacia afuera.**

Ya es así, y esta ADR lo fija como invariante en vez de dejarlo como accidente:
el outbound está permitido por defecto en el firewall de Windows, así que **un
solo lado necesita el permiso** — y es el lado que enseña el código, es decir, la
máquina donde la persona está mirando y puede darlo.

Nunca se invierte automáticamente. Un producto que «prueba al revés si no
conecta» duplica el número de máquinas que necesitan permiso para arreglar un
caso que la persona resuelve pulsando Recibir en el otro lado.

---

## 6. Decisión 4 — **cuándo aparece la cadena**

**En cuanto la pantalla de recibir se abre, antes de que nadie se conecte y antes
incluso de ligar.** El puerto se conoce, la huella se conoce, las IP se enumeran
al instante.

Consecuencia honesta que hay que dibujar en la pantalla: **la cadena aparece
antes de que el socket esté ligado**, así que existe una ventana —milisegundos—
en la que el código se enseña y el puerto todavía no acepta. Es irrelevante para
una persona que lo está tecleando en otro aparato, y **decirlo aquí es más barato
que descubrirlo**. Si ligar falla, la pantalla sustituye la cadena por el error
del puerto ocupado.

---

## 7. Lo que esta ADR NO decide

- **El descubrimiento automático.** No cruza el FFI y esta fase no lo cruza. Lo
  que sí hace es dejar de anunciarlo donde hoy se anuncia sin existir; la fase 14
  lo conecta de verdad.
- **La forma de la API de `qyro_session`.** Sin cambios. `Bound`/`accept` es de
  la fase 14.
- **IPv6.** La cadena lleva IPv4 en la v1.x. IPv6 global funcionaría, pero
  link-local es el caso que importa y lo lleva la 14 con su índice de interfaz.

---

## 8. Alternativas descartadas

**Puerto efímero y consultar el socket.** Es lo que QYR-0322 pedía y lo que la
fase 14 acabará teniendo. Descartada **para hoy** porque cuesta cambiar la API
pública del crate frontera para comprar algo que un número fijo da gratis, y
porque el puerto efímero cuesta un diálogo de firewall por sesión.

**Adivinar la IP «principal».** Abrir un socket UDP hacia 8.8.8.8 y leer la
dirección local es el truco habitual para saber «por dónde saldría». Descartada
por dos razones y la segunda es la que manda: adivina la ruta a **internet**,
cuando el caso de uso es una LAN sin internet; y **un producto que promete no
hablar con la nube no escribe la IP de Google en su código**, ni siquiera para no
enviar nada.

**Un símbolo C que enumere interfaces.** `if-addrs` ya está en el grafo por
`mdns-sd`. Descartada: sólo está en Windows, la frontera crece, y `dart:io` ya lo
da en las dos plataformas.
