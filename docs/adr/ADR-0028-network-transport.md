# ADR-0028 — Un socket TCP debajo del motor

- Estado: **congelada** antes de escribir una sola línea de `qyro_net`.
- Fecha: 2026-08-11
- Sprint: 6A
- Usa, sin modificar: ADR-0016 (framing), ADR-0018 (errores estructurales y
  semánticos), ADR-0021 (handshake de cuatro mensajes), ADR-0022 (AEAD de frame),
  ADR-0026 (sesión de transferencia).
- Fuera: descubrimiento, FFI, UI, selector de archivos, reconexión, NAT, TLS.

## Contexto

Hasta hoy el «transporte» de Qyro es un `Vec<u8>` que pasa de una variable a otra
dentro del mismo proceso. Todo lo demás es real: el handshake firma de verdad, el
sellado cifra de verdad, el motor cuenta chunks de verdad. Lo único simulado es el
cable.

Poner el cable no es «abrir un socket». Son tres problemas, y esta ADR existe para
decidirlos por escrito antes de que la primera línea de código los decida por
accidente:

1. **Un `read` de TCP no es un mensaje.** Devuelve los bytes que haya.
2. **Un socket es memoria que un desconocido controla.** Antes del handshake, el
   peer no está autenticado y puede mandar lo que quiera.
3. **Un cierre no es un final.** Hay varias formas de que una transferencia acabe y
   son distintas entre sí.

La decisión de no usar async está tomada fuera de esta ADR y no se reabre aquí:
`std::net` y `std::thread`. Qyro tiene **una** conexión; el valor de un runtime
async es multiplexar miles sobre pocos hilos, y con N=1 un hilo bloqueante es más
simple, más fácil de depurar y produce backtraces legibles. Lo que esta ADR decide
es todo lo demás.

---

## 1. El framing sobre el stream: el frame va tal cual, sin prefijo de longitud

**Decisión: los bytes que salen de `Frame::encode()` / `SealedFrame::encode()` se
escriben en el socket tal cual. No hay prefijo de longitud propio del transporte, ni
delimitador, ni sobre.**

El motivo es de seguridad, no de elegancia. La cabecera de 48 bytes **ya** lleva
`payload_len` y `trailer_len`, y la cabecera completa es el dato asociado del AEAD
(ADR-0022). Un prefijo de longitud propio sería un segundo sitio donde vive la misma
longitud, **fuera del tag**. Eso tiene dos consecuencias, las dos malas:

- Un atacante activo sobre el stream —TCP no ofrece integridad frente a nadie que
  esté en el camino— puede alterar el prefijo sin romper el tag, porque el tag no lo
  cubre. Con el prefijo, decide cuántos bytes reserva y espera el receptor. Sin el
  prefijo, la única longitud del sistema es la que el tag autentica.
- Dos longitudes pueden discrepar, y entonces hay que decidir cuál gana. Ese estado
  no debe existir.

**Lo que esta elección garantiza, con precisión:**

- Hay **exactamente una** longitud en el sistema. No existe el estado «dos campos de
  longitud que no coinciden».
- La longitud que decide cuánta memoria se reserva es la misma que el AEAD
  autentica. Un atacante que la cambie produce un frame que falla en `open()`.
- Se conserva el re-encoding byte-exacto, que ADR-0018 declara precondición de que
  la cabecera sirva como datos asociados.

**Lo que esta elección NO garantiza, y hay que decirlo:** la longitud se lee *antes*
de autenticarla. Es inevitable: para saber cuántos bytes esperar hay que leer la
cabecera, y para verificar el tag hay que tener el frame entero. Lo que hay entre
esos dos momentos es una ventana en la que un valor no autenticado gobierna una
espera y una reserva. Esa ventana está acotada por dos cosas y sólo por dos: el techo
`MAX_BUFFER_LEN` del decodificador (1 049 664 bytes), que convierte una longitud
absurda en `BufferLimitExceeded` en vez de en una reserva; y el límite de bytes
previos a autenticar de §3, que la cierra del todo mientras el peer sea un
desconocido. **Una mentira sobre la longitud se detecta después y se acota antes; no
se impide.**

### Los cuatro mensajes del handshake también van dentro de frames

**Decisión: los cuatro mensajes de ADR-0021 viajan como payload de frames planos de
tipo `Hello`, no en crudo sobre el socket.**

El discriminador ya existe dentro del propio mensaje: los tres primeros bytes de cada
uno son `HANDSHAKE_VERSION`, `CRYPTO_SUITE_ID` y el tipo (1 a 4), y `qyro_crypto` los
valida. No hace falta un tipo de mensaje nuevo por cada uno, y por tanto **esta ADR no
necesita tocar `message.rs` ni `header.rs`** — que además es de otro agente en este
run.

El motivo de fondo es que haya **un solo framing en la conexión desde el byte cero**.
La alternativa —handshake en crudo y luego frames— mete un cambio de modo en el
punto exacto de la conexión donde el peer todavía no está autenticado, y los cambios
de modo son donde viven los fallos de desincronización. Con esta decisión hay un
decodificador, un sitio que impone cotas, y ninguna transición.

Tamaños resultantes, que son los números que usa §3:

| Mensaje | Cuerpo | Con cabecera de 48 B |
|---|---|---|
| `InitiatorHello` | 100 | 148 |
| `ResponderHello` | 164 | 212 |
| `InitiatorFinish` | 99 | 147 |
| `ResponderFinish` | 35 | 83 |

Cada extremo **recibe** exactamente 295 bytes de handshake: el que marca recibe
212 + 83, el que escucha recibe 148 + 147. La simetría es casualidad aritmética, no
un invariante; lo que importa es que 295 es el número contra el que se dimensiona el
límite de §3.

---

## 2. El tamaño del búfer de lectura: 65 536 bytes

**Decisión: `READ_BUFFER_LEN = 65 536`.**

Primero, qué es y qué no es. Este búfer es el sitio donde aterrizan los bytes entre
`TcpStream::read` y `FrameDecoder::push`. **No acota la memoria del proceso** —eso lo
hace `MAX_BUFFER_LEN` del decodificador— y **no acota el tamaño de un frame**. Lo
único que gobierna es cuántas llamadas al sistema cuesta mover un byte.

Las cuatro razones del número:

1. **Es del orden del frame dominante.** El frame que domina esta conexión es un
   `DataChunk`: 48 de cabecera + 8 de cuerpo + 65 536 de contenido + 16 de tag =
   65 608 bytes. Con 64 KiB un chunk entra en unas dos lecturas; con 8 KiB, en nueve.
2. **Está a la altura de lo que el kernel suele tener encolado**, así que una lectura
   normalmente drena lo que hay en vez de dejar un resto que obliga a otra vuelta.
3. **Es el 6,25 % de `MAX_BUFFER_LEN`.** Una lectura completa encima de un
   decodificador casi lleno no puede colarse: `push` la rechaza con
   `BufferLimitExceeded` **dejando el búfer intacto**, que es el contrato documentado,
   y el llamante drena frames y reintenta.
4. **Ya es el número de la casa.** `CHUNK_SIZE` es 65 536 y `HASH_BUFFER_LEN` es
   65 536. Un número que razonar en vez de tres.

**Por qué no más grande** (por ejemplo, del tamaño del techo del decodificador): es
memoria residente por conexión y no compra nada, porque un `read` casi nunca devuelve
un megabyte y el decodificador se alimenta igual.

**Por qué no 8192**, que es el valor reflejo: nueve lecturas y nueve `push` por chunk
en vez de dos. Y `push` puede disparar una compactación, así que lecturas más
pequeñas hacen correr el calendario amortizado de memmove más veces para los mismos
bytes.

**Honestidad sobre este número: es razonado, no medido.** Nada en este sprint mide
rendimiento contra tamaño de búfer, y un socket de bucle invertido sería el sitio
equivocado para medirlo —no pierde paquetes, no reordena y no se parece al MTU de
ninguna red real—. Lo que aquí se fija es una cota y su argumento, no un óptimo. Lo
mismo que ADR-0026 dijo de `CHUNK_SIZE`, y por el mismo motivo.

**Durante el handshake este búfer no se usa entero.** Ver §3.

---

## 3. Los límites antes de autenticar

Un peer que abre 10 000 conexiones y no dice nada es la denegación de servicio más
barata que existe. Los tres números:

| Límite | Valor | Qué acota |
|---|---|---|
| `MAX_PREAUTH_BYTES` | **4096** por conexión | Bytes que el proceso acepta de un peer no autenticado |
| `HANDSHAKE_DEADLINE` | **10 s** por conexión | Tiempo total desde que la conexión existe hasta que la sesión está establecida |
| `MAX_PENDING_HANDSHAKES` | **8** simultáneas | Conexiones aceptadas y todavía no autenticadas |

Y uno más, que no es defensivo sino de política:

| `MAX_ESTABLISHED_SESSIONS` | **4** | Sesiones autenticadas vivas a la vez |

### 3.1 `MAX_PREAUTH_BYTES = 4096`, y cómo se impone de verdad

Un handshake legítimo necesita recibir 295 bytes (§1). 4096 es un orden de magnitud
de holgura, suficiente para una revisión futura del handshake sin que el número
cueste nada hoy.

Lo que hace este límite defendible no es el valor, es **dónde se comprueba**. La
forma ingenua —leer hasta 64 KiB y luego mirar si nos hemos pasado— no es un límite:
ya has aceptado los bytes cuando lo compruebas. En su lugar:

> **Mientras la sesión no esté establecida, ninguna lectura se emite con un búfer
> mayor que la asignación que queda.** Es decir `read(&mut buf[..remaining])`, con
> `remaining = MAX_PREAUTH_BYTES - recibidos_hasta_ahora`. Cuando `remaining` llega a
> cero sin sesión establecida, la conexión se cierra con `PreAuthByteLimitExceeded`.

Así el límite es una propiedad de lo que el proceso **puede** recibir, no de lo que
decide conservar después de haberlo recibido. Y es medible: el contador que lo
comprueba registra bytes que salieron de `read`, no la constante.

Consecuencia deliberada: **el búfer de 64 KiB de §2 no se asigna hasta que la sesión
está establecida.** Mientras el peer es un desconocido, el búfer de lectura mide
`MAX_PREAUTH_BYTES`.

**El decodificador es uno solo para toda la vida de la conexión**, construido con
`FrameDecoder::new()` y su techo por defecto. No se sustituye al autenticar, porque
los bytes del primer frame sellado pueden llegar en la misma lectura que el último
mensaje del handshake, y tirar el decodificador tiraría esos bytes. Su capacidad
crece sólo con lo que se le empuja, y antes de autenticar no se le empuja más de
4096, así que un desconocido no puede hacerle reservar más que eso.

Memoria total que un desconocido puede comprometer:
`MAX_PENDING_HANDSHAKES × (MAX_PREAUTH_BYTES + capacidad del decodificador)`,
con la capacidad acotada por la política de crecimiento de `reserve_for` a partir de
lo empujado. El orden es de decenas de kilobytes, no de megabytes. **El número exacto
se mide en la Fase 2 con un contador que registra `buffer_capacity()` observada, no
la constante del límite** — la trampa 2 de §11 del prompt es exactamente escribir
aquí un número y luego «comprobarlo» comparándolo consigo mismo.

### 3.2 `HANDSHAKE_DEADLINE = 10 s`, y por qué es total y no por mensaje

El handshake son dos vueltas y algo de criptografía: dos X25519, una firma Ed25519,
una verificación, HKDF y HMAC. Eso son milisegundos de un dígito en cualquier
dispositivo. Lo que domina es el RTT: menos de 5 ms en una LAN, y quizá cientos de
milisegundos en una Wi-Fi degradada con retransmisiones. 10 s es unas veinte veces el
caso degradado.

**El plazo es sobre el handshake entero, no sobre cada mensaje, y eso es lo
importante.** Un plazo por mensaje lo reinicia indefinidamente un peer que suelta un
byte justo antes de cada vencimiento: es el slowloris clásico, y un plazo por mensaje
no lo detiene. Un plazo total sí.

### 3.3 `MAX_PENDING_HANDSHAKES = 8` y `MAX_ESTABLISHED_SESSIONS = 4`

Son dos límites distintos y confundirlos es el error. El primero acota lo que un
**desconocido** puede crear; el segundo es una decisión de producto sobre cuántas
transferencias simultáneas tiene sentido aceptar.

8 pendientes porque Qyro es una aplicación uno a uno: el caso legítimo es una
conexión, y 8 deja sitio para reintentos y para que un usuario reciba de dos personas
a la vez sin que la novena conexión hostil llegue a asignar nada. Al llegar al
límite, el listener **acepta y cierra inmediatamente** en vez de dejar de aceptar:
dejar de aceptar llena la cola del kernel y castiga al siguiente peer legítimo, que
es justo al revés de lo que se quiere. El cierre inmediato produce
`TooManyPendingConnections` en el lado que marca.

---

## 4. Los timeouts, y cómo se distingue un peer lento de uno muerto

| Timeout | Valor | Qué significa que venza |
|---|---|---|
| `CONNECT_TIMEOUT` | **10 s** | La dirección no responde. `ConnectTimedOut` |
| `READ_TIMEOUT` | **250 ms** | **Nada. Es el latido del bucle.** No es un error |
| `IDLE_TIMEOUT` | **60 s** sin un solo byte | El peer está muerto. `PeerSilent` |
| Timeout de escritura | **ninguno, deliberadamente** | Ver §4.3 |

### 4.1 El `READ_TIMEOUT` de 250 ms no es un plazo de vida

Es el intervalo con el que un hilo parado en `read` se despierta para mirar si le han
pedido cancelar. Un `WouldBlock` (Linux) o un `TimedOut` (Windows) producido por él
**no es un final y no es un error**: es la condición normal de una conexión que
espera. Mapearlo a un error tipado mataría toda transferencia con una pausa de más de
un cuarto de segundo.

250 ms porque acota cuánto tarda una cancelación en notarse, y está por debajo de lo
que una persona percibe como «el botón no ha hecho nada». Cuesta cuatro despertares
por segundo y por conexión, que no es nada.

### 4.2 Lento contra muerto: la diferencia es el progreso, no la velocidad

Éste es el punto que un transporte hecho a la ligera se equivoca, y se equivoca
siempre igual: pone un plazo al total de la transferencia. Con eso, un archivo de
4 GiB por una Wi-Fi lenta se convierte en una conexión colgada.

**Decisión: la transferencia no tiene ningún plazo total. Ninguno.** Un archivo de
4 GiB a 100 KiB/s tarda once horas y media y eso está permitido. Lo que no está
permitido es **60 segundos sin que llegue un solo byte**.

El plazo se mide sobre «tiempo desde el último byte recibido» y lo reinicia
*cualquier* byte, no un frame completo. Un peer lento entrega bytes continuamente,
sólo que pocos; un peer muerto no entrega ninguno, nunca. Esa es la diferencia
observable, y es la única que un transporte puede medir sin inventarse una teoría
sobre la red.

60 s porque tiene que ser cómodamente mayor que la peor pausa legítima —una Wi-Fi
que se reasocia, un móvil que cambia de celda, un disco que se atraganta escribiendo
un chunk de 64 KiB— y cómodamente menor que la paciencia de una persona delante de
una barra de progreso parada.

**Lo que este plazo NO cubre, y queda escrito:** un peer **ya autenticado** que
entregue un byte cada 59 segundos para siempre mantiene la conexión viva
indefinidamente. Es un slowloris de la fase de datos. No se cierra en este sprint, y
la razón por la que se acepta es que ese peer no es un desconocido: es un dispositivo
cuya identidad se verificó con una firma Ed25519, la memoria que puede comprometer
está acotada por la ventana (1 MiB por dirección, ADR-0026 §6), y lo que desperdicia
es una conexión y dos hilos que el usuario decidió conceder. Un límite de caudal
mínimo sería la respuesta, y necesita una medición que este sprint no tiene.

### 4.3 No hay timeout de escritura, y ése es el diseño

`set_write_timeout` parece la simetría obvia y es una trampa. Un `write` con plazo
que vence **a mitad de un frame** deja el frame escrito por la mitad, y de eso no se
vuelve: el decodificador del peer tiene medio frame y ADR-0018 prohíbe expresamente
resincronizar, porque resincronizar es adivinar. Un timeout de escritura convierte
una congestión pasajera en una conexión permanentemente desincronizada.

**Decisión: `set_write_timeout(None)`. Las escrituras son bloqueantes y completas.**
Lo que desbloquea a un escritor atascado es `shutdown`, no un plazo.

El caso que preocupa —el peer dejó de leer— sí está cubierto, por otro camino: si el
peer no lee, tampoco escribe, así que el `IDLE_TIMEOUT` del **hilo lector** vence a
los 60 s, y el lector llama a `shutdown`, y eso devuelve al escritor con error. Hay
**una sola autoridad sobre la vitalidad de la conexión —el hilo lector— y un solo
mecanismo para desbloquear al otro hilo**. Dos autoridades sobre lo mismo es como se
consiguen dos diagnósticos distintos del mismo silencio.

---

## 5. La taxonomía de finales

Cinco formas de acabar, más las que el transporte descubre por su cuenta. Cada una
con su error tipado. **Ninguno se llama `Io` a secas**; el genérico existe pero lleva
la operación que fallaba.

| # | Final | Cómo se observa | Tipo | ¿Envenena? |
|---|---|---|---|---|
| 1 | Terminó bien | Ambos extremos en `Phase::Done` con veredicto de integridad | `Ending::Completed` | No |
| 2 | El emisor canceló | `Cancel` recibido o enviado; el motor pasa a `Cancelled` | `Ending::CancelledBySender` | No |
| 3 | El receptor rechazó | `TransferReject`, o `IntegrityResult` con veredicto negativo | `Ending::RefusedByReceiver` | No |
| 4a | Cierre ordenado prematuro | `read` devuelve `Ok(0)` en frontera de frame, antes de un final | `NetError::PeerClosedEarly` | No |
| 4b | Cierre ordenado a mitad de frame | `read` devuelve `Ok(0)` con un frame incompleto en el decodificador | `NetError::PeerClosedMidFrame { buffered }` | No |
| 5 | El peer se esfumó | `ECONNRESET` / `ECONNABORTED` / `EPIPE` | `NetError::PeerVanished { kind }` | No |
| 6 | Silencio | 60 s sin un byte (§4.2) | `NetError::PeerSilent { idle }` | No |
| 7 | Los bytes mintieron | El decodificador se envenena, o el AEAD falla | `NetError::Framing(..)` / `NetError::NotAuthenticated` | **Sí** |

Más los rechazos previos a autenticar, que también son finales:
`PreAuthByteLimitExceeded`, `HandshakeDeadlineExceeded`, `TooManyPendingConnections`,
`ConnectTimedOut`, y `NetError::Handshake(..)` para una firma que no verifica.

### 5.1 Lo que TCP no puede distinguir, dicho en voz alta

El prompt pide separar «la conexión se cortó a mitad» de «el proceso remoto murió».
**A nivel de TCP eso no se puede distinguir de forma fiable, y esta ADR no va a
fingir que sí.** Lo que se observa es:

- **FIN** → `read` devuelve `Ok(0)`. Es un cierre *ordenado*: alguien llamó a `close`
  o a `shutdown`. Un proceso que muere limpiamente también produce esto, porque el
  kernel cierra sus descriptores.
- **RST** → `ECONNRESET`. El socket del otro lado dejó de existir sin cierre
  ordenado. Es lo típico de un proceso matado con datos sin leer en la cola, pero
  también lo produce un firewall, un NAT que expiró la entrada, o un cable.
- **Nada** → ni FIN ni RST, sólo silencio. Es lo típico de una máquina que se
  suspendió, un Wi-Fi que se cayó o un cable tirado, porque nadie queda al otro lado
  para mandar el RST.

Por eso los nombres de los tipos 4b, 5 y 6 **describen lo observado, no la causa
inferida**: `PeerClosedMidFrame`, `PeerVanished`, `PeerSilent`. Un tipo llamado
`RemoteProcessKilled` sería una mentira: el transporte no sabe eso. La Fase 5 mata un
proceso de verdad con `Child::kill()` y comprueba que el superviviente produce **uno
de** {`PeerVanished`, `PeerClosedMidFrame`}, no un pánico y no un cuelgue; cuál de
los dos depende de si había datos sin leer en la cola, que es una condición de
carrera del kernel y no una propiedad de Qyro. **Fijar cuál de los dos sería una cota
extrapolada de una muestra de uno** — la trampa 4 de §11 del prompt.

### 5.2 La regla del envenenamiento: envenena lo que miente, no lo que se acaba

Sólo la fila 7 deja la sesión en `Poisoned`. El principio:

> **Se envenena cuando llegaron bytes y eran falsos. No se envenena cuando los bytes
> dejaron de llegar.**

Un frame estructuralmente inválido o un tag que no verifica significa que lo recibido
no es lo que dice ser, y a partir de ahí nada de esa conexión es interpretable: el
decodificador se envenena (ADR-0018), el motor se envenena (ADR-0026), y `reset()`
no se llama nunca porque no hay nada que reanudar. En cambio un cierre, un reset o un
silencio no dicen nada falso: dicen que se acabó. El motor se queda en la fase en la
que estaba y el transporte informa del final. Confundir las dos cosas convierte «se
fue la Wi-Fi» en «alguien te está atacando», y borra la señal que sí importa.

---

## 6. El modelo de hilos

**Dos hilos por conexión.** Ni uno ni tres.

| Hilo | Posee | Hace |
|---|---|---|
| **Lector** | Un `TcpStream` clonado, el `FrameDecoder`, el motor (`Sender`/`Receiver`), el plazo de inactividad | Única cosa que llama a `read`. Decodifica, entrega al motor, bombea, y encola lo que salga |
| **Escritor** | Otro `TcpStream` clonado, el extremo receptor de un `mpsc` | Única cosa que llama a `write`. Drena el canal y escribe frames completos |

El reparto del socket sale de `TcpStream::try_clone()`, que da dos manejadores
independientes al mismo socket. Un tercer clon se queda en el manejador de la
conexión, para poder llamar a `shutdown` desde fuera.

**Por qué el escritor tiene que ser un hilo aparte.** Si el mismo hilo leyera y
escribiera, una escritura bloqueada —el peer dejó de leer— impediría leer los ACK. Y
con go-back-N y ventana 16, el emisor no puede avanzar sin ACK. Eso es un
interbloqueo, y es el clásico de este tipo de protocolos: los dos extremos escribiendo
en un socket cuya ventana de recepción está llena, ninguno leyendo. Separar el
escritor lo hace imposible.

**Por qué el motor vive en el hilo lector y no en un tercero.** El motor sólo avanza
por dos causas: llegan bytes, o hay que bombear. Los bytes los ve el lector, y el
bombeo es útil justo cuando llegan ACK, que también los ve el lector. Un tercer hilo
sólo añadiría un `Mutex` alrededor de una máquina de estados que nadie más toca. El
bucle es:

```
leer (vence a los 250 ms como caso normal)
  → push al decodificador
  → next_frame en bucle hasta Ok(None)
  → engine.deliver(...) por cada frame
  → engine.pump(...)
  → encolar lo que salga hacia el escritor
```

Como el `read` vence cada 250 ms, **el bombeo ocurre al menos cuatro veces por
segundo aunque no llegue nada**, que es lo que arranca la primera ventana sin esperar
a un ACK que todavía no puede existir.

### 6.1 Cancelación: bandera **y** shutdown, con trabajos distintos

Las dos, y no es indecisión:

- **`AtomicBool`** (`cancel_requested`), que el lector consulta en cada despertar de
  250 ms. Es el camino **cooperativo**: el lector se entera, le pide al motor
  `request_cancel()`, el frame `Cancel` sale por el escritor, y el peer se entera de
  que ha sido una cancelación y no una caída. Es lo que hace que el final 2 sea
  distinguible del final 5.
- **`shutdown(Shutdown::Both)`**, que se llama sobre el clon del manejador. Es el
  camino **forzoso**: desbloquea de inmediato al lector y, sobre todo, es lo único
  que devuelve a un escritor parado dentro de `write`.

Se necesitan las dos porque **una bandera no despierta una llamada al sistema
bloqueada, y un `shutdown` no manda un frame `Cancel`**. Con sólo la bandera, un
escritor atascado no sale nunca. Con sólo `shutdown`, el peer ve una conexión rota y
no puede distinguirla de un fallo. El orden es: bandera primero (para que salga el
`Cancel`), `shutdown` después de un margen o si la bandera no basta.

### 6.2 Que no quede ningún hilo vivo

El manejador de la conexión posee los dos `JoinHandle` y hace `join` de los dos al
cerrarse. Ésa es la propiedad que la Fase 5 cuenta, en las cinco formas de acabar. Un
hilo que se queda esperando en `read` sin que nadie tenga su `JoinHandle` es una fuga
que ninguna prueba de camino feliz ve.

---

## 7. Quién escucha, quién marca, y de dónde sale el puerto

**Decisión: el que recibe escucha; el que envía marca.** Y la asignación es la misma
en las dos capas:

| Rol de producto | Rol TCP | Rol del handshake (ADR-0021) |
|---|---|---|
| Recibe (`serve`) | `TcpListener`, acepta | **Responder** |
| Envía (`send`) | `TcpStream::connect`, marca | **Initiator** |

Una sola asignación de roles para las dos capas, así que no existe la combinación
«cliente TCP pero responder del handshake» y nadie tiene que razonar sobre ella. El
motivo de producto es que el dispositivo que recibe es el que tiene que estar
disponible, y el que envía es el que acaba de ejecutar una acción del usuario.

**El puerto lo elige el llamante.** No hay puerto por defecto en este sprint, y es
deliberado: un puerto por defecto es una decisión de descubrimiento, y el
descubrimiento es 6B. Poner uno ahora sería congelar una decisión de otro sprint sin
sus razones.

**El puerto 0 significa «que lo elija el sistema»**, y el listener informa del que le
tocó vía `local_addr()`. Esto no es una comodidad: es lo que hace que las pruebas de
red no sean intermitentes. Un puerto fijo en una prueba falla en cuanto dos pruebas
corren a la vez, o en cuanto el runner tiene ese puerto ocupado, y la reacción típica
a esa intermitencia es añadir un `sleep`, que esconde el problema en vez de
quitarlo.

**La dirección de escucha también la elige el llamante.** Escuchar en `0.0.0.0` es una
decisión del que llama, no de la biblioteca. Las pruebas usan `127.0.0.1`.

`set_nodelay(true)` en los dos extremos, obligatorio. Con go-back-N, Nagle interactúa
con el ACK retardado de TCP y produce pausas de cientos de milisegundos que parecen
un problema del motor y no lo son.

---

## 8. Lo que esta ADR no promete

Sección obligatoria, y la más importante para quien lea esto dentro de seis meses.

- **No hay descubrimiento.** Ni mDNS, ni Bonjour, ni `NsdManager`, ni broadcast. La
  dirección del peer se la pasa el llamante como `IpAddr:puerto`. Sin eso, no hay
  conexión. Es el sprint 6B.
- **No hay NAT, ni internet, ni relay.** Los dos extremos tienen que poder alcanzarse
  por IP tal cual. No hay STUN, no hay agujereado de UDP, no hay servidor intermedio.
  Fuera de una misma red local, esto no conecta.
- **No hay reconexión automática.** Una conexión que termina, termina. No hay
  reintento, ni backoff, ni reanudación de una transferencia interrumpida sobre una
  conexión nueva. La reanudación de `qyro_fs` es sobre disco, no sobre red, y nadie
  la conecta a este transporte en este sprint.
- **No está probado en hardware físico.** Ni en dos máquinas distintas. Todo lo que
  este sprint demuestre ocurre sobre `127.0.0.1`, y una interfaz de bucle invertido
  no pierde paquetes, no reordena, no fragmenta, tiene un MTU que no se parece al de
  ninguna red real y un RTT de microsegundos. **Dos procesos en 127.0.0.1 no son dos
  dispositivos en una Wi-Fi.** El go-back-N y el control de flujo se ejercitan de
  verdad en cuanto a lógica, pero no se ejercitan contra pérdida real, porque en
  bucle invertido no hay pérdida real.
- **No está probado en Windows.** Concretamente, y esto es una lista de riesgos
  conocidos, no una lista vacía por optimismo:
  - Un `read` que vence por `SO_RCVTIMEO` devuelve `WouldBlock` en Linux y `TimedOut`
    en Windows. **Las dos son el latido de §4.1 y ninguna es un error.** Tratar una de
    las dos como final rompería toda transferencia en una de las dos plataformas, y es
    el tipo de fallo que sólo aparece en la plataforma que no se probó.
  - El recuento de descriptores de la Fase 5 usa `/proc/self/fd`, que en Windows no
    existe. Esa prueba será `#[cfg(target_os = "linux")]` y **la propiedad queda sin
    comprobar en Windows**, no comprobada por otro medio.
  - El comportamiento exacto de `shutdown` sobre un `read` bloqueado en otro hilo no
    está garantizado igual en las dos plataformas.
  - Los códigos de error que se mapean a `PeerVanished` (`ECONNRESET`,
    `ECONNABORTED`, `EPIPE`) tienen equivalentes en Windows que `std` normaliza a los
    mismos `io::ErrorKind`, pero eso no se ha ejecutado.
- **No hay IPv6 más allá de que funcione si el `IpAddr` es v6.** No se ha probado, no
  hay manejo de scope ids de enlace local, y no hay preferencia entre familias.
- **No hay TLS y no lo habrá.** Qyro tiene su propio handshake autenticado; poner TLS
  encima sería cifrar dos veces y traer una dependencia enorme.
- **No hay límite de caudal mínimo**, así que un peer ya autenticado puede sostener
  una conexión indefinidamente entregando un byte cada 59 segundos (§4.2).
- **No hay medición de rendimiento.** El tamaño del búfer de lectura es un argumento,
  no un óptimo (§2), y nada en este sprint mide throughput, latencia ni uso de CPU.
- **No hay control de congestión propio.** Lo que haya es lo que haga TCP. La ventana
  de 16 chunks de ADR-0026 es una cota de memoria, no un algoritmo de congestión, y no
  reacciona a la pérdida.
- **No hay multiplexación.** Una conexión lleva una sesión y una transferencia. Los
  campos `session_id`, `transfer_id` y `stream_id` de la cabecera existen, pero este
  transporte no los usa para encaminar nada.
- **Esto no habilita los botones.** `qyro_ffi` no cambia, la UI no cambia, y Qyro
  sigue sin transferir archivos para un usuario. Lo que este sprint persigue es que
  dos *procesos de prueba* se pasen un archivo.

---

## Alternativas descartadas

- **Un prefijo de longitud propio del transporte.** Es la forma más común de framing
  sobre TCP y aquí es un error: la longitud ya existe dentro del AEAD, y un segundo
  campo fuera del tag es un campo que un atacante activo cambia sin romper nada. §1.
- **Un delimitador o secuencia de sincronización entre frames.** El payload sellado es
  indistinguible de aleatorio, así que puede contener cualquier secuencia de bytes.
  Haría falta escapado, y escapar cambia los bytes que el AEAD autenticó.
- **Un runtime async.** Decidido fuera de esta ADR. Con una conexión no compra nada y
  obliga a reescribir un motor que ya es una máquina de estados síncrona.
- **Un hilo por conexión en vez de dos.** Interbloqueo garantizado en cuanto el peer
  deje de leer: la escritura se bloquea y con ella la lectura de los ACK que la
  desbloquearían. §6.
- **Tres hilos, con el motor en el suyo.** Añade un `Mutex` alrededor de una máquina
  de estados que sólo el lector toca, a cambio de nada.
- **Un plazo total de transferencia.** Convierte un archivo grande por una red lenta
  en una conexión colgada. La propiedad correcta es el progreso, no el tiempo total.
  §4.2.
- **`set_write_timeout` con un valor.** Deja frames escritos a medias, que es un
  estado del que ADR-0018 dice explícitamente que no se sale. §4.3.
- **Un plazo de handshake por mensaje.** Lo reinicia un peer que suelta un byte antes
  de cada vencimiento. §3.2.
- **Dejar de aceptar conexiones al llegar al límite de pendientes.** Llena la cola del
  kernel y castiga al siguiente peer legítimo. Se acepta y se cierra. §3.3.
- **Un tipo `RemoteProcessKilled`.** El transporte no puede saber eso. Los tipos
  nombran lo observado. §5.1.
- **Un puerto por defecto.** Es una decisión de descubrimiento y el descubrimiento es
  6B. §7.
- **Sustituir el decodificador al pasar de handshake a sesión**, para bajarle el techo
  mientras el peer es desconocido. Tira los bytes que hayan llegado adelantados en la
  misma lectura. El límite se impone en el `read`, que es más simple y no pierde
  nada. §3.1.

## No objetivos

Descubrimiento, FFI, UI, selector de archivos, reconexión, NAT, internet, TLS,
Android Keystore, iOS Keychain, historial, emparejamiento, release. Y los tres
identificadores de cabecera de QYR-0068, que son de otro agente en este run: si este
transporte los necesitara, eso sería un hallazgo del informe, no una excusa para
tocar `header.rs`.

---

## Enmienda 1 (2026-08-31, fase 28) — la peor pausa legítima no era una Wi-Fi que se reasocia: era una persona

La §4.2 elige 60 s por dos razones escritas, y las dos son sobre la red o sobre
la impaciencia: «cómodamente mayor que la peor pausa legítima —una Wi-Fi que se
reasocia, un móvil que cambia de celda, un disco que se atraganta— y cómodamente
menor que la paciencia de una persona delante de **una barra de progreso
parada**».

**Falta una pausa legítima en esa lista, y es la más larga de todas: la de una
persona a la que se le acaba de preguntar algo.** No está delante de una barra
parada; está leyendo un nombre de archivo, mirando el otro aparato, comparando
una huella. Y mientras lo hace **no cruza un solo byte**, porque
`MessageType::Heartbeat` existe en el formato desde el principio y **nadie lo
emite**.

### Medido

Sesenta y cinco segundos de espera humana, sobre el camino de producción entero,
con dos sesiones y un socket de verdad:

| | Emisor | Receptor | Materializado |
|---|---|---|---|
| Antes | `Err(PeerUnreachable)` a los **60,11 s** | `Err(PeerUnreachable)` | nada |
| Después | `Ok(Completed)` a los **65,76 s** | `Ok(Completed)` | 1 archivo |

Y el mensaje que veía la persona era **«el otro aparato no responde»**: una
acusación falsa contra una red que funcionaba, en el momento exacto en que ella
acababa de contestar.

La traza dice dónde estaba cada lado durante esos 65 s: el emisor dio **227
pasos en `Transferring` sin producir un solo frame**. Ya lo había mandado todo y
esperaba acuses que no llegaban porque al otro lado nadie estaba leyendo.

### Decisión: el reloj mide **el silencio del otro**, no la espera de éste

Dos reglas, porque los dos lados se callan por razones distintas. **Ninguna de
las dos sube `IDLE_TIMEOUT`**: ese número sigue siendo 60 s y sigue significando
lo mismo mientras el contenido se mueve.

1. **Un lado que no tiene nada que poner en el cable no está midiendo la red.**
   Un emisor sin frames que producir —ventana llena, o todo mandado ya— espera
   al otro extremo, y el otro extremo puede ser una persona o un SHA-256 sobre
   cuatro gigas, que también pasa de sesenta segundos sin que nada vaya mal. En
   ese estado el plazo es `DECISION_DEADLINE`, **diez minutos**. En cuanto vuelve
   a haber algo que mandar, vuelve a ser sesenta.

2. **El tiempo en que este lado no estuvo escuchando no cuenta.** Cuando el
   consumidor deja de dar pasos —el receptor sale de `await_offer` y pregunta a
   una persona— nadie estaba escuchando, así que ese silencio no es prueba de
   nada, y contarlo es culpar al par de una pausa propia. Al volver, la ventana
   **se reinicia**; no se alarga. Un par de verdad muerto se descubre igual,
   sesenta segundos después de que este lado vuelva a escuchar.

### Lo que esto cuesta, dicho

Un emisor cuyo par desaparece **por un agujero negro de red** —sin RST, sin
FIN— mientras no le queda nada que mandar tarda ahora diez minutos en decirlo, y
antes tardaba uno. Se acepta: es el lado donde hay una persona mirando y un
Ctrl-C a mano, y el caso contrario —matar una transferencia sana porque alguien
tardó en contestar— le pasa a todo el mundo, no a un agujero negro.

### Lo que no arregla

**Sigue sin haber latido.** `MessageType::Heartbeat` sigue sin emisor, y esto es
una política de plazos, no un mensaje en el cable. Si algún día una pausa humana
puede durar más de diez minutos —una notificación que el sistema no entrega, un
teléfono que se bloquea— la respuesta correcta seguirá siendo un latido y no un
número más grande.
