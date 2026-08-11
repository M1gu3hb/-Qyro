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

## 2. Qué hice, punto por punto

- Fase 0: verifiqué el SHA base, leí íntegros los documentos y fuentes exigidos y reproduje la línea base en Windows y en CI Linux.
- Fase 1/M1: sustituí temporalmente `O_NOFOLLOW` por `0`, ejecuté el workspace en Linux y confirmé que las 388 pruebas siguieron en verde; restauré el control en `dae0996`.
- Fase 1/M2: sustituí temporalmente el bucle de `digest_of` por `read_to_end`; `tests::building_a_manifest_from_disk_does_not_load_the_file` siguió pasando.
- Fase 1/M3: hice el huérfano de 8192 bytes frente a un payload de 2048; `tests::a_leftover_part_file_is_recovered_or_discarded_by_policy` falló con `DigestMismatch { item_id: 1 }`.
- Fase 1/M4: busqué llamantes productivos de `ResumeState::decode`; el recuento fue cero.

## 3. Cómo lo hice y decisiones

- Las mutaciones se aplicaron una por una y se restauraron antes de la siguiente. M1 necesitaba semántica Unix, así que se ejecutó en el job `rust` de CI sobre el commit mutante; M2–M4 se reprodujeron localmente.
- No se modificó ningún archivo prohibido, no se añadió ninguna dependencia y Cargo.lock permaneció intacto.

## 4. Errores detectados fuera del prompt

- Fase 0: el host local Windows ejecuta 394 pruebas/2 ignoradas, no 388/2, por la selección `cfg` de plataforma. El mismo árbol en Linux (CI 31520332918) ejecuta 388/2.
- Fase 0: `cargo clippy --workspace --all-targets -- -D warnings` falla en Windows por `qyro_store_smoke::UNSUPPORTED_PLATFORM` sin uso; el mismo comando pasa en Linux. El archivo está fuera del alcance permitido de 5C.
- Fase 0: el host trae Windows PowerShell 5, no PowerShell 7; los scripts declaran `#requires -Version 7.0`. Git Bash ejecutó tres checks; `check_repo_portability.sh` agotó 120 s por su coste de procesos en Windows. Los ocho checks pasaron en CI Linux con Bash y PowerShell 7.
- Fase 1: conflicto de requisitos. Esta sección 1 debe contener el prompt verbatim, que cita QYR-0073, QYR-0074 y QYR-0075. `check_docs_consistency` exige una entrada en `BUGS_PENDING.md` para todo ID citado. El mismo prompt prohíbe modificar `BUGS_PENDING.md`. Al versionar el reporte, el check falla por esos tres IDs; no hay forma de satisfacer a la vez los tres requisitos sin cambiar alcance.
- Fase 1: `git diff --name-only origin/main...HEAD` contiene 353 rutas, incluidos todos los documentos raíz y workflows prohibidos, porque la rama base exigida todavía no está integrada en `main`. El delta propio contra `origin/claude/qyro-filesystem-5b1` es únicamente este reporte. Por tanto, el criterio literal contra `origin/main` tampoco puede pasar sin reescribir historia, fusionar o cambiar la base, tres acciones prohibidas.

## 5. Errores arreglados y no arreglados

- Fase 1 sólo reprodujo: no arregló todavía QYR-0073/74/75 ni QYR-0068.
- El conflicto reporte/ledger no se arregló porque las dos correcciones posibles violan una orden expresa: quitar o alterar el prompt deja de ser verbatim; añadir las entradas toca un archivo prohibido.

## 6. Impacto de cada defecto

- M1 demuestra que el control del componente final podía desaparecer sin regresión visible.
- M2 demuestra que la prueba de memoria no medía la lectura real y aceptaba cargar el archivo completo.
- M3 demuestra que un huérfano más largo contamina la nueva transferencia y la hace fallar por digest.
- M4 demuestra que los metadatos escritos por producción no tenían lector productivo.
- El conflicto documental vuelve rojo el job `documentation` en cuanto el reporte obligatorio se versiona, impidiendo pasar cualquier puerta posterior aun con el código intacto.

## 7. Resultado contra cada objetivo

- Reproducir M1–M4 antes de arreglar: **cumplido**.
- Pasar Puerta 1: **no hecho/bloqueado** por el conflicto reporte/ledger descrito arriba.
- Fases 2–6: **no empezadas**, conforme a la orden de no avanzar con una puerta fallida.

## 8. Clase de evidencia por afirmación

- Línea base Linux: probado en unidad e integración en GitHub Actions sobre `15934aae3dda7f469b5496c8341eb78d9e32f335`, run 31520332918.
- Línea base Windows: probado en unidad localmente con Rust 1.88.0; 394 passed, 0 failed, 2 ignored.
- Hardware físico: no probado.
- M1: probado en integración en Linux, CI run 31521002851, job `rust`; `cargo test --workspace` pasó con `O_NOFOLLOW = 0`.
- M2/M3/M4: probado en unidad o inspección estructural en Windows host. M3 falló por el motivo esperado, no por compilación ni fixture.

## 9. Las seis puertas

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

### Puerta 1 — 2026-08-11 — BLOCKED

- M1–M4: reproducidas y restauradas.
- `cargo fmt --all --check`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: FAIL en Windows por el warning base de `qyro_store_smoke::UNSUPPORTED_PLATFORM`; PASS en Linux en el job `rust` del commit M1.
- `cargo test -p qyro_fs`: PASS tras restaurar las mutaciones.
- Lectura de aserciones/contadores/nombres: sin código de producción ni tests nuevos en Fase 1.
- Archivos de código al terminar: idénticos a la base; sólo el reporte cambia respecto a la base.
- `check_docs_consistency.sh`: FAIL por QYR-0073, QYR-0074 y QYR-0075 citados en el prompt verbatim sin entradas en el ledger prohibido.
- CI 31521002851: job `rust` PASS, job `documentation` FAIL por la misma causa; run global en curso/failure a la hora de esta puerta.
- Decisión obligada por §11: no empezar Fase 2. La puerta no puede pasar sin autorización para modificar `BUGS_PENDING.md`, para excluir el reporte del checker, o para omitir/alterar el prompt verbatim.

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

## 13. `git diff --name-only origin/main...HEAD`

+La salida literal actual contiene 353 rutas heredadas de la rama base, porque `main` no contiene los sprints previos:

```text
.gitattributes
.github/scripts/android_crypto_smoke.sh
.github/workflows/android-runtime.yml
.github/workflows/ci.yml
.github/workflows/crypto-fuzz.yml
.github/workflows/crypto-platform.yml
.github/workflows/ios-runtime.yml
.github/workflows/platform-builds.yml
.gitignore
AGENTS.md
ARCHITECTURE.md
BUGS_PENDING.md
CHANGELOG.md
Cargo.lock
Cargo.toml
DECISIONS.md
FILE_MAP.md
HANDOFF.md
NEXT_STEPS.md
PROJECT_CONTEXT.md
PROTOCOL.md
README.md
SECURITY.md
STATUS.md
TESTING.md
THIRD_PARTY_NOTICES.md
THREAT_MODEL.md
apps/qyro/android/app/src/main/res/drawable-v21/launch_background.xml
apps/qyro/android/app/src/main/res/drawable/launch_background.xml
apps/qyro/android/app/src/main/res/values-night/styles.xml
apps/qyro/android/app/src/main/res/values/colors.xml
apps/qyro/android/app/src/main/res/values/styles.xml
apps/qyro/assets/brand/qyro-logo.png
apps/qyro/assets/generated/logo_ascii.json
apps/qyro/assets/generated/logo_ascii.txt
apps/qyro/assets/generated/logo_ascii_preview.png
apps/qyro/integration_test/native_abi_smoke_test.dart
apps/qyro/ios/.gitignore
apps/qyro/ios/Native/README.md
apps/qyro/ios/Runner.xcodeproj/project.pbxproj
apps/qyro/ios/Runner/Base.lproj/LaunchScreen.storyboard
apps/qyro/ios/RunnerTests/RunnerTests.swift
apps/qyro/l10n.yaml
apps/qyro/lib/app.dart
apps/qyro/lib/boot/ascii_logo_model.dart
apps/qyro/lib/boot/ascii_logo_painter.dart
apps/qyro/lib/boot/boot_screen.dart
apps/qyro/lib/boot/boot_sequence_controller.dart
apps/qyro/lib/boot/boot_status_model.dart
apps/qyro/lib/boot/cipher_rain_painter.dart
apps/qyro/lib/boot/scramble_decode_engine.dart
apps/qyro/lib/boot/scrambled_line.dart
apps/qyro/lib/ffi/qyro_native_api.dart
apps/qyro/lib/generated/branding.g.dart
apps/qyro/lib/home/home_screen.dart
apps/qyro/lib/l10n/app_en.arb
apps/qyro/lib/l10n/app_es.arb
apps/qyro/lib/startup/native_bridge.dart
apps/qyro/lib/startup/production_startup.dart
apps/qyro/lib/startup/startup_coordinator.dart
apps/qyro/pubspec.lock
apps/qyro/pubspec.yaml
apps/qyro/test/boot_painters_test.dart
apps/qyro/test/boot_visual_contract_test.dart
apps/qyro/test/boot_wordmark_test.dart
apps/qyro/test/branding_generator_test.dart
apps/qyro/test/ffi/qyro_native_api_test.dart
apps/qyro/test/localization_contract_test.dart
apps/qyro/test/qyro_app_test.dart
apps/qyro/test/scramble_decode_engine_test.dart
apps/qyro/test/startup_coordinator_test.dart
apps/qyro/windows/runner/main.cpp
apps/qyro/windows/runner/win32_window.cpp
design/brand/source/README.md
docs/LICENSE_AUDIT.md
docs/REPOSITORY_RENAME.md
docs/adr/ADR-0012-build-time-branding.md
docs/adr/ADR-0013-startup-coordinator.md
docs/adr/ADR-0014-canonical-logo.md
docs/adr/ADR-0015-branch-reconciliation.md
docs/adr/ADR-0016-qyro1-wire-framing.md
docs/adr/ADR-0017-manifest-serialization.md
docs/adr/ADR-0018-protocol-semantic-errors.md
docs/adr/ADR-0019-manifest-display-name.md
docs/adr/ADR-0020-device-identity-foundation.md
docs/adr/ADR-0021-authenticated-handshake.md
docs/adr/ADR-0022-qyro1-frame-aead.md
docs/adr/ADR-0023-crypto-platform-test-harness.md
docs/adr/ADR-0024-secure-identity-storage.md
docs/adr/ADR-0025-android-keystore-identity-storage.md
docs/adr/ADR-0026-transfer-session.md
docs/adr/ADR-0027-filesystem-materialisation.md
docs/audits/CLAUDE_RECOVERY_AUDIT.md
docs/audits/SPRINT4B_HANDSHAKE_AUDIT.md
docs/audits/SPRINT4C1_CRYPTO_PLATFORM_AUDIT.md
docs/audits/SPRINT4C2_AUDIT_CLOSURE.md
docs/audits/SPRINT4C3_RESOURCE_BOUNDS.md
docs/audits/SPRINT4C_AEAD_AUDIT.md
docs/audits/SPRINT4D1_SECURE_STORAGE.md
docs/audits/external/README.md
docs/prompts/2026-08-04-master.md
docs/protocols/manifest-format.md
docs/protocols/qyro1-wire-format.md
docs/reports/5C-codex.md
docs/security/authenticated-handshake.md
docs/security/device-identity.md
docs/security/frame-encryption.md
docs/security/handshake-state-machine.md
docs/security/handshake-threat-analysis.md
docs/security/identity-storage.md
docs/security/nonce-lifecycle.md
docs/security/parser-threats.md
docs/security/replay-window.md
docs/security/secret-lifecycle-audit.md
docs/security/test-vectors/README.md
docs/security/test-vectors/aead-v1.json
docs/security/test-vectors/aead-v1.schema.json
docs/security/test-vectors/handshake-v1.json
docs/security/test-vectors/handshake-v1.schema.json
docs/security/test-vectors/identity-v1.json
docs/security/test-vectors/rfc4231-hmac-sha256.json
docs/security/test-vectors/rfc7748-x25519.json
docs/security/test-vectors/rfc8032-ed25519.json
docs/security/test-vectors/rfc8439-chacha20poly1305.json
docs/security/test-vectors/storage-v1.json
docs/security/test-vectors/storage-v1.schema.json
docs/testing/crypto-fuzzing.md
docs/testing/crypto-platform-matrix.md
rust/crates/qyro_core/src/lib.rs
rust/crates/qyro_crypto/Cargo.toml
rust/crates/qyro_crypto/src/aead/corpus.rs
rust/crates/qyro_crypto/src/aead/error.rs
rust/crates/qyro_crypto/src/aead/guards.rs
rust/crates/qyro_crypto/src/aead/mod.rs
rust/crates/qyro_crypto/src/aead/replay.rs
rust/crates/qyro_crypto/src/aead/tests.rs
rust/crates/qyro_crypto/src/aead/vectors.rs
rust/crates/qyro_crypto/src/error.rs
rust/crates/qyro_crypto/src/fingerprint.rs
rust/crates/qyro_crypto/src/fuzzing.rs
rust/crates/qyro_crypto/src/guards.rs
rust/crates/qyro_crypto/src/handshake/closure_tests.rs
rust/crates/qyro_crypto/src/handshake/error.rs
rust/crates/qyro_crypto/src/handshake/mod.rs
rust/crates/qyro_crypto/src/handshake/schedule.rs
rust/crates/qyro_crypto/src/handshake/tests.rs
rust/crates/qyro_crypto/src/handshake/transcript.rs
rust/crates/qyro_crypto/src/handshake/vectors.rs
rust/crates/qyro_crypto/src/identity.rs
rust/crates/qyro_crypto/src/lib.rs
rust/crates/qyro_crypto/src/schema.rs
rust/crates/qyro_crypto/src/signature.rs
rust/crates/qyro_crypto/src/vectors.rs
rust/crates/qyro_ffi/Cargo.toml
rust/crates/qyro_ffi/tests/c_abi_contract.rs
rust/crates/qyro_fs/Cargo.toml
rust/crates/qyro_fs/src/error.rs
rust/crates/qyro_fs/src/guards.rs
rust/crates/qyro_fs/src/io.rs
rust/crates/qyro_fs/src/lib.rs
rust/crates/qyro_fs/src/manifest_builder.rs
rust/crates/qyro_fs/src/resume.rs
rust/crates/qyro_fs/src/safe_path.rs
rust/crates/qyro_fs/src/tests.rs
rust/crates/qyro_identity_store/Cargo.toml
rust/crates/qyro_identity_store/src/blob.rs
rust/crates/qyro_identity_store/src/error.rs
rust/crates/qyro_identity_store/src/guards.rs
rust/crates/qyro_identity_store/src/lib.rs
rust/crates/qyro_identity_store/src/tests.rs
rust/crates/qyro_manifest/Cargo.toml
rust/crates/qyro_manifest/src/codec.rs
rust/crates/qyro_manifest/src/error.rs
rust/crates/qyro_manifest/src/guards.rs
rust/crates/qyro_manifest/src/lib.rs
rust/crates/qyro_manifest/src/limits.rs
rust/crates/qyro_manifest/src/model.rs
rust/crates/qyro_manifest/src/path.rs
rust/crates/qyro_manifest/tests/ancestor_collision_contract.rs
rust/crates/qyro_manifest/tests/common/mod.rs
rust/crates/qyro_manifest/tests/corpus_smoke.rs
rust/crates/qyro_manifest/tests/decode_guard_contract.rs
rust/crates/qyro_manifest/tests/manifest_contract.rs
rust/crates/qyro_manifest/tests/portable_collision_contract.rs
rust/crates/qyro_manifest/tests/property.rs
rust/crates/qyro_manifest/tests/unicode_path_contract.rs
rust/crates/qyro_protocol/Cargo.toml
rust/crates/qyro_protocol/src/decoder.rs
rust/crates/qyro_protocol/src/envelope.rs
rust/crates/qyro_protocol/src/error.rs
rust/crates/qyro_protocol/src/frame.rs
rust/crates/qyro_protocol/src/guards.rs
rust/crates/qyro_protocol/src/header.rs
rust/crates/qyro_protocol/src/lib.rs
rust/crates/qyro_protocol/src/limits.rs
rust/crates/qyro_protocol/src/message.rs
rust/crates/qyro_protocol/src/session.rs
rust/crates/qyro_protocol/src/version.rs
rust/crates/qyro_protocol/tests/corpus_smoke.rs
rust/crates/qyro_protocol/tests/forward_compatibility.rs
rust/crates/qyro_protocol/tests/plain_encrypted_boundary.rs
rust/crates/qyro_protocol/tests/property.rs
rust/crates/qyro_protocol/tests/public_api_contract.rs
rust/crates/qyro_protocol/tests/session_id_contract.rs
rust/crates/qyro_protocol/tests/wire_contract.rs
rust/crates/qyro_transfer/Cargo.toml
rust/crates/qyro_transfer/src/error.rs
rust/crates/qyro_transfer/src/guards.rs
rust/crates/qyro_transfer/src/lib.rs
rust/crates/qyro_transfer/src/session.rs
rust/crates/qyro_transfer/src/tests.rs
rust/crates/qyro_transfer/src/wire.rs
rust/crates/qyro_win_dpapi/Cargo.toml
rust/crates/qyro_win_dpapi/src/ffi.rs
rust/crates/qyro_win_dpapi/src/guards.rs
rust/crates/qyro_win_dpapi/src/lib.rs
rust/crates/qyro_win_dpapi/src/store.rs
rust/crates/qyro_win_dpapi/src/tests.rs
rust/fuzz/Cargo.lock
rust/fuzz/Cargo.toml
rust/fuzz/corpus/frame_decoder/all_ff.bin
rust/fuzz/corpus/frame_decoder/all_flags.bin
rust/fuzz/corpus/frame_decoder/all_zero.bin
rust/fuzz/corpus/frame_decoder/bad_magic.bin
rust/fuzz/corpus/frame_decoder/bad_major.bin
rust/fuzz/corpus/frame_decoder/data_chunk.bin
rust/fuzz/corpus/frame_decoder/empty.bin
rust/fuzz/corpus/frame_decoder/empty_hello.bin
rust/fuzz/corpus/frame_decoder/future_minor.bin
rust/fuzz/corpus/frame_decoder/header_len_too_big.bin
rust/fuzz/corpus/frame_decoder/header_len_too_small.bin
rust/fuzz/corpus/frame_decoder/heartbeat.bin
rust/fuzz/corpus/frame_decoder/hostile_payload_len.bin
rust/fuzz/corpus/frame_decoder/max_ids.bin
rust/fuzz/corpus/frame_decoder/one_byte.bin
rust/fuzz/corpus/frame_decoder/reserved_byte_set.bin
rust/fuzz/corpus/frame_decoder/reserved_flag_set.bin
rust/fuzz/corpus/frame_decoder/sealed_bad_sequence.bin
rust/fuzz/corpus/frame_decoder/sealed_bad_tag.bin
rust/fuzz/corpus/frame_decoder/sealed_both_flags.bin
rust/fuzz/corpus/frame_decoder/sealed_empty_payload.bin
rust/fuzz/corpus/frame_decoder/sealed_flag_without_trailer.bin
rust/fuzz/corpus/frame_decoder/sealed_missing_tag.bin
rust/fuzz/corpus/frame_decoder/sealed_oversize_trailer.bin
rust/fuzz/corpus/frame_decoder/sealed_responder.bin
rust/fuzz/corpus/frame_decoder/sealed_short_payload.bin
rust/fuzz/corpus/frame_decoder/sealed_truncated_header.bin
rust/fuzz/corpus/frame_decoder/sealed_truncated_tag.bin
rust/fuzz/corpus/frame_decoder/sealed_two_frames.bin
rust/fuzz/corpus/frame_decoder/sealed_wrong_session.bin
rust/fuzz/corpus/frame_decoder/trailer_present.bin
rust/fuzz/corpus/frame_decoder/truncated_header.bin
rust/fuzz/corpus/frame_decoder/truncated_payload.bin
rust/fuzz/corpus/frame_decoder/two_frames.bin
rust/fuzz/corpus/frame_decoder/type_zero.bin
rust/fuzz/corpus/frame_decoder/unknown_type.bin
rust/fuzz/corpus/manifest_decoder/absolute.bin
rust/fuzz/corpus/manifest_decoder/all_zero.bin
rust/fuzz/corpus/manifest_decoder/backslash.bin
rust/fuzz/corpus/manifest_decoder/bad_magic.bin
rust/fuzz/corpus/manifest_decoder/bad_option_tag.bin
rust/fuzz/corpus/manifest_decoder/bad_version.bin
rust/fuzz/corpus/manifest_decoder/directory.bin
rust/fuzz/corpus/manifest_decoder/drive_prefix.bin
rust/fuzz/corpus/manifest_decoder/empty.bin
rust/fuzz/corpus/manifest_decoder/hostile_item_count.bin
rust/fuzz/corpus/manifest_decoder/huge_total.bin
rust/fuzz/corpus/manifest_decoder/large_item_count.bin
rust/fuzz/corpus/manifest_decoder/nested.bin
rust/fuzz/corpus/manifest_decoder/nul_in_path.bin
rust/fuzz/corpus/manifest_decoder/one_file.bin
rust/fuzz/corpus/manifest_decoder/reserved_name.bin
rust/fuzz/corpus/manifest_decoder/sha256_hash.bin
rust/fuzz/corpus/manifest_decoder/trailing_bytes.bin
rust/fuzz/corpus/manifest_decoder/traversal.bin
rust/fuzz/corpus/manifest_decoder/v2_bad_hash_len.bin
rust/fuzz/corpus/manifest_decoder/v2_blake3.bin
rust/fuzz/corpus/manifest_decoder/v2_case_collision.bin
rust/fuzz/corpus/manifest_decoder/v2_dir_with_hash.bin
rust/fuzz/corpus/manifest_decoder/v2_directory.bin
rust/fuzz/corpus/manifest_decoder/v2_empty.bin
rust/fuzz/corpus/manifest_decoder/v2_file_without_hash.bin
rust/fuzz/corpus/manifest_decoder/v2_hostile_count.bin
rust/fuzz/corpus/manifest_decoder/v2_illegal_char.bin
rust/fuzz/corpus/manifest_decoder/v2_mime_mtime.bin
rust/fuzz/corpus/manifest_decoder/v2_nested.bin
rust/fuzz/corpus/manifest_decoder/v2_nfd_collision.bin
rust/fuzz/corpus/manifest_decoder/v2_one_file.bin
rust/fuzz/corpus/manifest_decoder/v2_trailing.bin
rust/fuzz/corpus/manifest_decoder/v2_traversal.bin
rust/fuzz/corpus/manifest_decoder/v2_unicode.bin
rust/fuzz/corpus/manifest_decoder/with_mime_and_mtime.bin
rust/fuzz/corpus/relative_path/absolute.txt
rust/fuzz/corpus/relative_path/backslash.txt
rust/fuzz/corpus/relative_path/control.txt
rust/fuzz/corpus/relative_path/deep.txt
rust/fuzz/corpus/relative_path/dot_segment.txt
rust/fuzz/corpus/relative_path/double_slash.txt
rust/fuzz/corpus/relative_path/drive.txt
rust/fuzz/corpus/relative_path/empty.txt
rust/fuzz/corpus/relative_path/invalid_utf8.txt
rust/fuzz/corpus/relative_path/long_segment.txt
rust/fuzz/corpus/relative_path/nested.txt
rust/fuzz/corpus/relative_path/nul_byte.txt
rust/fuzz/corpus/relative_path/only_slash.txt
rust/fuzz/corpus/relative_path/reserved_com1_ext.txt
rust/fuzz/corpus/relative_path/reserved_con.txt
rust/fuzz/corpus/relative_path/simple.txt
rust/fuzz/corpus/relative_path/space_name.txt
rust/fuzz/corpus/relative_path/trailing_dot.txt
rust/fuzz/corpus/relative_path/trailing_space.txt
rust/fuzz/corpus/relative_path/traversal.txt
rust/fuzz/corpus/relative_path/unc.txt
rust/fuzz/corpus/relative_path/unicode.txt
rust/fuzz/fuzz_targets/encrypted_envelope.rs
rust/fuzz/fuzz_targets/frame_decoder.rs
rust/fuzz/fuzz_targets/frame_opener.rs
rust/fuzz/fuzz_targets/manifest_decoder.rs
rust/fuzz/fuzz_targets/relative_path.rs
rust/fuzz/fuzz_targets/replay_window.rs
rust/guards/source_guard.rs
rust/tools/qyro_crypto_smoke/Cargo.toml
rust/tools/qyro_crypto_smoke/include/module.modulemap
rust/tools/qyro_crypto_smoke/include/qyro_crypto_smoke.h
rust/tools/qyro_crypto_smoke/ios/Package.swift
rust/tools/qyro_crypto_smoke/ios/Tests/QyroCryptoSmokeTests/QyroCryptoSmokeTests.swift
rust/tools/qyro_crypto_smoke/src/lib.rs
rust/tools/qyro_crypto_smoke/src/main.rs
rust/tools/qyro_store_smoke/Cargo.toml
rust/tools/qyro_store_smoke/src/main.rs
scripts/check_crypto_platform_evidence.ps1
scripts/check_crypto_platform_evidence.sh
scripts/check_docs_consistency.ps1
scripts/check_docs_consistency.sh
scripts/check_harness_isolation.ps1
scripts/check_harness_isolation.sh
scripts/check_repo_portability.ps1
scripts/check_repo_portability.sh
scripts/tests/crypto_platform_evidence_contract_test.ps1
scripts/tests/crypto_platform_evidence_contract_test.sh
scripts/tests/docs_consistency_contract_test.ps1
scripts/tests/docs_consistency_contract_test.sh
scripts/tests/launch_surface_contract_test.ps1
scripts/tests/launch_surface_contract_test.sh
scripts/tests/repo_portability_contract_test.ps1
scripts/tests/repo_portability_contract_test.sh
scripts/tests/windows_package_contract_test.ps1
scripts/verify_windows_package.ps1
tools/branding_generator/bin/generate.dart
tools/branding_generator/lib/branding_generator.dart
tools/branding_generator/pubspec.yaml
tools/logo_ascii_generator/generate.py
tools/logo_ascii_generator/test_logo_ascii_generator.py
```

El delta propio contra la base exigida es:

```text
docs/reports/5C-codex.md
```

## 14. Todos los runs de CI de la rama

| Run | Commit | Workflow | Evento | Conclusión |
|---|---|---|---|---|
| 31520332918 | 15934aae3dda7f469b5496c8341eb78d9e32f335 | CI | workflow_dispatch | success |
| 31521002851 | a1c7398fbc2d7ef903282f3d64cfb19da23dcf42 | CI | workflow_dispatch | failure global; `rust` PASS, `documentation` FAIL por ledger |

La lista se reconstruirá por API al cierre, sin filtrar fallos ni cancelaciones.

## 15. Qué NO debe leerse como progreso

Este sprint no mueve el producto: cierra deuda de pruebas y de contrato. No hay red, sockets, descubrimiento, FFI del motor ni selector de archivos; Enviar y Recibir siguen deshabilitados. No hay persistencia de identidad en Android ni iOS. QYR-0072 sigue abierta deliberadamente. Nada se ha probado en hardware físico.

## 16. Documentación desfasada y handoff al sprint siguiente

El sprint no llegó a cambiar la superficie de cabecera. El siguiente paso necesita resolver primero la incompatibilidad entre el reporte verbatim, la regla de IDs del checker y la prohibición de tocar el ledger. Después, la superficie de cabecera que resulte de la Fase 5 debe comunicarse al agente de red sin modificar los documentos raíz prohibidos.
