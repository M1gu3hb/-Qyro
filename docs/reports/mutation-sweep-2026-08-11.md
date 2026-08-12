# Barrido de mutación de Sprint 5C — inventario completo

Generado desde los `outcomes.json` originales conservados fuera del repositorio. La tabla contiene un renglón por cada uno de los 939 mutantes potenciales del barrido principal; no convierte automáticamente `MISSED` ni `TIMEOUT` en deuda.

Comandos originales: `cargo-mutants 27.1.0 mutants --no-config -p <crate> -j 4 --baseline skip --timeout 90`. El barrido principal fue Windows; `qyro_fs` se repitió completo en Linux para resolver `cfg`. `NOT_RUN` significa que esa plataforma no se ejecutó, no que el mutante pasara.

## Alcance por crate

| Crate | Potenciales | CAUGHT Windows | MISSED Windows | UNVIABLE Windows | TIMEOUT Windows | CAUGHT Linux | MISSED Linux | UNVIABLE Linux |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| qyro_protocol | 281 | 176 | 54 | 39 | 12 | 0 | 0 | 0 |
| qyro_manifest | 220 | 146 | 44 | 30 | 0 | 0 | 0 | 0 |
| qyro_identity_store | 29 | 23 | 2 | 4 | 0 | 0 | 0 | 0 |
| qyro_fs | 87 | 55 | 24 | 8 | 0 | 59 | 20 | 8 |
| qyro_crypto | 322 | 190 | 33 | 99 | 0 | 0 | 0 | 0 |
| **Total** | **939** | **590** | **157** | **180** | **12** | **59** | **20** | **8** |

## Criterio de clasificación humana

La unidad de clasificación no es el texto del mutante sino su consecuencia
observable. El barrido dio 161 supervivientes al resolver la unión de plataformas
de `qyro_fs`: 25 ya tenían evidencia posterior que los cerraba y quedaban 136
abiertos. Los doce `TIMEOUT` no cuentan como supervivientes y se investigan por
alcanzabilidad en una sección separada.

| Clase humana | Protocol | Manifest | Identity store | FS | Crypto | Total | Destino |
|---|---:|---:|---:|---:|---:|---:|---|
| Ruido o equivalencia | 21 | 4 | 1 | 1 | 22 | 49 | Fuera del ledger |
| Cobertura funcional sin consecuencia de seguridad | 8 | 11 | 0 | 4 | 1 | 24 | QYR-0290, QYR-0292, QYR-0294 y QYR-0296 |
| Entrada de peer, decisión de rechazo o integridad/clave | 25 | 26 | 0 | 11 | 1 | 63 | QYR-0291, QYR-0293, QYR-0295 y QYR-0297 |
| **Abiertos clasificados** | **54** | **41** | **1** | **16** | **33** | **136** | **Ocho familias humanas** |

Se excluye como ruido únicamente un cambio de `Display`/`Debug` o formato sin
decisión de protocolo, un accesor trivial cuyo valor ya está cubierto en el uso,
o un mutante semánticamente equivalente para todos los estados construibles. No
se usa esa etiqueta para aritmética de límites, validadores, decisiones de rechazo,
material de clave ni persistencia. Así, borrar el texto de un error sale del
ledger; cambiar una frontera de longitud permanece, aunque hoy sólo tenga impacto
funcional.

Los 24 huecos funcionales se agrupan por frontera de buffer, límites derivados
del manifiesto, propagación de E/S y disponibilidad del sellador. Los 63 de mayor
consecuencia se agrupan por framing controlado por el peer, validación del
manifiesto, materialización íntegra en disco y frontera antireplay. Esta
clasificación creó ocho fichas de deuda; la reparación del ledger y la conclusión
de los timeouts usan otras dos. En total son diez fichas nuevas, no un volcado de
188 resultados.

## Investigación individual de los doce `TIMEOUT`

Hay dos preguntas distintas. En el código real, `FrameHeader::total_len` es
`48 + payload + trailer`; `parse` exige exactamente 48 bytes de cabecera y acota
los otros dos sumandos antes de construir el valor. Por tanto **no existe** una
cabecera de 48 bytes aceptada cuyo total real sea cero o menor que 48: no hay un
P0 presente en esta revisión. En el binario mutado, en cambio, diez cambios sí
pueden ser disparados por bytes de un peer y los antiguos tests entraban en un
bucle sin presupuesto. Esa carencia de observabilidad se corrigió.

La columna siguiente no atribuye al peer la capacidad de cambiar el binario:
pregunta si el código real admite el estado defectuoso. Para los doce casos la
respuesta es no. Algunos inputs alcanzan la rama afectada en un binario mutado,
pero la condición de falta de progreso exige primero esa regresión interna. Como
defensa P2 adicional, `require_frame_progress` convierte todo total inferior a
la cabecera ya leída en `FrameError::DecoderNoProgress`; sus cinco mutantes
focales terminaron 5 caught en 31 s.

| # | Mutación que agotó 90 s | ¿Peer produce la condición en código real? | Argumento estructural / límite focal | Reejecución |
|---:|---|---|---|---|
| 1 | `decoder.rs:269:9 push -> Ok(())` | No: requiere que `push` deje de insertar | `push` real extiende tras comprobar `checked_add`; los tests tienen número finito de pushes y exigen el cambio de longitud | CAUGHT |
| 2 | `decoder.rs:360:27 += -> *=` | No: requiere cambiar el operador de avance | `read` nace en cero; multiplicarlo no consume. El drenaje admite como máximo el número de frames derivado del buffer | CAUGHT |
| 3 | `decoder.rs:375:23 += -> *=` | No: requiere cambiar el operador de avance | Mismo argumento, en la rama `Encrypted`; el presupuesto de frames convierte la repetición en fallo | CAUGHT |
| 4 | `decoder.rs:388:19 += -> *=` | No: requiere cambiar el operador de avance | Mismo argumento, en la rama `Message`; una llamada extra debe devolver `None` | CAUGHT |
| 5 | `decoder.rs:423:40 + -> *` | No: requiere cambiar la aritmética de reserva | No crea un total de wire inválido; amplifica la reserva. La medida focal compara la capacidad tras dos lecturas iguales contra crecimiento geométrico | CAUGHT en la segunda reejecución |
| 6 | `frame.rs:144:9 encode -> vec![]` | No: es construcción local de salida | Un peer no invoca `Frame::encode`; los workloads derivados ya no pueden iterar según una longitud cero | CAUGHT |
| 7 | `header.rs:293:9 FrameHeader::total_len -> 0` | No: `48 + u32 + u8 >= 48` | El test focal exige total 48 para el frame mínimo y consumo completo en una llamada; la guarda P2 rechaza una regresión futura | CAUGHT |
| 8 | `header.rs:293:27 + -> *` | No: el operador real es suma | En el mutante `48 * 0 + 0 = 0`; en producción el primer operador es suma. Mismo test focal y drenajes acotados | CAUGHT |
| 9 | `header.rs:536:9 ParsedHeader::total_len -> 0` | No: delega a totales de al menos 48 | El enum real sólo delega a los dos totales que incluyen 48; el límite de drenaje ve la repetición | CAUGHT |
| 10 | `header.rs:546:9 UnknownHeader::total_len -> 0` | No: `48 + u32 + u8 >= 48` | El valor real incluye siempre 48; el drenaje de desconocidos está limitado por frames disponibles | CAUGHT |
| 11 | `header.rs:546:27 + -> *` | No: el operador real es suma | El mutante puede producir cero; el operador real no. El mismo presupuesto detecta la falta de progreso | CAUGHT |
| 12 | `limits.rs:49:49 + -> *` | No: un peer no cambia constantes de compilación | El mutante agrandaba los workloads a ~1 GiB. Los tests trabajan con la suma independiente y exigen que `MAX_FRAME_LEN` sea exactamente esa suma | CAUGHT |

La primera reejecución focal usó un regex de líneas, seleccionó 24 mutantes
(los doce originales más doce variantes vecinas) y terminó `22 caught, 1
unviable, 1 timeout` bajo `--timeout 30`; el único timeout restante fue el #5.
Esto se registra como run fallido, no como éxito parcial. Tras añadir la medida
de dos lecturas iguales, la reejecución exacta del #5 terminó `1 caught` en 12 s.
En consecuencia, los doce resultados originales son ahora `CAUGHT`, ninguno
queda como ausencia de veredicto y no se abrió una ficha por mutante.

## Cierre focal de validación, rechazo e integridad

Se reconstruyeron los 63 nombres literales desde el ledger del commit base y se
comprobó con `cargo-mutants --list` que los regex exactos seleccionaban 25 de
protocolo, 26 de manifest, 11 de filesystem y uno de crypto. El primer intento
de protocolo con `--test-workspace true --timeout 45` agotó el límite externo de
10 min: el JSON parcial contenía 2 caught y 15 timeout. Es un run fallido y
demuestra que ejecutar consumidores con loops no acotados no es una prueba focal.

| Familia | Alcance exacto | Resultado tras contratos | Equivalencias demostradas | Estado humano |
|---|---:|---|---|---|
| QYR-0291 framing | 25 | 17 caught | 8 | Resuelta |
| QYR-0293 manifest hostil | 26 | 24 caught | 2 | Resuelta |
| QYR-0295 materialización | 11 | 6 caught | 4 sin consecuencia de aceptación; 1 control Windows pendiente | Abierta y acotada |
| QYR-0297 replay | 1 | 0 caught | 1 | Resuelta |

En protocolo, el primer rerun local terminó 16 caught/9 missed. Un contrato de
tipo desconocido cifrado añadió el trailer al caso observable y su mutante exacto
terminó caught; los ocho restantes no distinguen estados construibles: defensas
duplicadas detrás de headers ya validados, ramas posteriores a una igualdad que
prueba el índice, un rango `zip` que toma exactamente `WIDTH` elementos aunque
su extremo sea mayor, un check de total inalcanzable por las constantes, y
`1 << 0 == 1 >> 0`.

En manifest, los dos missed restantes también son equivalentes. Un
`ManifestItem` de directorio con tamaño/hash de archivo no puede construirse, y
los índices de dos entradas distintas producidos por `enumerate` nunca son
iguales, por lo que `<` y `<=` dan el mismo orden. Los otros 24 murieron con
fronteras explícitas para encoded bytes, total, item count, mínimo por item,
límite de modelo, suma, colisión portable y segmentos.

En filesystem, contratos reales observaron `ContentSink::write_at`, preservación
al abrir sin append, contención dentro/fuera y `AlreadyExists`. En Windows se
creó además una junction NTFS real —reparse point sin privilegio de symlink— y
el mutante exacto `metadata_is_link_or_reparse_point -> false` terminó 1 caught
en 41 s. Cuatro restantes no eluden contención: uno borra un `!` pero
`OpenOptions` ya tiene `truncate(false)` por defecto, y tres sólo cambian la
clasificación/propagación de un error que después vuelve a pasar por
canonicalización o apertura atómica. La guarda del handle de un symlink de
archivo sí importa y queda abierta: el test con `windows-reparse-test` existe,
pero este host no posee `CreateSymbolicLink` (error 1314), así que no se inventa
evidencia de mutación.

Finalmente, el mutante de replay es equivalente: `record` ejecuta `check`
primero; si `sequence == highest`, el bit cero ya existe y se retorna
`ReplayDetected` antes de evaluar `>` o `>=` en el `match`.

## Fase 5 — módulo de confianza y peers conocidos

Alcance declarado: únicamente los dos archivos de producción nuevos de
`qyro_identity_store`, no el crate histórico entero:

```text
cargo-mutants 27.1.0 mutants -p qyro_identity_store \
  --file rust/crates/qyro_identity_store/src/known_peers.rs \
  --file rust/crates/qyro_identity_store/src/known_peer_types.rs \
  --timeout 30 --test-workspace false -j 4
```

| Run | Árbol probado | CAUGHT | MISSED | UNVIABLE | TIMEOUT | Veredicto |
|---|---|---:|---:|---:|---:|---|
| Intento de salida relativa | implementación inicial | 0 | 0 | 0 | 0 | Falló antes del baseline: el padre `work/` no existía; ningún mutante ejecutado |
| Primer barrido completo | siete contratos de aceptación | 73 | 39 | 12 | 0 | RED útil: faltaban fronteras, duplicados, timestamps, entropía y accesores |
| Segundo barrido completo | 17 contratos y constantes de wire explícitas | 95 | 0 | 9 | 0 | PASS; 104 mutantes en 5 min |
| Barrido final completo | mismo código más zeroización del cuerpo claro | 95 | 0 | 9 | 0 | PASS; 104 mutantes en 5 min, baseline 26 s build + 3 s test |

El inventario bajó de 124 a 104 porque las cuentas que definen constantes del
wire (`2 MiB`, 51/52/306 bytes y 1 269 764 bytes) se sustituyeron por sus
valores literales congelados. No se excluyó ningún operador ni función del
barrido final. Los contratos añadidos ejercen ambos lados de cada límite, dos
clases de duplicado, tiempos negativos/invertidos, el dominio de entropía,
accesores, representación humana y display de error. El store con 4096 peers se
construye de verdad y `len()` devuelve el resultado medido; 4097 se rechaza.

El control que importa se ejecutó también fuera de la herramienta: se retiró
temporalmente `known.identity == candidate.identity` y el test exacto
`a_known_peer_whose_key_changed_is_refused_by_name` falló con
`left: KnownAndMatches`, `right: KnownAndChanged`. La comparación se restauró
antes del barrido final. No se creó ninguna ficha de ledger por este barrido.

## Fase 6 — historial local append-only

Alcance declarado: sólo los dos archivos nuevos de historia en `qyro_fs`:

```text
cargo-mutants 27.1.0 mutants -p qyro_fs \
  --file rust/crates/qyro_fs/src/history.rs \
  --file rust/crates/qyro_fs/src/history_types.rs \
  --timeout 30 --test-workspace false -j 4
```

| Run | CAUGHT | MISSED | UNVIABLE | TIMEOUT | Veredicto |
|---|---:|---:|---:|---:|---|
| Primer barrido completo | 73 | 17 | 21 | 0 | RED útil: faltaban bordes de tamaño/tiempo, estados por wire y algunos accesores/diagnósticos |
| Barrido final completo | 80 | 0 | 20 | 0 | PASS; 100 mutantes en 4 min, baseline 24 s build + 1 s test |

Entre ambos runs, el máximo de 16 MiB se volvió una constante literal con una
función de frontera probada en exacto/uno más; timestamps iguales y decrecientes
se ejercen tanto al append como al parse; los tres estados y ambas direcciones
atraviesan el wire; y la creación se redujo a una sola apertura atómica con
`create(true), truncate(false)`, retirando ramas de carrera que no aportaban
semántica al formato. No se excluyó operador ni función del barrido final.

La medida asociada no depende de `cargo-mutants`: 10 000 registros ocupan
exactamente 720 012 bytes y el run Windows con `--nocapture` midió 72.6051 ms
en perfil debug frente a un presupuesto explícito de 500 ms. Un test separado
inyecta 500 ms + 1 ns y exige que el detector falle, y otro prueba que el
contador de trabajo crece de 10 a 20 cuando crece el archivo. No se añadió
ninguna ficha al ledger por estos 100 mutantes.

## Inventario completo

| Crate | Archivo:línea | Mutación literal | Windows | Linux |
|---|---|---|---|---|
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:41` | `replace UnsupportedFrame::from -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:54` | `replace UnsupportedFrame::message_type_value -> u8 with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:60` | `replace UnsupportedFrame::payload_len -> u32 with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:60` | `replace UnsupportedFrame::payload_len -> u32 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:54` | `replace UnsupportedFrame::message_type_value -> u8 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:66` | `replace UnsupportedFrame::total_len -> usize with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:66` | `replace UnsupportedFrame::total_len -> usize with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:72` | `replace UnsupportedFrame::session_id -> u64 with 1` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:72` | `replace UnsupportedFrame::session_id -> u64 with 0` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:78` | `replace UnsupportedFrame::transfer_id -> u64 with 0` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:78` | `replace UnsupportedFrame::transfer_id -> u64 with 1` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:107` | `replace DecodedFrame::message_type -> Option<MessageType> with Some(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:107` | `replace DecodedFrame::message_type -> Option<MessageType> with None` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:84` | `replace UnsupportedFrame::sequence -> u64 with 0` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:84` | `replace UnsupportedFrame::sequence -> u64 with 1` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:121` | `replace DecodedFrame::plaintext -> Option<&[u8]> with None` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:121` | `replace DecodedFrame::plaintext -> Option<&[u8]> with Some(Vec::leak(Vec::new()))` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:121` | `replace DecodedFrame::plaintext -> Option<&[u8]> with Some(Vec::leak(vec![0]))` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:121` | `replace DecodedFrame::plaintext -> Option<&[u8]> with Some(Vec::leak(vec![1]))` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:136` | `replace DecodedFrame::try_encode -> Result<Vec<u8>, FrameError> with Ok(vec![])` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:148` | `replace DecodedFrame::as_plain -> Option<&Frame> with Some(Box::leak(Box::new(Default::default())))` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:136` | `replace DecodedFrame::try_encode -> Result<Vec<u8>, FrameError> with Ok(vec![0])` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:136` | `replace DecodedFrame::try_encode -> Result<Vec<u8>, FrameError> with Ok(vec![1])` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:159` | `replace DecodedFrame::as_encrypted -> Option<&EncryptedEnvelope> with Some(Box::leak(Box::new(Default::default())))` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:218` | `replace FrameDecoder::with_max_buffer_len -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:148` | `replace DecodedFrame::as_plain -> Option<&Frame> with None` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:159` | `replace DecodedFrame::as_encrypted -> Option<&EncryptedEnvelope> with None` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:218` | `replace > with < in FrameDecoder::with_max_buffer_len` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:218` | `replace > with == in FrameDecoder::with_max_buffer_len` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:236` | `replace FrameDecoder::buffered_len -> usize with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:236` | `replace FrameDecoder::buffered_len -> usize with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:218` | `replace > with >= in FrameDecoder::with_max_buffer_len` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:236` | `replace - with + in FrameDecoder::buffered_len` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:236` | `replace - with / in FrameDecoder::buffered_len` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:248` | `replace FrameDecoder::is_poisoned -> bool with true` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:248` | `replace FrameDecoder::is_poisoned -> bool with false` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:242` | `replace FrameDecoder::buffer_capacity -> usize with 1` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:242` | `replace FrameDecoder::buffer_capacity -> usize with 0` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:256` | `replace FrameDecoder::reset with ()` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:280` | `replace > with == in FrameDecoder::push` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:280` | `replace > with < in FrameDecoder::push` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:280` | `replace > with >= in FrameDecoder::push` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:294` | `replace && with \|\| in FrameDecoder::push` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:293` | `replace > with == in FrameDecoder::push` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:293` | `replace > with < in FrameDecoder::push` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:295` | `replace \|\| with && in FrameDecoder::push` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:293` | `replace > with >= in FrameDecoder::push` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:294` | `replace > with < in FrameDecoder::push` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:294` | `replace > with == in FrameDecoder::push` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:294` | `replace + with * in FrameDecoder::push` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:294` | `replace > with >= in FrameDecoder::push` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:294` | `replace + with - in FrameDecoder::push` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:295` | `replace >= with < in FrameDecoder::push` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:295` | `replace / with % in FrameDecoder::push` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:295` | `replace / with * in FrameDecoder::push` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:317` | `replace FrameDecoder::next_frame -> Result<Option<DecodedFrame>, FrameError> with Ok(Some(Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:317` | `replace FrameDecoder::next_frame -> Result<Option<DecodedFrame>, FrameError> with Ok(None)` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:320` | `replace < with == in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:320` | `replace < with > in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:320` | `replace < with <= in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:327` | `replace + with - in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:327` | `replace + with * in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:345` | `replace > with == in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:345` | `replace > with < in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:352` | `replace < with == in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:352` | `replace < with > in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:345` | `replace > with >= in FrameDecoder::next_frame` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:269` | `replace FrameDecoder::push -> Result<(), FrameError> with Ok(())` | TIMEOUT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:352` | `replace < with <= in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:360` | `replace += with -= in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:372` | `replace + with - in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:372` | `replace + with - in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:372` | `replace + with * in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:372` | `replace + with * in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:375` | `replace += with -= in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:382` | `replace + with - in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:382` | `replace + with * in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:382` | `replace + with - in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:382` | `replace + with * in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:385` | `replace + with - in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:385` | `replace + with * in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:388` | `replace += with -= in FrameDecoder::next_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:401` | `replace FrameDecoder::compact with ()` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:401` | `replace == with != in FrameDecoder::compact` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:406` | `replace += with -= in FrameDecoder::compact` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:406` | `replace += with *= in FrameDecoder::compact` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:406` | `replace - with + in FrameDecoder::compact` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:360` | `replace += with *= in FrameDecoder::next_frame` | TIMEOUT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:423` | `replace FrameDecoder::reserve_for with ()` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:406` | `replace - with / in FrameDecoder::compact` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:423` | `replace + with - in FrameDecoder::reserve_for` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:375` | `replace += with *= in FrameDecoder::next_frame` | TIMEOUT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:429` | `replace - with + in FrameDecoder::reserve_for` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:424` | `replace <= with > in FrameDecoder::reserve_for` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:429` | `replace - with / in FrameDecoder::reserve_for` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:439` | `replace FrameDecoder::poison -> FrameError with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:77` | `replace EncryptedEnvelope::from_plain_frame -> Result<Self, FrameError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:388` | `replace += with *= in FrameDecoder::next_frame` | TIMEOUT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:78` | `replace > with < in EncryptedEnvelope::from_plain_frame` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:112` | `replace EncryptedEnvelope::header -> &FrameHeader with Box::leak(Box::new(Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:118` | `replace EncryptedEnvelope::message_type -> MessageType with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:78` | `replace > with == in EncryptedEnvelope::from_plain_frame` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:78` | `replace > with >= in EncryptedEnvelope::from_plain_frame` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:132` | `replace EncryptedEnvelope::associated_data -> [u8; HEADER_LEN] with [0; HEADER_LEN]` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:132` | `replace EncryptedEnvelope::associated_data -> [u8; HEADER_LEN] with [1; HEADER_LEN]` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:138` | `replace EncryptedEnvelope::ciphertext -> &[u8] with Vec::leak(Vec::new())` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:138` | `replace EncryptedEnvelope::ciphertext -> &[u8] with Vec::leak(vec![0])` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:138` | `replace EncryptedEnvelope::ciphertext -> &[u8] with Vec::leak(vec![1])` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:144` | `replace EncryptedEnvelope::tag -> &[u8] with Vec::leak(Vec::new())` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/decoder.rs:423` | `replace + with * in FrameDecoder::reserve_for` | TIMEOUT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:144` | `replace EncryptedEnvelope::tag -> &[u8] with Vec::leak(vec![0])` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:144` | `replace EncryptedEnvelope::tag -> &[u8] with Vec::leak(vec![1])` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:150` | `replace EncryptedEnvelope::encode -> Vec<u8> with vec![]` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:166` | `replace EncryptedEnvelope::from_parts -> Result<Self, FrameError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:150` | `replace EncryptedEnvelope::encode -> Vec<u8> with vec![0]` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:150` | `replace EncryptedEnvelope::encode -> Vec<u8> with vec![1]` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:166` | `delete ! in EncryptedEnvelope::from_parts` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:166` | `replace \|\| with && in EncryptedEnvelope::from_parts` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:166` | `replace == with != in EncryptedEnvelope::from_parts` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:173` | `replace != with == in EncryptedEnvelope::from_parts` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:173` | `replace + with - in EncryptedEnvelope::from_parts` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:173` | `replace + with * in EncryptedEnvelope::from_parts` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:176` | `replace + with - in EncryptedEnvelope::from_parts` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:176` | `replace + with * in EncryptedEnvelope::from_parts` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:185` | `replace + with - in EncryptedEnvelope::from_parts` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:64` | `replace Frame::from_parts -> Result<Self, FrameError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/envelope.rs:185` | `replace + with * in EncryptedEnvelope::from_parts` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:64` | `replace != with == in Frame::from_parts` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/error.rs:149` | `replace <impl fmt::Display for FrameError>::fmt -> fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:64` | `replace != with == in Frame::from_parts` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:82` | `replace Frame::header -> &FrameHeader with Box::leak(Box::new(Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:64` | `replace \|\| with && in Frame::from_parts` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:70` | `replace != with == in Frame::from_parts` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:88` | `replace Frame::payload -> &[u8] with Vec::leak(Vec::new())` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:88` | `replace Frame::payload -> &[u8] with Vec::leak(vec![0])` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:88` | `replace Frame::payload -> &[u8] with Vec::leak(vec![1])` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:100` | `replace Frame::message_type -> MessageType with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:116` | `replace Frame::with_identifiers -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:125` | `replace Frame::with_sequence -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:137` | `replace Frame::with_flags -> Result<Self, FrameError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:94` | `replace Frame::into_payload -> Vec<u8> with vec![]` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:94` | `replace Frame::into_payload -> Vec<u8> with vec![0]` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:94` | `replace Frame::into_payload -> Vec<u8> with vec![1]` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:144` | `replace Frame::encode -> Vec<u8> with vec![0]` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:144` | `replace Frame::encode -> Vec<u8> with vec![1]` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:153` | `replace Frame::encode_into with ()` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:55` | `replace field -> [u8; WIDTH] with [0; WIDTH]` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:55` | `replace field -> [u8; WIDTH] with [1; WIDTH]` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:97` | `replace FrameHeader::within_limits -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:118` | `replace FrameHeader::clone_for_envelope -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:62` | `replace + with - in field` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:62` | `replace + with * in field` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:141` | `replace FrameHeader::message_type -> MessageType with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:147` | `replace FrameHeader::flags -> Flags with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:135` | `replace FrameHeader::version_minor -> u8 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:135` | `replace FrameHeader::version_minor -> u8 with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:156` | `replace FrameHeader::header_len -> u16 with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:156` | `replace FrameHeader::header_len -> u16 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:162` | `replace FrameHeader::trailer_len -> u8 with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:162` | `replace FrameHeader::trailer_len -> u8 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:178` | `replace FrameHeader::session_id -> SessionId with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:168` | `replace FrameHeader::payload_len -> u32 with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:168` | `replace FrameHeader::payload_len -> u32 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:184` | `replace FrameHeader::transfer_id -> u64 with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:184` | `replace FrameHeader::transfer_id -> u64 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:190` | `replace FrameHeader::stream_id -> u32 with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/frame.rs:144` | `replace Frame::encode -> Vec<u8> with vec![]` | TIMEOUT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:190` | `replace FrameHeader::stream_id -> u32 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:196` | `replace FrameHeader::item_id -> u32 with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:215` | `replace FrameHeader::with_header_len -> Result<Self, FrameError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:196` | `replace FrameHeader::item_id -> u32 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:237` | `replace FrameHeader::with_identifiers -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:202` | `replace FrameHeader::sequence -> u64 with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:259` | `replace FrameHeader::with_transport_flags -> Result<Self, FrameError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:247` | `replace FrameHeader::with_sequence -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:202` | `replace FrameHeader::sequence -> u64 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:215` | `replace == with != in FrameHeader::with_header_len` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:276` | `replace FrameHeader::encrypted -> Result<Self, FrameError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:259` | `delete ! in FrameHeader::with_transport_flags` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:276` | `replace > with < in FrameHeader::encrypted` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:276` | `replace == with != in FrameHeader::encrypted` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:276` | `replace > with == in FrameHeader::encrypted` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:276` | `replace \|\| with && in FrameHeader::encrypted` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:276` | `replace > with >= in FrameHeader::encrypted` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:293` | `replace FrameHeader::total_len -> u64 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:293` | `replace + with - in FrameHeader::total_len` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:293` | `replace + with * in FrameHeader::total_len` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:293` | `replace + with - in FrameHeader::total_len` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:299` | `replace FrameHeader::encode -> [u8; HEADER_LEN] with [0; HEADER_LEN]` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:333` | `replace FrameHeader::decode -> Result<Self, FrameError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:299` | `replace FrameHeader::encode -> [u8; HEADER_LEN] with [1; HEADER_LEN]` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:351` | `replace FrameHeader::parse -> Result<ParsedHeader, FrameError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:363` | `replace != with == in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:368` | `replace != with == in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:381` | `replace < with == in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:381` | `replace \|\| with && in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:381` | `replace < with > in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:381` | `replace < with <= in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:381` | `replace > with < in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:293` | `replace FrameHeader::total_len -> u64 with 0` | TIMEOUT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:381` | `replace > with == in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:388` | `replace != with == in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:400` | `replace != with == in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:381` | `replace > with >= in FrameHeader::parse` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:293` | `replace + with * in FrameHeader::total_len` | TIMEOUT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:412` | `replace == with != in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:412` | `replace \|\| with && in FrameHeader::parse` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:412` | `replace > with == in FrameHeader::parse` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:412` | `replace > with < in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:417` | `replace != with == in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:425` | `replace != with == in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:412` | `replace > with >= in FrameHeader::parse` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:434` | `replace > with == in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:434` | `replace > with < in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:434` | `replace > with >= in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:531` | `replace ParsedHeader::parse_from -> Result<Self, FrameError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:492` | `replace > with < in FrameHeader::parse` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:492` | `replace > with == in FrameHeader::parse` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:536` | `replace ParsedHeader::total_len -> u64 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:492` | `replace > with >= in FrameHeader::parse` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:546` | `replace UnknownHeader::total_len -> u64 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:546` | `replace + with - in UnknownHeader::total_len` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:546` | `replace + with * in UnknownHeader::total_len` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:546` | `replace + with - in UnknownHeader::total_len` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/limits.rs:27` | `replace * with +` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/limits.rs:27` | `replace * with /` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/limits.rs:49` | `replace + with -` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:536` | `replace ParsedHeader::total_len -> u64 with 0` | TIMEOUT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/limits.rs:49` | `replace + with -` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:546` | `replace UnknownHeader::total_len -> u64 with 0` | TIMEOUT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/limits.rs:49` | `replace + with *` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:76` | `replace MessageType::to_wire -> u8 with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:86` | `replace MessageType::from_wire -> Result<Self, FrameError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:76` | `replace MessageType::to_wire -> u8 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/header.rs:546` | `replace + with * in UnknownHeader::total_len` | TIMEOUT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:122` | `replace << with >>` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:126` | `replace << with >>` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:124` | `replace << with >>` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:128` | `replace << with >>` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:133` | `delete !` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:146` | `replace Flags::protected_bits -> u8 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:146` | `replace Flags::protected_bits -> u8 with 0` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:146` | `replace & with \| in Flags::protected_bits` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:146` | `replace & with ^ in Flags::protected_bits` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:152` | `replace Flags::unimplemented_bits -> u8 with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:152` | `replace Flags::unimplemented_bits -> u8 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:152` | `replace & with \| in Flags::unimplemented_bits` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/limits.rs:49` | `replace + with *` | TIMEOUT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:152` | `replace & with ^ in Flags::unimplemented_bits` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:158` | `replace Flags::is_transport_only -> bool with true` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:158` | `replace Flags::is_transport_only -> bool with false` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:158` | `replace == with != in Flags::is_transport_only` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:158` | `replace & with \| in Flags::is_transport_only` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:158` | `replace & with ^ in Flags::is_transport_only` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:158` | `delete ! in Flags::is_transport_only` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:173` | `replace Flags::from_bits -> Result<Self, FrameError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:164` | `replace Flags::bits -> u8 with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:164` | `replace Flags::bits -> u8 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:173` | `replace != with == in Flags::from_bits` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:173` | `replace & with \| in Flags::from_bits` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:173` | `replace & with ^ in Flags::from_bits` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:185` | `replace Flags::contains -> bool with true` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:185` | `replace Flags::contains -> bool with false` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:185` | `replace == with != in Flags::contains` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:191` | `replace Flags::union -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:185` | `replace & with \| in Flags::contains` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/session.rs:36` | `replace SessionId::from_be_bytes -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:185` | `replace & with ^ in Flags::contains` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:191` | `replace \| with & in Flags::union` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/session.rs:48` | `replace SessionId::as_bytes -> &[u8; SESSION_ID_LEN] with Box::leak(Box::new([0; SESSION_ID_LEN]))` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/session.rs:48` | `replace SessionId::as_bytes -> &[u8; SESSION_ID_LEN] with Box::leak(Box::new([1; SESSION_ID_LEN]))` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/session.rs:54` | `replace SessionId::from_u64 -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/session.rs:42` | `replace SessionId::to_be_bytes -> [u8; SESSION_ID_LEN] with [0; SESSION_ID_LEN]` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/session.rs:42` | `replace SessionId::to_be_bytes -> [u8; SESSION_ID_LEN] with [1; SESSION_ID_LEN]` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/message.rs:191` | `replace \| with ^ in Flags::union` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/session.rs:63` | `replace SessionId::to_u64 -> u64 with 0` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/session.rs:69` | `replace <impl fmt::Debug for SessionId>::fmt -> fmt::Result with Ok(Default::default())` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/session.rs:63` | `replace SessionId::to_u64 -> u64 with 1` | CAUGHT | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/session.rs:79` | `replace <impl fmt::Display for SessionId>::fmt -> fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/session.rs:88` | `replace <impl From<u64> for SessionId>::from -> Self with Default::default()` | MISSED | NOT_RUN |
| qyro_protocol | `rust/crates/qyro_protocol/src/session.rs:94` | `replace <impl From<[u8; SESSION_ID_LEN]> for SessionId>::from -> Self with Default::default()` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:31` | `replace encoded_len -> Result<usize, ManifestError> with Ok(0)` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:31` | `replace + with * in encoded_len` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:31` | `replace + with - in encoded_len` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:31` | `replace encoded_len -> Result<usize, ManifestError> with Ok(1)` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:31` | `replace + with - in encoded_len` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:31` | `replace + with - in encoded_len` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:31` | `replace + with * in encoded_len` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:31` | `replace + with - in encoded_len` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:31` | `replace + with * in encoded_len` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:48` | `replace + with - in encoded_len` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:31` | `replace + with * in encoded_len` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:58` | `replace + with - in encoded_len` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:31` | `replace + with - in encoded_len` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:31` | `replace + with * in encoded_len` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:48` | `replace + with * in encoded_len` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:58` | `replace + with * in encoded_len` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:83` | `replace > with < in encoded_len` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:83` | `replace > with == in encoded_len` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:104` | `replace encode -> Result<Vec<u8>, ManifestError> with Ok(vec![])` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:104` | `replace encode -> Result<Vec<u8>, ManifestError> with Ok(vec![0])` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:83` | `replace > with >= in encoded_len` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:104` | `replace encode -> Result<Vec<u8>, ManifestError> with Ok(vec![1])` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:176` | `replace decode -> Result<TransferManifest, ManifestError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:133` | `replace encode_item with ()` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:157` | `replace encode_optional_string with ()` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:151` | `replace encode_string with ()` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:176` | `replace > with < in decode` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:185` | `replace != with == in decode` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:176` | `replace > with == in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:190` | `replace != with == in decode` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:176` | `replace > with >= in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:200` | `replace > with < in decode` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:200` | `replace > with == in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:210` | `replace > with == in decode` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:210` | `replace > with < in decode` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:200` | `replace > with >= in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:210` | `replace > with >= in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:219` | `replace + with - in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:219` | `replace + with * in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:219` | `replace + with - in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:219` | `replace + with * in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:219` | `replace + with - in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:219` | `replace + with * in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:219` | `replace + with - in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:219` | `replace + with * in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:219` | `replace + with * in decode` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:219` | `replace + with - in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:219` | `replace + with - in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:219` | `replace + with * in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:219` | `replace + with - in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:221` | `replace > with == in decode` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:219` | `replace + with * in decode` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:221` | `replace > with < in decode` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:250` | `replace decode_item -> Result<ManifestItem, ManifestError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:233` | `delete ! in decode` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:221` | `replace > with >= in decode` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:239` | `replace != with == in decode` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:303` | `replace Reader<'a>::remaining -> usize with 0` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:303` | `replace Reader<'a>::remaining -> usize with 1` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:303` | `replace - with + in Reader<'a>::remaining` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:303` | `replace - with / in Reader<'a>::remaining` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:307` | `replace Reader<'a>::is_empty -> bool with true` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:307` | `replace Reader<'a>::is_empty -> bool with false` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:307` | `replace == with != in Reader<'a>::is_empty` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:311` | `replace Reader<'a>::take_exact -> Result<&'a[u8], ManifestError> with Ok(Vec::leak(Vec::new()))` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:311` | `replace Reader<'a>::take_exact -> Result<&'a[u8], ManifestError> with Ok(Vec::leak(vec![0]))` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:311` | `replace Reader<'a>::take_exact -> Result<&'a[u8], ManifestError> with Ok(Vec::leak(vec![1]))` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:311` | `replace < with == in Reader<'a>::take_exact` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:311` | `replace < with > in Reader<'a>::take_exact` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:311` | `replace < with <= in Reader<'a>::take_exact` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:341` | `replace Reader<'a>::take_array -> Result<[u8; N], ManifestError> with Ok([0; N])` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:341` | `replace Reader<'a>::take_array -> Result<[u8; N], ManifestError> with Ok([1; N])` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:348` | `replace Reader<'a>::take_u8 -> Result<u8, ManifestError> with Ok(0)` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:348` | `replace Reader<'a>::take_u8 -> Result<u8, ManifestError> with Ok(1)` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:349` | `delete match arm [byte] in Reader<'a>::take_u8` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:362` | `replace Reader<'a>::take_option_tag -> Result<bool, ManifestError> with Ok(true)` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:362` | `replace Reader<'a>::take_option_tag -> Result<bool, ManifestError> with Ok(false)` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:379` | `replace Reader<'a>::take_length_prefixed -> Result<&'a[u8], ManifestError> with Ok(Vec::leak(Vec::new()))` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:379` | `replace Reader<'a>::take_length_prefixed -> Result<&'a[u8], ManifestError> with Ok(Vec::leak(vec![0]))` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:379` | `replace Reader<'a>::take_length_prefixed -> Result<&'a[u8], ManifestError> with Ok(Vec::leak(vec![1]))` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:380` | `replace > with == in Reader<'a>::take_length_prefixed` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:380` | `replace > with < in Reader<'a>::take_length_prefixed` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:391` | `replace Reader<'a>::take_string -> Result<String, ManifestError> with Ok(String::new())` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:391` | `replace Reader<'a>::take_string -> Result<String, ManifestError> with Ok("xyzzy".into())` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/codec.rs:380` | `replace > with >= in Reader<'a>::take_length_prefixed` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/error.rs:396` | `replace <impl From<PathError> for ManifestError>::from -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/error.rs:80` | `replace <impl fmt::Display for PathError>::fmt -> fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/error.rs:295` | `replace <impl fmt::Display for ManifestField>::fmt -> fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/error.rs:310` | `replace <impl fmt::Display for ManifestError>::fmt -> fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/limits.rs:10` | `replace * with +` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/limits.rs:10` | `replace * with /` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/limits.rs:10` | `replace * with +` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/limits.rs:10` | `replace * with /` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/limits.rs:10` | `replace * with +` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/limits.rs:31` | `replace * with /` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/limits.rs:10` | `replace * with /` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/limits.rs:31` | `replace * with +` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:21` | `replace ItemKind::to_wire -> u8 with 0` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/limits.rs:31` | `replace * with /` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:30` | `replace ItemKind::from_wire -> Result<Self, ManifestError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/limits.rs:31` | `replace * with +` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:21` | `replace ItemKind::to_wire -> u8 with 1` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:61` | `replace HashAlgorithm::digest_len -> usize with 0` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:79` | `replace HashAlgorithm::from_wire -> Result<Self, ManifestError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:70` | `replace HashAlgorithm::to_wire -> u8 with 0` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:61` | `replace HashAlgorithm::digest_len -> usize with 1` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:114` | `replace Compression::from_wire -> Result<Self, ManifestError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:70` | `replace HashAlgorithm::to_wire -> u8 with 1` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:135` | `replace HashMetadata::none -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:167` | `replace HashMetadata::algorithm -> HashAlgorithm with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:105` | `replace Compression::to_wire -> u8 with 1` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:105` | `replace Compression::to_wire -> u8 with 0` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:173` | `replace HashMetadata::digest -> &[u8] with Vec::leak(Vec::new())` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:173` | `replace HashMetadata::digest -> &[u8] with Vec::leak(vec![0])` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:173` | `replace HashMetadata::digest -> &[u8] with Vec::leak(vec![1])` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:210` | `replace ManifestItem::file -> Result<Self, ManifestError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:228` | `replace ManifestItem::directory -> Result<Self, ManifestError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:179` | `replace HashMetadata::is_present -> bool with true` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:179` | `replace != with == in HashMetadata::is_present` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:179` | `replace HashMetadata::is_present -> bool with false` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:303` | `replace ManifestItem::path -> &RelativePath with Box::leak(Box::new(Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:297` | `replace ManifestItem::item_id -> u32 with 0` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:297` | `replace ManifestItem::item_id -> u32 with 1` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:319` | `replace ManifestItem::kind -> ItemKind with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:313` | `replace ManifestItem::display_name -> &str with ""` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:313` | `replace ManifestItem::display_name -> &str with "xyzzy"` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:325` | `replace ManifestItem::size -> u64 with 0` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:325` | `replace ManifestItem::size -> u64 with 1` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:331` | `replace ManifestItem::mime_type -> Option<&str> with None` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:331` | `replace ManifestItem::mime_type -> Option<&str> with Some("xyzzy")` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:331` | `replace ManifestItem::mime_type -> Option<&str> with Some("")` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:337` | `replace ManifestItem::modified_unix_seconds -> Option<i64> with None` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:343` | `replace ManifestItem::hash -> &HashMetadata with Box::leak(Box::new(Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:337` | `replace ManifestItem::modified_unix_seconds -> Option<i64> with Some(0)` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:349` | `replace ManifestItem::compression -> Compression with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:358` | `replace ManifestItem::with_mime_type -> Result<Self, ManifestError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:337` | `replace ManifestItem::modified_unix_seconds -> Option<i64> with Some(1)` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:337` | `replace ManifestItem::modified_unix_seconds -> Option<i64> with Some(-1)` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:372` | `replace ManifestItem::with_modified_unix_seconds -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:433` | `replace TransferManifest::from_sorted -> Result<Self, ManifestError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:358` | `replace > with == in ManifestItem::with_mime_type` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:358` | `replace > with < in ManifestItem::with_mime_type` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:433` | `replace > with < in TransferManifest::from_sorted` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:358` | `replace > with >= in ManifestItem::with_mime_type` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:433` | `replace > with == in TransferManifest::from_sorted` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:440` | `replace != with == in TransferManifest::from_sorted` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:433` | `replace > with >= in TransferManifest::from_sorted` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:457` | `replace TransferManifest::transfer_id -> u64 with 0` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:457` | `replace TransferManifest::transfer_id -> u64 with 1` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:463` | `replace TransferManifest::created_unix_seconds -> i64 with -1` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:463` | `replace TransferManifest::created_unix_seconds -> i64 with 0` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:463` | `replace TransferManifest::created_unix_seconds -> i64 with 1` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:469` | `replace TransferManifest::item_count -> usize with 0` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:469` | `replace TransferManifest::item_count -> usize with 1` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:475` | `replace TransferManifest::total_bytes -> u64 with 0` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:481` | `replace TransferManifest::items -> &[ManifestItem] with Vec::leak(vec![Default::default()])` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:475` | `replace TransferManifest::total_bytes -> u64 with 1` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:481` | `replace TransferManifest::items -> &[ManifestItem] with Vec::leak(Vec::new())` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:507` | `replace && with \|\| in validate_items` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:490` | `replace validate_items -> Result<u64, ManifestError> with Ok(1)` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:490` | `replace validate_items -> Result<u64, ManifestError> with Ok(0)` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:507` | `replace == with != in validate_items` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:507` | `replace != with == in validate_items` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:507` | `replace \|\| with && in validate_items` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:516` | `replace > with < in validate_items` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:516` | `replace > with == in validate_items` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:536` | `replace == with != in validate_items` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:516` | `replace > with >= in validate_items` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:567` | `replace && with \|\| in validate_items` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:566` | `replace && with \|\| in validate_items` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:537` | `replace < with == in validate_items` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:537` | `replace < with > in validate_items` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:110` | `replace RelativePath::parse -> Result<Self, PathError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:537` | `replace < with <= in validate_items` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:569` | `replace == with != in validate_items` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/model.rs:587` | `replace == with != in validate_items` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:113` | `replace > with == in RelativePath::parse` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:113` | `replace > with < in RelativePath::parse` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:157` | `replace > with == in RelativePath::parse` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:157` | `replace > with < in RelativePath::parse` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:181` | `replace RelativePath::parse_bytes -> Result<Self, PathError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:113` | `replace > with >= in RelativePath::parse` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:198` | `replace RelativePath::segments -> core::str::Split<'_, char> with Split::new()` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:192` | `replace RelativePath::as_str -> &str with ""` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:198` | `replace RelativePath::segments -> core::str::Split<'_, char> with Split::from_iter([Default::default()])` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:198` | `replace RelativePath::segments -> core::str::Split<'_, char> with Split::new(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:198` | `replace RelativePath::segments -> core::str::Split<'_, char> with Split::from(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:157` | `replace > with >= in RelativePath::parse` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:192` | `replace RelativePath::as_str -> &str with "xyzzy"` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:204` | `replace RelativePath::segment_count -> usize with 0` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:204` | `replace RelativePath::segment_count -> usize with 1` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:210` | `replace RelativePath::file_name -> &str with ""` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:210` | `replace RelativePath::file_name -> &str with "xyzzy"` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:219` | `replace RelativePath::byte_len -> usize with 0` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:219` | `replace RelativePath::byte_len -> usize with 1` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:233` | `replace == with != in validate_segment` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:230` | `replace validate_segment -> Result<(), PathError> with Ok(())` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:225` | `replace <impl core::fmt::Display for RelativePath>::fmt -> core::fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:236` | `replace == with != in validate_segment` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:246` | `replace > with == in validate_segment` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:246` | `replace > with < in validate_segment` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:246` | `replace > with >= in validate_segment` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:254` | `replace \|\| with && in validate_segment` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:331` | `replace is_unicode_format -> bool with true` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:331` | `replace is_unicode_format -> bool with false` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:333` | `replace < with == in is_unicode_format` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:333` | `replace < with > in is_unicode_format` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:333` | `replace < with <= in is_unicode_format` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:335` | `replace > with == in is_unicode_format` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:335` | `replace > with < in is_unicode_format` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:335` | `replace > with >= in is_unicode_format` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:345` | `replace is_windows_reserved -> bool with true` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:397` | `replace PortableCollisionKey::of -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:354` | `replace has_drive_prefix -> bool with true` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:345` | `replace is_windows_reserved -> bool with false` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:354` | `replace has_drive_prefix -> bool with false` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:404` | `replace PortableCollisionKey::as_str -> &str with ""` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:404` | `replace PortableCollisionKey::as_str -> &str with "xyzzy"` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:410` | `replace fold_segment -> String with String::new()` | CAUGHT | NOT_RUN |
| qyro_manifest | `rust/crates/qyro_manifest/src/path.rs:410` | `replace fold_segment -> String with "xyzzy".into()` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/lib.rs:125` | `replace entropy_for -> Vec<u8> with vec![]` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/lib.rs:143` | `replace seal_identity -> Result<Vec<u8>, StoreError> with Ok(vec![0])` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/lib.rs:143` | `replace seal_identity -> Result<Vec<u8>, StoreError> with Ok(vec![1])` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/lib.rs:162` | `replace open_identity -> Result<DeviceIdentity, StoreError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/lib.rs:171` | `replace != with == in open_identity` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:74` | `replace BlobHeader::entropy_prefix -> [u8; ENTROPY_HEADER_LEN] with [0; ENTROPY_HEADER_LEN]` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:74` | `replace BlobHeader::entropy_prefix -> [u8; ENTROPY_HEADER_LEN] with [1; ENTROPY_HEADER_LEN]` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/lib.rs:125` | `replace entropy_for -> Vec<u8> with vec![1]` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/lib.rs:125` | `replace entropy_for -> Vec<u8> with vec![0]` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:89` | `replace BlobHeader::encode -> [u8; HEADER_LEN] with [0; HEADER_LEN]` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/lib.rs:143` | `replace seal_identity -> Result<Vec<u8>, StoreError> with Ok(vec![])` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:106` | `replace parse -> Result<(BlobHeader, &[u8]), StoreError> with Ok((Default::default(), Vec::leak(Vec::new())))` | UNVIABLE | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:106` | `replace parse -> Result<(BlobHeader, &[u8]), StoreError> with Ok((Default::default(), Vec::leak(vec![0])))` | UNVIABLE | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:106` | `replace parse -> Result<(BlobHeader, &[u8]), StoreError> with Ok((Default::default(), Vec::leak(vec![1])))` | UNVIABLE | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:89` | `replace BlobHeader::encode -> [u8; HEADER_LEN] with [1; HEADER_LEN]` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:106` | `replace < with == in parse` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:106` | `replace < with > in parse` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:106` | `replace < with <= in parse` | MISSED | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:115` | `replace != with == in parse` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:123` | `replace != with == in parse` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:131` | `delete ! in parse` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:140` | `replace != with == in parse` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:153` | `replace == with != in parse` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:179` | `replace encode -> Result<Vec<u8>, StoreError> with Ok(vec![])` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:179` | `replace encode -> Result<Vec<u8>, StoreError> with Ok(vec![0])` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/blob.rs:179` | `replace encode -> Result<Vec<u8>, StoreError> with Ok(vec![1])` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/error.rs:82` | `replace <impl fmt::Display for StoreError>::fmt -> fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/error.rs:130` | `replace StoreError::is_absent -> bool with true` | CAUGHT | NOT_RUN |
| qyro_identity_store | `rust/crates/qyro_identity_store/src/error.rs:130` | `replace StoreError::is_absent -> bool with false` | CAUGHT | NOT_RUN |
| qyro_fs | `rust/crates/qyro_fs/src/error.rs:87` | `replace <impl From<std::io::Error> for FsError>::from -> Self with Default::default()` | UNVIABLE | UNVIABLE |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:45` | `replace open_part -> Result<File, FsError> with Ok(Default::default())` | UNVIABLE | UNVIABLE |
| qyro_fs | `rust/crates/qyro_fs/src/error.rs:55` | `replace <impl fmt::Display for FsError>::fmt -> fmt::Result with Ok(Default::default())` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:47` | `delete ! in open_part` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:67` | `replace match guard metadata_is_link_or_reparse_point(&file.metadata()?) with true in open_part` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/error.rs:88` | `delete - in <impl From<std::io::Error> for FsError>::from` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:67` | `replace match guard metadata_is_link_or_reparse_point(&file.metadata()?) with false in open_part` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:75` | `replace match guard metadata_is_link_or_reparse_point(&metadata) with true in open_part` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:100` | `replace final_component_link -> FsError with Default::default()` | UNVIABLE | UNVIABLE |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:90` | `delete ! in open_part` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:75` | `replace match guard metadata_is_link_or_reparse_point(&metadata) with false in open_part` | MISSED | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:107` | `replace metadata_is_link_or_reparse_point -> bool with true` | MISSED | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:107` | `replace metadata_is_link_or_reparse_point -> bool with false` | MISSED | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:112` | `replace metadata_is_link_or_reparse_point -> bool with true` | CAUGHT | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:112` | `replace metadata_is_link_or_reparse_point -> bool with false` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:117` | `replace != with == in metadata_is_link_or_reparse_point` | CAUGHT | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:117` | `replace & with \| in metadata_is_link_or_reparse_point` | CAUGHT | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:117` | `replace & with ^ in metadata_is_link_or_reparse_point` | CAUGHT | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:129` | `replace libc_o_nofollow -> i32 with 0` | MISSED | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:129` | `replace libc_o_nofollow -> i32 with -1` | MISSED | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:129` | `replace libc_o_nofollow -> i32 with 1` | MISSED | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:181` | `replace FileSource::try_read -> Option<usize> with None` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:181` | `replace FileSource::try_read -> Option<usize> with Some(0)` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:190` | `replace < with == in FileSource::try_read` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:190` | `replace < with > in FileSource::try_read` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:190` | `replace < with <= in FileSource::try_read` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:204` | `replace <impl ContentSource for FileSource>::read_at -> usize with 0` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:266` | `replace FileSink::resume_path -> PathBuf with Default::default()` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:272` | `replace FileSink::committed_progress -> Result<Option<u64>, FsError> with Ok(None)` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:272` | `replace FileSink::committed_progress -> Result<Option<u64>, FsError> with Ok(Some(0))` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:272` | `replace FileSink::committed_progress -> Result<Option<u64>, FsError> with Ok(Some(1))` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:274` | `replace match guard error.kind() == std::io::ErrorKind::NotFound with true in FileSink::committed_progress` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:274` | `replace match guard error.kind() == std::io::ErrorKind::NotFound with false in FileSink::committed_progress` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:274` | `replace == with != in FileSink::committed_progress` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:286` | `replace FileSink::part_for -> Result<&mut PartFile, FsError> with Ok(Box::leak(Box::new(Default::default())))` | UNVIABLE | UNVIABLE |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:278` | `replace != with == in FileSink::committed_progress` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:286` | `delete ! in FileSink::part_for` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:305` | `replace match guard error.kind() == std::io::ErrorKind::NotFound with true in FileSink::part_for` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:305` | `replace match guard error.kind() == std::io::ErrorKind::NotFound with false in FileSink::part_for` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:305` | `replace == with != in FileSink::part_for` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:365` | `replace FileSink::progress -> ResumeState with Default::default()` | UNVIABLE | UNVIABLE |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:348` | `replace FileSink::put -> Result<(), FsError> with Ok(())` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:384` | `replace FileSink::persist_progress -> Result<(), FsError> with Ok(())` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:400` | `replace FileSink::finish_item -> Result<PathBuf, FsError> with Ok(Default::default())` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:413` | `replace != with == in FileSink::finish_item` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:181` | `replace FileSource::try_read -> Option<usize> with Some(1)` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:439` | `replace <impl ContentSink for FileSink>::write_at with ()` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:453` | `replace digest_of -> Result<Vec<u8>, FsError> with Ok(vec![])` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:453` | `replace digest_of -> Result<Vec<u8>, FsError> with Ok(vec![0])` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:204` | `replace <impl ContentSource for FileSource>::read_at -> usize with 1` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/manifest_builder.rs:58` | `replace manifest_from_disk -> Result<TransferManifest, FsError> with Ok(Default::default())` | UNVIABLE | UNVIABLE |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:453` | `replace digest_of -> Result<Vec<u8>, FsError> with Ok(vec![1])` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/io.rs:462` | `replace == with != in digest_of` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/manifest_builder.rs:87` | `replace file_size -> Result<u64, FsError> with Ok(1)` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/manifest_builder.rs:87` | `replace file_size -> Result<u64, FsError> with Ok(0)` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/resume.rs:53` | `replace ResumeState::encode -> Vec<u8> with vec![0]` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/resume.rs:53` | `replace ResumeState::encode -> Vec<u8> with vec![]` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/resume.rs:77` | `replace ResumeState::decode -> Result<Self, FsError> with Ok(Default::default())` | UNVIABLE | UNVIABLE |
| qyro_fs | `rust/crates/qyro_fs/src/resume.rs:53` | `replace ResumeState::encode -> Vec<u8> with vec![1]` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/resume.rs:89` | `replace != with == in ResumeState::decode` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/resume.rs:83` | `replace != with == in ResumeState::decode` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/resume.rs:95` | `replace != with == in ResumeState::decode` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/resume.rs:110` | `replace != with == in ResumeState::decode` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/resume.rs:136` | `replace ResumeState::progress_of -> Option<u64> with None` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/resume.rs:136` | `replace ResumeState::progress_of -> Option<u64> with Some(0)` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/resume.rs:136` | `replace ResumeState::progress_of -> Option<u64> with Some(1)` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/resume.rs:138` | `replace == with != in ResumeState::progress_of` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:48` | `replace resolve_under -> Result<Resolved, FsError> with Ok(Default::default())` | UNVIABLE | UNVIABLE |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:32` | `replace part_name -> PathBuf with Default::default()` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:51` | `delete ! in resolve_under` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:61` | `replace \|\| with && in resolve_under` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:61` | `replace == with != in resolve_under` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:61` | `replace == with != in resolve_under` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:73` | `replace match guard error.kind() == std::io::ErrorKind::AlreadyExists with true in resolve_under` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:81` | `delete ! in resolve_under` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:73` | `replace == with != in resolve_under` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:73` | `replace match guard error.kind() == std::io::ErrorKind::AlreadyExists with false in resolve_under` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:102` | `replace assert_not_a_symlink -> Result<(), FsError> with Ok(())` | MISSED | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:103` | `replace match guard metadata.file_type().is_symlink() with true in assert_not_a_symlink` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:108` | `replace match guard error.kind() == std::io::ErrorKind::NotFound with true in assert_not_a_symlink` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:103` | `replace match guard metadata.file_type().is_symlink() with false in assert_not_a_symlink` | MISSED | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:108` | `replace match guard error.kind() == std::io::ErrorKind::NotFound with false in assert_not_a_symlink` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:108` | `replace == with != in assert_not_a_symlink` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:122` | `replace is_inside -> Result<bool, FsError> with Ok(true)` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:122` | `replace is_inside -> Result<bool, FsError> with Ok(false)` | MISSED | MISSED |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:134` | `replace has_no_traversal -> bool with true` | CAUGHT | CAUGHT |
| qyro_fs | `rust/crates/qyro_fs/src/safe_path.rs:134` | `replace has_no_traversal -> bool with false` | CAUGHT | CAUGHT |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:165` | `replace Direction::label -> &'static[u8] with Vec::leak(Vec::new())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:165` | `replace Direction::label -> &'static[u8] with Vec::leak(vec![0])` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:165` | `replace Direction::label -> &'static[u8] with Vec::leak(vec![1])` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:133` | `replace - with +` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:133` | `replace - with /` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:215` | `replace DirectionalKeys::derive -> Result<Self, AeadError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:246` | `replace DirectionalKeys::cipher -> ChaCha20Poly1305 with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:184` | `replace info_for -> Vec<u8> with vec![]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:184` | `replace info_for -> Vec<u8> with vec![0]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:184` | `replace info_for -> Vec<u8> with vec![1]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:349` | `replace FrameSealer::seal -> Result<SealedFrame, AeadError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:263` | `replace nonce_for -> [u8; NONCE_LEN] with [0; NONCE_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:376` | `replace FrameSealer::seal_at -> Result<SealedFrame, AeadError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:263` | `replace nonce_for -> [u8; NONCE_LEN] with [1; NONCE_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:253` | `replace <impl Drop for DirectionalKeys>::drop with ()` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:439` | `replace != with == in FrameSealer::seal_at` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:471` | `replace FrameSealer::check_fault -> Result<(), AeadError> with Ok(())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:539` | `replace SealedFrame::envelope -> &EncryptedEnvelope with Box::leak(Box::new(Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:516` | `replace <impl core::fmt::Debug for FrameSealer>::fmt -> core::fmt::Result with Ok(Default::default())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:466` | `replace FrameSealer::fault_is -> bool with true` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:549` | `replace SealedFrame::nonce -> [u8; NONCE_LEN] with [0; NONCE_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:549` | `replace SealedFrame::nonce -> [u8; NONCE_LEN] with [1; NONCE_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:555` | `replace SealedFrame::encode -> Vec<u8> with vec![]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:555` | `replace SealedFrame::encode -> Vec<u8> with vec![0]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:555` | `replace SealedFrame::encode -> Vec<u8> with vec![1]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:610` | `replace FrameOpener::open -> Result<AuthenticatedFrame, AeadError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:562` | `replace <impl core::fmt::Debug for SealedFrame>::fmt -> core::fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:696` | `replace AuthenticatedFrame::payload -> &[u8] with Vec::leak(Vec::new())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:621` | `replace != with == in FrameOpener::open` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:665` | `replace <impl core::fmt::Debug for FrameOpener>::fmt -> core::fmt::Result with Ok(Default::default())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:711` | `replace AuthenticatedFrame::into_zeroizing_payload -> Zeroizing<Vec<u8>> with Zeroizing::new()` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:711` | `replace AuthenticatedFrame::into_zeroizing_payload -> Zeroizing<Vec<u8>> with Zeroizing::from_iter([vec![]])` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:696` | `replace AuthenticatedFrame::payload -> &[u8] with Vec::leak(vec![0])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:711` | `replace AuthenticatedFrame::into_zeroizing_payload -> Zeroizing<Vec<u8>> with Zeroizing::from_iter([vec![0]])` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:696` | `replace AuthenticatedFrame::payload -> &[u8] with Vec::leak(vec![1])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:711` | `replace AuthenticatedFrame::into_zeroizing_payload -> Zeroizing<Vec<u8>> with Zeroizing::new(vec![])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:711` | `replace AuthenticatedFrame::into_zeroizing_payload -> Zeroizing<Vec<u8>> with Zeroizing::from(vec![])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:711` | `replace AuthenticatedFrame::into_zeroizing_payload -> Zeroizing<Vec<u8>> with Zeroizing::from_iter([vec![1]])` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:711` | `replace AuthenticatedFrame::into_zeroizing_payload -> Zeroizing<Vec<u8>> with Zeroizing::new(vec![0])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:711` | `replace AuthenticatedFrame::into_zeroizing_payload -> Zeroizing<Vec<u8>> with Zeroizing::from(vec![0])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:717` | `replace AuthenticatedFrame::message_type -> MessageType with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:723` | `replace AuthenticatedFrame::session_id -> SessionId with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:711` | `replace AuthenticatedFrame::into_zeroizing_payload -> Zeroizing<Vec<u8>> with Zeroizing::new(vec![1])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:711` | `replace AuthenticatedFrame::into_zeroizing_payload -> Zeroizing<Vec<u8>> with Zeroizing::from(vec![1])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:729` | `replace AuthenticatedFrame::sequence -> u64 with 0` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:729` | `replace AuthenticatedFrame::sequence -> u64 with 1` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:735` | `replace AuthenticatedFrame::transfer_id -> u64 with 0` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:735` | `replace AuthenticatedFrame::transfer_id -> u64 with 1` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:741` | `replace AuthenticatedFrame::stream_id -> u32 with 0` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:753` | `replace AuthenticatedFrame::flags -> Flags with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:747` | `replace AuthenticatedFrame::item_id -> u32 with 0` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:747` | `replace AuthenticatedFrame::item_id -> u32 with 1` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:741` | `replace AuthenticatedFrame::stream_id -> u32 with 1` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:791` | `replace frame_crypto -> Result<(FrameSealer, FrameOpener), AeadError> with Ok((Default::default(), Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:55` | `replace IdentityFingerprint::compute -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:69` | `replace IdentityFingerprint::as_bytes -> &[u8; FINGERPRINT_LEN] with Box::leak(Box::new([0; FINGERPRINT_LEN]))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:69` | `replace IdentityFingerprint::as_bytes -> &[u8; FINGERPRINT_LEN] with Box::leak(Box::new([1; FINGERPRINT_LEN]))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/mod.rs:760` | `replace <impl core::fmt::Debug for AuthenticatedFrame>::fmt -> core::fmt::Result with Ok(Default::default())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/error.rs:61` | `replace <impl fmt::Display for IdentityError>::fmt -> fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:75` | `replace IdentityFingerprint::to_hex -> String with String::new()` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:75` | `replace IdentityFingerprint::to_hex -> String with "xyzzy".into()` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:115` | `replace IdentityFingerprint::parse -> Result<Self, IdentityError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:84` | `replace IdentityFingerprint::to_grouped_hex -> String with String::new()` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:84` | `replace IdentityFingerprint::to_grouped_hex -> String with "xyzzy".into()` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:117` | `replace * with + in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:117` | `replace * with / in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:119` | `replace == with != in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:121` | `replace - with + in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:121` | `replace == with != in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:121` | `replace - with / in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:121` | `replace + with - in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:121` | `replace + with * in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:127` | `replace \|\| with && in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:127` | `replace != with == in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:127` | `replace != with == in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:139` | `replace \|\| with && in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:139` | `delete ! in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:146` | `replace * with + in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:147` | `replace + with - in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:146` | `replace * with / in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:147` | `replace + with * in IdentityFingerprint::parse` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:163` | `replace <impl fmt::Display for IdentityFingerprint>::fmt -> fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fingerprint.rs:157` | `replace <impl fmt::Debug for IdentityFingerprint>::fmt -> fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fuzzing.rs:54` | `replace entropy -> [u8; 64] with [1; 64]` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fuzzing.rs:54` | `replace entropy -> [u8; 64] with [0; 64]` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fuzzing.rs:56` | `replace ^ with \| in entropy` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fuzzing.rs:56` | `replace ^ with & in entropy` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fuzzing.rs:79` | `replace deterministic_session -> Option<FuzzSession> with Some(Default::default())` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fuzzing.rs:79` | `replace deterministic_session -> Option<FuzzSession> with None` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:123` | `replace + with -` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fuzzing.rs:114` | `replace plain_frame -> Option<Frame> with None` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fuzzing.rs:114` | `replace plain_frame -> Option<Frame> with Some(Default::default())` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:123` | `replace + with -` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:123` | `replace + with *` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:123` | `replace + with -` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/fuzzing.rs:123` | `replace replay_window -> crate::aead::FuzzReplayWindow with Default::default()` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:129` | `replace + with -` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:123` | `replace + with *` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:129` | `replace + with *` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:132` | `replace + with -` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:123` | `replace + with *` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:132` | `replace + with -` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:135` | `replace + with -` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:132` | `replace + with *` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:132` | `replace + with *` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:141` | `replace + with -` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:135` | `replace + with *` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:159` | `replace + with -` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:141` | `replace + with *` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:193` | `replace EphemeralKeyPair::from_secret_bytes -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:159` | `replace + with *` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:201` | `replace EphemeralKeyPair::public -> &X25519PublicKey with Box::leak(Box::new(Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:160` | `replace + with -` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:160` | `replace + with *` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:214` | `replace EphemeralKeyPair::diffie_hellman -> Result<Zeroizing<[u8; 32]>, HandshakeError> with Ok(Zeroizing::new())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:214` | `replace EphemeralKeyPair::diffie_hellman -> Result<Zeroizing<[u8; 32]>, HandshakeError> with Ok(Zeroizing::from_iter([[0; 32]]))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:214` | `replace EphemeralKeyPair::diffie_hellman -> Result<Zeroizing<[u8; 32]>, HandshakeError> with Ok(Zeroizing::from_iter([[1; 32]]))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:214` | `replace EphemeralKeyPair::diffie_hellman -> Result<Zeroizing<[u8; 32]>, HandshakeError> with Ok(Zeroizing::new([0; 32]))` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:214` | `replace EphemeralKeyPair::diffie_hellman -> Result<Zeroizing<[u8; 32]>, HandshakeError> with Ok(Zeroizing::from([0; 32]))` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:214` | `replace EphemeralKeyPair::diffie_hellman -> Result<Zeroizing<[u8; 32]>, HandshakeError> with Ok(Zeroizing::new([1; 32]))` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:214` | `replace EphemeralKeyPair::diffie_hellman -> Result<Zeroizing<[u8; 32]>, HandshakeError> with Ok(Zeroizing::from([1; 32]))` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:215` | `delete ! in EphemeralKeyPair::diffie_hellman` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:248` | `replace split_entropy -> (EphemeralKeyPair, [u8; NONCE_LEN]) with (Default::default(), [0; NONCE_LEN])` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:248` | `replace split_entropy -> (EphemeralKeyPair, [u8; NONCE_LEN]) with (Default::default(), [1; NONCE_LEN])` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:236` | `replace system_entropy -> Result<[u8; HANDSHAKE_ENTROPY_LEN], HandshakeError> with Ok([0; HANDSHAKE_ENTROPY_LEN])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:236` | `replace system_entropy -> Result<[u8; HANDSHAKE_ENTROPY_LEN], HandshakeError> with Ok([1; HANDSHAKE_ENTROPY_LEN])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:227` | `replace <impl core::fmt::Debug for EphemeralKeyPair>::fmt -> core::fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:271` | `replace > with == in check_prefix` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:271` | `replace > with >= in check_prefix` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:271` | `replace check_prefix -> Result<(), HandshakeError> with Ok(())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:271` | `replace > with < in check_prefix` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:276` | `replace < with == in check_prefix` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:273` | `replace - with + in check_prefix` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:273` | `replace - with / in check_prefix` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:276` | `replace < with <= in check_prefix` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:293` | `replace != with == in check_prefix` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:299` | `replace != with == in check_prefix` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:276` | `replace < with > in check_prefix` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:305` | `replace != with == in check_prefix` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:326` | `replace fixed_field -> Result<[u8; WIDTH], HandshakeError> with Ok([0; WIDTH])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:344` | `replace hello_unsigned -> Result<&[u8; HELLO_UNSIGNED_LEN], HandshakeError> with Ok(Box::leak(Box::new([0; HELLO_UNSIGNED_LEN])))` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:357` | `replace parse_hello_body -> Result<(X25519PublicKey, [u8; NONCE_LEN], PublicIdentity), HandshakeError> with Ok((Default::default(), [0; NONCE_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:326` | `replace fixed_field -> Result<[u8; WIDTH], HandshakeError> with Ok([1; WIDTH])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:357` | `replace parse_hello_body -> Result<(X25519PublicKey, [u8; NONCE_LEN], PublicIdentity), HandshakeError> with Ok((Default::default(), [1; NONCE_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:344` | `replace hello_unsigned -> Result<&[u8; HELLO_UNSIGNED_LEN], HandshakeError> with Ok(Box::leak(Box::new([1; HELLO_UNSIGNED_LEN])))` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:382` | `replace + with - in write_hello_unsigned` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:378` | `replace write_hello_unsigned -> [u8; HELLO_UNSIGNED_LEN] with [0; HELLO_UNSIGNED_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:378` | `replace write_hello_unsigned -> [u8; HELLO_UNSIGNED_LEN] with [1; HELLO_UNSIGNED_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:382` | `replace + with * in write_hello_unsigned` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:364` | `delete match arm IdentityError::WeakPublicKey in parse_hello_body` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:384` | `replace + with - in write_hello_unsigned` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:384` | `replace + with * in write_hello_unsigned` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:391` | `replace signature_at -> Result<IdentitySignature, HandshakeError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:385` | `replace + with - in write_hello_unsigned` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:385` | `replace + with * in write_hello_unsigned` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:407` | `replace sign_transcript -> Result<IdentitySignature, HandshakeError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:395` | `replace mac_at -> Result<[u8; FINISHED_MAC_LEN], HandshakeError> with Ok([0; FINISHED_MAC_LEN])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:395` | `replace mac_at -> Result<[u8; FINISHED_MAC_LEN], HandshakeError> with Ok([1; FINISHED_MAC_LEN])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:454` | `replace InitiatorStart<'identity>::send_hello -> Result<([u8; INITIATOR_HELLO_LEN], InitiatorAwaitResponder<'identity>,), HandshakeError, > with Ok(([0; INITIATOR_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:454` | `replace InitiatorStart<'identity>::send_hello -> Result<([u8; INITIATOR_HELLO_LEN], InitiatorAwaitResponder<'identity>,), HandshakeError, > with Ok(([1; INITIATOR_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:477` | `replace InitiatorStart<'identity>::send_hello_with_entropy -> Result<([u8; INITIATOR_HELLO_LEN], InitiatorAwaitResponder<'identity>,), HandshakeError, > with Ok(([0; INITIATOR_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:477` | `replace InitiatorStart<'identity>::send_hello_with_entropy -> Result<([u8; INITIATOR_HELLO_LEN], InitiatorAwaitResponder<'identity>,), HandshakeError, > with Ok(([1; INITIATOR_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:417` | `replace verify_transcript -> Result<(), HandshakeError> with Ok(())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:491` | `replace InitiatorStart<'identity>::send_hello_with_entropy -> Result<([u8; INITIATOR_HELLO_LEN], InitiatorAwaitResponder<'identity>,), HandshakeError, > with Ok(([0; INITIATOR_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:504` | `replace InitiatorStart<'identity>::build_hello -> Result<([u8; INITIATOR_HELLO_LEN], InitiatorAwaitResponder<'identity>,), HandshakeError, > with Ok(([0; INITIATOR_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:504` | `replace InitiatorStart<'identity>::build_hello -> Result<([u8; INITIATOR_HELLO_LEN], InitiatorAwaitResponder<'identity>,), HandshakeError, > with Ok(([1; INITIATOR_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:491` | `replace InitiatorStart<'identity>::send_hello_with_entropy -> Result<([u8; INITIATOR_HELLO_LEN], InitiatorAwaitResponder<'identity>,), HandshakeError, > with Ok(([1; INITIATOR_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:542` | `replace InitiatorAwaitResponder<'_>::receive_responder_hello -> Result<([u8; INITIATOR_FINISH_LEN], InitiatorAwaitResponderFinish), HandshakeError> with Ok(([0; INITIATOR_FINISH_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:542` | `replace InitiatorAwaitResponder<'_>::receive_responder_hello -> Result<([u8; INITIATOR_FINISH_LEN], InitiatorAwaitResponderFinish), HandshakeError> with Ok(([1; INITIATOR_FINISH_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:578` | `replace + with - in InitiatorAwaitResponder<'_>::receive_responder_hello` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:580` | `replace + with - in InitiatorAwaitResponder<'_>::receive_responder_hello` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:607` | `replace InitiatorAwaitResponderFinish::peer_identity -> &PublicIdentity with Box::leak(Box::new(Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:621` | `replace InitiatorAwaitResponderFinish::receive_responder_finish -> Result<EstablishedInitiator, HandshakeError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:578` | `replace + with * in InitiatorAwaitResponder<'_>::receive_responder_hello` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:664` | `replace ResponderStart<'identity>::receive_initiator_hello_from_system -> Result<([u8; RESPONDER_HELLO_LEN], ResponderAwaitInitiatorFinish), HandshakeError> with Ok(([0; RESPONDER_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:664` | `replace ResponderStart<'identity>::receive_initiator_hello_from_system -> Result<([u8; RESPONDER_HELLO_LEN], ResponderAwaitInitiatorFinish), HandshakeError> with Ok(([1; RESPONDER_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:580` | `replace + with * in InitiatorAwaitResponder<'_>::receive_responder_hello` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:680` | `replace ResponderStart<'identity>::receive_initiator_hello -> Result<([u8; RESPONDER_HELLO_LEN], ResponderAwaitInitiatorFinish), HandshakeError> with Ok(([0; RESPONDER_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:680` | `replace ResponderStart<'identity>::receive_initiator_hello -> Result<([u8; RESPONDER_HELLO_LEN], ResponderAwaitInitiatorFinish), HandshakeError> with Ok(([1; RESPONDER_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:697` | `replace ResponderStart<'identity>::answer_hello -> Result<([u8; RESPONDER_HELLO_LEN], ResponderAwaitInitiatorFinish), HandshakeError> with Ok(([0; RESPONDER_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:689` | `replace ResponderStart<'identity>::receive_initiator_hello -> Result<([u8; RESPONDER_HELLO_LEN], ResponderAwaitInitiatorFinish), HandshakeError> with Ok(([0; RESPONDER_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:697` | `replace ResponderStart<'identity>::answer_hello -> Result<([u8; RESPONDER_HELLO_LEN], ResponderAwaitInitiatorFinish), HandshakeError> with Ok(([1; RESPONDER_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:689` | `replace ResponderStart<'identity>::receive_initiator_hello -> Result<([u8; RESPONDER_HELLO_LEN], ResponderAwaitInitiatorFinish), HandshakeError> with Ok(([1; RESPONDER_HELLO_LEN], Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:759` | `replace ResponderAwaitInitiatorFinish::receive_initiator_finish -> Result<ResponderFinishPending, HandshakeError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:775` | `replace + with - in ResponderAwaitInitiatorFinish::receive_initiator_finish` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:820` | `replace ResponderFinishPending::encoded_finish -> &[u8; RESPONDER_FINISH_LEN] with Box::leak(Box::new([0; RESPONDER_FINISH_LEN]))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:820` | `replace ResponderFinishPending::encoded_finish -> &[u8; RESPONDER_FINISH_LEN] with Box::leak(Box::new([1; RESPONDER_FINISH_LEN]))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:829` | `replace ResponderFinishPending::peer_identity -> &PublicIdentity with Box::leak(Box::new(Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:844` | `replace ResponderFinishPending::confirm_sent -> EstablishedResponder with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:775` | `replace + with * in ResponderAwaitInitiatorFinish::receive_initiator_finish` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:42` | `replace + with -` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:61` | `replace IdentitySecret::from_bytes -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:42` | `replace + with *` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/mod.rs:852` | `replace <impl core::fmt::Debug for ResponderFinishPending>::fmt -> core::fmt::Result with Ok(Default::default())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:116` | `replace DeviceIdentity::generate -> Result<Self, IdentityError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:73` | `replace IdentitySecret::as_bytes -> &[u8; SEED_LEN] with Box::leak(Box::new([0; SEED_LEN]))` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:136` | `replace DeviceIdentity::from_test_seed -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:140` | `replace DeviceIdentity::from_seed -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:73` | `replace IdentitySecret::as_bytes -> &[u8; SEED_LEN] with Box::leak(Box::new([1; SEED_LEN]))` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:164` | `replace DeviceIdentity::export_secret -> IdentitySecret with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:177` | `replace DeviceIdentity::from_secret -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:183` | `replace DeviceIdentity::public_identity -> &PublicIdentity with Box::leak(Box::new(Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:79` | `replace <impl fmt::Debug for IdentitySecret>::fmt -> fmt::Result with Ok(Default::default())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:189` | `replace DeviceIdentity::fingerprint -> &IdentityFingerprint with Box::leak(Box::new(Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:213` | `replace DeviceIdentity::try_sign -> Result<IdentitySignature, IdentityError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:241` | `replace PublicIdentity::from_verifying_key -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:265` | `replace PublicIdentity::from_bytes -> Result<Self, IdentityError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:297` | `replace PublicIdentity::decode -> Result<Self, IdentityError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:283` | `replace PublicIdentity::encode -> [u8; PUBLIC_IDENTITY_WIRE_LEN] with [1; PUBLIC_IDENTITY_WIRE_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:283` | `replace PublicIdentity::encode -> [u8; PUBLIC_IDENTITY_WIRE_LEN] with [0; PUBLIC_IDENTITY_WIRE_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:223` | `replace <impl fmt::Debug for DeviceIdentity>::fmt -> fmt::Result with Ok(Default::default())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:317` | `replace PublicIdentity::from_versioned_bytes -> Result<Self, IdentityError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:317` | `replace != with == in PublicIdentity::from_versioned_bytes` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:335` | `replace PublicIdentity::as_bytes -> &[u8; PUBLIC_KEY_LEN] with Box::leak(Box::new([0; PUBLIC_KEY_LEN]))` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:329` | `replace PublicIdentity::version -> u8 with 1` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:329` | `replace PublicIdentity::version -> u8 with 0` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:341` | `replace PublicIdentity::fingerprint -> &IdentityFingerprint with Box::leak(Box::new(Default::default()))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:335` | `replace PublicIdentity::as_bytes -> &[u8; PUBLIC_KEY_LEN] with Box::leak(Box::new([1; PUBLIC_KEY_LEN]))` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:367` | `replace PublicIdentity::verify -> Result<(), IdentityError> with Ok(())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/identity.rs:378` | `replace <impl fmt::Debug for PublicIdentity>::fmt -> fmt::Result with Ok(Default::default())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/signature.rs:62` | `replace SignatureDomain::to_wire -> u8 with 0` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/signature.rs:62` | `replace SignatureDomain::to_wire -> u8 with 1` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/signature.rs:75` | `replace SignatureDomain::is_available -> bool with false` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/signature.rs:75` | `replace SignatureDomain::is_available -> bool with true` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/signature.rs:82` | `replace SignatureDomain::ensure_available -> Result<(), IdentityError> with Ok(())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/signature.rs:97` | `replace SignatureDomain::signing_input -> Vec<u8> with vec![]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/signature.rs:115` | `replace IdentitySignature::from_bytes -> Self with Default::default()` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/signature.rs:124` | `replace IdentitySignature::from_slice -> Result<Self, IdentityError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/signature.rs:137` | `replace IdentitySignature::as_bytes -> &[u8; SIGNATURE_LEN] with Box::leak(Box::new([0; SIGNATURE_LEN]))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/signature.rs:97` | `replace SignatureDomain::signing_input -> Vec<u8> with vec![0]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/signature.rs:137` | `replace IdentitySignature::as_bytes -> &[u8; SIGNATURE_LEN] with Box::leak(Box::new([1; SIGNATURE_LEN]))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/signature.rs:97` | `replace SignatureDomain::signing_input -> Vec<u8> with vec![1]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/signature.rs:143` | `replace IdentitySignature::to_hex -> String with String::new()` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/signature.rs:143` | `replace IdentitySignature::to_hex -> String with "xyzzy".into()` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/signature.rs:150` | `replace <impl fmt::Debug for IdentitySignature>::fmt -> fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/error.rs:99` | `replace <impl fmt::Display for AeadError>::fmt -> fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:23` | `replace / with *` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:23` | `replace / with %` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:66` | `replace ReplayWindow::check -> Result<(), AeadError> with Ok(())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:70` | `replace > with == in ReplayWindow::check` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:70` | `replace > with < in ReplayWindow::check` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:70` | `replace > with >= in ReplayWindow::check` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:74` | `replace - with + in ReplayWindow::check` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:74` | `replace - with / in ReplayWindow::check` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:75` | `replace >= with < in ReplayWindow::check` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:84` | `replace == with != in ReplayWindow::check` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:103` | `replace ReplayWindow::slot -> Result<u64, AeadError> with Ok(0)` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:103` | `replace ReplayWindow::slot -> Result<u64, AeadError> with Ok(1)` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:105` | `replace / with % in ReplayWindow::slot` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:105` | `replace / with * in ReplayWindow::slot` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:107` | `replace & with \| in ReplayWindow::slot` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:107` | `replace & with ^ in ReplayWindow::slot` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:107` | `replace << with >> in ReplayWindow::slot` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:107` | `replace % with / in ReplayWindow::slot` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:107` | `replace % with + in ReplayWindow::slot` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:120` | `replace ReplayWindow::record -> Result<(), AeadError> with Ok(())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:128` | `replace match guard sequence > highest with true in ReplayWindow::record` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:128` | `replace match guard sequence > highest with false in ReplayWindow::record` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:128` | `replace > with == in ReplayWindow::record` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:128` | `replace > with < in ReplayWindow::record` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:128` | `replace > with >= in ReplayWindow::record` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:129` | `replace - with / in ReplayWindow::record` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:129` | `replace - with + in ReplayWindow::record` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:130` | `replace >= with < in ReplayWindow::record` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:144` | `replace - with + in ReplayWindow::record` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:144` | `replace - with / in ReplayWindow::record` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:164` | `replace ReplayWindow::set -> Result<(), AeadError> with Ok(())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:166` | `replace / with % in ReplayWindow::set` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:166` | `replace / with * in ReplayWindow::set` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:168` | `replace \|= with &= in ReplayWindow::set` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:168` | `replace << with >> in ReplayWindow::set` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:168` | `replace % with / in ReplayWindow::set` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:168` | `replace % with + in ReplayWindow::set` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:182` | `replace / with % in ReplayWindow::shift` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:179` | `replace >= with < in ReplayWindow::shift` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:179` | `replace ReplayWindow::shift -> Result<(), AeadError> with Ok(())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:182` | `replace / with * in ReplayWindow::shift` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:183` | `replace % with / in ReplayWindow::shift` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:183` | `replace % with + in ReplayWindow::shift` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:195` | `replace && with \|\| in ReplayWindow::shift` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:194` | `replace << with >> in ReplayWindow::shift` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:195` | `replace > with == in ReplayWindow::shift` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:195` | `replace > with >= in ReplayWindow::shift` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:195` | `replace > with == in ReplayWindow::shift` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:195` | `replace > with < in ReplayWindow::shift` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:195` | `replace > with >= in ReplayWindow::shift` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:195` | `replace > with < in ReplayWindow::shift` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:199` | `replace - with + in ReplayWindow::shift` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:199` | `replace - with / in ReplayWindow::shift` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:201` | `replace \|= with &= in ReplayWindow::shift` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:201` | `replace - with + in ReplayWindow::shift` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:201` | `replace - with / in ReplayWindow::shift` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/schedule.rs:34` | `replace Label::as_bytes -> &'static[u8] with Vec::leak(Vec::new())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/schedule.rs:34` | `replace Label::as_bytes -> &'static[u8] with Vec::leak(vec![0])` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/aead/replay.rs:201` | `replace >> with << in ReplayWindow::shift` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/schedule.rs:34` | `replace Label::as_bytes -> &'static[u8] with Vec::leak(vec![1])` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/error.rs:101` | `replace <impl fmt::Display for HandshakeError>::fmt -> fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/schedule.rs:66` | `replace SessionKey::as_bytes -> &[u8; DERIVED_KEY_LEN] with Box::leak(Box::new([0; DERIVED_KEY_LEN]))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/schedule.rs:66` | `replace SessionKey::as_bytes -> &[u8; DERIVED_KEY_LEN] with Box::leak(Box::new([1; DERIVED_KEY_LEN]))` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/schedule.rs:109` | `replace Schedule::derive -> Result<Self, HandshakeError> with Ok(Default::default())` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/schedule.rs:145` | `replace expand_exact -> Result<[u8; N], HandshakeError> with Ok([0; N])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/schedule.rs:78` | `replace <impl core::fmt::Debug for SessionKey>::fmt -> core::fmt::Result with Ok(Default::default())` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/schedule.rs:72` | `replace <impl Drop for SessionKey>::drop with ()` | MISSED | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/schedule.rs:145` | `replace expand_exact -> Result<[u8; N], HandshakeError> with Ok([1; N])` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/schedule.rs:183` | `replace finished_mac -> [u8; FINISHED_MAC_LEN] with [0; FINISHED_MAC_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/schedule.rs:183` | `replace finished_mac -> [u8; FINISHED_MAC_LEN] with [1; FINISHED_MAC_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/schedule.rs:210` | `replace verify_finished_mac -> Result<(), HandshakeError> with Ok(())` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/transcript.rs:35` | `replace base_transcript -> [u8; TRANSCRIPT_LEN] with [0; TRANSCRIPT_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/transcript.rs:35` | `replace base_transcript -> [u8; TRANSCRIPT_LEN] with [1; TRANSCRIPT_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/transcript.rs:57` | `replace auth_transcript -> [u8; TRANSCRIPT_LEN] with [1; TRANSCRIPT_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/transcript.rs:57` | `replace auth_transcript -> [u8; TRANSCRIPT_LEN] with [0; TRANSCRIPT_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/transcript.rs:68` | `replace responder_signing_message -> [u8; TRANSCRIPT_LEN] with [0; TRANSCRIPT_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/transcript.rs:82` | `replace + with - in initiator_signing_message` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/transcript.rs:82` | `replace + with * in initiator_signing_message` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/transcript.rs:83` | `replace + with - in initiator_signing_message` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/transcript.rs:83` | `replace + with * in initiator_signing_message` | UNVIABLE | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/transcript.rs:68` | `replace responder_signing_message -> [u8; TRANSCRIPT_LEN] with [1; TRANSCRIPT_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/transcript.rs:83` | `replace initiator_signing_message -> [u8; TRANSCRIPT_LEN +64] with [0; TRANSCRIPT_LEN +64]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/transcript.rs:83` | `replace initiator_signing_message -> [u8; TRANSCRIPT_LEN +64] with [1; TRANSCRIPT_LEN +64]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/transcript.rs:103` | `replace update_with_length with ()` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/transcript.rs:108` | `replace finish -> [u8; TRANSCRIPT_LEN] with [0; TRANSCRIPT_LEN]` | CAUGHT | NOT_RUN |
| qyro_crypto | `rust/crates/qyro_crypto/src/handshake/transcript.rs:108` | `replace finish -> [u8; TRANSCRIPT_LEN] with [1; TRANSCRIPT_LEN]` | CAUGHT | NOT_RUN |
