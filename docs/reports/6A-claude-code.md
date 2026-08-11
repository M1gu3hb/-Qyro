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

---

## 1. El prompt recibido, verbatim y completo

Reproducido tal y como se recibió, sin parafrasear, sin resumir y sin reordenar. El
formato de origen no traía marcas de código; se conserva el texto plano.

```text
1. Título del sprint
Sprint 6A — qyro_net: que dos procesos de verdad, por un socket de verdad, se pasen un archivo cifrado y verificado.
Sin descubrimiento, sin FFI, sin UI, sin selector de archivos. La dirección del peer se la pasa el llamante como IpAddr:puerto.
Es el sprint que convierte todo lo construido en cinco meses en algo que ocurre entre dos procesos separados. Hoy, el «transporte» de Qyro es un Vec<u8> que pasa de una variable a otra dentro del mismo proceso. Al terminar este sprint, tiene que ser un socket TCP.
2. Cómo se trabaja en este sprint — léelo antes que nada
Este prompt es largo a propósito y tiene seis fases. No es una lista de deseos: es una secuencia, y cada fase termina en una puerta de auto-auditoría que tienes que pasar antes de empezar la siguiente.
No avances de fase con la anterior a medias. Si una puerta no pasa, arréglalo y vuelve a pasarla. Si no puedes arreglarlo, para y reporta; no lo dejes para después ni lo compenses con trabajo de la fase siguiente.
El protocolo de puerta está en §12 y es idéntico para las seis. Léelo ahora, antes de escribir una línea.
Trabajas en paralelo con otro agente (Codex) que está tocando otras partes del mismo repositorio, al mismo tiempo, en otra rama. Las reglas de no interferencia son §5 y son estrictas: si tocas un archivo que no te toca, el merge se rompe y los dos sprints se pierden.
3. Repositorio, rama y HEAD
https://github.com/M1gu3hb/-Qyro
Rama base: claude/qyro-filesystem-5b1. Crea una rama nueva: claude/qyro-net-6a.
git fetch --all --prune
git rev-parse origin/claude/qyro-filesystem-5b1
# debe imprimir 15934aae3dda7f469b5496c8341eb78d9e32f335
Si imprime otra cosa, detente y reporta.
Verified commit de partida: e3fbaf10073faef91c21350937356be5d861c666.
Aviso de honestidad sobre esa evidencia: el sprint 5B.1 declara seis workflows en verde sobre e3fbaf1 con los IDs 31232028441 / 378 / 429 / 435 / 405 / 433. El supervisor no pudo verificarlos (la API de GitHub devolvió 403 desde su entorno). Reproduce tú la línea base localmente en la Fase 0 y no des por buena la tabla ajena.
4. Lo que ya existe, y las costuras exactas por donde entras
Crate	Qué te da
qyro_protocol	Cabecera fija de 48 bytes, tipos de mensaje, y un decodificador incremental acotado que arma mensajes conforme llegan los bytes. MAX_PAYLOAD_LEN 1 MiB, MAX_BUFFER_LEN 1 049 664, semántica de envenenamiento, errores estructurales y semánticos separados (ADR-0018)
qyro_crypto	Identidad Ed25519, handshake autenticado de cuatro mensajes (X25519 + firmas sobre el transcript + HKDF-SHA256 + HMAC-SHA256), FrameSealer/FrameOpener con ChaCha20-Poly1305 y la cabecera de 48 bytes completa como AAD, nonce sin repetición, ventana de replay de 1024
qyro_transfer	Sender/Receiver, chunks de 64 KiB, ventana de 16, go-back-N con ACK acumulativo, pausa/reanudación/cancelación desde ambos lados, veredicto de SHA-256 por archivo, ocho rechazos por estado tipados
qyro_fs	FileSource/FileSink sobre archivos reales, .qyro-part, rename tras digest verificado, resolución segura de rutas
qyro_manifest	Manifest canónico con digest obligatorio y validación dura de rutas
El decodificador incremental de qyro_protocol es la pieza clave y ya está resuelto. No escribas otro. Un socket TCP te entrega bytes en trozos arbitrarios, que es exactamente el problema que ese decodificador existe para resolver. Tu trabajo es alimentarlo desde un TcpStream, no reimplementarlo.
Línea base a reproducir: 388 tests, 0 failed, 2 ignored. 61 paquetes en Cargo.lock. Clippy y fmt limpios. Los cuatro check_* en verde.
5. Reglas de no interferencia — un agente en paralelo
Otro agente trabaja al mismo tiempo en codex/qyro-gap-closure-5c, sobre qyro_fs, qyro_protocol/src/header.rs y las guardas compartidas.
Archivos que TÚ tocas:
rust/crates/qyro_net/** — todo nuevo, es tuyo entero
Cargo.toml de la raíz — una línea, añadir qyro_net a members
Cargo.lock — el efecto de lo anterior
docs/adr/ADR-0028-network-transport.md — nuevo
docs/reports/6A-claude-code.md — tu reporte
tools/qyro_net_smoke/** si decides hacer el binario de dos procesos como herramienta (ver §8, Fase 4)
Archivos que NO tocas bajo ninguna circunstancia:
rust/crates/qyro_fs/** — es del otro agente
rust/crates/qyro_protocol/src/header.rs — es del otro agente
rust/guards/source_guard.rs — es del otro agente. Tú haces include! de él sin modificarlo, igual que hacen los otros crates
STATUS.md, HANDOFF.md, NEXT_STEPS.md, CHANGELOG.md, BUGS_PENDING.md, DECISIONS.md — ninguno de los dos agentes los toca en este run
.github/workflows/** — los seis workflows ya cubren tu caso
main — no commits, no merge, no PR, no rebase, no force-push
Esto es un cambio de proceso respecto a los cinco sprints anteriores y es deliberado. Todo lo que normalmente iría en STATUS.md y BUGS_PENDING.md va en tu reporte de docs/reports/6A-claude-code.md, con la estructura de §13. El supervisor consolida los seis documentos raíz después, en un commit aparte.
Si necesitas tocar un archivo de la lista prohibida, para y reporta por qué. No lo toques «sólo un poco».
6. La decisión que ya está tomada, para que no la investigues
Nada de async. std::net y std::thread.
El supervisor lo investigó con fuentes primarias el 2026-08-11 y la conclusión es firme:
Qyro tiene una conexión TCP. El valor de un runtime async es multiplexar miles de conexiones sobre pocos hilos; con N=1 un hilo bloqueante es más simple, más fácil de depurar y produce backtraces legibles.
El motor ya es una máquina de estados síncrona con su propio windowing. Envolverlo en async obliga a reescribirlo.
TcpStream::try_clone() da el split lectura/escritura: un hilo escribe frames, otro lee ACKs.
set_nodelay(true) es obligatorio con go-back-N — sin él, Nagle interactúa con el ACK retardado y produce pausas de cientos de milisegundos.
set_read_timeout(Some(...)) da el despertar periódico para comprobar cancelación.
shutdown(Shutdown::Both) desde otro hilo hace que un read bloqueado retorne.
Coordinación con std::sync::mpsc, Mutex y Condvar, todo en std.
Medido, por si te tienta: tokio con sólo net+rt son 5 crates; completo, 19; smol, 32; async-std, 34 y deprecado por sus propios autores.
Cero dependencias externas nuevas. Este proyecto lleva seis sprints sin añadir ninguna. Si crees que necesitas una, para y explica antes de añadirla.
7. Los tres problemas de verdad de este sprint
No son «abrir un socket». Son estos, y la ADR de la Fase 1 tiene que decidirlos:
7.1 — Un read de TCP no es un mensaje. TcpStream::read devuelve los bytes que haya, que pueden ser medio frame, tres frames y medio, o un byte. El decodificador incremental de qyro_protocol resuelve el reensamblado; lo que tienes que decidir tú es el tamaño del búfer de lectura, qué pasa cuando el decodificador se envenena, y cómo se distingue un peer lento de un peer muerto.
7.2 — Un socket es memoria que un desconocido controla. Antes del handshake, el peer no está autenticado y puede mandar lo que quiera. Decide y prueba: cuántos bytes acepta el proceso antes de que el handshake termine, cuánto tiempo puede tardar un handshake antes de cortar, y cuántas conexiones simultáneas acepta un listener. Un peer que abre 10 000 conexiones y no dice nada es la denegación de servicio más barata que existe.
7.3 — Un cierre no es un final. Hay al menos cinco formas de que una transferencia acabe y son distintas: terminó bien; el emisor canceló; el receptor rechazó; la conexión se cortó a mitad; el proceso remoto murió. El motor ya tiene rechazos tipados para las tres primeras. Las dos últimas son nuevas y llegan como un read que devuelve Ok(0) o un Err(ConnectionReset). Decide qué error tipado produce cada una y no las mezcles en un Io(...) genérico.
8. Las seis fases
Fase 0 — Línea base verificada por ti
Antes de escribir nada:
Clona, crea la rama, y reproduce: cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, cargo fmt --all --check, los cuatro check_*, y wc -l de Cargo.lock contando paquetes.
Anota los números que obtienes tú, no los que dice STATUS.
Lee qyro_protocol/src/ entero. Especialmente el decodificador incremental: su API, qué hace cuando se envenena, y qué errores distingue.
Lee qyro_transfer/src/session.rs entero. Especialmente cómo Sender y Receiver producen y consumen bytes, y qué esperan del llamante.
Lee ADR-0018 y ADR-0026.
Puerta 0: los números de la línea base coinciden con los declarados, o has escrito en qué difieren. Sabes decir, sin volver a mirar, cuál es la firma exacta por la que el decodificador incremental recibe bytes.
Fase 1 — ADR-0028, congelada antes de una sola línea de código
docs/adr/ADR-0028-network-transport.md. Decide, con el razonamiento escrito y las alternativas descartadas:
El framing sobre el stream. ¿El frame de 48 bytes + payload va tal cual sobre TCP, o lleva un prefijo de longitud propio? Pista fuerte: la cabecera ya lleva la longitud del payload y está autenticada por el AEAD. Un prefijo de longitud fuera del AEAD es un campo que un atacante puede alterar sin romper el tag. Decide, y di qué garantiza tu elección.
El tamaño del búfer de lectura, con el número y el porqué. No «8192 porque sí».
Los límites antes de autenticar (§7.2): bytes máximos, tiempo máximo de handshake, conexiones simultáneas máximas. Los tres con número.
Los timeouts: de lectura, de escritura, de inactividad. Y cómo se distingue un peer lento de uno muerto — un archivo de 4 GiB por Wi-Fi lento no puede parecer una conexión colgada.
La taxonomía de finales (§7.3): los cinco casos, el error tipado de cada uno, y cuál de ellos deja la sesión en Poisoned.
El modelo de hilos: cuántos hilos por conexión, quién es dueño de qué, y cómo se cancela desde fuera. Nombra el mecanismo (shutdown, bandera atómica, canal) y di por qué ése.
Quién escucha y quién marca. ¿Los dos extremos hacen las dos cosas, o hay un rol? Y cómo se elige el puerto.
Lo que esta ADR no promete. Sección obligatoria, con al menos: no hay descubrimiento, no hay NAT ni internet, no hay reconexión automática, no está probado en hardware físico, y lo que no hayas probado en Windows.
Puerta 1 (protocolo de §12). Además: la ADR está commiteada antes del primer commit de código y se puede comprobar en el historial.
Fase 2 — El transporte crudo: bytes por un socket
Crate nuevo rust/crates/qyro_net. Con #![forbid(unsafe_code)] y el mismo bloque #![deny(clippy::unwrap_used, expect_used, panic, unreachable, todo, unimplemented, indexing_slicing)] que usan qyro_fs y qyro_transfer.
Lo que existe al final de esta fase:
Un listener que acepta una conexión en un puerto y aplica los límites de la ADR §3.
Un dialer que conecta a IpAddr:puerto con timeout.
Un FrameStream que envuelve un TcpStream y ofrece dos operaciones: escribir un frame completo, y leer el siguiente frame alimentando el decodificador incremental de qyro_protocol, con el búfer acotado de la ADR §2.
Errores tipados propios, uno por cada final de §7.3. Ninguno se llama Io a secas.
Pruebas de esta fase, todas sobre sockets reales en 127.0.0.1:
a_frame_split_across_three_reads_is_reassembled — escribe un frame en tres write con pausas y comprueba que llega uno solo, íntegro.
three_frames_in_one_read_are_all_delivered — el caso contrario.
a_peer_that_sends_nothing_times_out_and_says_so — con el error tipado correcto, no un Io.
a_peer_that_disconnects_mid_frame_is_a_typed_end — cierra el socket a mitad de un frame y comprueba el error exacto.
a_peer_cannot_make_us_buffer_more_than_the_declared_limit — manda una cabecera que declara un payload gigantesco y comprueba que el proceso no reserva esa memoria. Instruméntalo con un contador bajo cfg(test), no con un cronómetro. Y mide lo que realmente se reservó, no la constante del límite — ver §11, es un error que este proyecto acaba de cometer.
a_legitimate_frame_still_round_trips — la comprobación de que los rechazos no pasan por rechazarlo todo.
Puerta 2 (protocolo de §12).
Fase 3 — El handshake y el sellado, sobre socket real
Al final de esta fase, dos extremos en el mismo proceso pero en hilos distintos, comunicados por un socket real de 127.0.0.1, completan el handshake de cuatro mensajes y se mandan frames sellados.
El handshake de qyro_crypto corre sobre FrameStream. No inventes un handshake nuevo ni un doble. Si tienes que cambiar algo de qyro_crypto para que quepa, eso es un hallazgo y hay que registrarlo, igual que 5A registró QYR-0068 y QYR-0069.
Los límites de §7.2 se aplican durante el handshake, no después.
Un peer que falla la firma se rechaza sin que un solo byte de aplicación pase.
Pruebas:
two_endpoints_over_a_real_socket_agree_on_a_session_key — y compara las huellas de sesión de los dos lados, que son valores distintos calculados por caminos distintos, no la misma llamada dos veces.
a_peer_with_a_wrong_signature_never_reaches_the_application — y comprueba que nada llegó al otro lado.
a_handshake_that_stalls_is_cut_by_the_deadline — con el número de la ADR.
a_flipped_bit_after_the_handshake_poisons_the_session — tres aserciones: la variante exacta, la sesión en Poisoned, y nada entregado.
Puerta 3 (protocolo de §12).
Fase 4 — Dos procesos de verdad
Ésta es la fase que define el sprint. Dos hilos en el mismo proceso no prueban lo mismo que dos procesos: comparten el asignador, el estado global y el runtime de pruebas.
Un binario de prueba —bajo tools/ o como [[bin]] del crate— que corre en dos modos: serve <puerto> <directorio-destino> y send <ip:puerto> <archivo...>.
Una prueba de integración que lanza el proceso servidor con std::process::Command, espera a que escuche, lanza el cliente, y espera a los dos.
El archivo llega byte a byte idéntico, comparado byte a byte y no por veredicto.
Y el archivo tiene que ser grande de verdad —al menos 8 MiB— para que la ventana, el go-back-N y el control de flujo se ejerciten realmente. Genera el contenido desde una semilla, no lo guardes: lo que se mide es el motor, no el fixture.
Pruebas:
a_file_crosses_two_real_processes_byte_identical
the_receiver_refuses_a_file_whose_digest_does_not_match — corrompe un byte en vuelo y comprueba que no aparece el archivo final y que no queda .qyro-part.
memory_held_by_the_sender_does_not_grow_with_the_file — contador bajo cfg(test) que registre el tamaño real de los búferes vivos, no una constante.
Puerta 4 (protocolo de §12). Y una comprobación extra específica: corre esta prueba diez veces seguidas. Una prueba de red que pasa una vez y falla la tercera es peor que no tenerla. Si hay flakiness, es un hallazgo y va al reporte con su causa, no un sleep más largo.
Fase 5 — Lo que pasa cuando algo va mal
Los cinco finales de §7.3, cada uno provocado de verdad y comprobado.
El emisor cancela a mitad — el receptor se entera con el error tipado correcto y no deja el archivo final.
El receptor rechaza el manifest — el emisor se entera y para; no sigue mandando chunks a un peer que dijo que no.
El proceso remoto muere a mitad — mátalo de verdad con Child::kill(). El superviviente produce el error tipado de §7.3, no un pánico y no un cuelgue.
La conexión se corta a mitad — shutdown desde el otro lado.
Un peer que abre conexiones y no habla — el listener no se queda sin recursos. Con el número de la ADR §3.
Y dos de recursos:
Ningún hilo queda vivo después de que una transferencia termine, de cualquiera de las cinco formas. Cuéntalos.
Ningún descriptor de archivo queda abierto. En Linux se cuenta con /proc/self/fd.
Puerta 5 (protocolo de §12).
Fase 6 — Guardas, barrido completo y reporte
guards.rs en qyro_net con include! del análisis compartido —sin modificarlo— y las cuatro guardas que usan los demás crates: no_production_path_can_panic, every_production_file_is_listed, every_..._error_has_a_construction_site, y la de forbid(unsafe_code). Recuerda QYR-0071: ese análisis leía menos de la mitad de un archivo hasta hace un sprint. Comprueba que assert_analysis_reached_the_end pasa sobre cada uno de tus archivos nuevos y di el número de bytes analizados de cada uno.
Barrido de mutación completo sobre todo lo escrito en este sprint. Por cada control de producción: bórralo, corre la suite, anota qué test falló. Un control que sobrevive a su propio borrado no está cubierto — arréglalo o regístralo con ID de ledger en el reporte.
El reporte de §13.
Puerta 6 (protocolo de §12) + el reporte completo + los seis workflows en verde sobre el commit final.
9. No objetivos — estrictos
Descubrimiento, mDNS, Bonjour, NsdManager. Nada. La dirección se la pasa el llamante. El descubrimiento es nativo por plataforma y es otro sprint.
FFI. qyro_ffi no cambia y sigue sin alcanzar qyro_crypto, qyro_transfer, qyro_fs ni qyro_net.
UI. Los botones siguen deshabilitados.
Los archivos de §5. En particular qyro_fs y header.rs, que los está tocando el otro agente ahora mismo.
QYR-0068. Los tres identificadores de la cabecera siguen sin setter público y tú no se los pones. Los está resolviendo el otro agente con su propia ADR. Si tu diseño los necesita, eso es un hallazgo y va al reporte, no una excusa para tocar header.rs.
Los tres huecos de prueba de qyro_fs (QYR-0073, 0074, 0075). Son del otro agente.
Reconexión automática, NAT traversal, internet, IPv6 más allá de que funcione si el IpAddr es v6.
TLS. Qyro tiene su propio handshake autenticado; añadir TLS encima sería cifrar dos veces y traer una dependencia enorme.
Android Keystore, iOS Keychain, historial, emparejamiento, release.
Si encuentras algo fuera de alcance, regístralo en tu reporte con un ID propuesto QYR-00XX y sigue.
10. Criterios de aceptación
ADR-0028 congelada antes del primer commit de código, comprobable en el historial.
Las seis puertas pasadas y escritas en el reporte con su resultado.
Un archivo de al menos 8 MiB cruza dos procesos de sistema operativo distintos por un socket TCP y llega byte a byte idéntico, comparado byte a byte.
La prueba anterior pasa diez veces seguidas.
Los cinco finales de §7.3 tienen error tipado propio y prueba que lo produce.
Un peer no autenticado no puede hacer que el proceso reserve más memoria que el límite de la ADR, medido con un contador que registre lo reservado de verdad.
Ningún hilo ni descriptor queda vivo tras terminar, de las cinco formas.
Cero dependencias externas nuevas. Cargo.lock pasa de 61 a 62 paquetes y el que entra es qyro_net.
Barrido de mutación completo, con tabla, y cero controles supervivientes sin registrar.
assert_analysis_reached_the_end pasa sobre cada archivo nuevo, con el número de bytes analizados de cada uno en el reporte.
#![forbid(unsafe_code)] en qyro_net; la lista de crates exentos sigue con tres entradas.
cargo fmt --all --check, cargo clippy --workspace --all-targets -- -D warnings, cargo test --workspace, cargo test --doc, cargo audit --deny warnings: PASS. Las pruebas suben desde 388; di cuánto y qué prueba cada una.
Los cuatro check_* pasan en Bash y en PowerShell.
Los seis workflows en success sobre el commit final, por push, con todos los runs de la rama listados, incluidos los fallidos y los cancelados.
git diff --name-only origin/main...HEAD no contiene ni un solo archivo de la lista prohibida de §5. Pega la salida literal en el reporte.
git status --short limpio. Sin commits en main, sin merge, sin PR, sin force-push.
11. Las trampas que este proyecto ya ha pisado — no las repitas
Cinco, y las cinco han ocurrido de verdad en este repositorio:
Una aserción cuyos dos lados son la misma llamada. Cinco veces. La última, hace tres días, en una prueba de seguridad: digest_of(&victim) == digest_of(&victim) dentro de un assert!. Antes de commitear, lee cada aserción nueva y comprueba que los dos términos pueden diferir.
Un contador que registra una constante en vez de lo medido. Hace tres días: PEAK_BUILDER_READ.fetch_max(HASH_BUFFER_LEN, ...) seguido de assert_eq!(peak, HASH_BUFFER_LEN). La prueba «no carga el archivo en memoria» pasaba aunque el archivo se cargara entero. Tu contador tiene que registrar el valor que salió de la operación, no el que esperabas.
Una prueba cuyo nombre enuncia una propiedad y no la ejerce. Hace tres días: a_symlink_at_the_final_component_is_refused nunca abría un archivo, y desactivar O_NOFOLLOW entero dejaba los 388 tests en verde.
Una cota extrapolada de una muestra de uno.
Un Ok que nadie mira. Hace tres días: FileSink escribe metadatos de reanudación que ningún código de producción lee jamás.
Regla general, y es la del proyecto: por cada propiedad que declares probada, borra el control que la produce y comprueba que alguna prueba falla con nombre. Una propiedad que sobrevive al borrado de su propio control no está cubierta.
12. El protocolo de puerta — idéntico en las seis fases
No pasas de fase hasta que las nueve comprobaciones pasan.
cargo fmt --all --check — PASS.
cargo clippy --workspace --all-targets -- -D warnings — PASS.
cargo test --workspace — PASS, sin tests ignorados nuevos.
Barrido de mutación de la fase. Por cada propiedad nueva: aplica la mutación que debería romperla, confirma que falla un test con nombre, restaura. Si sobrevive, no has terminado la fase.
Lectura de aserciones. Lee cada assert!/assert_eq! nuevo y comprueba que los dos lados pueden diferir. Trampa 1 de §11.
Lectura de contadores. Si la fase añadió un contador bajo cfg(test), comprueba que registra un valor derivado de la operación. Trampa 2.
Lectura de nombres. Por cada test nuevo: ¿el cuerpo ejerce lo que el nombre dice? Trampa 3.
git diff --name-only de la fase — ni un archivo de la lista prohibida de §5.
Escribe el resultado de la puerta en docs/reports/6A-claude-code.md antes de empezar la fase siguiente. Fecha, comprobaciones, tabla de mutación de la fase, y lo que encontraste.
Si una comprobación falla: arréglalo y repite la puerta entera. No la parchees en la fase siguiente.
Si no puedes arreglarlo: para, escribe por qué en el reporte, y reporta. Una fase declarada cerrada que no lo está envenena todo lo que viene detrás — es exactamente lo que pasó con QYR-0071, que hizo que cuatro sprints de evidencia estructural midieran menos de lo que decían.
13. El reporte — docs/reports/6A-claude-code.md
Créalo en la Fase 1 y ve escribiéndolo fase a fase, no al final. Es un requisito, no un resumen.
Dieciséis secciones, todas obligatorias:
El prompt recibido, verbatim y completo. No parafraseado.
Qué hiciste, punto por punto contra los objetivos de §8.
Cómo lo hiciste: las decisiones de ADR-0028 y las alternativas descartadas, con el motivo.
Errores detectados — todo lo que encontraste y no estaba en el prompt.
Cuáles arreglaste y cuáles no, y para los que no: por qué no, con ID QYR-00XX propuesto.
A qué afectaba cada defecto: qué se rompía, para quién, en qué escenario.
Resultado final contra el objetivo, objetivo por objetivo: cumplido / parcial / no hecho. «Parcial» es una respuesta válida; «cumplido» sin evidencia no lo es.
Clase de evidencia por cada afirmación: compilado / probado en unidad / probado en integración / probado entre dos procesos / probado en emulador / probado en simulador / probado en hardware físico. Una afirmación sin clase se audita como no probada. No conviertas «compiló en Linux» en «funciona».
Las seis puertas, con su resultado y su fecha.
Tabla de mutación completa: propiedad → mutación aplicada → test que falló → commit. Y los controles que sobrevivieron, si los hubo, con ID.
Tests antes (388) y después, con una línea por test nuevo diciendo qué prueba.
Delta de dependencias: paquetes antes (61) y después, y el diff de Cargo.lock.
git diff --name-only origin/main...HEAD, salida literal. Es la prueba de que no pisaste al otro agente.
Todos los runs de CI de la rama, sin filtrar, con ID, commit, workflow y conclusión. Los fallos y las cancelaciones también. Una lista de la que se caen los fallos no es evidencia, es un resumen favorable.
Qué NO debe leerse como progreso. La sección más importante. Como mínimo: no hay descubrimiento, no hay FFI, no hay UI, los botones siguen deshabilitados, no hay persistencia de identidad en Android ni iOS, y nada se ha probado en hardware físico ni entre dos máquinas distintas — dos procesos en 127.0.0.1 no son dos dispositivos en una Wi-Fi.
Qué documentación del repositorio quedó desfasada por lo que hiciste, y qué necesita saber el sprint siguiente.
14. Commits sugeridos
docs: freeze ADR-0028 before opening a single socket
feat(net): a frame stream over TCP that feeds the incremental decoder
test(net): a frame split across three reads is reassembled
test(net): an unauthenticated peer cannot make us buffer without bound
feat(net): typed ends for the five ways a transfer stops
feat(net): run the four-message handshake over a real socket
test(net): a peer with a wrong signature never reaches the application
feat(net): a listener and a dialer with the deadlines ADR-0028 froze
test(net): a file crosses two real processes byte identical
test(net): a remote process killed mid-transfer is a typed end, not a hang
test(net): no thread and no descriptor survives a finished transfer
test(guards): the shared analysis reaches the end of every new file
docs(report): sprint 6A, gate by gate
15. Si te quedas sin contexto
Para después de una puerta, nunca a mitad de una fase. Escribe en el reporte en qué puerta estás y qué falta. Deja la rama en verde. No dejes una fase declarada cerrada que no pasó su puerta.
Prohibido: tocar cualquier archivo de la lista prohibida de §5, añadir una dependencia sin justificarla y pararte a preguntar, meter un runtime async, inventar un handshake propio, usar unsafe, dejar la rama en rojo, omitir un run fallido del reporte, declarar una fase cerrada sin su puerta, o commitear en main.
16. Próxima tarea (no la empieces)
Sprint 6B — el descubrimiento LAN, que es nativo por plataforma: NsdManager en Android, NWBrowser en iOS, mdns-sd sólo bajo cfg(windows). Detrás de un trait PeerDiscovery en el core, con cero dependencias en móvil. Y el fallback de IP manual o QR desde el primer día, porque es lo único que funciona en el 100 % de las redes.
Después: el FFI del motor con NativeCallable.listener para el progreso, el selector de archivos, y la UI.
```

---

## 2. Qué hice, punto por punto contra §8

| Fase | Objetivo de §8 | Estado |
|---|---|---|
| 0 | Línea base reproducida por mí, no heredada | **Hecha.** Puerta 0 pasada, §9 |
| 1 | ADR-0028 congelada antes de una sola línea de código | Pendiente |
| 2 | `qyro_net`: listener, dialer, `FrameStream`, errores tipados, seis pruebas sobre sockets reales | Pendiente |
| 3 | El handshake de cuatro mensajes sobre socket real, cuatro pruebas | Pendiente |
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

Pendiente — Fase 1.

---

## 4. Errores detectados que no estaban en el prompt

| # | Hallazgo | Fase | Dónde |
|---|---|---|---|
| 1 | Falso verde propio en la reproducción de la línea base: `&&` tras una tubería lee el estado de `tail`, y la ruta `rust/Cargo.toml` no existe. Detectado y rehecho en la misma fase; no llegó a ningún commit ni a ninguna afirmación del informe salvo ésta | 0 | §2 |

Pendiente de ampliar conforme avancen las fases.

---

## 5. Cuáles arreglé y cuáles no

| # | Arreglado | Cómo, o por qué no | ID propuesto |
|---|---|---|---|
| 1 | Sí | Medición rehecha capturando `$?` de cada proceso. La tabla de la línea base de §2 sale de la segunda medición | — |

---

## 6. A qué afectaba cada defecto

**Hallazgo 1 — el falso verde de la línea base.** Qué se rompía: nada en el producto;
el defecto estaba en mi propio instrumento de medida. Para quién: para el siguiente
lector de este informe y para el supervisor, que es precisamente quien no pudo
verificar la evidencia de 5B.1 y a quien se le pidió a este sprint que reprodujera la
línea base por su cuenta. En qué escenario: si no llego a mirar el resto de la
salida, la Fase 0 se habría declarado cerrada con una línea base que no se midió, que
es la misma forma de fallo que QYR-0071 —una fase declarada cerrada que no lo estaba—
y que el propio §12 del prompt cita como el motivo de que cuatro sprints de evidencia
estructural midieran menos de lo que decían.

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

### Puertas 1 a 6

Pendientes.

---

## 10. Tabla de mutación

Pendiente. Una fila por control de producción: propiedad → mutación aplicada → test
que falló → commit. Los controles que sobrevivan a su propio borrado se registran con
ID de ledger, no se ocultan.

La Fase 0 no añade controles de producción, así que no tiene filas.

---

## 11. Tests antes y después

**Antes: 388 passed, 0 failed, 2 ignored** (medido, no heredado — §2).
Después: pendiente. Una línea por test nuevo diciendo qué prueba.

---

## 12. Delta de dependencias

**Antes: 61 paquetes en `Cargo.lock`** (medido con `grep -c '^\[\[package\]\]'`).
Después: pendiente. El objetivo de §10 es 62, y el que entra debe ser `qyro_net`.
Cero dependencias externas nuevas.

---

## 13. `git diff --name-only origin/main...HEAD`

Salida literal, pendiente hasta que haya commits. La Fase 0 no modificó ningún
archivo del repositorio.

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
