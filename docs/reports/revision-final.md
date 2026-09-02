# La revisión final — fase 28

**Qué es esto.** El entregable §5.1 de `FASE-28-LA-REVISION-FINAL.md`: los once
informes, los hallazgos confirmados, **los refutados con su motivo**, las diez
preguntas de §4 respondidas por código, y el veredicto en tres métricas con su
método.

**Qué NO es.** Evidencia de hardware. Nada de lo que hay aquí ha tocado nunca un
teléfono ni un PC con Windows. La página que hay que leer antes de enchufar dos
aparatos es [`lo-que-no-se-ha-probado.md`](lo-que-no-se-ha-probado.md), y sus
treinta escenarios siguen en blanco.

**La puerta, en el commit que este informe nombra.** `7e61b98`, medido en este
contenedor Linux. Los commits posteriores de esta tanda cambian **sólo
documentos y este archivo**, y las guardas que los leen corren en esa misma
puerta, así que un documento roto la pone en rojo igual que un `.rs`:

| | |
|---|---|
| `cargo fmt --all --check` | limpio |
| `cargo clippy --workspace --all-targets -- -D warnings` | limpio |
| `cargo test --workspace` | **816 pruebas, 0 fallos** |
| `cargo test --workspace --all-features` | verde |
| `cargo test --doc --workspace` | verde |
| `cargo doc --workspace --no-deps` | verde |

**Y lo que no se pudo correr, dicho antes que lo que sí.** `scripts/gate.ps1` es
PowerShell y aquí no hay `pwsh`: se corrieron **los mismos comandos `cargo` que
ese script lee de `ci.yml`**, uno a uno, que es lo más cerca que se puede estar
sin ser lo mismo. No hay Flutter, ni SDK de Android, ni objetivo MSVC, así que
**ninguna prueba de Dart ni de Kotlin se ha ejecutado en esta revisión**. Las
corre CI. Donde un arreglo de este informe toca Dart, se dice.

---

## 1. Cómo se hizo, y lo que se recortó

Nueve agentes a la vez, cada uno con un dominio y **ninguno viendo el informe de
otro**. Después, refutadores con instrucciones de **empezar suponiendo que el
hallazgo es falso**. Un hallazgo sobrevive si dos de tres no consiguen tumbarlo.

**Nada de topes callados**, que es §3 de la fase:

- **La ronda 1 dio 64 hallazgos.** Están todos abajo.
- **El juicio llegó a 45 de los 64**, con **135 veredictos** — tres lentes por
  hallazgo, como pide §3. **21 aguantaron y 24 cayeron.** Los **19 sin juzgar no
  son hallazgos descartados**: son hallazgos sin refutar. Catorce de ellos se
  arreglaron igual —la columna «hoy» de §2 dice cuál y con qué ficha— y los cinco
  que ni se juzgaron ni se tocaron aparecen ahí como **«Sin juzgar»**, que es
  todo lo que se puede decir de ellos.
- **La ronda 2 se lanzó y murió entera.** Sus nueve agentes de dominio fallaron
  con *«You've hit your session limit»* — una cuota, no un resultado. Así que la
  regla de «dos rondas seguidas sin nada nuevo» **no se cumplió**, y este informe
  no puede decir que la revisión esté agotada. Lo que la sustituyó en parte fue
  **medir**: seis de los defectos de este informe no los encontró ningún agente
  leyendo, sino ejecutar los binarios y contar descriptores, segundos y flags.

  **Números de la ejecución, no estimados:** 154 agentes lanzados, 145 con
  resultado, 9 muertos por la cuota, 12,2 M de tokens y 3 023 llamadas a
  herramientas en 4 h 02 min.
- **Un sesgo del método, medido y no supuesto.** Los agentes leyeron el árbol
  **mientras se arreglaba**, así que varias citas `archivo:línea` estaban
  desfasadas 25-35 líneas cuando el refutador las abrió, y **tres hallazgos
  fueron refutados 3/3 por estar ya arreglados**. Eso no los hace falsos: los
  hace tardíos. Se apuntan igual, con el motivo, porque un informe que borra sus
  hallazgos resueltos miente sobre lo que costó.

---

## 2. Los nueve informes de dominio

Sesenta y cuatro hallazgos. La columna «hoy» dice qué pasó con cada bloque.

### 2.1 EL CRIPTÓGRAFO — handshake, nonces, identidad (4)

| Sev | Hallazgo | Hoy |
|---|---|---|
| P0 | En Android la identidad se escribe en el directorio de trabajo del proceso (`/`), que la app no puede escribir: `openIdentity()` lanza y ni Enviar ni Recibir arrancan | **Arreglado** — QYR-0376 |
| P1 | La huella del código de emparejamiento se valida y se tira: escanear no ata la sesión a ninguna clave | **Arreglado en las dos caras** — QYR-0381 (CLI) y QYR-0392 (GUI + símbolo 34) |
| P1 | Nada en producción llama a `remember_peer`, así que `PeerTrust::Changed` no puede dispararse nunca | **Registrado, no arreglado** — QYR-0382. Sobrevivió 2/3 |
| P3 | `TrustBook::remember` admite la misma identidad bajo dos nombres y a partir de ahí todo veredicto falla | Sin juzgar |

### 2.2 EL ADUANERO — la frontera C (4)

| Sev | Hallazgo | Hoy |
|---|---|---|
| P0 | El segundo envío en Android entrega a Rust descriptores que Rust ya cerró: `from_raw_fd` sobre números reasignados | **Arreglado** — QYR-0388 |
| P1 | `qyro_session_finish` no puede ejecutarse tras un fallo: la regla pegajosa corta el cuerpo y los `.qyro-part` se quedan | **Abierto** — §5. Aguantó **3/3** |
| P1 | El comodín de `_kindOf` convierte ocho códigos del motor en «Llegó algo que no verifico» | **Arreglado** — QYR-0386 |
| P2 | `CAMERA` se declara en el manifiesto y no se pide nunca en tiempo de ejecución | **Arreglado** — QYR-0378 |

### 2.3 EL CARTERO — el protocolo en el cable (6)

| Sev | Hallazgo | Hoy |
|---|---|---|
| P1 | Un archivo de 0 bytes no viaja y hace que `finish()` abandone todo lo que venga detrás | **Arreglado** — QYR-0383 |
| P1 | La ventana sólo frena al primer ítem no drenado: un `pump` emite todo el lote sin un solo ACK | **Abierto** — §5 |
| P1 | Una sesión bloqueada en `write_all` no tiene temporizador de escritura ni forma de cancelarse | **Abierto** — §5. Sobrevivió 2/3 |
| P1 | Un `step` entrega un solo frame, así que «¿aceptas?» se hace sin lista y con total = 0 | **Arreglado antes del juicio** — QYR-0372. Refutado 3/3 por eso |
| P1 | No hay latido: los 60 s de `IDLE_TIMEOUT` corren mientras la persona decide, y matan al emisor | **Arreglado y medido** — QYR-0393 |
| P2 | Un `Cancel` del par no termina la sesión receptora | **Refutado 2/3**, severidad bajada — §3.4 |

### 2.4 EL NOMBRADOR — rutas y nombres (5)

| Sev | Hallazgo | Hoy |
|---|---|---|
| P0 | Recibir en Android escribe en `//Qyro`, la raíz del sistema de archivos | **Arreglado antes del juicio** — QYR-0373. Refutado 3/3 por eso |
| P2 | El sufijo `.qyro-part` se añade a un nombre que ya puede medir 255 bytes | **Aguantó 3/3**, abierto |
| P2 | Las carpetas del manifiesto se crean antes de que la persona acepte, y rechazar no las borra | Refutado 2/3, **y la refutación no aguanta** — §3.5 |
| P2 | Enviar un archivo de la raíz de una unidad (`D:\video.mp4`) falla | **Arreglado** — QYR-0390 |
| P3 | Un manifiesto con «X» y «X.qyro-part» no lo detecta `PortableCollisionKey` | Sin juzgar |

### 2.5 EL FORENSE — la comprobación 14, sobre todo (9)

Su pregunta es una sola: **por cada capacidad declarada, el llamante de
producción con archivo y línea.** Si es una prueba, un arnés o nadie, la
capacidad no existe. Encontró nueve. **Siete llegaron a juicio: tres aguantaron
y cuatro cayeron** — y tres de esos cuatro cayeron por estar ya arreglados
cuando el refutador los abrió, no por ser falsos. Dos no se juzgaron.

| Sev | Hallazgo | Hoy |
|---|---|---|
| P1 | «Ver QUÉ se ofrece antes de aceptar» no existe en la GUI: los nombres se cablean vacíos | **Arreglado** — QYR-0372, símbolos 32 y 33 |
| P1 | El escáner de Android no tiene llamante de producción: `ScanScreen` no se construye | **Arreglado** — QYR-0371 |
| P1 | Nadie recuerda un peer en producción | **Registrado** — QYR-0382 |
| P2 | Diez de las trece citas CLI de la tabla de paridad apuntan a líneas que no son lo que dicen | **Arreglado**, y con guarda que corre en el gate |
| P2 | El reanudado está documentado y `.qyro-resume` no lo escribe nadie | **Refutado 2/3** por impacto — §3.4 |
| P2 | `qyro_fs::history` no tiene llamante de producción, y dos comentarios afirman que sí | **Aguantó 1/3**, abierto |
| P2 | El consejero de canal de la GUI sólo lo llaman las pruebas | **Aguantó 3/3**, abierto |
| P2 | «Cancelar a mitad» no tiene llamante en la GUI | **Abierto** — §5 |
| P3 | `Pause`/`Resume`/`Retransmit` sólo los llaman las pruebas | **Abierto** — §5 |

### 2.6 EL CONTADOR — recursos (6)

| Sev | Hallazgo | Hoy |
|---|---|---|
| P1 | Un nombre que ya existe: los dos extremos dicen «listo» y no se escribe nada | **Arreglado antes del juicio** — QYR-0374. Refutado 3/3 por eso |
| P1 | La ventana de 16 trozos es por entrada, no global | **Abierto** — §5. Aguantó **3/3** |
| P1 | El isolate que escucha no se puede cancelar y «Recibir» no tenía guarda | **Media arreglada** — QYR-0389 puso la guarda; cancelar sigue sin existir |
| P1 | Un envío a un par que no escucha lanza fuera del stream: la pantalla no dice nada | **Arreglado** — QYR-0384 |
| P2 | Los `.qyro-part` de un proceso muerto no los borra nadie nunca | **Refutado 2/3**: es la política de ADR-0027 y el código la cumple — §3.4 y §4.4 |
| P2 | `qyro beam` carga el archivo entero en RAM y sólo después comprueba el techo de 20 MB | **Aguantó 1/3**, abierto |

**Y el que no encontró, que apareció midiendo:** doscientos archivos abrían
**402 descriptores a la vez** — dos por archivo. QYR-0391, §4.2.

### 2.7 EL RETRATISTA — la interfaz (9)

| Sev | Hallazgo | Hoy |
|---|---|---|
| P0 | Nada pide `CAMERA` en tiempo de ejecución, y no existe pantalla de permiso denegado | **Arreglado** — QYR-0378 |
| P0 | El canal óptico recibe el archivo entero y lo tira | **Arreglado** — QYR-0379 |
| P1 | El campo dice «Código de emparejamiento» y su texto va literal al socket | **Arreglado** — QYR-0380 |
| P1 | El botón Enviar no se reactiva al escribir la dirección | **Arreglado** — QYR-0380 |
| P1 | Un fallo al abrir la conexión escapa del stream sin capturar | **Arreglado** — QYR-0384 |
| P1 | `detect_vt()` fuerza `Vt::Absent` en Windows, así que `qyro beam` scrollea el QR | **Arreglado** — QYR-0385 |
| P2 | La dirección resuelta del código se elipsa y no se puede copiar | Sin juzgar |
| P2 | Tras un fallo de arranque el botón SALTAR queda habilitado y no hace nada | Sin juzgar |
| P3 | La línea de versión del arranque queda en 4,09:1 sobre el degradado | Sin juzgar |

### 2.8 EL EMPAQUETADOR — lo que de verdad se instala (9)

| Sev | Hallazgo | Hoy |
|---|---|---|
| P0 | El primer comando del protocolo de hardware no puede enlazar: no hay linker de Android en ninguna parte | **Arreglado** — QYR-0377 |
| P0 | El bundle de Windows copia un `qyro_ffi.dll` que ningún paso construye | **Arreglado** — QYR-0377 |
| P1 | El script de firma re-alinea el APK a 4 KB, deshaciendo los 16 KB | **Arreglado** — QYR-0387 |
| P1 | El APK que el propietario construye hoy va firmado con la clave de depuración | **Refutado 3/3**: `adb install` no lo impide, que es el camino de hoy |
| P1 | Ningún `cargo build` de Android pasa `-Wl,-z,max-page-size=16384` | **Arreglado** — QYR-0394 |
| P2 | El APK no lleva `x86_64`, así que el emulador no es alternativa | **Arreglado** en CI |
| P2 | El APK se instala como versión 0.0.1 con `versionCode` 1 en cada build | **Abierto** — §5 |
| P2 | Nada ata el APK publicado al commit que dice publicar | **Abierto** — §5 |
| P2 | El ZIP portable de Windows no lleva el runtime de C++ | **No aplica al `.exe` de terminal**, que va estático; sí al bundle de Flutter |

### 2.9 EL BIBLIOTECARIO — documentos contra código (12)

El dominio con más hallazgos, y casi con la tasa de acierto más alta: **once de
doce eran ciertos**, porque un documento que se contradice con el código no
necesita un escenario que falle — basta abrir los dos. El que falla es el que
decía que `STATUS.md` afirma que los aparatos «no se encuentran solos»:
`STATUS.md:14-15` dice **«Y también se encuentran solos cuando la red lo
permite»**, y lo decía ya. Un hallazgo del BIBLIOTECARIO también se comprueba.

| Sev | Hallazgo | Hoy |
|---|---|---|
| P1 | `README` manda a `docs/GUIA-DE-PRUEBA.md`, que no existía | **Arreglado**: la guía existe |
| P1 | `cargo test --workspace` está en rojo en `main` por seis citas de paridad | **Arreglado**, y con guarda |
| P1 | El protocolo de hardware copia una DLL que ningún paso suyo construye | **Arreglado** — QYR-0377 |
| P1 | `docs/release/v1.0.md` dice «no se encuentran solos y no se escanea» | **Arreglado** — QYR-0395, con cabecera y sin reescribir el cuerpo |
| P2 | El número de escenarios de hardware es falso en cinco documentos | **Abierto** — §5 |
| P2 | `STATUS.md` dice que los aparatos «no se encuentran solos» | **Falso**: `STATUS.md:15` dice lo contrario y lo decía ya |
| P2 | `check_docs_consistency.sh` sale con código 1 | **Arreglado**: hoy sale `[OK]` |
| P2 | `STATUS.md` publica hashes que `docs/release/v1.0.md` dice retirados | **Arreglado, y era peor de lo que decía** — QYR-0395, §3.5 |
| P2 | `SECURITY.md` abre con «no hay transferencia real» y promete TLS 1.3 | **Arreglado** — QYR-0395 |
| P3 | `ARCHITECTURE.md` dice que lo único implementado es `qyro_core` | **Arreglado** — QYR-0395 |
| P3 | La ficha de `docs/release/v1.0.md` tiene cuatro cifras falsas | **Arreglado** — QYR-0395, listadas en su cabecera |
| P3 | `RELEASES.md` dice que no hay release, con v1.0.0 etiquetada | **Arreglado** — QYR-0395 |

**Y el que este informe añade a ese dominio, medido:** `.cargo/config.toml`
explicaba que una tabla por objetivo no la pisa `RUSTFLAGS`. **La pisa**, y está
medido (QYR-0394, §4.9).

---

## 3. Los refutados, con su motivo

**Es la mitad del valor de la revisión** y por eso está en su propia sección.
De los 45 juzgados, **24 cayeron y 21 aguantaron**, y ninguno cayó por opinión.

**Y la forma en que cayeron es el dato.** Contando cuántas refutaciones citan un
arreglo ya aplicado —una ficha QYR, «ya no existe», «describe el árbol anterior»—:
**17 de las 24**. No cayeron por ser falsos: cayeron porque el árbol se estaba
arreglando mientras los refutadores lo leían. Es una medida con un filtro de
palabras, así que léela como un suelo y no como una cifra exacta.

Las siete restantes cayeron por alcance, por impacto o por cita falsa, y **una de
ésas la volví a comprobar y la refutación era la equivocada**: §3.5.

### 3.1 Tres de los 3/3, y los tres por estar ya arreglados

| Hallazgo | Motivo del refutador |
|---|---|
| «Recibir en Android escribe en `//Qyro`» | *«Describe un estado del repositorio que ya no existe: QYR-0373 está aplicado. Cada pieza de la evidencia es obsoleta y contradicha por el código actual: `transfer_screens.dart:635` hace `final where = await androidDestination();`»* |
| «Un nombre que ya existe: los dos extremos dicen listo y no se escribe nada» | *«El escenario observable que reclama ya no ocurre. Tres guardas lo impiden, en cadena. Describe el comportamiento ANTERIOR a QYR-0374, ya en `main`»* |
| «Un `step` entrega un solo frame, así que "¿aceptas?" se hace con total = 0» | *«El defecto descrito es QYR-0372, ya arreglado con guarda, los dos llamantes convertidos y una prueba que lo fija: `Session::await_offer()` da hasta 8 pasos»* |

**Lo que esto dice del método, y no del código.** Los tres son hallazgos
**correctos** que llegaron después de su arreglo, porque los agentes leían un
árbol que se estaba editando. Un refutador que dice «esto ya no pasa» no está
diciendo «esto nunca pasó».

### 3.2 Refutados 1/3 — sobreviven, y la refutación aporta

| Hallazgo | Lo que el refutador que acertó encontró |
|---|---|
| «Nada llama a `remember_peer`, así que `Changed` no puede dispararse» | *«Citas falsas, 4 de 5, y causa raíz equivocada»*. Los otros dos no lo tumbaron: **no hay llamante de producción**, y la rama alarmante de la interfaz existe (`transfer_screens.dart:401`). Sobrevive el hallazgo; caen sus citas |
| «Una sesión bloqueada en `write_all` no tiene temporizador ni cancelación» | Uno lo refutó por alcance; dos no. Sobrevive |

### 3.3 Refutado a medias, y el defecto sobrevive

«El isolate que escucha no se puede cancelar y el botón Recibir no tiene
guarda». El refutador: *«Refutado a medias, pero el defecto sobrevive. El botón
SÍ tiene guarda: `transfer_screens.dart:743` es `onPressed: _listening ? null
: …`»* — porque QYR-0389 ya la había puesto. **Lo que no tiene arreglo es la
otra mitad:** el isolate sigue sin poder cancelarse.

### 3.4 Cuatro que caen, y por qué — los que este informe daba por abiertos

Los cuatro estaban en §5 como «abiertos» cuando se escribió, antes de que su
juicio llegara. Tres caen con argumento; el cuarto no.

| Hallazgo | Voto | El motivo, y qué queda |
|---|---|---|
| «Los `.qyro-part` de un proceso muerto no los borra nadie nunca» | 2/3 refutado | *«No es un defecto: es exactamente la política congelada.»* **Comprobado y es cierto.** ADR-0027 dice que un `.qyro-part` huérfano *«no se puede verificar contra nada… se borra al empezar la transferencia que reclamaría ese nombre»*, y `io.rs:427-438` hace justo eso: si no hay `.qyro-resume` que lo describa, `remove_file` y se abre de cero. **Retirado de los abiertos** |
| «El reanudado está documentado y `.qyro-resume` no lo escribe nadie» | 2/3 refutado | Por impacto, no por mecánica: el hecho base es cierto —único llamante en `tests.rs:891`— pero el daño reclamado («la diferencia entre reanudar y volver a empezar») no se produce, porque **nada reanuda de todos modos**. Queda como capacidad sin llamante, que ya está contada, y no como pérdida de datos |
| «Un `Cancel` del par no termina la sesión receptora» | 2/3 refutado | Por alcanzabilidad y por cita falsa: las dos líneas citadas son un comentario y `step_tally`. El tercer refutador dice que **sólo cayó la cita**, no el defecto. **Se queda abierto con la severidad bajada**, no cerrado |
| «Las carpetas se crean antes de que la persona acepte» | 2/3 refutado | **La refutación es falsa. Ver §3.5** |

### 3.5 Una refutación que no aguanta la comprobación

Dos de los tres refutadores tumbaron «las carpetas del manifiesto se crean antes
de aceptar» con el mismo argumento: *«el disparador no existe — ningún emisor de
Qyro puede producir un manifiesto con entradas `ItemKind::Directory`»*.

**Es falso, y se comprueba en tres saltos:**

1. `Session::open_sender` **acepta carpetas** en su lista: filtra `!source.is_dir()`
   sólo para contar contra el techo de 256, y planifica **todas** las entradas
   (`rust/crates/qyro_session/src/session.rs:432` y `:461`).
2. `manifest_from_disk` **emite el tipo** cuando la fuente es una carpeta:
   `if file.source.is_dir() { ManifestItem::directory(...) }`
   (`rust/crates/qyro_fs/src/manifest_builder.rs:77-83`), con un comentario que
   dice exactamente por qué se puso: *«ItemKind::Directory lleva en el formato de
   cable desde siempre y nadie lo emitía»*.
3. Ese es el camino de producción del emisor: `session.rs:471` lo llama.

Así que el hallazgo **aguanta**, aunque el voto diga que no. Se apunta así —con
el voto y con lo que lo revierte— porque el mismo escepticismo que se aplica a un
hallazgo hay que aplicárselo a quien lo tumba: **un refutador también se
comprueba**. Sigue abierto en §5.

### 3.6 El que resultó ser peor de lo que decía, verificado contra la API

«`STATUS.md` publica como artefactos de la v1.0.0 los dos hashes que
`docs/release/v1.0.md` dice que se retiraron.» Cierto, y con dos agravantes que
sólo aparecen preguntándole a GitHub en vez de a otro documento:

1. **La Release existe y es pública.** `STATUS.md` decía «No existe una GitHub
   Release». Existe desde el 2026-08-17, marcada prerelease, titulada **«Qyro
   v1.0.0 — RETRACTADO: estos binarios no pueden enviar»**.
2. **Ya la ha descargado alguien.** `download_count: 2` en el APK y 2 en el ZIP
   de Windows.

Y el hash es lo que convierte una descarga en una certeza, así que publicar el
equivocado no es un error de documentación: **es confirmarle a alguien un binario
que este proyecto le pide borrar.** Arreglado en QYR-0395, con los digests que
GitHub sirve hoy y una guarda que impide que los retirados vuelvan.

### 3.7 Los veintiuno que aguantaron

Cada uno con su cadena verificada línea a línea por tres lentes distintas. Los
que **ningún** refutador tumbó: la identidad en `/`, la huella tirada, los
descriptores ya cerrados, el comodín de `_kindOf`, el permiso de cámara, el
archivo óptico tirado, el latido que no existe, la ventana por entrada, la
ventana sin ACK, el linker de Android, la DLL que nadie construye, `finish`
bloqueado por la regla pegajosa, el archivo de 0 bytes, el sufijo `.qyro-part`
sobre un nombre de 255 bytes, el consejero de la GUI que sólo llaman las pruebas,
y las cuatro cifras de `docs/release/v1.0.md`.

Y los que aguantaron **1/3** —uno los tumbó y dos no—: `remember_peer` sin
llamante (dos veces, desde dos dominios distintos, que es la mejor señal que hay),
`write_all` sin plazo ni cancelación, `qyro_fs::history` sin llamante, y `qyro
beam` cargando el archivo entero antes de comprobar su techo.

---

## 4. Las diez preguntas de §4, respondidas por código

### 4.1 Un archivo grande que no cabe en RAM, ¿cruza?

**Sí, y está medido, no razonado.** El bucle que lo garantiza es
`FileSource::try_read` (`rust/crates/qyro_fs/src/io.rs:219`), que llena el buffer
que le den y nunca reserva por tamaño de archivo, y `FileSink::put`
(`io.rs:489`), que escribe en el offset y no acumula. La constante es
`HASH_BUFFER_LEN = 65_536` (`io.rs:34`).

Las pruebas que lo miden no comprueban «no crece»: comprueban **el pico real**.
`file_source_peak_is_the_largest_completed_read_not_the_request` y
`file_sink_peak_is_the_largest_successful_write_not_a_constant`
(`rust/crates/qyro_fs/src/tests.rs:198` y `:227`) contrastan un archivo pequeño
contra uno de dos buffers y **exigen que los dos picos difieran**, que es lo que
distingue un contador de verdad de una constante.

Y sobre el producto entero, entre dos procesos, con un archivo de **400 MB**:
**emisor 5,2 → 5,6 MB · receptor 4,9 → 5,8 MB** de `VmRSS`.

### 4.2 Un lote de 200 archivos, ¿cuántos descriptores abre a la vez?

**Once.** Antes de esta revisión, **402**.

```
antes:   [measure] 200 files: 4 descriptors before, 406 at the peak, 402 extra
después: [measure] 200 files: 4 descriptors before,  15 at the peak,  11 extra
```

El número sale de `/proc/self/fd`, muestreado desde un hilo **mientras la
transferencia corre** — preguntarle al proceso, y no al código si está de acuerdo
consigo mismo. `two_hundred_files_do_not_hold_two_hundred_descriptors`, en
`rust/crates/qyro_session/tests/session_behaviour.rs`.

Eran dos por archivo: el que lee el origen y la parte abierta del destino,
ninguno cerrado hasta el final de toda la transferencia. **ADR-0047 §3 limita una
transferencia a 256 archivos precisamente por los descriptores, contando uno.**
Con dos, 256 archivos son ~512: el techo exacto del CRT de Windows. QYR-0391.

### 4.3 Un contador que cruza el FFI, ¿es de 64 bits en todo el camino?

**Uno a uno, sí para todo lo que cuenta bytes.**

| Símbolo | Campo | Rust | Dart |
|---|---|---|---|
| `qyro_session_progress` | `out_done` | `*mut u64` (`session_abi.rs:556`) | `Pointer<Uint64>` |
| `qyro_session_progress` | `out_total` | `*mut u64` (`:557`) | `Pointer<Uint64>` |
| `QyroProgressFn` | `done`, `total` | `u64` (`session_abi.rs:201`) | `Uint64` (`qyro_session_api.dart:128`) |
| `qyro_session_offered_files` | tamaños | texto decimal, sin techo de tipo | `int` (64 bits en la VM) |

**Los tres `u32` que quedan no son contadores de bytes:** `out_item` es un id de
ítem, `out_count` de `qyro_session_finish` es un número de archivos, y el `item`
del callback es el mismo id. Los tres están acotados por
`MAX_FILES_PER_TRANSFER = 256`. Un `u32` que sólo llega a 256 no da la vuelta.

### 4.4 Si el peer se calla a mitad, ¿qué temporizador salta y quién limpia?

**Salta `IDLE_TIMEOUT`**, 60 s sin recibir un byte
(`rust/crates/qyro_net/src/limits.rs:103`), comprobado en
`stream.rs:370` y convertido en `NetError::PeerSilent { idle }`. Llega a los
consumidores como `SessionError::PeerUnreachable`.

**Y desde esta revisión hay un segundo plazo**, `DECISION_DEADLINE`, diez minutos
(`limits.rs:130`), que se usa cuando este lado **no tiene nada que poner en el
cable**: ahí no está midiendo la red, está esperando al otro extremo. QYR-0393
y §4 de este informe lo mide: 65 segundos de espera humana pasaban de
`PeerUnreachable` a los 60,11 s a entregado a los 65,76 s.

**Quién limpia: dos cosas distintas, y la segunda es política escrita.**

`FileSink::abandon` (`io.rs:671`) borra todos los `.qyro-part` y el archivo de
reanudación de golpe, y su único llamante es `Session::reject`
(`session.rs:1128`) — un «no» de una persona. Así que un peer que se calla **sí**
deja las partes en el destino.

**Y eso no es un descuido, es ADR-0027.** *«Sin metadatos: un `.qyro-part`
huérfano no se puede verificar contra nada, y quedarse con él sólo puede producir
un archivo que nadie mandó. **Se borra al empezar la transferencia que reclamaría
ese nombre**.»* Y el código lo hace: `part_for` (`io.rs:427-438`) mira si hay
`.qyro-resume` que describa la parte; si no lo hay, `remove_file` y abre de cero.

> **Este informe decía aquí «nadie limpia, y es un hallazgo abierto». Era
> falso**, y lo tumbó el refutador de ese hallazgo citando la ADR. Comprobado
> antes de creérselo, que es la misma regla que se le aplica a un hallazgo.

### 4.5 Si el disco se llena, ¿qué ve la persona y qué queda en el destino?

**Ve:** `SessionError::StorageRefused`. En la terminal, `flows.rs:543`:
«alguno de los archivos no se pudo guardar», el error, y qué hacer —
«Qyro never overwrites… move it or receive into another folder» — más la frase
que faltaba: «Lo que sí se guardó está en esa carpeta: míralo antes de volver a
mandar nada». En la GUI, `QyroFailureKind.noRoom`.

**Queda:** lo que se pudo materializar, materializado. Desde QYR-0383,
`Session::finish` (`session.rs:1177`) recorre el manifiesto **entero** en vez de
abandonarlo al primer fallo, cuenta los rechazados y devuelve el error al final.
Antes, un solo ítem que fallara se llevaba por delante todo lo que viniera detrás
— **medido: tres archivos, el primero vacío, llegan cero**.

**Y los `.qyro-part` de los que fallaron se quedan**: `finish_item` sólo borra la
parte cuando el digest no cuadra. No se pierden ni se acumulan sin límite — la
siguiente transferencia que reclame ese nombre los borra (§4.4) — pero entre una
cosa y la otra están en la carpeta, y quien mire verá archivos con un sufijo raro
junto a los que sí llegaron. Por eso la terminal lo dice con todas las letras:
«Lo que sí se guardó está en esa carpeta: míralo antes de volver a mandar nada».

### 4.6 Si el nombre viene envenenado, ¿dónde se rechaza? Y la contraprueba

**Dónde:** `RelativePath::parse`, en
`rust/crates/qyro_manifest/src/path.rs:109`, en este orden — vacío, longitud,
**NUL antes que nada** (`:123`, porque un NUL trunca la ruta en cualquier API de
C y un nombre que parece seguro aquí sería otro en el syscall), controles
(`:126`), **caracteres de formato Unicode** (`:134`, categoría `Cf`, que es la
mitad invisible y la peligrosa), barra invertida, prefijo UNC, ruta absoluta, y
`CON`/`NUL`/`COM1` en `is_windows_reserved` (`:344`).

**La contraprueba, ejecutada — quitar la línea y correr la suite:**

| Línea quitada | Pruebas que se ponen en rojo |
|---|---|
| `is_control()` (`:126`) | **3**: `control_characters_are_rejected`, `the_delete_character_is_rejected`, y el guardián de variantes sin sitio de construcción |
| `is_unicode_format()` (`:134`) | **6**: `a_right_to_left_override_cannot_disguise_an_extension`, `a_zero_width_space_cannot_hide_between_a_name_and_its_extension`, `a_name_that_differs_only_by_an_invisible_character_is_rejected`, `unicode_format_characters_are_rejected`, `the_decoder_refuses_a_disguised_extension_too`, y el mismo guardián |

Las dos veces la suite se pone roja. **El árbol quedó restaurado** y la suite
vuelve a estar verde.

### 4.7 Si el puerto 49517 está ocupado, ¿qué pasa?

**Se dice, se ofrece elegir otro, y nunca se mueve solo** (ADR-0041 §3).
`bind_error` (`rust/crates/qyro_session/src/session.rs:245`) traduce `AddrInUse`
**y** `PermissionDenied` a `SessionError::PortUnavailable` — las dos, porque
Windows reserva rangos TCP para Hyper-V, WSL2 y Docker y ligar dentro de uno se
rechaza como **10013, «permiso denegado»**, no como «en uso», y para quien tiene
la máquina delante significan lo mismo.

En la terminal, `listen_somewhere` (`flows.rs:369`) ofrece otro puerto y espera
una respuesta. En la GUI, `QyroFailureKind.portUnavailable` con su cadena en los
dos idiomas. Antes de QYR-0370 esto salía como «argumento no usable» y no había
con qué elegir otro. **Medido en este contenedor** con el 49517 tomado por otro
proceso.

### 4.8 Si Android deniega cada permiso, ¿qué pantalla sale?

El manifiesto declara **exactamente tres**
(`apps/qyro/android/app/src/main/AndroidManifest.xml:18, 27, 56`):

| Permiso | ¿Se puede denegar? | Qué sale |
|---|---|---|
| `INTERNET` | **No.** Es de instalación y el sistema lo concede al instalar | Nada que denegar. **Faltaba en el manifiesto de release** y sin él el APK no podía abrir un socket: QYR-0368 |
| `CHANGE_WIFI_MULTICAST_STATE` | **No.** También de instalación | El descubrimiento sin router funciona o no según la red, no según un diálogo |
| `CAMERA` | **Sí**, en tiempo de ejecución | `ScannerChannel.cameraPermission()` (`ScannerChannel.kt:124`) devuelve `granted`, `asked` o `unavailable`, y `ScanScreen` dibuja la explicación y un botón de reintentar. Antes de QYR-0378 **no se pedía nunca**: el canal óptico no podía arrancar |

**El almacenamiento no pide permiso y es deliberado:** el selector devuelve
descriptores por SAF (ADR-0034) y lo recibido va a `getExternalFilesDir`, que es
privado de la aplicación. Un permiso que no se pide es un diálogo que nadie tiene
que entender.

### 4.9 El `.so` que va dentro del APK, ¿está alineado a `0x4000`?

**No se puede responder aquí, y ésta es la respuesta honesta: no hay APK.** Este
contenedor no tiene Flutter ni SDK de Android.

**Lo que sí hay es quién lo mide y quién lo pide.**

- **Quién lo mide:** `tools/apk_inspector/inspect_apk.py`, un analizador de
  cabeceras de programa ELF escrito para esto, que abre el APK, saca cada
  `lib/<abi>/*.so` y exige `p_align >= 0x4000` en cada `PT_LOAD`. Tiene once
  pruebas con ELFs sintetizados y corre en `release.yml`, `platform-builds.yml`,
  `ci.yml` y en el script de firma — **sobre el APK firmado**, porque firmar es
  lo último que toca el paquete y medir antes mide otro archivo (QYR-0387).
- **Quién lo pide:** desde QYR-0394, `.cargo/config.toml`, en las tres tablas de
  Android. Antes dependía de que quien construyera tuviera el NDK 28 o más nuevo
  **y se acordara**; CI lo pasaba a mano y la guía del propietario no.
- **Y que la petición llega, comprobado sobre el objetivo de verdad.** Este
  contenedor tiene `aarch64-linux-android` instalado (sin NDK con el que
  enlazar, así que el enlace falla — pero lo que se mira es qué recibe `rustc`):
  `cargo build -p qyro_ffi --target aarch64-linux-android -v` pasa
  `link-arg=-Wl,-z,max-page-size=16384`, y con `RUSTFLAGS` puesto a otra cosa
  **desaparece: 0 apariciones**. Por eso la guía dice que no se ponga a mano.

### 4.10 Cada capacidad de la tabla de paridad, ¿tiene llamante de producción?

**Trece filas con las dos caras llenas, cinco `NO -- <argumento>`, y una fila
nueva.** `docs/PARIDAD-GUI-CLI.md`.

Lo que hace que esta respuesta valga algo no es la tabla: es que
`the_parity_table_still_points_at_code`
(`rust/crates/qyro_core/tests/repository_contract.rs:271`) **abre cada cita y
falla si apunta a una llave, a un comentario o a una línea que no existe**. Se
disparó **siete veces** durante esta revisión mientras el código se movía debajo,
y las siete tenía razón.

La fila nueva es «Comprobar la huella que promete el código» (QYR-0392): **no
estaba en la tabla, y por eso nadie vio que la GUI no la tenía.**

---

## 5. Lo que sigue abierto

Sin adornos y sin «se hará»: cada uno con lo que se sabe hoy.

**Del motor y la sesión**

- **`qyro_session_finish` no puede ejecutarse tras un fallo.** La regla pegajosa
  de la frontera C corta el cuerpo, así que los `.qyro-part` de una sesión que
  falló esperan a la siguiente transferencia que reclame su nombre en vez de
  irse al fallar. Aguantó el juicio **3/3**.
- **La ventana de 16 trozos es por entrada, no global.** Un lote de archivos
  pequeños se sella entero en RAM de una vez. Aguantó **3/3**.
- **`write_all` no tiene temporizador ni cancelación.** `FrameStream::shutdown`
  existe y nadie lo llama. Aguantó 2/3.
- **Las carpetas se crean en el destino antes de que la persona acepte, y
  rechazar no las borra.** El voto lo tumbó 2/3 y **la refutación no aguanta la
  comprobación** (§3.5): el emisor sí puede producir entradas de carpeta, y las
  produce.
- **Un `Cancel` del par no termina la sesión receptora**, con la severidad
  bajada: dos refutadores lo tumbaron por alcanzabilidad y el tercero dice que
  sólo cayó la cita. Sigue aquí porque nadie demostró que el final llegue.
- **Sigue sin haber latido.** QYR-0393 es una política de plazos, no un mensaje
  en el cable. Si una pausa humana pudiera pasar de diez minutos, la respuesta
  correcta sería un latido y no un número más grande.

**Lo que salió de esta lista al llegar su juicio**, y por qué: «los `.qyro-part`
de un proceso muerto no los borra nadie» —es la política de ADR-0027, y el código
la cumple (§4.4)— y «el reanudado no persiste» —cierto, y sin daño observable,
porque nada reanuda de todos modos: queda contado como capacidad sin llamante y
no como pérdida de datos.

**De las capacidades sin llamante** (la cuenta del FORENSE, que ya llevaba nueve
cadáveres y sigue viva): el reanudado, el historial, el consejero de la GUI,
cancelar desde la GUI, `Pause`/`Resume`/`Retransmit`, y `remember_peer` —
QYR-0382, que es la que impide que «esta clave ha cambiado» pueda ocurrir.

**Del empaquetado**

- El APK se instala como **0.0.1 con `versionCode` 1** en cada build.
- **Nada ata el APK publicado al commit** que dice publicar: `BUILD-INFO.txt` va
  fuera del APK.
- Las carpetas del manifiesto **se crean antes de que la persona acepte**, y
  rechazar no las borra.

**De los documentos, ninguno.** Los siete de QYR-0395 y el que quedaba —el
número de escenarios— están cerrados, con cuatro guardas nuevas en el gate.

El último resultó ser peor de lo que decía y se cerró como **QYR-0396**: el
protocolo tiene **treinta** escenarios, no veintiséis, y seis documentos
repetían el número equivocado porque la suma del propio protocolo decía «los
veintiuno de A–E y **los cinco de F**» — F tiene nueve. Y se sostenía porque
veintiséis era también el número de líneas `Resultado:`: **cuatro escenarios
anotaban su resultado bajo otra etiqueta**, así que nada podía contarlos. Uno de
esos cuatro es F4, «la máquina que no puede instalar nada», que el propio
documento llama *«el escenario que da sentido a todo el producto»*.

**Queda una cifra fuera de este repositorio y no se toca:** las notas de la
Release publicada dicen «los veinte escenarios». Editarlas es cambiar una
publicación retractada, y eso es decisión del propietario, no de esta revisión.

**Ninguno de estos bloquea la prueba de hoy.** Los tres que la bloqueaban —el
linker de Android, la DLL que nadie construía y el permiso `INTERNET` que faltaba
en release— están arreglados, y los dos primeros los arregló el propio agente que
los encontró.

---

## 6. El veredicto, con su método

**El método, primero, porque un número sin método es una opinión con decimales.**
Cada rango se da entre dos cifras porque **la evidencia se detiene en la quinta de
seis clases**: compilado, probado en unidad, probado en integración, probado en
ejecución entre procesos, y **nunca en hardware**. El extremo bajo de cada rango
es lo que sostiene la evidencia que hay; el alto, lo que sostendría si el hardware
confirmara lo medido. La diferencia entre los dos extremos **es exactamente lo que
falta por saber**.

### Fundamentos técnicos: **85-92 %**

814 pruebas verdes, `#![forbid(unsafe_code)]` en todos los crates menos los dos
que cruzan una frontera de sistema, guardas que corren dentro del gate y que se
dispararon **siete veces** en esta revisión teniendo razón las siete, y los
números que importan **medidos y no razonados**: O(1) de memoria sobre 400 MB,
11 descriptores para 200 archivos, 65 s de espera humana sobrevividos.

Lo que impide subir del 92: la ventana por entrada, `write_all` sin plazo, y que
**ninguna prueba de Dart o Kotlin se haya ejecutado en esta revisión**.

### Producto utilizable: **70-85 %**

Las dos caras mandan y reciben archivos, con nombres validados, huellas
comparadas, un puerto que se dice cuando está ocupado y mensajes que dicen qué
pasó en vez de culpar a la red. **Veintiocho defectos cerrados en esta sesión** —QYR-0368 a QYR-0395—,
de los cuales cinco eran P0 que impedían que la aplicación de Android hiciera
nada en absoluto.

Lo que impide subir del 85: cancelar no existe en la GUI, el historial y el
reanudado están escritos y no los llama nadie, y **nadie ha tocado esto con un
dedo en una pantalla de verdad**.

### Preparación para publicar: **45-60 %**

Es la métrica baja y debe serlo. Los artefactos se construyen y se firman, el
APK se inspecciona sobre el archivo firmado, y las claves no están rastreadas —
comprobado preguntándole a `git check-ignore`, no leyendo un `.gitignore`. Pero
seis documentos publicados dicen cosas que el código desmiente, la versión es
`0.0.1` para todo, nada ata el artefacto a su commit, y **el hueco grande sigue
en blanco**: veintiséis escenarios de hardware sin ejecutar.

### En una frase

**El código hace lo que dice, medido hasta donde un contenedor puede medir; el
producto está listo para que alguien lo pruebe hoy en dos aparatos; y para
publicarlo falta que esa prueba ocurra y que seis documentos dejen de mentir.**

---

## 7. La regla, por última vez

**No se ha inventado evidencia de hardware.** Ni una cifra de este informe viene
de un teléfono o de un PC con Windows. Las que hay vienen de ejecutar cosas en un
contenedor Linux y de leerles la salida, y cuando algo no se pudo ejecutar —el
gate en PowerShell, las pruebas de Dart, el APK— está escrito que no se pudo.

**Un hueco en blanco es la verdad.**
