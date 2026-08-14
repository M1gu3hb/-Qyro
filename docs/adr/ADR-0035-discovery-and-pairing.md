# ADR-0035 — El descubrimiento y el emparejamiento

- **Estado:** aceptada
- **Fecha:** 2026-08-14
- **Fase:** 04, paso 1
- **iOS:** fuera de la v1.0 por ADR-0039. Lo que aquí se decide para Android y
  Windows vale; el `NWBrowser` queda aplazado, no cancelado.
- **No revoca** ADR-0031, que sigue siendo la decisión de confianza. Esta la
  **conecta**.

---

## 1. El orden, que es la decisión más importante

**El camino manual va primero, y el descubrimiento automático después.**

Una cadena `ip:puerto` y un QR sobre esa misma cadena funcionan en el 100 % de
los escenarios: aislamiento de cliente en el router, redes que filtran multicast,
una persona que deniega el permiso, y un emulador. El descubrimiento automático
no funciona en ninguno de esos cuatro.

Construirlo primero también significa que **la fase 05 puede empezar sin esperar
a tres integraciones nativas**, y que cuando el camino bonito falle siempre habrá
uno que no.

---

## 2. La cadena de emparejamiento

```
QYRO1|<socket-addr>|<32 hex en minúscula>
```

- **Tres campos, dos separadores, y el separador es `|`** — que no aparece ni en
  una dirección de socket ni en hexadecimal, así que dividir por él es exacto y
  no hay que escapar nada.
- `<socket-addr>` es exactamente lo que produce `SocketAddr::to_string()`:
  `192.168.1.7:47001` o `[fe80::1]:47001`. Se vuelve a leer con `FromStr`, que ya
  acepta las dos formas y rechaza el resto. **Cero dependencias.**
- `<32 hex>` son los **16 bytes de la huella** de la identidad pública del peer,
  en minúscula. Longitud fija, así que una cadena truncada no parsea.
- `QYRO1` delante para que una cadena suelta se reconozca, y para que una versión
  futura pueda cambiar el resto sin ambigüedad.

**Lo mismo va en el QR.** No hay un segundo formato: escanear es leer esta cadena.

### 2.1 — Y lo que la huella de la cadena NO es

**No es una credencial. Es una expectativa.**

Escanear un código **no establece confianza por sí solo**. Lo que hace es fijar
qué huella *tiene que* salir del handshake. La confianza la sigue decidiendo
`decide_trust` contra el almacén, y la identidad que se le pasa es la
**autenticada**, nunca la que venía en la cadena.

La regla que sale de ahí, y es la que hace que escanear valga la pena:

> **Si la cadena traía huella y no coincide con la autenticada, se rechaza sin
> preguntar a nadie.** Una persona que escaneó un código ya respondió la
> pregunta; volvérsela a hacer es enseñarle a decir que sí.

---

## 3. Cuándo se decide la confianza

**ADR-0031 dejó esto abierto. Se decide: el handshake se completa entero, y la
confianza se decide después, antes de que cruce un manifiesto o un byte de
contenido.**

**Por qué, y no es la opción cómoda:**

- **Una huella enseñada antes de autenticar es una afirmación, no un hecho.**
  Cualquiera puede poner cualquier clave pública en un `hello`. Enseñar ese
  número y pedirle a una persona que lo compare en voz alta le estaría enseñando
  a aprobar un dato que no significa nada. El handshake es lo que convierte «dice
  ser» en «posee la mitad privada».
- **Cortar antes hace el emparejamiento imposible**, no sólo incómodo: sin
  handshake no hay huella del otro que enseñar.
- **Lo que cuesta completar primero está acotado y es barato:** un X25519 y un
  HKDF con un desconocido. No se le ha enviado nada, no aprende nada que no
  supiera, y derivar una clave con alguien **no es confiar en él**. La sesión se
  suelta si la confianza se niega.

**La secuencia, entonces:**

1. Handshake completo → `PublicIdentity` **autenticada**.
2. Si la cadena de emparejamiento traía huella y no coincide → **se corta**, sin
   preguntar.
3. `decide_trust` contra el almacén:
   - `KnownAndMatches` → sigue.
   - `KnownAndChanged` → **se rechaza por nombre y no se pregunta.** En SSH esto
     es un aviso a gritos; aquí también. No hay «continuar de todos modos» que se
     pueda pulsar sin pasar antes por olvidar al peer a propósito.
   - `New` → se le pregunta a la persona, enseñando la huella formateada.
4. Sólo después de eso cruza un manifiesto.

---

## 4. Qué se guarda, y cuándo

- **Nunca automáticamente.** Un peer entra en el almacén cuando una persona lo
  marca, y no porque una transferencia haya salido bien.
- **`KnownAndChanged` no sobrescribe nunca.** Para volver a confiar en esa clave
  hay que **olvidar** el peer primero, que es una acción distinta y explícita.
  Sobrescribir en silencio convierte el almacén en un registro de lo último que
  se vio, que es exactamente lo que no protege de nada.
- La huella la formatea **el core**, con `HumanFingerprint::to_grouped_hex()`. Si
  dos aparatos la muestran distinta, compararla en voz alta no vale nada — así
  que la interfaz no tiene permiso para inventarse un formato.

---

## 5. Las dos fronteras que esto mueve, y por qué son estrechas

Medidas antes de decidir, no supuestas.

**(a) `qyro_net::Session` no publica la identidad del peer.** `qyro_crypto` sí la
tiene —`peer_identity()` sobre el estado establecido— y `qyro_net` la envuelve sin
republicarla. Sin eso no hay huella que enseñar. **Se ensancha `qyro_net` con un
accesor de sólo lectura a la identidad *pública*** del peer. No es material de
clave: es exactamente lo que viajó por el cable en claro y lo que el handshake
acaba de autenticar.

**(b) `qyro_session` no puede reexportar `TrustVerdict` ni `HumanFingerprint`.**
`qyro_session_re_exports_nothing_it_does_not_own` sólo admite `pub use` de
`crate`, `self`, `super`, `error` y `session`. **Eso es ADR-0032 funcionando, no
un obstáculo**: la fachada existe para acotar lo que `qyro_ffi` puede nombrar.

Así que **`qyro_session` posee su propio vocabulario de confianza** —su enum de
veredicto y la huella ya formateada como texto— y convierte por dentro. El coste
es un `match` de tres brazos; lo que compra es que la superficie C no crezca por
accidente cada vez que un crate interno añada una variante.

`qyro_session` gana una dependencia de **primera parte**, `qyro_identity_store`.
`qyro_ffi` sigue nombrando exactamente dos crates, y `CLOSURE` en
`c_abi_contract.rs` se actualiza porque es un registro de cambios, no una
prohibición.

---

## 6. El servicio anunciado, y lo que la red entera puede leer

- Nombre: **`_qyro._tcp`**.
- TXT: **la huella pública y nada más**, como `fp=<32 hex>`.
- **No va el nombre del usuario, ni el del aparato, ni el sistema operativo, ni
  la versión.** Lo que se anuncia lo lee toda la red, incluida la cafetería. La
  huella ya es pública por diseño —viaja en claro en el handshake— y sirve para
  que la interfaz pueda enseñar a quién está a punto de conectarse antes de
  conectarse.

---

## 7. `mdns-sd` sólo bajo `cfg(windows)`

La única dependencia externa que este plan contempla, y sólo ahí: Rust puro, sin
runtime async, Apache-2.0 OR MIT, 14 dependencias, limpio en `cargo audit`.

**En Android e iOS el descubrimiento no es de Rust y no puede serlo.** Las dos
plataformas están cerrando el acceso a la red local desde sockets crudos, y el
gate está por debajo de la API: un socket de Rust no lo esquiva. Van por
`NsdManager` y `NWBrowser`, detrás del mismo trait.

**El core conserva cero dependencias externas en los tres targets móviles**, y
eso se comprueba con `cargo tree --target`, no se afirma.

---

## 8. Lo que esta decisión NO promete

- **No promete NAT, internet ni reconexión automática.** Una red local, una
  sesión.
- **No promete que dos aparatos se vean.** El aislamiento de cliente es del
  router y no se puede arreglar desde aquí. Por eso el camino manual va primero.
- **No promete nada probado en una red real.** Todo lo de esta fase es loopback,
  runners y —si llega— un emulador. Dos procesos en `127.0.0.1` no son dos
  aparatos en una Wi-Fi, y eso no cambia hasta la fase 07.
- **No promete que el QR se lea con una cámara.** Esta fase congela la cadena y
  la prueba; leerla es interfaz, y la interfaz es la fase 05.
