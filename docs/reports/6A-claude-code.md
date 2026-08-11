# Sprint 6A — `qyro_net` — informe del agente Claude Code

**Rama:** `claude/qyro-net-6a`
**Rama base:** `claude/qyro-filesystem-5b1` @ `15934aae3dda7f469b5496c8341eb78d9e32f335`
**Agente:** Claude Code (`claude-opus-5`)
**Agente en paralelo:** Codex, en `codex/qyro-gap-closure-5c`. Reglas de no interferencia en §5 del prompt.
**Estado del informe:** en curso. Se escribe puerta a puerta, no al final.

> **Cómo leer este documento.** Cada sección se rellena conforme avanza el sprint. Una
> sección que dice «pendiente» está pendiente de verdad: no es un hueco de redacción
> que se completará con lo que salga, es trabajo que todavía no se ha hecho. Ninguna
> afirmación de este informe debe leerse sin su clase de evidencia (§8).

### Sobre los identificadores de hallazgo de este informe

Los hallazgos van numerados **`6A-n`**, no `QYR-00xx`. No es un descuido: es la
consecuencia directa de §5 de este sprint, y está explicado en el hallazgo 6A-1.
En corto: §5 prohíbe a los dos agentes tocar el ledger, y
`check_docs_consistency` bloquea cualquier identificador citado que no tenga entrada
en él. Acuñar aquí un número que no puedo registrar produciría exactamente el fallo
que ese control existe para detectar. **El supervisor asigna el número QYR-00xx de
cada hallazgo al consolidar**, y esa asignación es trivial porque cada uno lleva
abajo su descripción completa, su efecto y su estado.

---

## 1. El prompt recibido, verbatim y completo

**El prompt está reproducido verbatim, íntegro y sin una sola modificación en
[`docs/reports/6A-prompt.txt`](6A-prompt.txt)** (223 líneas, SHA-256
`c88b4bc54ae04de3ad12c55bf8b9df26aed77fe860916409f003929cbb368385`). El formato de
origen no traía marcas de código; se conserva el texto plano tal cual llegó.

Vive en un archivo hermano y no incrustado aquí por un motivo concreto que conviene
dejar escrito, porque de otro modo parecería una comodidad de maquetación y no lo
es:

El prompt cita, en su §9, los identificadores de los tres huecos de prueba de
`qyro_fs` que asigna al otro agente; el primero de los tres es el que importa aquí, y
este informe no lo escribe literalmente por el motivo que sigue.
`check_docs_consistency`, que `ci.yml` ejecuta en Bash y en PowerShell, bloquea
**cualquier** identificador `QYR-00xx` citado en un `.md` que no tenga su entrada en
`BUGS_PENDING.md`. Esa entrada le corresponde a Codex y aparecerá en su rama. Y §5
prohíbe a los dos agentes tocar
`BUGS_PENDING.md` en este run. Las tres reglas no pueden cumplirse a la vez mientras
el prompt esté dentro de un `.md`; es el hallazgo 6A-1.

De las cuatro salidas posibles, ésta es la única que **no debilita ningún control, no
toca ningún archivo ajeno y no deja la rama en rojo**. Las otras tres se descartaron
así:

| Salida descartada | Por qué |
|---|---|
| Dejar `ci.yml` en rojo hasta que el supervisor fusione | Incumple el criterio de §10 de seis workflows en verde, y §15 lo prohíbe expresamente |
| Escribir yo las entradas en `BUGS_PENDING.md` | Toca un archivo prohibido por §5 y provoca justo el conflicto de fusión que §5 existe para evitar: Codex está escribiendo esas mismas entradas |
| Eximir `docs/reports/` en `check_docs_consistency` | Debilita un control vivo para que pase mi propio artefacto. Y los informes son precisamente donde hay que cazar un hallazgo citado sin ficha |

Lo que queda es una distinción que además es correcta de por sí: **un documento
externo archivado verbatim no es el repositorio citando un hallazgo, es el
repositorio guardando el texto de otro.** El archivo hermano hace esa distinción
explícita en el sistema de archivos en vez de esconderla. El contenido no se ha
tocado: es byte a byte lo que se recibió, y el SHA-256 de arriba lo fija.

---

## 2. Qué hice, punto por punto contra §8

| Fase | Objetivo de §8 | Estado |
|---|---|---|
| 0 | Línea base reproducida por mí, no heredada | **Hecha.** Puerta 0 pasada, §9 |
| 1 | ADR-0028 congelada antes de una sola línea de código | **Hecha.** Puerta 1, §9. Commiteada en `db3ce79`, antes de que existiera ningún `.rs` |
| 2 | `qyro_net`: listener, dialer, `FrameStream`, errores tipados, seis pruebas sobre sockets reales | **Hecha.** Puerta 2, §9. Once pruebas, no seis |
| 3 | El handshake de cuatro mensajes sobre socket real, cuatro pruebas | **Hecha.** Puerta 3, §9. Seis pruebas, no cuatro |
| 4 | Dos procesos de sistema operativo, ≥8 MiB byte a byte, tres pruebas, diez ejecuciones seguidas | Pendiente |
| 5 | Los cinco finales provocados de verdad, más hilos y descriptores | Pendiente |
| 6 | Guardas, barrido de mutación completo, informe, seis workflows en verde | Pendiente |

### Fase 0 — línea base verificada por mí

El prompt (§3) advierte explícitamente de que la evidencia declarada por el sprint
5B.1 no pudo verificarla el supervisor, porque la API de GitHub le devolvió 403, y
pide no dar por buena la tabla ajena. Por eso la línea base de abajo no está copiada
de `STATUS.md` ni del informe de 5B.1: sale de comandos ejecutados en esta sesión
sobre `15934aae3dda7f469b5496c8341eb78d9e32f335`, y cada fila anota el código de
salida real del proceso, no la ausencia de texto en su salida.

Verificación previa exigida por §3:

```
$ git rev-parse origin/claude/qyro-filesystem-5b1
15934aae3dda7f469b5496c8341eb78d9e32f335
```

Coincide con el valor que el prompt exige, así que no hay motivo de parada.

### La línea base, medida el 2026-08-11T18:01:59Z sobre `15934aa`

| Comprobación | Declarado en §4 del prompt | Medido por mí | Cómo |
|---|---|---|---|
| `cargo fmt --all --check` | limpio | **exit 0** | código de salida del proceso |
| `cargo clippy --workspace --all-targets -- -D warnings` | limpio | **exit 0**, 0 líneas `^warning`/`^error` | código de salida + recuento |
| `cargo test --workspace` | 388 tests, 0 failed, 2 ignored | **passed=388 failed=0 ignored=2**, exit 0 | suma de todas las líneas `test result:` |
| Paquetes en `Cargo.lock` | 61 | **61** | `grep -c '^\[\[package\]\]'` |
| `check_repo_portability.sh` | verde | **exit 0** — `[OK]` | ejecutado |
| `check_harness_isolation.sh` | verde | **exit 0** — `[OK]` | ejecutado |
| `check_crypto_platform_evidence.sh` | verde | **exit 0** — `[OK]` | ejecutado |
| `check_docs_consistency.sh` | verde | **exit 0** — `[OK]` | ejecutado |

**Los cuatro números coinciden con los declarados. No hay nada que registrar como
divergencia.** Esto verifica la línea base *local*; no verifica los seis runs de CI
que 5B.1 declara sobre `e3fbaf1`, que siguen sin comprobar por parte del supervisor
y que yo tampoco he comprobado. Ver §14.

#### Un falso verde que me hice a mí mismo, y por qué está aquí

El primer intento de reproducir la línea base fue este:

```
cargo fmt --all --check --manifest-path rust/Cargo.toml 2>&1 | tail -5 && echo "FMT_PASS"
```

Imprimió `FMT_PASS`. No significaba nada. Dos defectos a la vez: `rust/Cargo.toml`
no existe —el workspace está en la raíz del repositorio, no bajo `rust/`—, y el
`&&` estaba leyendo el código de salida de `tail`, que vale 0 siempre. Es decir: el
comando fallaba y el mensaje decía que había pasado.

Queda escrito porque es exactamente la trampa 2 de §11 con otro disfraz —una
comprobación que informa de una constante en vez de lo medido— y porque un informe
que sólo enseña el intento que salió bien es un resumen favorable. La medición de
la tabla de arriba se rehízo capturando `$?` de cada proceso directamente, que es
el motivo de que la columna «Cómo» diga «código de salida» y no «no imprimió nada».

### La firma por la que el decodificador incremental recibe bytes

Segunda mitad de la Puerta 0. La costura es `qyro_protocol::FrameDecoder`, y las dos
firmas que la definen son:

```rust
pub fn push(&mut self, bytes: &[u8]) -> Result<(), FrameError>
pub fn next_frame(&mut self) -> Result<Option<DecodedFrame>, FrameError>
```

Lo que hay que saber de ellas para no romper nada en la Fase 2:

- **`push` toma un préstamo, no propiedad.** `&[u8]`, no `Vec<u8>`. Un `read` de TCP
  puede entregar su búfer prestado sin copiarlo a un vector intermedio.
- **`push` no reserva sobre una longitud declarada por el peer.** Reserva sobre
  `bytes.len()`, lo que de verdad llegó, y `reserve_for` acota el crecimiento a
  `max_buffer_len`. La longitud que declara el peer sólo se lee en `next_frame`, y
  allí se compara con el techo y se **rechaza**, no se reserva. Esto es lo que hace
  que la prueba `a_peer_cannot_make_us_buffer_more_than_the_declared_limit` de la
  Fase 2 sea comprobable: hay un número que medir que no es la constante del límite.
- **`Ok(None)` de `next_frame` significa «faltan bytes», no «fin de stream».** Un
  transporte que lo confunda con EOF corta conexiones sanas.
- **El envenenamiento es pegajoso y guarda el error, no un booleano.** El campo es
  `poisoned: Option<FrameError>`; tras envenenarse, *tanto* `push` *como*
  `next_frame` devuelven el mismo error indefinidamente. Sólo `reset()` lo limpia, y
  `reset()` además tira el búfer entero: no es «reanudar aquí».
- **Un tipo de mensaje desconocido no envenena.** Se consume entero y sale como
  `DecodedFrame::Unsupported`, con el stream sincronizado (ADR-0018). El transporte
  debe propagarlo como valor; mapearlo a «cierra el socket» reintroduce el defecto
  que ADR-0018 se escribió para arreglar.
- **El techo por defecto es `MAX_BUFFER_LEN = MAX_FRAME_LEN = 1 049 664` bytes**
  (`MAX_HEADER_LEN` 1024 + `MAX_PAYLOAD_LEN` 1 048 576 + `MAX_TRAILER_LEN` 64).
  `with_max_buffer_len` sólo puede **bajarlo**: pedir más se recorta en silencio a
  `MAX_BUFFER_LEN`, así que un llamante no puede ensanchar la cota del protocolo.

`FrameDecoder` deriva `Debug` y nada más — **no es `Clone`** — y no implementa
`Iterator`. El bucle de lectura es `push` y luego `next_frame` en bucle hasta
`Ok(None)`.

---

## 3. Cómo lo hice: las decisiones de ADR-0028 y las alternativas descartadas

`docs/adr/ADR-0028-network-transport.md`, congelada antes de la primera línea de
código y commiteada antes del primer commit de código —comprobable en el historial,
ver §9 Puerta 1—. Ocho decisiones; el documento lleva el razonamiento completo, aquí
va el resumen con el motivo y la alternativa que se descartó.

| # | Decisión | Valor | Alternativa descartada, y por qué |
|---|---|---|---|
| 1 | Framing sobre el stream | El frame va **tal cual**, sin prefijo de longitud propio | Un prefijo de longitud, que es lo habitual sobre TCP. La longitud ya vive en la cabecera de 48 B **y la cabecera es el AAD del AEAD**: un segundo campo fuera del tag es un campo que un atacante activo altera sin romper nada, y decide cuánto reserva y espera el receptor. También un delimitador: el payload sellado es indistinguible de aleatorio, haría falta escapado, y escapar cambia los bytes que el AEAD autenticó |
| 1b | Los cuatro mensajes del handshake | Dentro de frames planos `Hello` | En crudo sobre el socket, con cambio de modo al acabar. El cambio de modo cae justo donde el peer aún no está autenticado, que es donde viven los fallos de desincronización. Además así **no hace falta tocar `message.rs` ni `header.rs`**, que son del otro agente |
| 2 | Búfer de lectura | **65 536 B** | 8192, el valor reflejo: convierte un chunk en nueve lecturas y nueve `push`, y `push` puede compactar. Y el techo del decodificador: memoria residente por conexión que no compra nada. 64 KiB es del orden del frame dominante (un `DataChunk` sellado son 65 608 B) y ya es el número de la casa (`CHUNK_SIZE`, `HASH_BUFFER_LEN`) |
| 3 | Bytes antes de autenticar | **4096** por conexión | «Leer 64 KiB y luego comprobar», que no es un límite: ya aceptaste los bytes cuando lo compruebas. Se impone en el `read`, emitiendo la lectura con `buf[..remaining]`, así que es una propiedad de lo que el proceso *puede* recibir. Un handshake legítimo recibe 295 B |
| 3b | Plazo de handshake | **10 s**, sobre el handshake **entero** | Un plazo por mensaje, que lo reinicia indefinidamente un peer que suelta un byte antes de cada vencimiento — el slowloris clásico |
| 3c | Conexiones simultáneas | **8** sin autenticar, **4** establecidas | Un solo límite para las dos cosas. Son surfaces distintas: el primero acota lo que un desconocido crea, el segundo es política de producto. Al llegar al tope se **acepta y se cierra**, no se deja de aceptar: dejar de aceptar llena la cola del kernel y castiga al siguiente peer legítimo |
| 4 | Timeouts | Conexión 10 s · lectura **250 ms** · inactividad **60 s** · escritura **ninguno** | Un plazo total de transferencia, que convierte un archivo de 4 GiB por Wi-Fi lenta en una conexión colgada. La propiedad correcta es el **progreso**, no el tiempo total: el plazo cuenta «tiempo desde el último byte» y lo reinicia cualquier byte. Y `set_write_timeout` con valor, que deja frames escritos a medias — un estado del que ADR-0018 dice que no se sale, porque resincronizar es adivinar |
| 5 | Taxonomía de finales | Siete filas tipadas; **sólo envenena la última** | Un `Io(...)` genérico. La regla es «**envenena lo que miente, no lo que se acaba**»: un tag que no verifica dice que lo recibido no es lo que dice ser; un cierre o un silencio sólo dicen que se acabó. Y se descartó un tipo `RemoteProcessKilled`: TCP no puede saber eso, así que los tipos nombran lo observado (`PeerVanished`, `PeerSilent`, `PeerClosedMidFrame`) |
| 6 | Modelo de hilos | **Dos por conexión**: lector (posee decodificador y motor) y escritor | Uno solo, que se interbloquea en cuanto el peer deja de leer —la escritura se bloquea y con ella la lectura de los ACK que la desbloquearían—. Y tres, con el motor en el suyo: añade un `Mutex` alrededor de una máquina de estados que sólo el lector toca |
| 6b | Cancelación | `AtomicBool` **y** `shutdown`, con trabajos distintos | Cualquiera de las dos sola. **Una bandera no despierta una llamada al sistema bloqueada, y un `shutdown` no manda un frame `Cancel`.** La bandera hace que el peer distinga una cancelación de una caída; el `shutdown` es lo único que devuelve a un escritor parado dentro de `write` |
| 7 | Roles y puerto | Recibe = escucha = **responder**; envía = marca = **initiator**. Puerto del llamante, **0 = lo elige el sistema** | Un puerto por defecto, que es una decisión de descubrimiento y el descubrimiento es 6B. El puerto 0 no es comodidad: un puerto fijo hace intermitente cualquier prueba de red, y la reacción típica a esa intermitencia es añadir un `sleep`, que esconde el problema |

La sección obligatoria «lo que esta ADR no promete» es §8 del documento, con trece
entradas. Las que más importan: no hay descubrimiento, no hay NAT ni internet, no hay
reconexión, nada probado en hardware físico ni entre dos máquinas, y una lista
explícita de riesgos conocidos en Windows —empezando por que un `read` vencido
devuelve `WouldBlock` en Linux y `TimedOut` en Windows, y **tratar cualquiera de las
dos como final rompería toda transferencia en una de las dos plataformas**.

---

## 4. Errores detectados que no estaban en el prompt

| # | Hallazgo | Fase | Gravedad | Dónde |
|---|---|---|---|---|
| 6A-1 | **Tres reglas del propio sprint no pueden cumplirse a la vez.** §13.1 exige el prompt verbatim en el informe; el prompt cita el identificador del primero de los tres huecos de prueba de `qyro_fs` de su §9; `check_docs_consistency` bloquea todo identificador citado sin ficha en `BUGS_PENDING.md`; y §5 prohíbe a los dos agentes tocar `BUGS_PENDING.md`. Alcanza además a §13.5, que exige proponer identificadores para los hallazgos no arreglados: cada uno que acuñara dispararía el mismo bloqueo | 1 | Media — bloquea `ci.yml`, no afecta al producto | §1 |
| 6A-2 | Falso verde propio al reproducir la línea base: `&&` tras una tubería lee el estado de salida de `tail`, no el de `rustfmt`, y la ruta `rust/Cargo.toml` no existe —el workspace está en la raíz—. El comando fallaba e imprimía `FMT_PASS` | 0 | Baja — instrumento de medida, detectado y rehecho en la misma fase | §2 |
| 6A-3 | **`qyro_net` no se ejecuta en Windows en ningún workflow.** `cargo test --workspace` corre sólo en `ubuntu-latest` (`ci.yml:33,47`); los trabajos de Windows prueban `qyro_crypto` y `qyro_win_dpapi` y nada más. Y este crate es precisamente donde el comportamiento diverge entre plataformas | 2 | **Alta** para lo que se pueda afirmar de Windows; nula para Linux | §8, §15 |
| 6A-4 | Un control que sobrevivió a su propio borrado, encontrado por el barrido: quitar `TimedOut` de `is_read_timeout` no rompía nada, porque en Linux un `read` vencido da `WouldBlock` y sólo Windows da `TimedOut`. La mitad de Windows de esa rama no la defendía nadie. Es 6A-3 con consecuencias concretas | 2 | Media — habría roto **toda** transferencia en Windows sin que ninguna prueba lo dijera | §10 |
| 6A-7 | Segundo control superviviente, encontrado por el barrido de la Fase 3: **nada comprobaba que un frame sin sellar llegado después del handshake se rechace.** Cambiar esa rama a `Ok(None)` no rompía ninguna prueba | 3 | **Alta.** Es aceptar bytes que nada autenticó, en una conexión cuyo propósito entero es que todo en ella esté autenticado | §10 |
| 6A-8 | Me comí un fallo de clippy leyendo mal mi propia comprobación: canalicé la salida a `grep -c`, leí «4» como informativo y commiteé en rojo. El código de salida era 101 | 3 | Baja — un commit en rojo, detectado y enmendado antes de empujar | §9 |
| 6A-6 | **El criterio de §10 `git diff --name-only origin/main...HEAD` no puede pasar en esta rama, por diseño.** `origin/main` está en `e0041de`, anterior al sprint 4A, y esta rama se apoya en cuatro ramas de sprint sin fusionar, así que ese diff devuelve 319 archivos de cinco sprints — incluidos los cinco de la lista prohibida, ninguno tocado por este run | 2 | Baja para el producto, **alta para auditar este informe**: la comprobación literal alarma sin motivo | §13 |
| 6A-5 | Una mutación mal apuntada mía (M4): mutó sólo la rama autenticada de `read_window`, y la prueba que debía matarla usa una conexión sin autenticar, así que «sobrevivió» sin significar nada. Rehecha como M4b | 2 | Baja — error de instrumento, no de producto | §10 |

Pendiente de ampliar conforme avancen las fases.

---

## 5. Cuáles arreglé y cuáles no

| # | Arreglado | Cómo, o por qué no |
|---|---|---|
| 6A-1 | **Sí, en lo que me toca. La causa de fondo, no.** | El prompt se archiva verbatim en `docs/reports/6A-prompt.txt`, que ninguno de los dos comprobadores escanea (la lista es `*.md`, `*.rs`, `*.sh`, `*.ps1`, `*.yml` en las dos implementaciones), y los hallazgos de este informe se numeran `6A-n` en vez de acuñar identificadores que no puedo registrar. Con eso `ci.yml` queda en verde sin debilitar ningún control ni tocar ningún archivo ajeno. **Lo que no está arreglado es el choque de reglas**, que sigue ahí para el siguiente sprint que trabaje con el ledger fuera de su alcance. La decisión de fondo es del supervisor y está razonada en §1 |
| 6A-2 | Sí | Medición rehecha capturando `$?` de cada proceso por separado. La tabla de la línea base de §2 sale de esa segunda medición, no de la primera |
| 6A-3 | **No, y no puedo.** | El arreglo es añadir `qyro_net` a un trabajo de Windows, y `.github/workflows/**` está en la lista prohibida de §5. No lo toco «sólo un poco». Queda para el supervisor, y hasta entonces **ninguna afirmación de este informe sobre Windows tiene clase de evidencia mejor que «compilado»** — ni siquiera eso, porque tampoco se compila allí |
| 6A-4 | Sí, la mitad que se puede | Añadida `a_read_timeout_is_a_heartbeat_on_both_platforms`, que prueba el **mapeo**: quitar cualquiera de las dos ramas ahora rompe una prueba con nombre. Lo que **no** cierra, y la prueba lo dice en su propio comentario, es que Windows se comporte como se describe: eso necesita ejecutar el crate allí, que es 6A-3 |
| 6A-7 | Sí | Añadida `a_plain_frame_after_the_handshake_is_refused_and_poisons`, que mete un frame sin sellar por `write_sealed` —que escribe bytes tal cual— y comprueba variante, envenenamiento y que no se recupera |
| 6A-8 | Sí | Commit enmendado con la corrección (`map_err` → `inspect_err`), y desde entonces **compruebo el código de salida, no el recuento de líneas**. Es la misma forma de error que 6A-2: mirar la salida de un comando en vez de su estado |
| 6A-6 | **No es mío que arreglar** | El arreglo es fusionar las ramas de sprint a `main`, o cambiar el criterio para que use la rama base. Las dos son decisiones del supervisor. Lo que sí hago es dar en §13 **las dos** salidas, con la explicación de por qué la literal no dice nada de este sprint, en vez de pegar sólo la que me favorece |
| 6A-5 | Sí | Rehecha como M4b, mutando `read_window` entera. La mataron dos pruebas. Las dos filas quedan en la tabla de §10: esconder la mutación que no significaba nada dejaría la tabla más limpia y menos cierta |

---

## 6. A qué afectaba cada defecto

**6A-1 — el choque de reglas del ledger.** Qué se rompía: `ci.yml`, en los dos pasos
de `check_docs_consistency` (Bash y PowerShell), sobre cualquier commit que llevara el
informe con el prompt dentro de un `.md`. Nada del producto. Para quién: para este
sprint, que no puede cumplir a la vez §13.1, §13.5 y §5; y para el supervisor, que se
encuentra un control rojo cuya causa no está en el código sino en el reparto de
archivos que él mismo definió. En qué escenario: aparece en cuanto un informe cita un
identificador cuya ficha pertenece al otro agente, que es el escenario **normal**
—no excepcional— de dos agentes en paralelo con el ledger congelado. `docs/reports/`
no existía antes de este sprint, así que ningún sprint anterior lo tocó: es un
estreno, no una regresión.

Lo que conviene que sepa el supervisor: la parte que arreglé es cosmética —dónde vive
un archivo y cómo se numeran unas filas—. La parte de fondo es una decisión de
proceso, y las opciones reales son tres: que el ledger deje de estar congelado para
entradas nuevas, que `check_docs_consistency` distinga citar un hallazgo de archivar
un documento externo, o que los informes de sprint no acuñen identificadores y lo
haga sólo la consolidación. Esta rama implementa de hecho la tercera, pero sin
escribirla en ningún sitio canónico, porque no puede: los seis documentos raíz están
fuera de alcance.

**6A-3 — `qyro_net` sin Windows.** Qué se rompía: nada todavía; lo que falta es la
comprobación. Para quién: para cualquiera que lea «el transporte funciona» y asuma
que eso incluye la plataforma en la que Qyro tiene un instalador. En qué escenario:
en cuanto el transporte llegue a un usuario de Windows. Este crate es el peor sitio
posible para no ejecutar las pruebas, porque es donde el sistema operativo asoma —
códigos de error, semántica de `shutdown` sobre un `read` bloqueado, `SO_RCVTIMEO`—.
6A-4 es la demostración de que no es un riesgo teórico: el barrido encontró una rama
de Windows que llevaba escrita desde el primer commit del crate y que nadie defendía.

**6A-4 — el `TimedOut` sin cubrir.** Qué se rompía: si alguien hubiera «limpiado»
esa rama por parecer redundante, **toda** transferencia en Windows habría muerto en
el primer cuarto de segundo de espera, porque el latido se habría interpretado como
un final. Para quién: para todos los usuarios de Windows, que es la única plataforma
con paquete hoy. En qué escenario: cualquiera; no hace falta ni un fallo de red, sólo
una pausa normal. Y ninguna prueba lo habría dicho, en ningún sistema operativo,
porque en Linux esa rama es inalcanzable. Es el ejemplo exacto de por qué el barrido
de mutación existe: la línea estaba escrita, comentada y era correcta, y aun así no
estaba cubierta.

**6A-5 — la mutación mal apuntada.** Qué se rompía: mi propia tabla de mutación.
Para quién: para quien la lea y cuente los supervivientes. En qué escenario: si no
llego a mirar por qué sobrevivió, habría registrado un hueco de cobertura que no
existe —y peor, habría dejado sin comprobar el control que sí importa—. Un
superviviente puede significar dos cosas muy distintas, «la propiedad no está
cubierta» o «la mutación no tocó la propiedad», y no distinguirlas hace inútil el
barrido entero.

**6A-2 — el falso verde de la línea base.** Qué se rompía: nada en el producto; el
defecto estaba en mi propio instrumento de medida. Para quién: para el siguiente
lector de este informe y para el supervisor, que es precisamente quien no pudo
verificar la evidencia de 5B.1 y a quien se le pidió a este sprint que reprodujera la
línea base por su cuenta. En qué escenario: si no llego a mirar el resto de la
salida, la Fase 0 se habría declarado cerrada con una línea base que no se midió, que
es la misma forma de fallo que el hallazgo que dejó cuatro sprints de evidencia
estructural midiendo menos de lo que decían —una fase declarada cerrada que no lo
estaba—, y que el propio §12 del prompt pone como ejemplo de por qué una puerta no se
parchea en la fase siguiente.

---

## 7. Resultado final contra el objetivo, objetivo por objetivo

Pendiente. Se rellena en la Fase 6 contra los diecisiete criterios de §10. «Parcial»
es una respuesta válida; «cumplido» sin evidencia no lo es.

---

## 8. Clase de evidencia por cada afirmación

Clases usadas en este informe, de menor a mayor fuerza: **compilado** / **probado en
unidad** / **probado en integración** / **probado entre dos procesos** / **probado en
emulador** / **probado en simulador** / **probado en hardware físico**.

| Afirmación | Clase de evidencia |
|---|---|
| La línea base de `15934aa` es 388 tests, 0 failed, 2 ignored, 61 paquetes, fmt y clippy limpios, cuatro `check_*` en verde | **Probado en unidad e integración, en Linux, en esta sesión.** Códigos de salida capturados el 2026-08-11T18:01:59Z |
| `origin/claude/qyro-filesystem-5b1` está en `15934aae3dda7f469b5496c8341eb78d9e32f335` | **Comprobado**, `git rev-parse` |
| La firma de entrada del decodificador incremental es `push(&mut self, bytes: &[u8]) -> Result<(), FrameError>` | **Leído en el fuente**, `rust/crates/qyro_protocol/src/decoder.rs:268`. No es una afirmación de comportamiento en red |
| Los seis workflows que 5B.1 declara sobre `e3fbaf1` | **No verificada por mí.** Ver §14 |
| ADR-0028 está congelada antes del primer commit de código | **Comprobable en el historial.** `db3ce79` precede a `a77e657`, y en `db3ce79` no existe ningún `.rs` en la rama |
| Un frame partido en tres lecturas se reensambla; tres frames de una lectura salen los tres | **Probado en integración sobre sockets reales de 127.0.0.1**, en Linux. Ambas propiedades con recuento de lecturas medido, no supuesto |
| Un peer sin autenticar no consigue que el proceso acepte más de 4096 bytes ni reserve el megabyte que declara | **Probado en integración sobre socket real**, en Linux. Dos contadores medidos: bytes devueltos por `read`, y capacidad releída del decodificador |
| El silencio, el cierre en frontera, el cierre a mitad de frame y el puerto cerrado producen cuatro variantes tipadas distintas | **Probado en integración sobre sockets reales**, en Linux |
| `WouldBlock` y `TimedOut` cuentan como latido | **Probado en unidad, sobre la función de mapeo.** *No* probado en Windows: nada de este crate se ejecuta allí (6A-3) |
| La suite de `qyro_net` no es intermitente | **Probado en integración, diez ejecuciones consecutivas, 10/10**, en Linux, en una sola máquina |
| Todo lo relativo a Windows, macOS, Android, iOS | **No probado.** Ni ejecutado ni compilado allí en este run |
| Dos procesos de sistema operativo se pasan un archivo | **No hecho todavía.** Es la Fase 4. A fecha de la Puerta 2 nada ha cruzado entre dos procesos |

Nada de este sprint se ha ejecutado todavía en Windows, macOS, Android, iOS ni en
hardware físico. Los cuatro `check_*` se han corrido en Bash; **no** en PowerShell.

---

## 9. Las seis puertas

### Puerta 0 — 2026-08-11 — **PASADA**

Criterio del prompt: *«los números de la línea base coinciden con los declarados, o
has escrito en qué difieren. Sabes decir, sin volver a mirar, cuál es la firma exacta
por la que el decodificador incremental recibe bytes.»*

| Requisito | Resultado |
|---|---|
| `git rev-parse origin/claude/qyro-filesystem-5b1` == `15934aa…f335` | PASS |
| Los números de la línea base coinciden, o está escrito en qué difieren | PASS — coinciden los cuatro, tabla en §2 |
| La firma exacta de entrada del decodificador | PASS — §2, con las seis consecuencias que gobiernan la Fase 2 |
| Leído `qyro_protocol/src/` (decodificador, envenenamiento, errores) | PASS |
| Leído `qyro_transfer/src/session.rs` (API de `Sender`/`Receiver`) | PASS |
| Leídas ADR-0018 y ADR-0026 | PASS |
| `git diff --name-only` sin archivos de la lista prohibida de §5 | PASS — no hay diff; la Fase 0 no modificó ningún archivo del repositorio |

Las nueve comprobaciones de §12 no se aplican íntegras a la Fase 0: fmt, clippy y
tests son la propia línea base y están arriba; no hay mutación, aserciones,
contadores ni tests nuevos que leer, porque la Fase 0 no escribe código. Lo que sí
aplica —el diff sin archivos prohibidos— está comprobado.

### Puerta 1 — 2026-08-11 — **PASADA**

Las nueve comprobaciones de §12, más la condición extra que §8 Fase 1 añade.

| # | Comprobación de §12 | Resultado |
|---|---|---|
| 1 | `cargo fmt --all --check` | PASS — exit 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS — exit 0 |
| 3 | `cargo test --workspace`, sin ignorados nuevos | PASS — 388 passed, 0 failed, **2 ignored** (los mismos dos de la línea base) |
| 4 | Barrido de mutación de la fase | **No aplica.** La fase no añade ningún control de producción: es una ADR y un informe. Ver la nota de abajo |
| 5 | Lectura de aserciones | **No aplica.** Cero aserciones nuevas |
| 6 | Lectura de contadores | **No aplica.** Cero contadores nuevos |
| 7 | Lectura de nombres de test | **No aplica.** Cero tests nuevos |
| 8 | `git diff --name-only` sin archivos prohibidos | PASS — sólo `docs/adr/ADR-0028-network-transport.md`, `docs/reports/6A-claude-code.md` y `docs/reports/6A-prompt.txt`, los tres míos por §5 |
| 9 | Resultado escrito en el informe antes de empezar la fase siguiente | PASS — esto |

Condición extra de §8 Fase 1: *«la ADR está commiteada antes del primer commit de
código y se puede comprobar en el historial»*. PASS — y es comprobable con
`git log --oneline --name-only`: a fecha de la Puerta 1 no existe **ningún** archivo
`.rs` en la rama, porque `qyro_net` no se ha creado todavía.

Y los cuatro `check_*`, que no están entre las nueve de §12 pero sí en §10:

| Script | Resultado |
|---|---|
| `check_repo_portability.sh` | PASS |
| `check_harness_isolation.sh` | PASS |
| `check_crypto_platform_evidence.sh` | PASS |
| `check_docs_consistency.sh` | **Falló primero, PASS después.** Es el hallazgo 6A-1; ver §1, §4 y §6 |

**Sobre la comprobación 4 en una fase de documentación.** Un barrido de mutación
sobre una ADR no significa nada: no hay control que borrar y no hay suite que romper.
Decirlo es más honesto que inventarse una fila. Lo que sí sujeta esta fase es la
Puerta 2: cada número que ADR-0028 fija —4096, 10 s, 8, 250 ms, 60 s, 65 536— tiene
que aparecer en el código de la Fase 2 y tener una prueba que lo ejerza, y ahí sí hay
mutación. Una ADR cuyos números no aparecen en ninguna prueba es prosa.

### Puerta 2 — 2026-08-11 — **PASADA**

| # | Comprobación de §12 | Resultado |
|---|---|---|
| 1 | `cargo fmt --all --check` | PASS — exit 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS — exit 0, cero líneas `^warning`/`^error` |
| 3 | `cargo test --workspace`, sin ignorados nuevos | PASS — **399** passed, 0 failed, **2 ignored** (los mismos dos) |
| 4 | Barrido de mutación de la fase | PASS tras dos correcciones. Tabla completa en §10: siete mutaciones, un superviviente real (6A-4) cerrado, una mutación mal apuntada (6A-5) rehecha |
| 5 | Lectura de aserciones | PASS — leídas una a una, ver abajo |
| 6 | Lectura de contadores | PASS — tres contadores nuevos, ver abajo |
| 7 | Lectura de nombres de test | PASS — ver abajo |
| 8 | `git diff --name-only` sin archivos prohibidos | PASS — §13 |
| 9 | Resultado escrito en el informe antes de la fase siguiente | PASS — esto |

Y `cargo test --doc --workspace`: PASS. Los cuatro `check_*`: PASS los cuatro.
Paquetes en `Cargo.lock`: **62**, y el que entró es `qyro_net`.

**Extra que no pide §12 pero sí piden estas pruebas.** Son pruebas de red, y una que
pasa una vez y falla a la tercera es peor que no tenerla. La suite de `qyro_net` se
corrió **diez veces seguidas: 10/10**. La Fase 4 tiene esta exigencia por escrito;
adelantarla aquí es más barato que descubrir la intermitencia dos fases después.

#### Comprobación 5 — lectura de aserciones

Cada aserción nueva, leída para ver que sus dos lados pueden diferir. La trampa es
una aserción cuyos dos términos son la misma llamada, y este proyecto la ha cometido
cinco veces.

- Los payloads se comparan contra `payload_of(len, tag)`, que **recalcula** el patrón
  con la misma fórmula pero por un camino distinto del que construyó el frame. Los dos
  lados son valores independientes: si `plaintext()` devolviera basura, o el payload de
  otro frame, o un `None`, la comparación falla.
- `assert_eq!(stream.bytes_read(), HEADER_LEN + 600)`: contador medido contra suma
  aritmética que la prueba calcula.
- `assert!(reserved < declared)`: capacidad leída del decodificador contra el megabyte
  que el peer declaró. Dos números de origen distinto, y ésa es la aserción central de
  la prueba de memoria.
- `assert_ne!(ending, NetError::PeerClosedMidFrame { buffered: 0 })` en la prueba de
  cierre en frontera: existe justamente para que las dos variantes no colapsen en una.
- `assert!(waited >= deadline)`: reloj medido contra plazo que la prueba fijó.
- En `a_read_timeout_is_a_heartbeat_on_both_platforms` hay aserciones positivas **y**
  negativas sobre las dos funciones de mapeo. Sin las negativas, una función que
  devolviera `true` para todo pasaría.

#### Comprobación 6 — lectura de contadores

Tres contadores `cfg(test)` nuevos en `FrameStream`. La trampa es un contador que
registra la constante que esperabas en vez de lo medido; este proyecto la cometió con
`PEAK_BUILDER_READ.fetch_max(HASH_BUFFER_LEN, ...)` seguido de
`assert_eq!(peak, HASH_BUFFER_LEN)`.

| Contador | De dónde sale el valor | Contra qué se compara |
|---|---|---|
| `bytes_read` | `count` devuelto por `socket.read(..)`, sumado | Suma aritmética de la prueba, y `MAX_PREAUTH_BYTES` |
| `read_calls` | Incrementado donde `read` devolvió `Ok(count)` con `count > 0` | 3 y 3, valores que la prueba elige |
| `peak_decoder_capacity` | `self.decoder.buffer_capacity()`, **releído del decodificador** después de cada `push` | El megabyte declarado por el peer, y la asignación |

Ninguno se asigna desde una constante de límite. `peak_decoder_capacity` es el que más
importa y es el que en 5B.1 se hizo mal: aquí el valor no se escribe, se pregunta.

#### Comprobación 7 — lectura de nombres

Por cada prueba, ¿el cuerpo ejerce lo que el nombre dice? La trampa es
`a_symlink_at_the_final_component_is_refused`, que nunca abrió un archivo.

| Prueba | El nombre dice | El cuerpo hace |
|---|---|---|
| `a_frame_split_across_three_reads_is_reassembled` | Tres lecturas | Tres `write_all` con 80 ms de separación, y **asserta `read_calls() >= 3`**: si el kernel los uniera, la prueba falla en vez de pasar sin probar nada |
| `three_frames_in_one_read_are_all_delivered` | Una lectura, tres entregados | Un `write_all` con los tres concatenados; comprueba los tres payloads y `read_calls() < 3` |
| `a_peer_that_sends_nothing_times_out_and_says_so` | No manda nada, vence, y lo dice | El peer no escribe un byte; comprueba la variante, que hubo **latidos previos**, y que esperó al menos el plazo |
| `a_peer_that_disconnects_mid_frame_is_a_typed_end` | A mitad de frame | Manda `HEADER_LEN + 10` de un frame de 100 de payload y cierra; comprueba `buffered == 58` |
| `a_peer_that_disconnects_on_a_boundary_is_a_different_typed_end` | Otra variante | Cierra sin mandar nada; comprueba que **no** es la variante de la prueba anterior |
| `a_peer_cannot_make_us_buffer_more_than_the_declared_limit` | Declarar no es reservar | Cabecera legítima que declara 1 MiB, hecha con `Frame::new` sin parchear un byte, y luego un diluvio; mide lo reservado y lo aceptado |
| `a_legitimate_frame_still_round_trips` | Los rechazos no rechazan todo | Dos `FrameStream` reales, un frame en cada dirección |
| `a_read_timeout_is_a_heartbeat_on_both_platforms` | Las dos plataformas | Las dos `io::ErrorKind`, más las que no deben confundirse. **No** prueba Windows: el comentario de la prueba lo dice y §8 lo clasifica |
| `a_listener_reports_the_port_the_system_chose` | Informa del puerto | `bind` con puerto 0 y `assert_ne!(port, 0)` |
| `authenticating_releases_the_listener_budget_and_grows_the_buffer` | Suelta y crece | Comprueba `pending()` 1→0 y el búfer 4096→65 536 alrededor de la llamada |
| `a_dial_to_a_closed_port_is_typed_and_is_not_a_generic_io_error` | Tipado, no genérico | Marca a un puerto sin nadie; comprueba que la variante es una de las dos tipadas |

### Puerta 3 — 2026-08-11 — **PASADA**

| # | Comprobación de §12 | Resultado |
|---|---|---|
| 1 | `cargo fmt --all --check` | PASS — exit 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS — exit 0. **Falló primero**: ver 6A-8 |
| 3 | `cargo test --workspace`, sin ignorados nuevos | PASS — **405** passed, 0 failed, **2 ignored** |
| 4 | Barrido de mutación de la fase | PASS tras cerrar un superviviente real (6A-7). Tabla en §10 |
| 5 | Lectura de aserciones | PASS — ver abajo |
| 6 | Lectura de contadores | **No aplica.** La Fase 3 no añade contadores |
| 7 | Lectura de nombres de test | PASS — ver abajo |
| 8 | `git diff --name-only` sin archivos prohibidos | PASS — §13 |
| 9 | Resultado escrito antes de la fase siguiente | PASS — esto |

`cargo test --doc --workspace`: PASS. Los cuatro `check_*`: PASS. Paquetes: **62**,
sin cambio — `qyro_crypto` era una arista nueva, no un paquete nuevo. Suite de
`qyro_net` **diez veces seguidas: 10/10**.

#### Comprobación 5 — lectura de aserciones

La aserción central de esta fase es la de acuerdo de sesión, y es exactamente donde
vive la trampa 1 de §11 —dos lados que son la misma llamada—. Cómo se evita aquí:

- `assert_eq!(responder.session_id(), initiator.session_id())`. Son **dos objetos
  distintos, construidos en dos hilos distintos**, cada uno derivando el id de su
  propia vista del transcript. Preguntarle a una sesión su id dos veces no probaría
  nada; esto sí.
- Y además `assert_ne!(responder.session_id(), SessionId::ZERO)`, porque dos ceros
  también comparan iguales. Sin esta línea, un `session_id()` que devolviera el valor
  por defecto pasaría la comparación anterior.
- Las huellas se comparan **en cruz**: la del peer que ve el iniciador contra la
  huella del que escucha, calculada por la prueba **antes** de que corriera el
  handshake. Una sesión que devolviera su propia identidad fallaría. Y
  `assert_ne!(listening_print, dialling_print)` cierra el caso de dos identidades
  iguales, con el que la comparación cruzada no diría nada.
- En la prueba de firma corrupta, `assert_ne!(tampered, responder_hello)` comprueba
  que la manipulación **cambió un byte de verdad** antes de mandarla.
- En la del bit volteado, `assert_ne!(flipped, sealed)` hace lo mismo.

#### Comprobación 7 — lectura de nombres

| Prueba | El nombre dice | El cuerpo hace |
|---|---|---|
| `two_endpoints_over_a_real_socket_agree_on_a_session_key` | Dos extremos, socket real, acuerdo | Dos hilos, dos identidades Ed25519 reales, handshake completo sobre 127.0.0.1, y compara valores derivados por caminos distintos |
| `a_sealed_frame_crosses_a_real_socket_and_opens` | Cruza y abre | Sella con el sellador real, cruza el socket, abre y compara el payload. En las dos direcciones |
| `a_peer_with_a_wrong_signature_never_reaches_the_application` | Firma mala, nada llega | Calcula un `ResponderHello` **real** y voltea un bit de su firma Ed25519; todo lo demás intacto, así que lo único rechazable es la firma. Comprueba las tres cosas: variante tipada, que no existe `Session` alguna —y `Session` es lo único que puede mandar frames de aplicación—, y que al otro lado no llegó nada |
| `a_handshake_that_stalls_is_cut_by_the_deadline` | El plazo lo corta | Acepta y calla. Comprueba la variante, que el plazo es el pasado, que esperó **al menos** ese plazo, y que **no** esperó al de inactividad —que sería el plazo equivocado haciendo el trabajo— |
| `a_flipped_bit_after_the_handshake_poisons_the_session` | Envenena | Las tres aserciones que pide §8: variante exacta `NotAuthenticated`, `is_poisoned()`, y que un frame legítimo posterior **tampoco** se entrega |
| `a_plain_frame_after_the_handshake_is_refused_and_poisons` | Refusa y envenena | Mete un frame **sin sellar** por `write_sealed`, que escribe bytes tal cual. Nació del barrido (6A-7) |

### Puertas 4 a 6

Pendientes.

---

## 10. Tabla de mutación

Una fila por control de producción: propiedad → mutación aplicada → test que falló.
Los supervivientes están en la tabla, no escondidos.

Las fases 0 y 1 no añaden controles de producción —una línea base y una ADR— así que
no tienen filas.

### Fase 2 — commit `a77e657`, más el arreglo de 6A-4

| # | Propiedad | Mutación aplicada | Resultado |
|---|---|---|---|
| M1 | Un peer sin autenticar no consigue que el proceso acepte más de 4096 bytes | `read_window` devuelve `self.buffer.len()` siempre: la asignación deja de acotar la lectura | **Muerta** por `a_peer_cannot_make_us_buffer_more_than_the_declared_limit` |
| M2 | Un cierre a mitad de frame es una variante distinta de un cierre en frontera | `orderly_close` devuelve siempre `PeerClosedEarly` | **Muerta** por `a_peer_that_disconnects_mid_frame_is_a_typed_end` |
| M3 | Un `read` vencido es un latido, no un final | La rama de vencimiento devuelve siempre `PeerSilent`, sin mirar el plazo | **Muerta** por `a_peer_that_sends_nothing_times_out_and_says_so` (falla en `heartbeats >= 1`) |
| M4 | *(mal apuntada — 6A-5)* | `read_window` devuelve `1` **sólo en la rama autenticada** | **Sobrevivió, y no significa nada**: la prueba que debía matarla usa una conexión sin autenticar, que toma la otra rama. Rehecha como M4b |
| M4b | El decodificador se drena antes de volver al socket, así que varios frames de una lectura salen todos | `read_window` devuelve `1` siempre | **Muerta** por `three_frames_in_one_read_are_all_delivered` **y** `a_peer_cannot_make_us_buffer_more_than_the_declared_limit` |
| M5 | Autenticarse devuelve el hueco al presupuesto del listener | Se borra `self.pending_slot = None;` de `mark_authenticated` | **Muerta** por `authenticating_releases_the_listener_budget_and_grows_the_buffer` |
| M6 | El búfer de 64 KiB no se asigna hasta que el peer se autentica | Se borra el `resize` de `mark_authenticated` | **Muerta** por `authenticating_releases_the_listener_budget_and_grows_the_buffer` |
| M7 | `TimedOut` cuenta como latido, para Windows | Se quita `io::ErrorKind::TimedOut` de `is_read_timeout` | **SOBREVIVIÓ** en la primera pasada — hallazgo 6A-4. Tras añadir `a_read_timeout_is_a_heartbeat_on_both_platforms`: **muerta** por esa prueba |

### Fase 3 — commit `3021cee`, más el arreglo de 6A-7

| # | Propiedad | Mutación aplicada | Resultado |
|---|---|---|---|
| H1 | Un tag que no verifica envenena la sesión | Se borra `self.poisoned = true;` de la rama de error de `opener.open` | **Muerta** por `a_flipped_bit_after_the_handshake_poisons_the_session` |
| H2 | Una sesión envenenada no vuelve a entregar nada | Se borra la comprobación `if self.poisoned` de la entrada de `recv` | **Muerta** por `a_flipped_bit_...` **y** `a_plain_frame_after_the_handshake_...` |
| H3 | El plazo de handshake tiene variante propia y no se confunde con silencio | `HandshakeDeadlineExceeded` → `PeerSilent` | **Muerta** por `a_handshake_that_stalls_is_cut_by_the_deadline` |
| H4 | El stream se marca autenticado al establecer la sesión | Se borran las dos llamadas a `mark_authenticated()` | **Muerta** por `two_endpoints_over_a_real_socket_agree_on_a_session_key` |
| H5 | Un frame **sin sellar** después del handshake se rechaza | La rama de frame plano devuelve `Ok(None)` en vez de envenenar | **SOBREVIVIÓ** — hallazgo 6A-7. Tras añadir `a_plain_frame_after_the_handshake_is_refused_and_poisons`: **muerta** por esa prueba |

**Nota sobre lo que este barrido no cubre.** La verificación de la firma en sí es de
`qyro_crypto` y tiene su propio barrido allí; lo que se muta aquí es lo que yo
escribí. Que el error de firma se **propague** en vez de tragarse no es mutable sin
romper la compilación —el valor hace falta para continuar—, así que esa propiedad la
sujeta `a_peer_with_a_wrong_signature_never_reaches_the_application` directamente y
no por mutación.

**Supervivientes sin cerrar al final de la Fase 2: ninguno.** M7 se cerró en cuanto a
mapeo; lo que queda abierto no es un control sin prueba sino una plataforma sin
ejecución, y eso es 6A-3, que no puedo arreglar porque vive en
`.github/workflows/**`.

**Cómo se corrió.** Cada mutación se aplica sobre el árbol ya commiteado, se corre
`cargo test -p qyro_net`, se anota el nombre del test que falló, y se restaura el
archivo desde la copia en memoria antes de la siguiente. Commitear antes de mutar no
es ceremonia: en dos sprints anteriores un `git checkout --` se llevó por delante una
corrección sin commitear junto con la mutación que debía deshacer.

---

## 11. Tests antes y después

**Antes: 388 passed, 0 failed, 2 ignored** (medido, no heredado — §2).
**Después de la Fase 3: 405 passed, 0 failed, 2 ignored.** Diecisiete nuevos, todos
en `qyro_net`, ningún ignorado nuevo. La Fase 2 aportó once y la Fase 3 seis.

| Test nuevo | Qué prueba |
|---|---|
| `a_frame_split_across_three_reads_is_reassembled` | Que un frame partido en tres escrituras separadas por 80 ms sale del otro lado como **uno solo** e íntegro — y que de verdad llegó en tres lecturas, no en una |
| `three_frames_in_one_read_are_all_delivered` | El caso contrario: tres frames concatenados en una escritura salen los tres, en orden, con sus payloads distintos, en menos lecturas que frames |
| `a_peer_that_sends_nothing_times_out_and_says_so` | Que el silencio produce `PeerSilent` y no un `Io` — y, lo que importa más, que **antes** hubo latidos `Ok(None)`: sin ellos no habría dónde comprobar una cancelación |
| `a_peer_that_disconnects_mid_frame_is_a_typed_end` | Que cerrar a mitad de frame produce `PeerClosedMidFrame` con el número exacto de bytes varados |
| `a_peer_that_disconnects_on_a_boundary_is_a_different_typed_end` | Que ese caso **no** es el mismo que un cierre limpio: las dos variantes no colapsan |
| `a_peer_cannot_make_us_buffer_more_than_the_declared_limit` | Que una cabecera legítima declarando 1 MiB no reserva 1 MiB, y que el proceso nunca acepta más de 4096 bytes de un desconocido. Las dos cifras medidas |
| `a_legitimate_frame_still_round_trips` | Que los rechazos anteriores no funcionan rechazándolo todo. Un frame en cada dirección entre dos `FrameStream` reales |
| `a_read_timeout_is_a_heartbeat_on_both_platforms` | Que `WouldBlock` **y** `TimedOut` cuentan como latido, y que un reset no. Nació de que el barrido encontró la rama de Windows sin defender (6A-4) |
| `a_listener_reports_the_port_the_system_chose` | Que `bind` con puerto 0 informa del puerto real. Es lo que evita que estas pruebas sean intermitentes |
| `authenticating_releases_the_listener_budget_and_grows_the_buffer` | Que autenticarse devuelve el hueco al presupuesto y sólo entonces se asigna el búfer de 64 KiB |
| `a_dial_to_a_closed_port_is_typed_and_is_not_a_generic_io_error` | Que marcar a un puerto sin nadie da una variante tipada |
| `two_endpoints_over_a_real_socket_agree_on_a_session_key` | Que dos extremos, en dos hilos, sobre un socket real, derivan **el mismo** id de sesión por caminos distintos, y que cada uno aprendió la identidad **del otro** |
| `a_sealed_frame_crosses_a_real_socket_and_opens` | Que después del handshake un frame sellado cruza y abre con su payload intacto, en las dos direcciones |
| `a_peer_with_a_wrong_signature_never_reaches_the_application` | Que una firma Ed25519 con un bit cambiado se rechaza con error tipado, que no se crea ninguna `Session`, y que al otro lado no llega nada |
| `a_handshake_that_stalls_is_cut_by_the_deadline` | Que un peer que conecta y calla se corta por el plazo de handshake — y no por el de inactividad, que sería el plazo equivocado haciendo el trabajo |
| `a_flipped_bit_after_the_handshake_poisons_the_session` | Las tres cosas que pide §8: la variante exacta, la sesión envenenada, y que un frame legítimo posterior tampoco se entrega |
| `a_plain_frame_after_the_handshake_is_refused_and_poisons` | Que un frame **sin sellar** en una sesión establecida se rechaza. Nació del barrido (6A-7) |

---

## 12. Delta de dependencias

**Antes: 61 paquetes en `Cargo.lock`** (medido con `grep -c '^\[\[package\]\]'`).
**Después: 62.** El que entró es `qyro_net`, y es el diff entero:

```
+[[package]]
+name = "qyro_net"
+version = "0.0.1"
+dependencies = [
+ "qyro_protocol",
+]
```

**Cero dependencias externas nuevas**, que es el criterio de §10. `qyro_net` depende
de `qyro_protocol` y de `std`, y de nada más. No hay runtime async, no hay `socket2`,
no hay `mio`. La Fase 3 añadirá una arista a `qyro_crypto`, que ya está en el grafo
auditado: una arista nueva, no un paquete nuevo.

---

## 13. `git diff --name-only origin/main...HEAD`

**El criterio de §10 nombra una base que no sirve para lo que quiere comprobar, y hay
que decirlo antes de pegar nada.** Es el hallazgo 6A-6.

`origin/main` está en `e0041de` («Rename qyro-logo.png to no usar este logo»), que es
**anterior al sprint 4A**. Esta rama se apoya en cuatro ramas de sprint que nunca se
fusionaron a `main`, así que `origin/main...HEAD` devuelve **319 archivos**: cinco
sprints de trabajo acumulado. Entre ellos están, inevitablemente,
`.github/workflows/**`, `STATUS.md`, `rust/crates/qyro_fs/**`,
`qyro_protocol/src/header.rs` y `rust/guards/source_guard.rs` — los cinco de la lista
prohibida de §5, ninguno tocado por este run, todos heredados de sprints anteriores.

Es decir: **la comprobación literal de §10 no puede pasar en esta rama, y su fallo no
significa nada sobre este sprint.** Pegar sólo esa lista sería alarmante y falso;
pegar sólo la útil sería omitir lo que se pidió. Van las dos.

### La comprobación que sí responde a la pregunta

Lo que §5 quiere saber es si pisé al otro agente. La base correcta para eso es la
rama de la que salgo:

```
$ git diff --name-only origin/claude/qyro-filesystem-5b1..HEAD
Cargo.lock
Cargo.toml
docs/adr/ADR-0028-network-transport.md
docs/reports/6A-claude-code.md
docs/reports/6A-prompt.txt
rust/crates/qyro_net/Cargo.toml
rust/crates/qyro_net/src/error.rs
rust/crates/qyro_net/src/lib.rs
rust/crates/qyro_net/src/limits.rs
rust/crates/qyro_net/src/listener.rs
rust/crates/qyro_net/src/stream.rs
rust/crates/qyro_net/src/tests.rs
```

Doce archivos. Los doce están en la lista de «archivos que TÚ tocas» de §5:
`rust/crates/qyro_net/**` entero, el `Cargo.toml` de la raíz (una línea), su efecto en
`Cargo.lock`, la ADR y el informe. `docs/reports/6A-prompt.txt` es el archivo hermano
del informe que explica §1.

Filtrado explícito contra la lista prohibida:

```
$ git diff --name-only origin/claude/qyro-filesystem-5b1..HEAD \
    | grep -E "qyro_fs/|qyro_protocol/src/header.rs|guards/source_guard.rs|\
^STATUS.md|^HANDOFF.md|^NEXT_STEPS.md|^CHANGELOG.md|^BUGS_PENDING.md|\
^DECISIONS.md|^\.github/"
NONE — clean
```

Y los commits de esta rama, para que se pueda comprobar que no hay ninguno más:

```
$ git log --oneline origin/claude/qyro-filesystem-5b1..HEAD
a77e657 feat(net): a frame stream over a real socket, with typed ends
f6964c7 docs(report): sprint 6A gate 1, and a clash between three of the sprint's own rules
db3ce79 docs: freeze ADR-0028 before opening a single socket
79ffa9b docs(report): sprint 6A, baseline reproduced and gate 0
```

`git status --short`: limpio. Sin commits en `main`, sin merge, sin PR, sin
force-push.

---

## 14. Todos los runs de CI de la rama

Pendiente. Sin filtrar: con ID, commit, workflow y conclusión, incluidos los fallidos
y los cancelados.

**Sobre la evidencia heredada.** El prompt (§3) declara que 5B.1 registra seis
workflows en verde sobre `e3fbaf10073faef91c21350937356be5d861c666` con los IDs
31232028441 / 378 / 429 / 435 / 405 / 433, y que el supervisor no pudo verificarlos
porque la API de GitHub le devolvió 403. **Yo tampoco los he verificado.** Lo que sí
he reproducido es la línea base local sobre `15934aa`, que es lo que §8 Fase 0 pedía.
Esos seis IDs no se usan como evidencia de nada en este informe.

---

## 15. Qué NO debe leerse como progreso

Esta sección es válida desde ahora y no espera al final del sprint.

- **No hay descubrimiento.** Ni mDNS, ni Bonjour, ni `NsdManager`. La dirección del
  peer se la pasa el llamante como `IpAddr:puerto`. Es el sprint 6B.
- **No hay FFI.** `qyro_ffi` no cambia y sigue sin alcanzar `qyro_crypto`,
  `qyro_transfer`, `qyro_fs` ni `qyro_net`.
- **No hay UI.** Los botones de Enviar y Recibir siguen deshabilitados. Nada de este
  sprint los habilita.
- **Qyro sigue sin transferir archivos para un usuario.** Lo que este sprint persigue
  es que dos *procesos de prueba* se pasen un archivo por un socket. No es lo mismo.
- **No hay persistencia de identidad en Android ni en iOS.** Android sigue parado en
  QYR-0064; iOS no se ha empezado.
- **Nada se ha probado en hardware físico ni entre dos máquinas distintas.** Dos
  procesos en 127.0.0.1 no son dos dispositivos en una Wi-Fi: comparten el mismo
  kernel, la misma pila de red y una interfaz de bucle invertido que no pierde
  paquetes, no reordena, no fragmenta y tiene un MTU que no se parece al de una red
  real. Todo lo que este sprint demuestre sobre ventana, go-back-N y control de flujo
  hay que leerlo con esa reserva delante.
- **La Fase 0 no escribió código.** A fecha de esta línea, `qyro_net` no existe.

---

## 16. Documentación desfasada y qué necesita saber el sprint siguiente

Pendiente. Nota de proceso, ya vigente: por §5 del prompt, en este run **ningún**
agente toca `STATUS.md`, `HANDOFF.md`, `NEXT_STEPS.md`, `CHANGELOG.md`,
`BUGS_PENDING.md` ni `DECISIONS.md`. Todo lo que normalmente iría a esos seis
documentos vive en este informe, y el supervisor los consolida después en un commit
aparte. Eso significa que, mientras dure el sprint, esos seis archivos están
desfasados por construcción y no por descuido — y que `check_docs_consistency.sh`
sigue midiendo `STATUS.md` contra un HEAD que ya no lo refleja.
