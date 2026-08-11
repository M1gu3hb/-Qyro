# Sprint 5C — cierre de huecos de evidencia

- Rama: `codex/qyro-gap-closure-5c`
- Base verificada: `15934aae3dda7f469b5496c8341eb78d9e32f335`
- Fecha de trabajo: 2026-08-11

## 1. Prompt recibido, verbatim y completo

```text
0. Bienvenida y contexto — léelo entero antes de tocar nada

Bienvenido al proyecto Qyro. Es tu primera sesión aquí, así que esto empieza por el contexto completo. No es relleno: sin él, la mitad de las instrucciones de abajo no se entienden.

0.1 Qué es Qyro

Una aplicación para mandar archivos de un dispositivo a otro directamente por la red local, sin nube, sin cuentas, sin servidor, sin anuncios y sin telemetría. Android, iOS y Windows. La misma categoría que AirDrop o LocalSend, con una diferencia de enfoque: la parte criptográfica se construyó primero y con un nivel de verificación poco habitual, y la parte de producto vino después.

Hoy Qyro no transfiere archivos. Los botones «Enviar» y «Recibir» están en la pantalla y deshabilitados a propósito, en el código (onPressed: null), con un texto que lo explica. Esa decisión es deliberada y sigue en pie: nada se habilita hasta que exista una transferencia real, cifrada y comprobada de extremo a extremo.

0.2 Cómo está hecho

Flutter/Dart para la interfaz + un núcleo en Rust + un puente FFI deliberadamente estrecho. Rust 1.88, edition 2024. Ocho crates:

Crate	Qué hace
qyro_protocol	Cabecera fija de 48 bytes, tipos de mensaje, decodificador incremental acotado
qyro_manifest	La lista de lo que se envía: rutas, tamaños, SHA-256 obligatorio, y validación dura de rutas
qyro_crypto	Identidad Ed25519, handshake autenticado de 4 mensajes, HKDF, ChaCha20-Poly1305, ventana de replay
qyro_identity_store	Formato del blob de identidad en disco y el contrato por plataforma
qyro_win_dpapi	Envoltorio de Windows (DPAPI)
qyro_transfer	El motor: chunks, ventana, go-back-N, pausa/cancelación, veredicto por archivo
qyro_fs	FileSource/FileSink sobre archivos reales, .qyro-part, rename tras digest verificado
qyro_ffi	El puente hacia Dart

El puente es estrecho a propósito y hay una prueba que lo garantiza: qyro_ffi no puede alcanzar qyro_crypto, y una prueba consulta al compilador por el cierre transitivo de dependencias y falla si alguien conecta los dos. El motivo: si Dart nunca puede pedir una clave, no hay forma de que una clave se escape por ahí. La seguridad no depende de que nadie escriba mal el código; depende de que el camino no exista.

Cero dependencias externas. El workspace lleva seis sprints sin añadir una sola. Cargo.lock tiene 61 paquetes y todos son de primera parte o ya estaban. Esto no es estética: cada crate es superficie de cadena de suministro en una app que mueve archivos del usuario y maneja claves.

0.3 La cultura de verificación de este proyecto — esto es lo importante

Este repositorio tiene una regla que gobierna todo lo demás:

Por cada propiedad que declares probada, borra el control que la produce y comprueba que alguna prueba falla con nombre. Una propiedad que sobrevive al borrado de su propio control no está cubierta.

Se llama barrido de mutación y se hace a mano, en cada sprint. No es opcional y no es un extra: es el criterio con el que se decide si algo está terminado.

Y hay una segunda regla, sobre el lenguaje:

Nunca conviertas «compiló en Linux» en «funciona en todas las plataformas».

El proyecto distingue con rigor entre compilado, probado en unidad, probado en integración, probado entre procesos, probado en emulador, probado en simulador, probado en hardware físico. Hoy no hay ni una sola prueba en hardware físico, y está dicho así en la documentación.

Y existe un supervisor externo que audita cada sprint contra el código real, reproduce las pruebas, aplica sus propias mutaciones y emite un veredicto independiente. No basta con que tu reporte diga que algo funciona. Se comprueba.

0.4 Los cinco anti-patrones que este repositorio ya produjo

No son hipotéticos. Los cinco han ocurrido aquí, y tres de ellos hace tres días, en el sprint cuyo trabajo vienes a cerrar:

Una aserción cuyos dos lados son la misma llamada. Ha pasado cinco veces. La última: digest_of(&victim).unwrap() == digest_of(&victim).unwrap() dentro de un assert!, en una prueba de seguridad. Es true siempre.
Un contador que registra una constante en vez de lo medido. PEAK_BUILDER_READ.fetch_max(HASH_BUFFER_LEN, ...) seguido de assert_eq!(peak, HASH_BUFFER_LEN).
Una prueba cuyo nombre enuncia una propiedad y no la ejerce.
Una cota extrapolada de una muestra de uno.
Un Ok que nadie mira — un artefacto que se escribe y que ningún código de producción lee jamás.

Tu sprint consiste, casi entero, en arreglar instancias concretas de estos cinco.

0.5 Lo primero que haces: leer el repositorio

Antes de escribir una línea de código, lee, en este orden:

STATUS.md — el estado declarado. Léelo con escepticismo: parte de lo que afirma es lo que vienes a corregir.
BUGS_PENDING.md — el ledger. 71 entradas, 20 abiertas. Es el registro histórico de todo lo que se encontró y no se arregló, con severidad y motivo.
docs/adr/ADR-0027-filesystem-materialisation.md — la decisión congelada que gobierna qyro_fs. Es la que tu Fase 4 tiene que reconciliar con el código.
docs/adr/ADR-0026-* — las costuras ContentSource/ContentSink y el formato IntegrityResult.
rust/crates/qyro_fs/src/ entero: io.rs, safe_path.rs, resume.rs, manifest_builder.rs, error.rs, guards.rs, tests.rs. Son 1 417 líneas. Léelas todas.
rust/guards/source_guard.rs — el análisis estructural compartido por todos los crates. Acaba de tener un P1: leía 13 401 bytes de un archivo de 30 861 y nadie lo notaba durante un sprint entero (QYR-0071).
rust/crates/qyro_protocol/src/header.rs — la cabecera de 48 bytes y la nota sobre QYR-0068, que es tu Fase 5.
AGENTS.md y CODEX.md si existen, y CONTRIBUTING.md.
1. Título del sprint

Sprint 5C — Cerrar los tres huecos de prueba de qyro_fs y darle a la cabecera los identificadores que la red va a necesitar.

Cuatro entradas del ledger, cada una con la mutación que la demuestra ya escrita. No estás explorando: estás ejecutando contra criterios que ya existen.

2. Cómo se trabaja en este sprint — léelo antes que nada

Este prompt tiene seis fases y cada una termina en una puerta de auto-auditoría que tienes que pasar antes de empezar la siguiente. No avances de fase con la anterior a medias. Si una puerta no pasa, arréglalo y vuelve a pasarla. Si no puedes, para y reporta.

El protocolo de puerta está en §11 y es idéntico para las seis. Léelo ahora.

Trabajas en paralelo con otro agente (Claude Code) que está construyendo la capa de red en el mismo repositorio, al mismo tiempo, en otra rama. Las reglas de no interferencia son §4 y son estrictas.

3. Repositorio, rama y HEAD

https://github.com/M1gu3hb/-Qyro

Rama base: claude/qyro-filesystem-5b1. Crea una rama nueva: codex/qyro-gap-closure-5c.

git fetch --all --prune
git rev-parse origin/claude/qyro-filesystem-5b1
# debe imprimir 15934aae3dda7f469b5496c8341eb78d9e32f335

Si imprime otra cosa, detente y reporta.

Línea base a reproducir: cargo test --workspace → 388 passed, 0 failed, 2 ignored. 61 paquetes en Cargo.lock. Clippy y fmt limpios. Los cuatro check_* en verde.

Aviso de honestidad: el sprint anterior declara seis workflows en verde sobre e3fbaf1 con IDs 31232028441 / 378 / 429 / 435 / 405 / 433. El supervisor no pudo verificarlos (la API de GitHub devolvió 403 desde su entorno). Reproduce tú la línea base localmente y no des por buena la tabla ajena.

4. Reglas de no interferencia — un agente en paralelo

El otro agente trabaja al mismo tiempo en claude/qyro-net-6a, creando un crate nuevo rust/crates/qyro_net.

Archivos que TÚ tocas:

rust/crates/qyro_fs/**
rust/crates/qyro_protocol/src/header.rs y sus pruebas
rust/guards/source_guard.rs — eres el único que puede modificarlo
docs/adr/ADR-0027-filesystem-materialisation.md — sólo si la Fase 4 concluye que hay que enmendarla, y con enmienda fechada, sin reescribir la historia
docs/adr/ADR-0029-header-identifiers.md — nuevo, Fase 5
docs/reports/5C-codex.md — tu reporte

Archivos que NO tocas bajo ninguna circunstancia:

rust/crates/qyro_net/** — no existe todavía; lo está creando el otro agente
Cargo.toml de la raíz y Cargo.lock — el otro agente añade una línea a members. Tú no creas crates, así que no los necesitas
STATUS.md, HANDOFF.md, NEXT_STEPS.md, CHANGELOG.md, BUGS_PENDING.md, DECISIONS.md — ninguno de los dos agentes los toca en este run
.github/workflows/**
main — no commits, no merge, no PR, no rebase, no force-push

Esto es un cambio de proceso deliberado. Todo lo que normalmente iría en STATUS.md y BUGS_PENDING.md va en tu reporte de docs/reports/5C-codex.md, con la estructura de §12. El supervisor consolida los seis documentos raíz después.

Cuidado especial con rust/guards/source_guard.rs: lo comparten todos los crates por include!, incluido el crate nuevo que el otro agente está escribiendo. Si lo rompes, rompes su sprint también. Cualquier cambio ahí tiene que dejar los cinco crates existentes en verde, y lo compruebas con cargo test --workspace, no sólo con -p qyro_fs.

5. Los cuatro trabajos, con su evidencia ya reproducida

El supervisor ya aplicó estas mutaciones. Los números son suyos, del 2026-08-11, y tu Fase 1 consiste en reproducirlos antes de arreglar nada.

QYR-0073 — P1. El control O_NOFOLLOW no tiene ninguna prueba, y el código afirma que sí

rust/crates/qyro_fs/src/io.rs líneas 69-75 dicen, textualmente:

«The value is fixed by the platform ABI, and a_symlink_at_the_final_component_is_refused is what proves the number is the right one: a wrong constant makes that test pass a write through the link, loudly.»

Es falso. Mutación aplicada: libc_o_nofollow() devuelve 0 en Linux/Android. Resultado: 388 de 388 tests en verde. El control de seguridad puede desaparecer entero sin que nada se entere.

La prueba que supuestamente lo demuestra tiene dos defectos:

Su aserción es una tautología —digest_of(&victim) == digest_of(&victim)—, y por tanto no puede fallar.
La prueba nunca abre ningún archivo. Sólo llama a resolve_under, que comprueba final_path (a.bin, que no es enlace) y nunca mira part_path (a.bin.qyro-part, que sí lo es). El open_part con O_NOFOLLOW —el único sitio donde el flag existe— no se ejecuta en ninguna prueba del repositorio.

El código de producción es correcto. La constante 0o400_000 es la correcta para Linux/Android y el control funciona. Lo que no existe es la protección contra la regresión. ADR-0027 §1 declara que O_NOFOLLOW «cierra la carrera del componente final por completo», y esa es la mitad fuerte de la política de symlinks — la que no tiene ventana.

Y las constantes de macOS/iOS (0x0000_0100) y el FILE_FLAG_OPEN_REPARSE_POINT de Windows (0x0020_0000) nunca se han ejercitado en ninguna plataforma.

QYR-0074 — P2. La prueba de memoria del manifest mide una constante contra sí misma

rust/crates/qyro_fs/src/manifest_builder.rs líneas 60-64:

rust
#[cfg(test)]
PEAK_BUILDER_READ.fetch_max(
    crate::io::HASH_BUFFER_LEN,      // ← la CONSTANTE, no lo que se leyó
    std::sync::atomic::Ordering::Relaxed,
);

Y tests.rs:167 afirma peak == HASH_BUFFER_LEN. Es HASH_BUFFER_LEN == HASH_BUFFER_LEN.

Mutación aplicada: digest_of carga el archivo entero con read_to_end en vez de leer por trozos. Resultado: la prueba building_a_manifest_from_disk_does_not_load_the_file pasa.

Contrasta con el sprint 5A, donde peak_content_held se medía de buffer.len() —el búfer real— y por eso sí capturaba. Aquí se copió la forma y no la sustancia: el contador está en el llamante, no en digest_of, que es donde se lee, y no registra nada que dependa de lo ocurrido.

QYR-0075 — P2. ADR-0027 §5 está congelada y no implementada

La ADR decide, literalmente:

«Un .qyro-part de una ejecución anterior: se reanuda si hay .qyro-resume que lo describa, y se descarta si no. — Con metadatos: Qyro trunca el .qyro-part a bytes_committed… y sigue. — Sin metadatos: … Se borra al empezar la transferencia que reclamaría ese nombre.»

En el código:

ResumeState::decode no tiene ni un solo llamante en producción. Sólo en tests.rs.
No hay set_len en ninguna parte. El truncamiento no existe.
No hay borrado del huérfano. El único remove_file está en finish_item, para el digest incorrecto.
part_for abre el .qyro-part existente con truncate(false) y toma written = metadata.len(), sea lo que sea ese archivo.

FileSink escribe metadatos que nada lee jamás. Es el anti-patrón 5.

Y las dos pruebas que deberían cubrirlo no lo cubren:

an_interrupted_transfer_resumes_from_its_metadata — es el test quien lee los metadatos y quien calcula el offset. El código de producción no participa. La propiedad vive en el harness.
a_leftover_part_file_is_recovered_or_discarded_by_policy — pasa por aritmética: el huérfano son 17 bytes (b"bytes nobody sent") y el contenido real 2048, así que la escritura desde offset 0 lo tapa entero.

Demostrado con un huérfano más largo que el contenido: huérfano de 8192 bytes, contenido real de 2048, sin .qyro-resume → Err(DigestMismatch { item_id: 1 }). La transferencia falla, y falla precisamente en el escenario para el que la reanudación existe.

No es un agujero: el digest es el respaldo y ningún archivo malo llega al usuario. Es una ADR congelada cuyo §5 no está implementado.

QYR-0068 — P2. Tres identificadores autenticados que nadie puede rellenar

La cabecera de 48 bytes reserva transfer_id, stream_id e item_id. Están dentro de los datos asociados del AEAD, así que un peer no puede alterarlos sin romper el tag — que es exactamente la propiedad que los haría valiosos. Y no hay forma pública de ponerles un valor: with_identifiers no es pub y Frame::new los deja en cero.

El sprint 5A lo encontró y deliberadamente no lo arregló, con esta nota en header.rs:

«Recorded as QYR-0068 with the decision left open… This comment does not decide it — widening a frozen surface as a side effect of another sprint is how control of a format is lost.»

Esa decisión fue correcta entonces y ahora toca revertirla con su propia ADR, porque el otro agente está construyendo la red ahora mismo y estos tres campos dejan de ser decorativos en cuanto haya dos transferencias o dos archivos en vuelo: son los campos con los que un receptor sabe a qué transferencia y a qué archivo pertenece un frame.

6. Las seis fases
Fase 0 — Leer el repositorio y reproducir la línea base
Lee todo lo de §0.5.
Reproduce: cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, cargo fmt --all --check, los cuatro check_*, y el conteo de paquetes.
Anota los números que obtienes tú.

Puerta 0: los números coinciden con los declarados o has escrito en qué difieren. Y sabes decir, sin volver a mirar, qué hace assert_analysis_reached_the_end y por qué existe.

Fase 1 — Reproducir los cuatro hallazgos antes de arreglar ninguno

Empiezas confirmando, no arreglando. Un arreglo cuyo defecto no reprodujiste primero es un arreglo que no sabes si arregla algo.

Aplica las cuatro mutaciones, una a una, y anota el resultado exacto:

#	Mutación	Resultado esperado
M1	libc_o_nofollow() devuelve 0 en Linux/Android	388/388 en verde — sobrevive
M2	digest_of usa read_to_end en vez de leer por trozos	building_a_manifest_from_disk_does_not_load_the_file pasa — sobrevive
M3	Escribe un .qyro-part huérfano de 8192 bytes antes de una transferencia de 2048, sin .qyro-resume	La transferencia falla con DigestMismatch
M4	grep de llamantes de ResumeState::decode fuera de tests.rs	Cero

Restaura el árbol después de cada una.

Si alguna no reproduce lo esperado, para y reporta: significa que el árbol que tienes no es el que se auditó, y todo lo demás se apoya en eso.

Puerta 1 (protocolo de §11). Los cuatro resultados escritos en el reporte, con el comando exacto que usaste.

Fase 2 — QYR-0073: darle una prueba de verdad a O_NOFOLLOW

Es el P1 y va primero.

Borra la prueba tautológica a_symlink_at_the_final_component_is_refused y escribe una que ejerza el control de verdad. Tiene que:
crear una víctima fuera de la raíz de destino, con contenido conocido;
crear un symlink real en <destino>/<nombre>.qyro-part apuntando a la víctima;
ejecutar una transferencia real a través de FileSink, no sólo resolve_under;
afirmar tres cosas: la víctima intacta byte a byte, el archivo final ausente, y el error tipado correcto.
Comprueba que la prueba nueva falla con O_NOFOLLOW = 0 y pasa con el valor correcto. Esa es la única evidencia que vale.
Corrige el comentario de io.rs:69-75 para que diga la verdad sobre qué prueba qué, y sobre qué plataformas no está probado.
Windows y macOS/iOS. Decide y ejecuta una de las dos:
escribe la prueba equivalente con un reparse point en Windows y déjala correr en el workflow platform-builds; o
registra explícitamente en el reporte, con ID de ledger propio, que las constantes de Windows (0x0020_0000) y macOS/iOS (0x0000_0100) no están probadas en ninguna plataforma, y que ADR-0027 §1.4 afirma una garantía que sólo está verificada en Linux.
La segunda opción es aceptable. Lo que no es aceptable es dejarlo sin decir.
Y revisa si hay otras aserciones tautológicas en qyro_fs/src/tests.rs. La Fase 6 va a añadir una guarda que las detecte; aquí basta con mirar.

Puerta 2 (protocolo de §11).

Fase 3 — QYR-0074: que el contador cuente lo que pasó
Mueve el contador dentro de digest_of, registrando el count que devuelve file.read(). Exactamente como el sprint 5A hizo con buffer.len() en qyro_transfer/src/session.rs — ve a leer ese código antes de escribir el tuyo, es el modelo correcto.
Comprueba que la prueba falla con la mutación M2 (read_to_end) y pasa sin ella.
Revisa todos los contadores bajo cfg(test) del crate: PEAK_BUILDER_READ, FileSource::peak_read, FileSink::peak_write. Por cada uno, aplica la mutación que debería moverlo y comprueba que se mueve. peak_write es especialmente sospechoso: mira dónde se registra respecto a dónde se escribe de verdad.

Puerta 3 (protocolo de §11).

Fase 4 — QYR-0075: reconciliar ADR-0027 §5 con el código

Hay dos salidas legítimas y tienes que elegir una con el razonamiento escrito.

Salida A — implementar §5. En part_for:

leer .qyro-resume si existe y corresponde a este transfer_id;
si hay entrada para el item, truncar el .qyro-part a bytes_committed con set_len — lo que haya después nunca se confirmó;
si no hay metadatos que lo describan, borrar el huérfano antes de empezar;
si el transfer_id del .qyro-resume no coincide, tratarlo como huérfano. Este caso no está en la ADR y es un hallazgo si lo encuentras tú.

Salida B — degradar §5 a pendiente. Enmienda fechada en la ADR diciendo que §5 no está implementado y por qué, más una entrada de ledger, más renombrar las dos pruebas para que sus nombres dejen de afirmar lo que no hacen.

La salida A es la correcta y es barata. Elige B sólo si al implementarla descubres algo que lo impida — y si lo descubres, eso es el hallazgo del sprint.

Sea cual sea la salida, arregla las dos pruebas:

an_interrupted_transfer_resumes_from_its_metadata — el código de producción tiene que leer los metadatos, no el test. Si el test los lee, la propiedad vive en el harness.
a_leftover_part_file_is_recovered_or_discarded_by_policy — el huérfano tiene que ser más largo que el contenido real, para que la prueba no pueda pasar por aritmética. Y añade el caso simétrico, con huérfano más corto.

Y añade el caso que ninguna de las dos cubre: un .qyro-part con .qyro-resume válido cuyos bytes committed son correctos — la reanudación que sí funciona, para que los rechazos no pasen por rechazarlo todo.

Puerta 4 (protocolo de §11). Con una comprobación extra: borra la lectura de .qyro-resume que acabas de escribir y comprueba que falla una prueba con nombre. Si sobrevive, no has cerrado nada.

Fase 5 — QYR-0068: los identificadores de la cabecera, con ADR

No pongas setters sin ADR. Ensanchar una superficie congelada de refilón es exactamente lo que 5A se negó a hacer, y con razón.

docs/adr/ADR-0029-header-identifiers.md, congelada antes del código. Decide:

Qué API pública se añade y cuál es su superficie mínima. Un constructor que los tome, un setter, o un tipo FrameIdentifiers — y por qué ése.
Qué es un valor válido. ¿Cero es «sin identificador» o es un identificador válido? Si cero es válido, todos los frames existentes dicen algo. Si no lo es, hay que rechazarlo. Decide y prueba la elección.
Qué garantiza que estén en el AAD y qué no. Un peer no puede alterarlos sin romper el tag. Eso no significa que sean correctos: significa que son los que el emisor puso. Escríbelo así.
Qué pasa si un receptor ve un transfer_id que no reconoce, o un item_id que no está en el manifest. Errores tipados, no Io.
Que el formato de 48 bytes NO cambia. Los campos ya están ahí. Esto es ensanchar la API, no el formato. Dilo explícitamente, porque es la diferencia entre esta ADR y un cambio de formato.
Lo que esta decisión no promete. Sección obligatoria.

Después, el código y las pruebas:

identifiers_survive_a_seal_and_open_round_trip
altering_an_identifier_in_flight_breaks_the_tag — voltea un bit de transfer_id en el frame sellado y comprueba que el open falla, no que devuelve otro valor.
the_forty_eight_byte_layout_is_unchanged — un vector de bytes fijo y esperado, no header.len() == 48.

Y actualiza la nota de header.rs. Hoy dice que la decisión queda abierta. Ya no lo está.

Puerta 5 (protocolo de §11).

Fase 6 — La guarda que impide que esto vuelva a pasar

Esto es lo que hace que el sprint valga más que sus cuatro arreglos. El anti-patrón de la aserción tautológica ha aparecido cinco veces. Una guarda estructural lo mata para siempre.

En rust/guards/source_guard.rs, añade assert_no_assertion_compares_a_call_to_itself: recorre los archivos de prueba de un crate, encuentra cada assert!/assert_eq!/assert_ne!, y falla si los dos lados de una comparación son textualmente idénticos tras normalizar espacios.
Empieza por el caso literal X == X y assert_eq!(X, X). No intentes resolver el caso general; resuelve el que ha ocurrido cinco veces.
Prueba la guarda con una tautología introducida a propósito y comprueba que falla. Una guarda que no se ha visto fallar no es una guarda — es la lección de QYR-0052/0053/0054 y de QYR-0071.
Si hay falsos positivos legítimos, exímelos por nombre y con el argumento escrito, como hace VERDICTS_WITH_NO_CONSTRUCTION_SITE. Nunca con un allow global.
Actívala en los cinco crates que ya usan el análisis compartido. Si alguno falla, has encontrado una tautología más: arréglala y regístrala.
cargo test --workspace tiene que quedar en verde, no sólo -p qyro_fs. El otro agente depende de este archivo.
Barrido de mutación completo sobre todo lo que tocaste en las fases 2 a 5. Tabla: control → mutación → test que falló. Cero supervivientes sin registrar.
El reporte de §12, completo.

Puerta 6 (protocolo de §11) + reporte + los workflows en verde sobre el commit final.

7. No objetivos — estrictos
rust/crates/qyro_net/** y Cargo.toml/Cargo.lock. Son del otro agente.
Red, sockets, descubrimiento. Nada.
FFI, UI, selector de archivos. Los botones siguen deshabilitados.
QYR-0072 — la carrera de los componentes intermedios de la ruta. Cerrarla exige openat/dirfd, que no está en std y en Windows es otra cosa; eso significa una dependencia nueva o unsafe, y las dos merecen su propia decisión. Sigue abierta a propósito.
Android Keystore, iOS Keychain, historial, emparejamiento, release.
Dependencias externas. Cero. Cargo.lock tiene que quedar idéntico: 61 paquetes, sin una línea de diferencia. Si crees que necesitas una, para y explica antes.

Si encuentras algo fuera de alcance, regístralo en tu reporte con un ID propuesto QYR-00XX y sigue.

8. Criterios de aceptación
Las cuatro mutaciones de la Fase 1 reproducidas y escritas, antes de cualquier arreglo.
Las seis puertas pasadas y escritas en el reporte con su resultado.
O_NOFOLLOW = 0 hace fallar una prueba con nombre. Es el criterio del P1 y no hay otro.
La prueba de O_NOFOLLOW ejecuta una transferencia real a través de FileSink, no sólo resolve_under.
Las constantes de Windows y macOS/iOS o están probadas, o están registradas como no probadas con ID de ledger.
digest_of con read_to_end hace fallar building_a_manifest_from_disk_does_not_load_the_file.
Los tres contadores bajo cfg(test) de qyro_fs registran valores derivados de la operación, y cada uno tiene una mutación que lo mueve.
ADR-0027 §5 implementada (salida A) o degradada con enmienda fechada y pruebas renombradas (salida B). No hay tercera opción.
Un .qyro-part huérfano más largo que el contenido real no rompe la transferencia.
Borrar la lectura de .qyro-resume hace fallar una prueba con nombre.
ADR-0029 congelada antes del código de la Fase 5, comprobable en el historial.
Alterar un identificador en vuelo rompe el tag, con prueba.
assert_no_assertion_compares_a_call_to_itself existe, se ha visto fallar con una tautología introducida a propósito, y está activa en los cinco crates.
Cargo.lock idéntico. 61 paquetes. Cero dependencias nuevas.
cargo fmt --all --check, cargo clippy --workspace --all-targets -- -D warnings, cargo test --workspace, cargo test --doc, cargo audit --deny warnings: PASS. Las pruebas suben desde 388; di cuánto y qué prueba cada una.
Los cuatro check_* pasan en Bash y en PowerShell.
Los workflows que dispare tu rama en success sobre el commit final, con todos los runs listados, incluidos los fallidos y los cancelados.
git diff --name-only origin/main...HEAD no contiene ni un archivo de la lista prohibida de §4. Pega la salida literal en el reporte.
git status --short limpio. Sin commits en main, sin merge, sin PR, sin force-push.
9. Lo que no vale como «arreglado»
Un test que pasa. Un test que pasa y que falla cuando borras el control.
Cambiar el nombre de una prueba sin cambiar lo que ejerce.
Un assert! más en la misma prueba tautológica. Bórrala y escribe otra.
«No pude probar Windows» sin registrar que la garantía de Windows queda sin verificar.
Marcar la Fase 4 como cerrada con la salida B sin haber intentado la A.
Un #[allow(...)] para que la guarda nueva no se queje.
10. Sobre reconocer errores

Si al reproducir la Fase 1 descubres que el supervisor se equivocó en alguno de los cuatro hallazgos —que la mutación no reproduce, o que hay una prueba que sí lo cubre y no se vio—, dilo, con la evidencia. El supervisor ya se ha equivocado antes en este proyecto: en el sprint 4D.2a mandó usar un harness que estructuralmente no podía funcionar en Android, y el sprint que lo detectó paró, lo documentó y no improvisó un harness falso. Eso fue lo correcto.

No arregles algo que no está roto sólo porque el prompt lo dice.

11. El protocolo de puerta — idéntico en las seis fases

No pasas de fase hasta que las nueve comprobaciones pasan.

cargo fmt --all --check — PASS.
cargo clippy --workspace --all-targets -- -D warnings — PASS.
cargo test --workspace — PASS. El workspace entero, no sólo tu crate. El otro agente depende de las guardas compartidas.
Barrido de mutación de la fase. Por cada propiedad nueva o arreglada: aplica la mutación que debería romperla, confirma que falla un test con nombre, restaura. Si sobrevive, no has terminado la fase.
Lectura de aserciones. Lee cada assert!/assert_eq! nuevo y comprueba que los dos lados pueden diferir. Anti-patrón 1 de §0.4.
Lectura de contadores. Si la fase tocó un contador bajo cfg(test), comprueba que registra un valor derivado de la operación. Anti-patrón 2.
Lectura de nombres. Por cada test nuevo o renombrado: ¿el cuerpo ejerce lo que el nombre dice? Anti-patrón 3.
git diff --name-only de la fase — ni un archivo de la lista prohibida de §4.
Escribe el resultado de la puerta en docs/reports/5C-codex.md antes de empezar la fase siguiente.

Si una comprobación falla: arréglalo y repite la puerta entera.

Si no puedes arreglarlo: para, escribe por qué en el reporte, y reporta. Una fase declarada cerrada que no lo está envenena todo lo que viene detrás — es exactamente lo que pasó con QYR-0071, que hizo que cuatro sprints de evidencia estructural midieran menos de lo que decían.

12. El reporte — docs/reports/5C-codex.md

Crea el directorio docs/reports/ si no existe. El otro agente hará lo mismo con su propio archivo; no hay conflicto porque cada uno añade un archivo distinto y git no versiona directorios vacíos.

Créalo en la Fase 1 y ve escribiéndolo fase a fase, no al final.

Dieciséis secciones, todas obligatorias:

El prompt recibido, verbatim y completo. No parafraseado.
Qué hiciste, punto por punto contra los objetivos de §6.
Cómo lo hiciste: las decisiones y las alternativas descartadas, con el motivo. En particular: por qué la salida A o la B en la Fase 4.
Errores detectados — todo lo que encontraste y no estaba en el prompt.
Cuáles arreglaste y cuáles no, y para los que no: por qué no, con ID QYR-00XX propuesto.
A qué afectaba cada defecto: qué se rompía, para quién, en qué escenario.
Resultado final contra el objetivo, objetivo por objetivo: cumplido / parcial / no hecho. «Parcial» es válido; «cumplido» sin evidencia no lo es.
Clase de evidencia por cada afirmación: compilado / probado en unidad / probado en integración / probado entre procesos / probado en emulador / probado en simulador / probado en hardware físico. Una afirmación sin clase se audita como no probada. En particular, di explícitamente en qué plataformas se ha ejercitado O_NOFOLLOW y en cuáles no.
Las seis puertas, con su resultado y su fecha.
Tabla de mutación completa: control → mutación aplicada → test que falló → commit. Incluidas las cuatro de la Fase 1 antes de arreglar. Y los controles que sobrevivieron, si los hubo, con ID.
Tests antes (388) y después, con una línea por test nuevo o renombrado diciendo qué prueba.
Delta de dependencias: tiene que ser cero. Dilo, y pega el git diff de Cargo.lock que debe estar vacío.
git diff --name-only origin/main...HEAD, salida literal. Es la prueba de que no pisaste al otro agente.
Todos los runs de CI de la rama, sin filtrar, con ID, commit, workflow y conclusión. Los fallos y las cancelaciones también. Una lista de la que se caen los fallos no es evidencia, es un resumen favorable.
Qué NO debe leerse como progreso. La sección más importante. Como mínimo: esto no mueve el producto, es cierre de deuda; no hay red, no hay FFI, no hay selector, los botones siguen deshabilitados, no hay persistencia de identidad en Android ni iOS, QYR-0072 sigue abierta, y nada se ha probado en hardware físico.
Qué documentación del repositorio quedó desfasada por lo que hiciste, y qué necesita saber el sprint siguiente. En particular, si la Fase 5 cambió la superficie de la cabecera, el agente de la red necesita saberlo.
13. Commits sugeridos
test(fs): reproduce the four findings before fixing any of them
test(fs): make O_NOFOLLOW fail loudly when it is not there
docs(fs): say what the symlink test actually proves, and on which platforms
test(fs): let every cfg(test) counter record what happened, not a constant
feat(fs): read the resume metadata that the sink has been writing
test(fs): an orphan part file longer than the payload is discarded
docs: freeze ADR-0029 before widening the header surface
feat(protocol): let a caller set the three authenticated identifiers
test(protocol): altering an identifier in flight breaks the tag
test(guards): no assertion may compare a call to itself
docs(report): sprint 5C, gate by gate
14. Si te quedas sin contexto

Para después de una puerta, nunca a mitad de una fase. Escribe en el reporte en qué puerta estás y qué falta. Deja la rama en verde. No dejes una fase declarada cerrada que no pasó su puerta.

Prohibido: tocar cualquier archivo de la lista prohibida de §4, añadir una dependencia, usar unsafe, silenciar la guarda nueva con un allow, dejar la rama en rojo, omitir un run fallido del reporte, declarar una fase cerrada sin su puerta, o commitear en main.

15. Próxima tarea (no la empieces)

Depende de lo que salga de esta ronda. En el horizonte: el FFI del motor con NativeCallable.listener de dart:ffi para el progreso Rust→Dart —cero dependencias, ~30 líneas—, el selector de archivos por Storage Access Framework en Android, y QYR-0072 cuando alguien decida qué cuesta menos: openat con una dependencia, o unsafe acotado.
```

### Prompt de continuación recibido, verbatim y completo

```text
Continúa en codex/qyro-gap-closure-5c. Si empiezas sesión nueva, la §0 del prompt anterior sigue siendo válida como contexto; lo esencial se repite aquí.

0. Hiciste lo correcto. El bloqueo era mío. Queda levantado.

Auditado tu trabajo. Paraste bien.

Encontraste que tres reglas de mi prompt no se pueden cumplir a la vez:

§13.1 exige el prompt verbatim en el informe.
El prompt cita QYR-0073, QYR-0074 y QYR-0075.
check_docs_consistency bloquea todo identificador citado sin ficha en BUGS_PENDING.md.
§4 prohíbe tocar BUGS_PENDING.md.

Es un error mío y es real. El otro agente lo encontró por su cuenta el mismo día. Y era peor de lo que viste: check_docs_consistency tiene además una regla de que STATUS.md no puede nombrar un Verified commit a más de diez commits de HEAD — así que mi prohibición de tocar STATUS.md garantizaba que ci.yml se pusiera rojo por antigüedad, pasara lo que pasara con el código.

También tenías razón en lo segundo: git diff --name-only origin/main...HEAD devuelve 353 rutas porque la rama base nunca se fusionó a main. Ese criterio era imposible por diseño. La base correcta es 15934aa y ése es el criterio a partir de ahora.

Y no te disculpes por parar. En este proyecto, una fase declarada cerrada que no lo está envenena todo lo que viene detrás — es exactamente lo que pasó con QYR-0071. Parar y reportar era la respuesta correcta y la tomaste.

Lo que queda autorizado, sin ambigüedad

Tienes permiso para tocar todo lo que necesites. BUGS_PENDING.md, STATUS.md, HANDOFF.md, NEXT_STEPS.md, CHANGELOG.md, DECISIONS.md, .github/workflows/**, los scripts de scripts/, y cualquier crate salvo los tres de §3. No vuelvas a pedir autorización para editar un .md. Hazlo.

La única condición es la coherencia: que check_docs_consistency pase en Bash y en PowerShell, que cada identificador que cites tenga su ficha, y que STATUS.md diga la verdad sobre lo que existe y lo que no.

Tu rango de identificadores es QYR-0100 en adelante. El otro agente tiene QYR-0076–QYR-0099. No uses fuera de tu rango, y no edites ninguna ficha ajena. Añade siempre al final del archivo. Los dos vais a tocar BUGS_PENDING.md a la vez; con rangos disjuntos, el peor caso es un conflicto de fusión trivial que resuelvo yo — infinitamente más barato que un agente bloqueado.

Y QYR-0073, QYR-0074, QYR-0075 son tuyos: créales su ficha en BUGS_PENDING.md en la Fase 1bis. Eso desbloquea el checker y cierra el conflicto de raíz.

1. Lo que también encontraste, y vale más de lo que crees

Tres cosas de tu Fase 0 que registré como hallazgos reales del proyecto, no como ruido de entorno:

El baseline de Windows no está limpio. cargo clippy --workspace --all-targets -- -D warnings falla allí por qyro_store_smoke::UNSUPPORTED_PLATFORM sin usar. Nadie lo sabía porque el workspace sólo se prueba en ubuntu-latest — lo verifiqué: ci.yml:33 es runs-on: ubuntu-latest y el trabajo de Windows de platform-builds.yml:103 sólo hace cargo build --package qyro_ffi y Flutter.
Windows corre 394 tests y Linux 388. STATUS.md declara un número como si fuera el número. No lo es: es el de una plataforma.
Los scripts declaran #requires -Version 7.0 y el host trae PowerShell 5. Y check_repo_portability.sh agota 120 s en Windows por su coste de procesos.

Las tres son deuda real y las tres entran en tu alcance. Es la Fase 8.

Y una nota de método: ejecutaste M1 en CI sobre Linux para tener semántica Unix de verdad, en vez de aproximarla en Windows. Eso es exactamente el rigor que este proyecto pide y es más de lo que yo hice.

2. La regla que gobierna este prompt

No confíes en tu memoria. Confía en el código.

Vas a hacer diez fases. Lo que era cierto en la Fase 2 puede dejar de serlo en la Fase 8. Por tanto:

Antes de afirmar algo en el informe, vuelve a mirarlo en el código o ejecútalo. No lo recuerdes.
Antes de cerrar cada puerta, relee las secciones del informe que la fase pudiera haber invalidado y corrígelas. El informe entero tiene que ser cierto en el momento del último commit.
Cuando digas un número, di el comando con el que lo obtuviste, y vuelve a obtenerlo si han pasado fases. Al otro agente le acaba de pasar: su §4 dice 63 paquetes y su §12 dice 62, porque §12 se escribió en la puerta 2 y nadie la actualizó.
3. Reparto de archivos — actualizado

Tuyos, exclusivos:

rust/crates/qyro_fs/**
rust/crates/qyro_protocol/**
rust/guards/source_guard.rs — eres el único que puede modificarlo
rust/crates/qyro_identity_store/**, qyro_win_dpapi/**, qyro_crypto/**, qyro_manifest/**
rust/tools/qyro_store_smoke/**, qyro_crypto_smoke/**
scripts/**
docs/adr/ADR-0027-* (enmiendas fechadas), ADR-0029-* (nueva), docs/reports/5C-codex.md

Compartidos, con regla:

BUGS_PENDING.md, DECISIONS.md — ambos escriben. Sólo añades, en tu rango, al final. No editas fichas ajenas.
.github/workflows/** — el otro agente va a añadir un trabajo de Windows para su crate nuevo. Tú puedes tocarlos también (Fase 8), pero avisa en el informe exactamente qué líneas cambiaste, porque ahí sí puede haber conflicto de fusión.
STATUS.md, HANDOFF.md, NEXT_STEPS.md, CHANGELOG.md — el otro agente los consolida en su Fase 10. Tú escribe lo tuyo en tu informe, y toca STATUS.md sólo lo mínimo para que check_docs_consistency pase en tu rama: el Verified commit y los números. No reescribas secciones enteras.

De Claude Code, no los toques:

rust/crates/qyro_net/**, rust/tools/qyro_net_smoke/**
rust/crates/qyro_ffi/**, apps/qyro/**
rust/crates/qyro_transfer/** — lo está tocando de forma aditiva
Cargo.toml raíz y Cargo.lock — él añade dos crates; tú no creas ninguno

Nunca: main. Sin commits, merge, PR, rebase ni force-push.

4. Estado del otro agente, para que sepas con qué te vas a fusionar

Va por la Puerta 4 de 6, con 408 tests en verde y un crate qyro_net completo hasta mover un archivo entre dos procesos reales por un socket TCP. Añadió Receiver::manifest() a qyro_transfer de forma aditiva —el receptor guardaba el manifest y lo tiraba, y sobre un socket eso hacía imposible materializar nada—. Encontró que nada comprobaba que un frame sin sellar llegado después del handshake se rechace, y que la mitad de Windows del mapeo de timeouts no la defendía nadie.

Tu Fase 5 le importa directamente: en cuanto haya dos transferencias o dos archivos en vuelo, transfer_id e item_id dejan de ser decorativos. Hazla aditiva y avísale en el informe.

5. Las diez fases
Fase 1bis — Desbloquear la Puerta 1
Crea las fichas de QYR-0073, QYR-0074 y QYR-0075 en BUGS_PENDING.md, con el formato del archivo —plataforma, severidad, esperado, actual, resolución, estado, fecha— y con la evidencia de mutación que ya reprodujiste. Estado: abierto, porque todavía no los has arreglado. Los cerrarás en las fases 2, 3 y 4.
Devuelve el prompt verbatim al informe si lo habías movido, y añade éste.
check_docs_consistency en Bash y en PowerShell: PASS.
Cambia el criterio de git diff a base 15934aa en tu informe §13.
Cierra la Puerta 1.

Y una cosa más, que es tuya y es importante: QYR-0068 lo vas a cerrar en la Fase 5, así que su ficha existe y hay que actualizarla, no duplicarla. Lo mismo con QYR-0072 en la Fase 7.

Puerta 1 (reintento).

Fase 2 — QYR-0073, el P1: darle una prueba de verdad a O_NOFOLLOW
Borra la prueba tautológica a_symlink_at_the_final_component_is_refused y escribe una que ejerza el control. Tiene que:
crear una víctima fuera de la raíz de destino, con contenido conocido;
crear un symlink real en <destino>/<nombre>.qyro-part apuntando a la víctima;
ejecutar una transferencia real a través de FileSink, no sólo resolve_under;
afirmar tres cosas: la víctima intacta byte a byte, el archivo final ausente, y el error tipado correcto.
Comprueba que falla con O_NOFOLLOW = 0 y pasa con el valor correcto. Es la única evidencia que vale, y ya sabes cómo hacerlo bien: en Linux, no en Windows.
Corrige el comentario de io.rs:69-75 para que diga la verdad sobre qué prueba qué y sobre qué plataformas no está probado.
Windows y macOS/iOS. Ahora que puedes tocar workflows, la opción buena está disponible: escribe la prueba equivalente con un reparse point en Windows y haz que corra. Si al intentarlo descubres que no se puede —permisos de symlink en Windows sin modo desarrollador, por ejemplo—, regístralo con ficha y di que ADR-0027 §1.4 afirma una garantía verificada sólo en Linux.
Barre el resto de qyro_fs/src/tests.rs en busca de más tautologías. La guarda de la Fase 6 las va a encontrar; adelántate.

Puerta 2.

Fase 3 — QYR-0074: que los contadores cuenten lo que pasó
Mueve el contador dentro de digest_of, registrando el count que devuelve file.read(). El modelo correcto está en qyro_transfer/src/session.rs, donde 5A midió buffer.len(). Ve a leerlo antes de escribir el tuyo — pero no lo modifiques, es del otro agente.
Comprueba que la prueba falla con read_to_end y pasa sin él.
Revisa los tres contadores de qyro_fs: PEAK_BUILDER_READ, FileSource::peak_read, FileSink::peak_write. Por cada uno, aplica la mutación que debería moverlo y comprueba que se mueve. peak_write es especialmente sospechoso: mira dónde se registra respecto a dónde se escribe.
Y la lección que el otro agente aprendió con su hallazgo 6A-11, que aplica aquí: no basta con que el contador mida bien. La prueba tiene que distinguir un contador medido de uno constante. Si una constante satisface tus aserciones, la forma de la prueba está mal aunque el contador esté bien. La forma que sí distingue: dos tamaños de archivo, y una aserción de que el pico del pequeño es estrictamente menor que el del grande — una constante falla esa desigualdad.

Puerta 3.

Fase 4 — QYR-0075: reconciliar ADR-0027 §5 con el código

Dos salidas legítimas. La A es la correcta y es barata.

Salida A — implementar §5. En part_for:

leer .qyro-resume si existe y corresponde a este transfer_id;
si hay entrada para el item, truncar el .qyro-part a bytes_committed con set_len;
si no hay metadatos que lo describan, borrar el huérfano antes de empezar;
si el transfer_id no coincide, tratarlo como huérfano. Este caso no está en la ADR y es un hallazgo si lo encuentras tú.

Salida B — degradar §5 a pendiente, con enmienda fechada, ficha, y las dos pruebas renombradas.

Sea cual sea:

an_interrupted_transfer_resumes_from_its_metadata — el código de producción tiene que leer los metadatos, no el test.
a_leftover_part_file_is_recovered_or_discarded_by_policy — el huérfano más largo que el contenido, más el caso simétrico con huérfano más corto.
Y el caso que ninguna cubre: un .qyro-part con .qyro-resume válido y bytes correctos — la reanudación que sí funciona, para que los rechazos no pasen por rechazarlo todo.

Puerta 4, con una comprobación extra: borra la lectura de .qyro-resume que acabas de escribir y comprueba que falla una prueba con nombre.

Fase 5 — QYR-0068 y ADR-0029: los identificadores de la cabecera

No pongas setters sin ADR. docs/adr/ADR-0029-header-identifiers.md, congelada antes del código. Decide:

Qué API pública se añade y su superficie mínima. Constructor, setter, o un tipo FrameIdentifiers. Y por qué ése.
Qué es un valor válido. ¿Cero es «sin identificador» o es válido? Si es válido, todos los frames existentes dicen algo. Decide y prueba la elección.
Qué garantiza que estén en el AAD y qué no. Un peer no puede alterarlos sin romper el tag. Eso no significa que sean correctos: significa que son los que el emisor puso. Escríbelo así.
Qué pasa si un receptor ve un transfer_id que no reconoce, o un item_id que no está en el manifest. Errores tipados, no Io.
Que el formato de 48 bytes NO cambia. Es ensanchar la API, no el formato. Dilo explícitamente.
Lo que esta decisión no promete.

Pruebas:

identifiers_survive_a_seal_and_open_round_trip
altering_an_identifier_in_flight_breaks_the_tag — voltea un bit de transfer_id en el frame sellado y comprueba que el open falla, no que devuelve otro valor.
the_forty_eight_byte_layout_is_unchanged — un vector de bytes fijo y esperado, no header.len() == 48.

Y actualiza la nota de header.rs. Hoy dice que la decisión queda abierta. Ya no lo está. Y avisa en el informe al agente de la red, que va a querer usarla.

Puerta 5.

Fase 6 — La guarda que mata el anti-patrón para siempre

Esto es lo que hace que el sprint valga más que sus arreglos. La aserción tautológica ha aparecido cinco veces en este repositorio.

En rust/guards/source_guard.rs, añade assert_no_assertion_compares_a_call_to_itself: recorre los archivos de prueba, encuentra cada assert!/assert_eq!/assert_ne!, y falla si los dos lados de una comparación son textualmente idénticos tras normalizar espacios.
Empieza por X == X y assert_eq!(X, X). No resuelvas el caso general; resuelve el que ha ocurrido cinco veces.
Prueba la guarda con una tautología introducida a propósito y comprueba que falla. Una guarda que no se ha visto fallar no es una guarda.
Falsos positivos legítimos: exímelos por nombre y con el argumento escrito, como VERDICTS_WITH_NO_CONSTRUCTION_SITE. Nunca un allow global.
Actívala en todos los crates que usan el análisis compartido. Si alguno falla, has encontrado una tautología más: arréglala y regístrala.
cargo test --workspace en verde, no sólo -p qyro_fs. El otro agente depende de este archivo.

Puerta 6.

Fase 7 — QYR-0072: la carrera de los componentes intermedios

Es el último P2 abierto de qyro_fs y lleva un sprint esperando una decisión.

ADR-0027 §1 dice que O_NOFOLLOW cierra la carrera del último componente por completo, y no la de los intermedios: entre comprobar que fotos/ no es un enlace y abrir fotos/x.qyro-part hay una ventana.

Toma la decisión. Las tres opciones, y evalúalas de verdad:

(a) openat con O_NOFOLLOW por descriptor de directorio. No está en std. Requiere libc (una dependencia, 2 crates transitivos medidos) o unsafe con extern a mano —y este proyecto ya escribió a mano las tres funciones de DPAPI, así que hay precedente—. En Windows es otro mecanismo (NtCreateFile con OBJ_DONT_REPARSE, o abrir el directorio y usar rutas relativas al handle).
(b) Aceptar la ventana y documentarla mejor, con el argumento de que un atacante con escritura en el directorio de destino ya puede escribir lo que quiera ahí, y lo que las comprobaciones impiden es que use Qyro para escribir fuera.
(c) Una mitigación parcial sin dependencias: por ejemplo, canonicalizar el padre después de abrir y comprobar que sigue dentro de la raíz, y abortar si cambió. No cierra la carrera pero la detecta a posteriori.

Elige, argumenta, y si eliges (b) o (c), di exactamente qué queda sin cubrir. Si eliges (a) con unsafe, tiene que ir acotado, con SAFETY: escrito, y la lista de crates exentos de forbid(unsafe_code) pasa de tres a cuatro — lo cual hay que registrar y justificar, porque ese número es una guarda.

Sea cual sea la decisión, escríbela como enmienda fechada a ADR-0027 y actualiza la ficha QYR-0072.

Puerta 7.

Fase 8 — El baseline de Windows, que descubriste tú

Los tres hallazgos de tu Fase 0, cada uno con ficha en tu rango:

cargo clippy falla en Windows por qyro_store_smoke::UNSUPPORTED_PLATFORM sin usar. Arréglalo. Y la causa de fondo es que el workspace sólo se prueba en Linux: ci.yml:33 es runs-on: ubuntu-latest. Decide si ci.yml debe correr clippy y tests del workspace en Windows también, argumenta el coste contra el beneficio, y si dices que sí, hazlo. Avisa en el informe, porque el otro agente está añadiendo un trabajo de Windows para su crate y podéis chocar en el mismo archivo.
394 tests en Windows contra 388 en Linux. STATUS.md declara un número como si fuera el número. Arregla la afirmación: o dice los dos, o dice de qué plataforma es. Y comprueba que la diferencia son de verdad los cfg de plataforma y no un test que no corre donde debería. Cuéntalos y dilo.
#requires -Version 7.0 en los scripts, y check_repo_portability.sh agotando 120 s en Windows. Decide: ¿bajar el requisito a PowerShell 5, o documentar que hace falta 7? Y el timeout: ¿es coste real o hay un bucle que se puede arreglar? Mídelo antes de decidir.

Puerta 8.

Fase 9 — La deuda estructural: que ninguna guarda vuelva a perderse

El patrón: qyro_crypto tiene desde 4C.2 una guarda que qyro_transfer no tenía (QYR-0070) y qyro_fs tampoco (QYR-0073). Una guarda que existe en un crate y no se lleva al siguiente es una guarda que se pierde.

Haz el inventario. Por cada crate del workspace, qué guardas estructurales tiene y cuáles le faltan: no_production_path_can_panic, every_production_file_is_listed, every_*_has_a_construction_site, only_the_listed_crates_may_relax_forbid_unsafe, every_public_path_returning_key_material_is_listed, assert_analysis_reached_the_end, y la nueva de la Fase 6. Tabla completa en el informe.
Lleva las que falten a los crates que las necesiten.
Y la guarda meta que impide que vuelva a pasar: una comprobación que falle si un crate del workspace no tiene el conjunto mínimo de guardas. Excepciones por nombre y con argumento escrito.
Coordinación: el otro agente está creando qyro_net y qyro_net_smoke y va a escribirles guardas en su Fase 6. Si tu guarda meta los exige y todavía no existen en tu rama, exímelos por nombre con la nota de que llegan en la rama claude/qyro-net-6a. No los inventes.
El barrido de mutación de todo lo que sea tuyo. No sólo lo que tocaste: qyro_fs, qyro_protocol, qyro_manifest, qyro_identity_store, qyro_crypto. Por cada control de producción que gobierne una propiedad de seguridad o de integridad, bórralo y mira si algo falla. Es la primera vez que este proyecto lo haría de forma exhaustiva.
Prioriza: primero lo que valida entrada de un peer, luego lo que decide rechazar, luego lo demás.
Cada superviviente es una ficha. No los arregles todos si son muchos: regístralos con severidad y di cuáles arreglaste y cuáles no.
Y di cuántos controles barriste de cuántos, para que se sepa la cobertura del propio barrido. Un barrido que no dice su alcance se lee como exhaustivo sin serlo.

Puerta 9.

Fase 10 — Dejar la documentación diciendo la verdad
BUGS_PENDING.md: todas tus fichas, en tu rango, ninguna ajena tocada. QYR-0073, 0074, 0075 cerradas. QYR-0068 cerrada. QYR-0072 resuelta o con la decisión escrita.
DECISIONS.md: ADR-0029 y la enmienda a ADR-0027 registradas.
STATUS.md: lo mínimo para coherencia — Verified commit, los números de test con su plataforma, y una sección del sprint 5C. No reescribas secciones que el otro agente va a consolidar.
check_docs_consistency en Bash y en PowerShell: PASS.
Relee tu informe entero y corrige toda afirmación que las fases 2 a 9 hayan invalidado. Especialmente §11 («Después: pendiente») y §12.

Puerta 10 + los workflows que dispare tu rama en verde sobre el commit final.

6. No objetivos — estrictos
Los archivos de Claude Code (§3): qyro_net, qyro_net_smoke, qyro_ffi, apps/qyro, qyro_transfer, Cargo.toml, Cargo.lock.
Red, sockets, descubrimiento, FFI, UI, selector de archivos.
Crates nuevos. No creas ninguno: Cargo.toml es del otro agente.
Android Keystore, iOS Keychain, historial, emparejamiento, release.
Dependencias externas. Cero, con una excepción posible: si la Fase 7 concluye que libc es la respuesta correcta a QYR-0072, para, escríbelo, y espera confirmación. Es la única dependencia que este prompt contempla y no la añades sin decirlo.
7. Criterios de aceptación
Las diez puertas pasadas y escritas en el informe con su resultado.
check_docs_consistency PASS en Bash y en PowerShell, con el prompt verbatim dentro del .md.
O_NOFOLLOW = 0 hace fallar una prueba con nombre. Es el criterio del P1.
La prueba de O_NOFOLLOW ejecuta una transferencia real por FileSink.
Las constantes de Windows y macOS/iOS están probadas o están registradas como no probadas con ficha.
read_to_end en digest_of hace fallar building_a_manifest_from_disk_does_not_load_the_file.
Los tres contadores de qyro_fs registran valores derivados de la operación, y la prueba de cada uno distingue un contador medido de una constante.
ADR-0027 §5 implementada o degradada con enmienda fechada y pruebas renombradas. Y borrar la lectura de .qyro-resume hace fallar una prueba.
ADR-0029 congelada antes del código de la Fase 5. Alterar un identificador en vuelo rompe el tag, con prueba. El layout de 48 bytes comprobado contra un vector fijo.
assert_no_assertion_compares_a_call_to_itself existe, se ha visto fallar, y está activa en todos los crates que usan el análisis compartido.
QYR-0072 decidida, con la opción argumentada y lo que queda sin cubrir dicho explícitamente.
cargo clippy --workspace --all-targets -- -D warnings pasa en Windows.
La diferencia 394/388 explicada test por test, y STATUS.md corregido.
Inventario de guardas por crate, guardas que faltaban llevadas, y la guarda meta activa.
Barrido de mutación con su alcance declarado: cuántos controles de cuántos, cuántos supervivientes, cuántos arreglados, cuántos con ficha.
Cargo.lock idéntico: 61 paquetes en tu rama. Cero dependencias nuevas — o la excepción de §6 con confirmación previa.
cargo fmt --all --check, cargo clippy, cargo test --workspace, cargo test --doc, cargo audit --deny warnings: PASS. Tests antes (388 Linux / 394 Windows) y después, con la plataforma de cada número.
git diff --name-only 15934aa..HEAD no contiene ni un archivo de Claude Code (§3). Pega la salida literal. La base es 15934aa, no origin/main.
El informe entero es cierto en el momento del último commit. Ninguna sección contradice a otra, ninguna dice «pendiente» de algo que ya hiciste.
git status --short limpio. Sin commits en main, sin merge, sin PR, sin force-push.
8. Lo que no vale como «arreglado»
Un test que pasa. Un test que pasa y que falla cuando borras el control.
Cambiar el nombre de una prueba sin cambiar lo que ejerce.
Un assert! más en la misma prueba tautológica. Bórrala y escribe otra.
«No pude probar Windows» sin registrar que la garantía de Windows queda sin verificar.
Marcar la Fase 4 con la salida B sin haber intentado la A.
Un #[allow(...)] para que la guarda nueva no se queje.
Una guarda nueva que nunca se ha visto fallar.
Un barrido de mutación que no dice cuántos controles cubrió de cuántos.
9. El protocolo de puerta — ampliado con dos comprobaciones

No pasas de fase hasta que las once pasan.

cargo fmt --all --check — PASS, por código de salida del proceso, no por la salida de texto.
cargo clippy --workspace --all-targets -- -D warnings — PASS, por código de salida. En Linux siempre; en Windows a partir de la Fase 8.
cargo test --workspace — PASS. El workspace entero, no sólo tu crate.
Barrido de mutación de la fase. Por cada propiedad nueva o arreglada: aplica la mutación, confirma que falla un test con nombre, restaura. Si sobrevive, no has terminado.
Lectura de aserciones: los dos lados de cada assert! nuevo pueden diferir.
Lectura de contadores: registran un valor derivado de la operación, y la prueba distingue un contador medido de una constante.
Lectura de nombres: cada test ejerce lo que su nombre dice.
git diff --name-only 15934aa..HEAD — ni un archivo de Claude Code.
check_docs_consistency en Bash. Ahora que tocas documentos raíz, es parte de la puerta y no algo que se descubre al final.
NUEVO — coherencia del informe. Relee las secciones que esta fase pudiera haber invalidado —conteos, tablas, «pendiente», clases de evidencia— y corrígelas contra el código actual, no contra tu memoria.
Escribe el resultado de la puerta en el informe antes de empezar la fase siguiente.

Si una comprobación falla: arréglalo y repite la puerta entera.

Si no puedes arreglarlo: ahora tienes permiso para casi todo, así que la situación anterior no debería repetirse. Pero si vuelve a pasar —y sobre todo si descubres que este prompt se contradice a sí mismo otra vez—, para, escríbelo, y reporta. Lo hiciste bien la primera vez.

10. El informe — docs/reports/5C-codex.md

Las mismas dieciséis secciones. Y ahora:

§1 con los dos prompts verbatim, el primero y éste, en el .md.
§9 con las diez puertas.
§10 con la tabla de mutación completa, incluidas las cuatro de la Fase 1 y el barrido exhaustivo de la Fase 9, con su alcance declarado.
§11 con tests antes y después, y la plataforma de cada número.
§13 con base 15934aa.
§8 con la clase de evidencia por plataforma: qué corre en Linux, qué corre en Windows tras la Fase 8, y qué sigue sin correr en ningún sitio.
§15 — qué NO debe leerse como progreso. Como mínimo: esto no mueve el producto, es cierre de deuda y endurecimiento; no hay red en esta rama, no hay FFI, no hay selector, los botones siguen deshabilitados, no hay persistencia de identidad en Android ni iOS, y nada se ha probado en hardware físico.
§16 — qué necesita saber el otro agente. En particular, la API nueva de header.rs de la Fase 5, cualquier cambio en source_guard.rs, y cualquier línea que hayas tocado en .github/workflows/**.
11. Si te quedas sin contexto

Para después de una puerta, nunca a mitad de una fase. Escribe en el informe en qué puerta estás y qué falta. Deja la rama en verde y check_docs_consistency pasando.

Prohibido: tocar los archivos de Claude Code, usar identificadores fuera de tu rango, editar una ficha ajena del ledger, crear un crate, añadir una dependencia sin pararte a explicar y esperar confirmación, silenciar una guarda con un allow, dejar la rama en rojo, omitir un run fallido, declarar una fase cerrada sin su puerta, o commitear en main.

Y lo que ya no está prohibido, para que no vuelvas a pararte: editar cualquier .md, editar BUGS_PENDING.md, editar STATUS.md, editar los workflows, editar los scripts. Tienes permiso. Úsalo.
```

## 2. Qué hice, punto por punto

- Fase 0: verifiqué el SHA base, leí íntegros los documentos y fuentes exigidos y reproduje la línea base en Windows y en CI Linux.
- Fase 1/M1: sustituí temporalmente `O_NOFOLLOW` por `0`, ejecuté el workspace en Linux y confirmé que las 388 pruebas siguieron en verde; restauré el control en `dae0996`.
- Fase 1/M2: sustituí temporalmente el bucle de `digest_of` por `read_to_end`; `tests::building_a_manifest_from_disk_does_not_load_the_file` siguió pasando.
- Fase 1/M3: hice el huérfano de 8192 bytes frente a un payload de 2048; `tests::a_leftover_part_file_is_recovered_or_discarded_by_policy` falló con `DigestMismatch { item_id: 1 }`.
- Fase 1/M4: busqué llamantes productivos de `ResumeState::decode`; el recuento fue cero.
- Fase 1bis: registré QYR-0073, QYR-0074 y QYR-0075 al final del ledger,
  incorporé íntegro el prompt de continuación y cambié el criterio de diff a la
  base exacta `15934aa`.
- Fase 1bis: el nuevo prompt destapó QYR-0100. Añadí un contrato que primero
  falló y corregí los checkers Bash y PowerShell para no confundir límites de
  rangos reservados con hallazgos concretos.

## 3. Cómo lo hice y decisiones

- Las mutaciones se aplicaron una por una y se restauraron antes de la siguiente. M1 necesitaba semántica Unix, así que se ejecutó en el job `rust` de CI sobre el commit mutante; M2–M4 se reprodujeron localmente.
- No se modificó ningún archivo prohibido, no se añadió ninguna dependencia y Cargo.lock permaneció intacto.
- QYR-0100 conserva la regla estricta para citas concretas. Sólo elimina antes
  del escaneo las formas de reserva `QYR-NNNN–QYR-NNNN`, `QYR-NNNN onward`,
  `QYR-NNNN en adelante` y `QYR-NNNN+`.

## 4. Errores detectados fuera del prompt

- Fase 0: el host local Windows ejecuta 394 pruebas/2 ignoradas, no 388/2, por la selección `cfg` de plataforma. El mismo árbol en Linux (CI 31520332918) ejecuta 388/2.
- Fase 0: `cargo clippy --workspace --all-targets -- -D warnings` falla en Windows por `qyro_store_smoke::UNSUPPORTED_PLATFORM` sin uso; el mismo comando pasa en Linux. El archivo está fuera del alcance permitido de 5C.
- Fase 0: el host trae Windows PowerShell 5, no PowerShell 7; los scripts declaran `#requires -Version 7.0`. Git Bash ejecutó tres checks; `check_repo_portability.sh` agotó 120 s por su coste de procesos en Windows. Los ocho checks pasaron en CI Linux con Bash y PowerShell 7.
- Fase 1, histórico: el prompt inicial exigía citar QYR-0073/74/75 y a la vez
  prohibía registrarlos; la continuación reconoce el conflicto y autoriza el
  ledger. Quedó resuelto en Fase 1bis.
- Fase 1, histórico: el criterio contra `origin/main` atribuía 353 rutas
  heredadas a este trabajo. La continuación fija la base correcta en
  `15934aa`; §13 usa ya ese criterio.
- Fase 1bis: el checker trataba los límites de rangos de propiedad del segundo
  prompt como hallazgos sin ficha. QYR-0100 registra y corrige el defecto sin
  crear entradas en el rango ajeno 0076–0099.

## 5. Errores arreglados y no arreglados

- Fase 1 sólo reprodujo; QYR-0073/74/75 y QYR-0068 siguen abiertos hasta sus
  fases funcionales.
- Fase 1bis resolvió el conflicto reporte/ledger y QYR-0100. No presenta ese
  cierre documental como corrección de O_NOFOLLOW, memoria, reanudación o API.

## 6. Impacto de cada defecto

- M1 demuestra que el control del componente final podía desaparecer sin regresión visible.
- M2 demuestra que la prueba de memoria no medía la lectura real y aceptaba cargar el archivo completo.
- M3 demuestra que un huérfano más largo contamina la nueva transferencia y la hace fallar por digest.
- M4 demuestra que los metadatos escritos por producción no tenían lector productivo.
- QYR-0100 hacía que una declaración de coordinación exigiera fichas falsas en
  un rango ajeno y volvía rojo `documentation` aun después de registrar los
  tres hallazgos reales.

## 7. Resultado contra cada objetivo

- Reproducir M1–M4 antes de arreglar: **cumplido**.
- Resolver el bloqueo documental de Puerta 1: **cumplido**. Bash y PowerShell 7,
  sus contratos, Clippy Linux, las suites Rust y audit pasaron en CI
  31528757962.
- Fases 2–10: **no empezadas** al cerrar esta puerta.

## 8. Clase de evidencia por afirmación

- Línea base Linux: probado en unidad e integración en GitHub Actions sobre `15934aae3dda7f469b5496c8341eb78d9e32f335`, run 31520332918.
- Línea base Windows: probado en unidad localmente con Rust 1.88.0; 394 passed, 0 failed, 2 ignored.
- Hardware físico: no probado.
- M1: probado en integración en Linux, CI run 31521002851, job `rust`; `cargo test --workspace` pasó con `O_NOFOLLOW = 0`.
- M2/M3/M4: probado en unidad o inspección estructural en Windows host. M3 falló por el motivo esperado, no por compilación ni fixture.
- QYR-0100: contrato Bash rojo→verde local; primer contrato PowerShell 7 rojo
  por entrada vacía en CI 31528281381; ambos contratos verdes en Linux con
  PowerShell 7 en CI 31528757962.

## 9. Las diez puertas de trabajo y la línea base

### Puerta 0 — 2026-08-11 — PASS con diferencias de plataforma registradas

- `cargo fmt --all --check`: PASS local Windows y CI Linux.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS CI Linux; FAIL base en Windows por `qyro_store_smoke::UNSUPPORTED_PLATFORM`, fuera de alcance.
- `cargo test --workspace`: PASS. Linux CI: 388 passed, 0 failed, 2 ignored. Windows local: 394 passed, 0 failed, 2 ignored.
- `cargo test --doc --workspace`: PASS CI Linux.
- `cargo audit --deny warnings`: PASS CI Linux; 61 paquetes.
- Cuatro `check_*` en Bash y PowerShell: PASS los ocho en CI Linux. Local: tres PASS en Git Bash; PowerShell 5 es incompatible y el cuarto Bash agotó 120 s.
- SHA base: coincide exactamente.
- Cargo.lock: 61 paquetes.
- `assert_analysis_reached_the_end`: compara la última línea no vacía del fuente crudo con el resultado analizado y falla si el stripper consumió el resto; existe por QYR-0071.
- Mutación de fase: no aplica; Fase 0 sólo reproduce línea base.
- Lectura de aserciones/contadores/nombres: sin aserciones, contadores ni tests nuevos.
- `git diff --name-only`: sólo `docs/reports/5C-codex.md` al iniciar Fase 1; ningún archivo prohibido.

### Puerta 1 — 2026-08-11 — PASS

- M1–M4: reproducidas y restauradas.
- `cargo fmt --all --check`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS Linux en CI
  31528757962; el warning Windows base se resolverá en Fase 8 como exige la
  continuación.
- `cargo test --workspace`: PASS. Linux 388 passed/2 ignored en CI 31528757962;
  Windows local 394 passed/2 ignored.
- Mutaciones de fase: M1 y M2 sobrevivieron como se esperaba; M3 falló con el
  error esperado; M4 confirmó cero lectores productivos. Todas quedaron
  restauradas antes de validar.
- Lectura de aserciones/contadores/nombres: confirmó la tautología de M1, el
  contador constante de M2 y que el nombre del test M3 no cubría el huérfano
  largo. El contrato nuevo de QYR-0100 sí distingue rango reservado de cita
  concreta.
- Delta desde `15934aa`: sólo ledger, informe y ambos checkers/contratos; ninguna
  ruta Claude ni crate reservado (§13).
- `check_docs_consistency.sh`: PASS tras registrar QYR-0073/74/75 y corregir
  QYR-0100. El contrato Bash observó rojo antes de la corrección y pasa después.
- `check_docs_consistency.ps1` y su contrato: PASS con PowerShell 7 en CI
  31528757962. El run 31528281381 falló primero porque `Get-Content -Raw`
  devolvía `$null` para fixtures vacíos; el fallo queda registrado.
- CI 31521002851: run global `failure`; jobs `rust`, `scripts` y `flutter` PASS, job `documentation` FAIL por la misma causa.
- Coherencia del informe: segundo prompt comparado íntegro con el adjunto;
  §13 usa la base exacta y esta puerta contiene los once controles exigidos.
- Gate escrito antes de empezar Fase 2. CI 31528757962: **success** en los cuatro
  jobs, incluidas 388 pruebas Linux, doc tests, audit y 61 paquetes.

### Puerta 2

Pendiente.

### Puerta 3

Pendiente.

### Puerta 4

Pendiente.

### Puerta 5

Pendiente.

### Puerta 6

Pendiente.

### Puerta 7

Pendiente.

### Puerta 8

Pendiente.

### Puerta 9

Pendiente.

### Puerta 10

Pendiente.

## 10. Tabla completa de mutaciones

| Fase | Control | Mutación aplicada | Resultado/test que falló | Commit |
|---|---|---|---|---|
| 1/M1 | `O_NOFOLLOW` Linux/Android | `libc_o_nofollow()` devuelve `0` | Sobrevivió: `cargo test --workspace`, 388/388 en verde en el job `rust` | `a1c7398` |
| 1/M2 | lectura acotada de `digest_of` | `read_to_end` carga el archivo completo | Sobrevivió: `tests::building_a_manifest_from_disk_does_not_load_the_file` pasó (1/1) | Mutación local restaurada |
| 1/M3 | descarte de huérfano largo | `.qyro-part` de 8192 bytes frente a contenido de 2048, sin metadata | Falló `tests::a_leftover_part_file_is_recovered_or_discarded_by_policy` con `DigestMismatch { item_id: 1 }` | Mutación local restaurada |
| 1/M4 | lectura productiva de `ResumeState::decode` | `rg -n 'ResumeState::decode' rust/crates/qyro_fs/src -g '*.rs'`, excluyendo `tests.rs` | Cero llamantes productivos | N/A |

## 11. Tests antes y después

- Antes, Linux: 388 passed, 0 failed, 2 ignored.
- Antes, Windows: 394 passed, 0 failed, 2 ignored.
- Después: pendiente.

## 12. Delta de dependencias

- Paquetes antes: 61.
- Paquetes después de Fase 1: 61.
- Dependencias externas nuevas: ninguna prevista.
- `git diff origin/claude/qyro-filesystem-5b1...HEAD -- Cargo.lock`: vacío.

## 13. `git diff --name-only 15934aae3dda7f469b5496c8341eb78d9e32f335...HEAD`

La continuación corrige el criterio imposible contra `origin/main`. El delta propio desde la base exacta es:

```text
BUGS_PENDING.md
docs/reports/5C-codex.md
scripts/check_docs_consistency.ps1
scripts/check_docs_consistency.sh
scripts/tests/docs_consistency_contract_test.ps1
scripts/tests/docs_consistency_contract_test.sh
```

No contiene `CLAUDE.md`, `.claude/**` ni ningún archivo de los crates reservados al otro agente.

## 14. Todos los runs de CI de la rama

| Run | Commit | Workflow | Evento | Conclusión |
|---|---|---|---|---|
| 31520332918 | 15934aae3dda7f469b5496c8341eb78d9e32f335 | CI | workflow_dispatch | success |
| 31521002851 | a1c7398fbc2d7ef903282f3d64cfb19da23dcf42 | CI | workflow_dispatch | failure global; `rust` PASS, `documentation` FAIL por ledger |
| 31528281381 | 6175820a28d1e2a79fe5a70a56d2bff60a4a4663 | CI | workflow_dispatch | failure global; `documentation`, `rust` y `flutter` PASS; contrato PowerShell de QYR-0100 FAIL por entrada vacía |
| 31528757962 | d6701a149fc3a3249c446cf65ffe01b7fc62e986 | CI | workflow_dispatch | success; cuatro jobs PASS |

Lista reconstruida por API al cerrar la Fase 1bis. No hubo runs cancelados; los
dos fallos se conservan y no se filtran.

## 15. Qué NO debe leerse como progreso

Este sprint no mueve el producto: cierra deuda de pruebas y de contrato. No hay red, sockets, descubrimiento, FFI del motor ni selector de archivos; Enviar y Recibir siguen deshabilitados. No hay persistencia de identidad en Android ni iOS. QYR-0072 sigue abierta hasta la Fase 7. Nada se ha probado en hardware físico.

## 16. Documentación desfasada y handoff al sprint siguiente

El sprint todavía no cambió la superficie de cabecera. El bloqueo documental y
la Puerta 1 están cerrados. Cuando la Fase 5 congele e implemente ADR-0029, §16
documentará el API exacto para el agente de red. Los
cambios compartidos actuales son las entradas añadidas al final del ledger y
las líneas de escaneo de rangos en ambos checkers y sus contratos.
