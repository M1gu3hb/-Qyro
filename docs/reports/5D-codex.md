# Sprint 5D — reparar el ledger y construir la confianza

## 0. Identidad y alcance

- Rama: `codex/qyro-trust-5d`.
- Base exacta: `ebdffb919bf029dbe971c5dd07864fc672a186af`.
- Inicio: árbol limpio; no se modificó `main`.
- Rango propio nuevo: QYR-0289 en adelante.
- Archivos excluidos: `qyro_net`, `qyro_net_smoke`, `qyro_ffi`,
  `qyro_transfer`, `apps/qyro`, `Cargo.toml`, `Cargo.lock`, workflows y
  `MINIMUM_GUARD_SET_EXCEPTIONS`.

## 1. Baseline reproducido antes de tocar código

| Comando | Resultado |
|---|---|
| `bash scripts/doctor.sh` | FAIL ambiental esperado: Flutter y Dart no están en `PATH`; Rust/Cargo 1.88.0 sí están |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS en Windows; 434 passed, 0 failed, 2 ignored |
| `cargo test --doc --workspace` | PASS |
| `cargo-audit 0.22.2 audit --deny warnings` desde la raíz | PASS; 61 dependencias escaneadas |
| `bash scripts/check_docs_consistency.sh` | PASS |
| `powershell.exe -NoProfile -File scripts/check_docs_consistency.ps1` | PASS en Windows PowerShell 5.1 |

El conteo del workspace se obtiene sumando los `test result` emitidos por
`cargo test --workspace`: 434 passed y dos generadores de vectores ignored. El
conteo Linux heredado y reproducido en CI 31550242552 es 428/2.

## 2. Protocolo de puertas

Cada puerta registra formato, Clippy, workspace, mutación con límite, lectura de
aserciones/contadores/nombres, falsabilidad de mediciones nuevas, número de
entradas abiertas, alcance desde `ebdffb9`, coherencia del informe y checker
Bash. Si una comprobación falla, la puerta permanece abierta.

## 3. Puerta 1 — ledger legible

Cerrada en el árbol candidato de Fase 1.

El punto de partida real de esta rama, medido con `Get-Content` y expresiones
regulares sobre encabezados y campos `Estado`, fue 262 fichas y 162 abiertas.
La rama base aún no incluye las fichas del otro agente; por eso no se repiten
aquí los 279/167 del árbol fusionado del prompt. Se retiraron las 173 fichas
mecánicas consecutivas 0115–0287 y el resultado es **99 fichas, 22 abiertas**.
El comando fue:

```powershell
$lines = Get-Content BUGS_PENDING.md -Encoding UTF8
@($lines | ? { $_ -match '^## QYR-\d{4} —' }).Count
@($lines | ? { $_ -match '^- Estado:\s*abierto(?:[;\s]|$)' }).Count
```

El inventario no se perdió: `mutation-sweep-2026-08-11.md` contiene los 939
mutantes del barrido principal, uno por fila, y cinco filas de totales por
crate. La clasificación manual de los 136 supervivientes que seguían abiertos
es 49 ruido/equivalencia, 24 cobertura funcional y 63 validación, rechazo o
integridad. Los doce timeouts quedan separados. Se crearon diez fichas humanas
en 5D: ocho familias de supervivientes, una para los timeouts y una para la
propia avería del ledger; no se superó el máximo de quince.

Se eligió guardar **volumen**, no intentar reconocer títulos «humanos» con una
regex. El volumen tiene una frontera determinista y ataca la causa observada;
una heurística de estilo sería fácil de satisfacer con texto igualmente opaco.
Los checkers Bash y PowerShell 5.1 permiten 59 abiertas y bloquean 60. Antes de
añadir la regla, ambos contratos fallaron porque el fixture de 60 obtenía
`[OK]`; después pasan. También se ejecuta el lado positivo de 59, de modo que la
aserción puede distinguir ambos lados y el contador proviene de las líneas del
fixture, no de una constante.

### Doce comprobaciones

| # | Comprobación | Resultado |
|---:|---|---|
| 1 | `cargo fmt --all --check` | PASS por código 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS por código 0 |
| 3 | `cargo test --workspace` | PASS Windows: 434 passed, 0 failed, 2 ignored ya existentes |
| 4 | Mutación de fase con límite | No cambió Rust. Mutante focal equivalente «omitir la guarda de volumen»: RED inmediato en Bash y PS 5.1; restaurada, ambos contratos PASS, sin timeout |
| 5 | Lectura de aserciones | 59 y 60 son observables distintos; no se compara una llamada consigo misma |
| 6 | Lectura de contadores | El valor se deriva contando `Estado: abierto` en el ledger del fixture |
| 7 | La medida se ve fallar | RED previo documentado en ambos contratos con 60 abiertas |
| 8 | Lectura de nombres | `at_open_limit` prueba aceptación del borde y `too_many_open` prueba su rechazo |
| 9 | Ledger legible | 22 abiertas; diez fichas nuevas 5D, todas tituladas por consecuencia |
| 10 | Alcance desde `ebdffb9` | Sólo ledger, dos checkers, dos contratos y los dos informes 5D; ningún archivo excluido ni cambio en `MINIMUM_GUARD_SET_EXCEPTIONS` |
| 11 | Coherencia del informe | Releídas las secciones 0–3 y el criterio del informe de mutación contra el árbol actual |
| 12 | `check_docs_consistency` | PASS con Git Bash y PASS con Windows PowerShell 5.1 |

Runs fallidos que no se ocultan: el primer `bash` resolvió a WSL sin una distro
instalada; se repitió con `C:\Program Files\Git\bin\bash.exe`. Dos comandos de
orquestación intentaron `scripts/check_diff_scope.{sh,ps1}`, que no existen;
ambos fallaron después de que los contratos/checkers correspondientes ya habían
pasado. No se presenta ninguno como evidencia y no hubo run cancelado.

## 4. Puerta 2 — timeouts

Cerrada. La tabla individual y los argumentos estructurales viven junto al
inventario en `mutation-sweep-2026-08-11.md`.

La pregunta prioritaria tiene respuesta negativa en el código real: `parse`
acepta sólo `header_len == 48`, limita payload/trailer y `total_len` suma esos
tres valores. No existe una cabecera aceptada con total cero ni menor que su
propia cabecera, por lo que no se halló el P0 remoto planteado. Sí había un hueco
de prueba: diez binarios mutados repetían trabajo cuando un input alcanzaba la
rama alterada, pero ningún peer puede crear esa alteración interna en el código
real. Los bucles de drenaje tampoco tenían presupuesto propio. Ahora todo
drenaje está limitado por frames/bytes disponibles; generación vacía y
constantes infladas también fallan sin escalar el workload.

Enmienda P2: aunque hoy esa longitud inválida no es alcanzable desde wire, el
decodificador ya no depende implícitamente de ello. `require_frame_progress`
rechaza con `FrameError::DecoderNoProgress` cualquier total menor que los 48
bytes de cabecera que hubo que leer para calcularlo. La prueba nació RED por la
ausencia de función y variante, pasó después del cambio, y los cinco mutantes
focales de la guarda terminaron **5 caught, 0 missed, 0 timeout** en 31 s.

La primera reejecución a 30 s seleccionó por error de regex 24 mutantes en vez de
doce y terminó con `22 caught, 1 unviable, 1 timeout`; permanece registrada como
fallida. El restante era `reserve_for: + -> *`: aunque tres tests ya fallaban,
otros seguían gastando el presupuesto. La medida focal de dos lecturas de 48
bytes verifica que la capacidad crece como máximo geométricamente. La
reejecución exacta terminó `1 caught` en 12 s. Cruzados los nombres contra el
JSON original, los doce antiguos `TIMEOUT` son ahora `CAUGHT`.

### Doce comprobaciones

| # | Comprobación | Resultado |
|---:|---|---|
| 1 | `cargo fmt --all --check` | PASS por código 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS por código 0 |
| 3 | `cargo test --workspace` | PASS Windows: 436 passed, 0 failed, 2 ignored ya existentes; +2 tests frente al baseline |
| 4 | Mutación con límite | Primera pasada amplia falló 22/1/1; reejecución focal del único timeout: 1 CAUGHT bajo 30 s. Los doce originales quedan CAUGHT; la enmienda de progreso terminó 5/0/0 |
| 5 | Lectura de aserciones | Cada presupuesto distingue `Some` que consume de repetición; el mínimo distingue 48 de 0; reserva distingue crecimiento geométrico de multiplicativo |
| 6 | Lectura de contadores | Límites derivados de `frames.len()`, `data.len() / HEADER_LEN`, bytes empujados y capacidad observada |
| 7 | La medida se ve fallar | Los JSON/logs originales prueban doce timeouts; el primer rerun prueba que la nueva medida de reserva aún era insuficiente; el segundo mata ese mutante |
| 8 | Lectura de nombres | El test de cabecera ejerce total y consumo; el de suma ejerce la suma; corpus/property ejercen progreso acotado |
| 9 | Ledger legible | 21 abiertas; no se añadió ficha y QYR-0298 quedó resuelta con una conclusión |
| 10 | Alcance desde `ebdffb9` | Sólo protocolo propio, ledger, checkers/contratos e informes; ningún archivo de Claude Code ni constante prohibida |
| 11 | Coherencia del informe | Releídas secciones 0–4 y la clasificación/timeouts del inventario contra código y JSON de ambos reruns |
| 12 | `check_docs_consistency` | PASS Git Bash y PASS Windows PowerShell 5.1 |

## 5. Puerta 3 — supervivientes importantes

Cerrada con un residual Windows explícito, no con una falsa declaración de
cobertura. Los 63 nombres se recuperaron literalmente del ledger de `ebdffb9` y
`cargo-mutants --list` confirmó 25 protocolo + 26 manifest + 11 filesystem + 1
crypto.

El intento inicial de usar todos los tests del workspace para protocolo agotó
10 min; el JSON parcial 2 caught/15 timeout prueba que consumidores con loops no
acotados oscurecen el control. Los reruns por crate a 30 s son la evidencia:
protocolo 17 caught/8 equivalentes; manifest 24 caught/2 equivalentes;
filesystem 6 caught/4 sin consecuencia de aceptación/1 pendiente; replay una
equivalencia. QYR-0291, QYR-0293 y QYR-0297 quedan resueltas. QYR-0295 sigue P1
con un único control material: inspección del handle de un symlink de archivo en
Windows. El test real existe y CI lo ejecuta con feature, pero localmente la
creación falla por privilegio (Win32 1314); se conserva abierto hasta poder
mutarlo bajo ese fixture.

### Doce comprobaciones

| # | Comprobación | Resultado |
|---:|---|---|
| 1 | `cargo fmt --all --check` | PASS por código 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS por código 0 |
| 3 | `cargo test --workspace` | PASS Windows: 459 passed, 0 failed, 2 ignored ya existentes |
| 4 | Mutación con límite | 17/8 protocolo, 24/2 manifest, 6/4/1 FS, replay equivalente; cada rerun `--timeout 30` y residual nombrado |
| 5 | Lectura de aserciones | Cada frontera tiene lado exacto/uno más; contención tiene hijo/outsider; associated data no acepta placeholders |
| 6 | Lectura de contadores | Counts/longitudes derivan de bytes, items y paths construidos; no se añadió medición de rendimiento |
| 7 | La medida se ve fallar | Cada contrato está ligado a su diff de mutante; junction real mata `-> false`; el run workspace fallido demuestra el detector inadecuado |
| 8 | Lectura de nombres | Los nombres describen ciphertext, trailer, offsets, límites, colisión, escritura, contención y junction ejercidos realmente |
| 9 | Ledger legible | 18 abiertas; cero fichas nuevas en la fase, tres familias resueltas y una residual acotada |
| 10 | Alcance desde `ebdffb9` | Sólo crates propios, ledger e informes; sin archivos de Claude Code ni constante prohibida |
| 11 | Coherencia del informe | Releídas secciones 0–5 y las tres secciones humanas del informe de mutación contra JSON/código actual |
| 12 | `check_docs_consistency` | PASS Git Bash y PASS Windows PowerShell 5.1 |

Fallos conservados: run protocolo workspace con timeout externo; primer run
protocolo local 16/9 antes del contrato de unknown+trailer; manifest 24/2 por
equivalencia; FS 5/6 antes del fixture junction; test FS falló inicialmente por
comparar un path canónico Windows con su spelling no canónico; el test de
symlink con feature falló por privilegio 1314. Ninguno se presenta como PASS.

## 6. Puerta 4 — ADR-0031

Cerrada. `docs/adr/ADR-0031-trust-and-pairing.md` queda congelada antes de
cualquier código de confianza. La decisión combina TOFU explícito con
comparación opcional fuera de banda: un primer contacto es `New`, nunca
«trusted» automático; una clave distinta para el registro local esperado es
`KnownAndChanged` y termina la sesión sin sobrescribir nada.

La forma humana son los primeros 128 bits de la huella SHA-256 canónica,
codificados como cuatro grupos de ocho hexadecimales minúsculos. Un match
dirigido cuesta en esperanza `2^128 ≈ 3.40 × 10^38` claves; la colisión por
cumpleaños ronda `2^64 ≈ 1.84 × 10^19`. La decisión de confianza sigue
comparando la identidad pública completa, no el prefijo mostrado.

El formato queda fijado antes de implementarlo: cabecera exterior de 16 bytes
con magic `QYRO-KPS`, versión y wrapper rechazados por nombre, reservado cero y
cuerpo envuelto limitado a 2 MiB; cuerpo todo-o-nada con máximo 4096 registros,
longitud por registro, identidad pública canónica, nombre local UTF-8 acotado y
fechas de primer/último contacto. No hay fallback en claro. Windows usa DPAPI
en `%LOCALAPPDATA%`; Android/iOS quedan sin persistencia hasta tener los
wrappers de plataforma que este sprint excluye.

Se completa primero el handshake para autenticar la clave que se muestra, pero
el estado establecido queda en cuarentena: no hay manifest, transferencia ni
datos de aplicación antes del veredicto y una negativa destruye la sesión. La
ADR declara explícitamente que sin UI sólo existe el mecanismo, no la política
interactiva.

### Doce comprobaciones

| # | Comprobación | Resultado |
|---:|---|---|
| 1 | `cargo fmt --all --check` | PASS por código 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS por código 0 |
| 3 | `cargo test --workspace` | PASS Windows: 459 passed, 0 failed, 2 ignored ya existentes; la fase no cambió ejecutables |
| 4 | Mutación con límite | No aplica: la fase sólo añade una ADR y no existe módulo Rust nuevo que mutar |
| 5 | Lectura de aserciones | No aplica: no se añadieron aserciones; la evidencia exigida se congela por nombre para Fase 5 |
| 6 | Lectura de contadores | Los límites 4096/255/2 MiB son cotas de formato, no resultados fingidos de una operación |
| 7 | La medida se ve fallar | No hay medición nueva: `2^128` y `2^64` son costes analíticos del ancho elegido, no benchmarks |
| 8 | Lectura de nombres | Los cinco contratos exigidos nombran literalmente match, cambio, nuevo, versión futura y truncado que deberán ejercer |
| 9 | Ledger legible | 18 abiertas por el contador canónico; cero fichas nuevas en la fase y diez en todo 5D |
| 10 | Alcance desde `ebdffb9` | La fase añade ADR-0031 e informe; no toca archivo de Claude Code, crate excluido, Cargo ni `MINIMUM_GUARD_SET_EXCEPTIONS` |
| 11 | Coherencia del informe | Releídas secciones 0–6 y el informe de mutación; nada de Fases 1–3 queda invalidado por una decisión documental |
| 12 | `check_docs_consistency` | PASS Git Bash (16 s) y PASS Windows PowerShell 5.1 (51.3 s) |

## 7. Puerta 5 — confianza implementada

Cerrada. `qyro_identity_store` contiene el módulo nuevo sin crate ni
dependencia nueva. La API pública separa `KnownAndMatches`,
`KnownAndChanged` y `New`; la decisión pura localiza por el nombre local
esperado y compara la identidad pública completa. El primer test escrito no
compiló por los nueve símbolos todavía ausentes, que fue el RED correcto.

El store aplica el formato congelado: cabecera `QYRO-KPS`, versión/wrapper
tipados, reservado cero, cuerpo envuelto máximo 2 MiB, máximo 4096 registros,
longitud exacta por registro, identidad pública canónica, nombre UTF-8 de
1–255 bytes y tiempos válidos. Rechaza duplicados de nombre o clave y parsea
todo-o-nada. El cuerpo claro se mantiene en `Zeroizing` durante el sellado y el
wrapper ya devuelve un buffer zeroizing al abrir. `HumanFingerprint` muestra
exactamente 16 bytes en cuatro grupos hexadecimales; la confianza no usa ese
prefijo.

El primer barrido completo encontró 124 mutantes: 73 caught, 39 missed, 12
unviable y cero timeouts. Los contratos de frontera redujeron el árbol final a
104 mutantes materiales/generables; los dos barridos completos posteriores
terminaron **95 caught, 0 missed, 9 unviable, 0 timeouts** en 5 min. El último
corresponde al código final con zeroización. Además se retiró literalmente la
comparación de claves: el test requerido falló `KnownAndMatches` frente a
`KnownAndChanged`, y después de restaurarla vuelve a pasar.

### Doce comprobaciones

| # | Comprobación | Resultado |
|---:|---|---|
| 1 | `cargo fmt --all --check` | PASS por código 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS por código 0 |
| 3 | `cargo test --workspace` | PASS Windows: 478 passed, 0 failed, 2 ignored ya existentes; +19 tests frente a Puerta 4 |
| 4 | Mutación con límite | Final: 95 caught, 0 missed, 9 unviable, 0 timeout; `--timeout 30`, sólo los dos módulos nuevos |
| 5 | Lectura de aserciones | Match/cambio/nuevo son tres valores; exacto/uno más, duplicado nombre/clave y tiempos válido/inválido pueden diferir |
| 6 | Lectura de contadores | `len()` se observa con 0, 2 y un store realmente construido con 4096 peers; 4097 se rechaza |
| 7 | La medida se ve fallar | No hay benchmark nuevo; la falsabilidad material es la comparación retirada, que produce el RED literal exigido |
| 8 | Lectura de nombres | Los tests nombran y ejercen cambio de clave, nuevo no trusted, positivo, versión futura, truncado, límites, duplicados, timestamps y entropía |
| 9 | Ledger legible | 18 abiertas; cero fichas nuevas en la fase y diez en todo 5D |
| 10 | Alcance desde `ebdffb9` | Sólo crate propio, ADR, ledger/checkers heredados e informes; sin Cargo, crate excluido, archivo de Claude Code ni constante prohibida |
| 11 | Coherencia del informe | Releídas secciones 0–7 y las secciones humanas del informe de mutación contra el código final y sus tres `outcomes.json` |
| 12 | `check_docs_consistency` | PASS Git Bash (13.9 s) y PASS Windows PowerShell 5.1 (47.5 s) después del último cambio de documentación pública |

Runs fallidos que se conservan: RED de compilación por imports ausentes; primer
crate completo con dos guardas estructurales mal apuntadas; primer intento de
mutación sin padre de salida; barrido 73/39/12; y RED deliberado al retirar la
comparación. El error de guardas se corrigió separando declaraciones y sitios de
construcción, no eximiendo variantes. Un intento final lanzó Cargo y los
checkers en paralelo; PowerShell enumeró un lock incremental de `target/` que
Cargo retiró antes de `Get-Content` y falló por la carrera. Se repitió después
de terminar Cargo y ambos checkers pasaron. No hubo run cancelado.

La puerta también comprobó el contrato de dependencias: `Cargo.lock` conserva
el blob exacto `307d09269e6738b06d9d59123c354d405fe1e540` de `ebdffb9` y contiene 61
secciones `[[package]]`.

## 8. Puerta 6 — historial local

Cerrada. `qyro_fs` incorpora un historial local sin crate ni
dependencia nueva: cabecera `QYRO-HST`, versión 1, reservado cero y registros
append-only de `length + 64 bytes + CRC32`. Cada registro retiene sólo tiempo,
transfer ID, huella completa del peer, dirección, estado, número de items y
bytes. La vista en memoria es `Vec<HistoryRecord>` ordenada y las consultas son
iteradores para últimos N, peer y estado.

El fixture de caída escribe realmente medio tercer registro. Al reabrir, los
dos anteriores sobreviven, `HistoryRepair::TailDiscarded` dice cuántos bytes se
retiraron y el archivo se trunca al último límite válido. Otro fixture corrompe
el CRC del segundo de tres y comprueba que se descartan segundo y tercero. Una
versión futura se rechaza por `UnsupportedHistoryVersion { found }`; no se
«repara» una cabecera que este build no comprende.

El primer intento de recuperación falló en Windows con Win32 5: el handle
abierto con append no permitía `set_len`. Se corrigió abriendo con lectura y
escritura, buscando el final sólo al append. El primer run completo del crate
falló además porque `InvalidTimestamp` se construía junto a su declaración y la
guarda no podía verlo; se movió el constructor al módulo de comportamiento sin
eximir la variante. Clippy encontró luego un `panic!` en un helper de medición y
se eliminó construyendo directamente el fixture, sin `allow`.

La medida Windows debug de 10 000 registros fue **720 012 bytes y 72.6051 ms**
de parseo, bajo presupuesto de 500 ms. `a_slow_parse_would_be_visible...`
inyecta 500 ms + 1 ns y demuestra que el detector puede fallar; el contador de
trabajo observa 10 y 20 registros en archivos de esos tamaños. El barrido final
de los dos módulos nuevos terminó 80 caught, 0 missed, 20 unviable y 0 timeout
sobre 100 mutantes.

### Doce comprobaciones

| # | Comprobación | Resultado |
|---:|---|---|
| 1 | `cargo fmt --all --check` | PASS por código 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS por código 0 |
| 3 | `cargo test --workspace` | PASS Windows: 493 passed, 0 failed, 2 ignored ya existentes; +15 tests frente a Puerta 5 |
| 4 | Mutación con límite | Final: 80 caught, 0 missed, 20 unviable, 0 timeout; `--timeout 30`, dos módulos nuevos |
| 5 | Lectura de aserciones | Mitad de registro/anteriores, CRC segundo/tail, exacto/uno más, igual/decreciente y filtros con matches distintos |
| 6 | Lectura de contadores | Tamaño deriva de bytes codificados; trabajo cuenta 10/20; `len()` observa registros realmente parseados |
| 7 | La medida se ve fallar | 500 ms + 1 ns se rechaza explícitamente contra el presupuesto de 500 ms |
| 8 | Lectura de nombres | Cada test ejerce append/reopen, futuro, mitad real, CRC real, queries, orden, timestamp, tamaño y detector que nombra |
| 9 | Ledger legible | 18 abiertas; cero fichas nuevas en la fase y diez en todo 5D |
| 10 | Alcance desde `ebdffb9` | Sólo crates propios, ADR, ledger/checkers e informes; salida prohibida vacía para Cargo, workflows, crates ajenos y excepción mínima |
| 11 | Coherencia del informe | Releídas secciones 0–8 y las secciones humanas del informe de mutación contra código, dos JSON y gates finales |
| 12 | `check_docs_consistency` | PASS Git Bash (12.4 s) y PASS Windows PowerShell 5.1 (45.1 s) |

Runs fallidos conservados: RED de compilación por siete imports ausentes;
fixture de recuperación Win32 5; guarda de `InvalidTimestamp`; Clippy por
`panic!` de fixture; primer barrido 73/17/21. No hubo run cancelado.

`Cargo.lock` conserva el blob exacto de la base
`307d09269e6738b06d9d59123c354d405fe1e540` y 61 paquetes. La corrección del
usuario recibida durante esta fase no cambió ni reinició lo ya cerrado: confirmó
que `total_len -> 0` no es P0 porque el valor real es `48 + u32 + u8`. La
enmienda P2 quedó cerrada después de Puerta 6 con un error tipado, una prueba
RED→GREEN y 5/5 mutantes focales atrapados.

## 9. Puerta 7 — barrido y guardas

Cerrada. El árbol final de 5D tiene tres superficies productivas nuevas o
endurecidas y las tres se barrieron con 30 s por mutante: 104 mutantes de peers
conocidos, 100 del historial y 5 de la guarda de progreso. El agregado es
**209 mutantes: 180 caught, 0 missed, 29 unviable y 0 timeout**. No se excluyó
ningún operador ni función dentro de esos módulos y no se creó ninguna ficha:
los `unviable` no compilan y no hay superviviente material que clasificar.

Las guardas mínimas enumeran `known_peers.rs` y `known_peer_types.rs` en
`qyro_identity_store`, `history.rs` y `history_types.rs` en `qyro_fs`; ambos
crates pasan prohibición de panic, inventario completo de archivos y sitios de
construcción para sus errores/veredictos. `qyro_protocol` elevó a 16 el mínimo
de variantes de `FrameError` y construye `DecoderNoProgress` en producción. No
se añadió `allow`, no se editó `source_guard.rs` y no se tocó
`MINIMUM_GUARD_SET_EXCEPTIONS`.

La revisión histórica de `ci: require filesystem mutation evidence` encontró
que el job de `3cbd220` no fallaba por `MISSED`: el barrido llevaba
`continue-on-error` y la puerta sólo exigía `outcomes.json` y el artefacto. Era
un recolector puntual de evidencia y `dc7725e` lo retiró después del run válido.
Si vuelve como guarda recurrente, el umbral decidido es **cero identidades
nuevas en `MISSED ∪ TIMEOUT` sobre una línea base explícita por plataforma**,
no cero supervivientes totales. Así los supervivientes históricos ya
clasificados no bloquean cada merge, pero un simple conteo tampoco permite que
un superviviente nuevo se esconda porque otro antiguo desapareció. Los
`UNVIABLE` quedan fuera del umbral; son fallos de compilación del mutante.

### Doce comprobaciones

| # | Comprobación | Resultado |
|---:|---|---|
| 1 | `cargo fmt --all --check` | PASS por código 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS por código 0 |
| 3 | `cargo test --workspace` | PASS Windows: 494 passed, 0 failed, 2 ignored ya existentes |
| 4 | Mutación con límite | 209 mutantes bajo 30 s: 180 caught, 0 missed, 29 unviable, 0 timeout |
| 5 | Lectura de aserciones | Confianza distingue tres veredictos y todos los bordes del formato; historia distingue prefijo válido/tail; progreso distingue 47/48 |
| 6 | Lectura de contadores | Store deriva 4096/4097 entradas; historia mide bytes y trabajo 10/20; progreso usa `HEADER_LEN` real |
| 7 | La medida se ve fallar | Comparación de clave retirada falla por nombre; 500 ms + 1 ns falla el presupuesto; cinco mutantes de progreso fallan |
| 8 | Lectura de nombres | Los contratos nombran clave cambiada, peer nuevo, versión futura, truncado, recuperación, filtros y error tipado que ejercen |
| 9 | Ledger legible | 18 abiertas; cero fichas nuevas en la fase y diez en todo 5D |
| 10 | Alcance desde `ebdffb9` | Salida prohibida vacía; sin Cargo, workflows, crates ajenos ni excepción mínima |
| 11 | Coherencia del informe | Releídas secciones 0–9 y ambos informes contra los tres `outcomes.json`, guardas y workflow histórico/final |
| 12 | `check_docs_consistency` | PASS Git Bash y PASS Windows PowerShell 5.1 |

Run de orquestación conservado: el wrapper del barrido final de identidad agotó
su espera local de 120 s con código 124 cuando iban 36 outcomes; no canceló el
proceso hijo. `cargo-mutants` continuó y cerró su JSON a los 277 s con los
104/104 mutantes clasificados 95/0/9/0. El barrido de historia terminó por su
comando normal en 264.6 s con 80/0/20/0. No hubo run mutacional cancelado.

## 10. Puerta 8 — documentación y CI final

Pendiente sólo de evidencia remota. `DECISIONS.md` ya incorpora ADR-0031 y
`STATUS.md` cambió lo mínimo necesario: rama/ancla, confianza, historial, las
fronteras que siguen sin UI/red y la siguiente tarea. El ledger continúa en 99
fichas/18 abiertas, diez nuevas de 5D. `Cargo.lock` conserva el hash
`307d09269e6738b06d9d59123c354d405fe1e540`, 61 paquetes y cero cambios desde
`ebdffb9`.

La validación local Windows está completa: fmt y Clippy por código 0;
`cargo test --workspace` 494/0/2; doc tests 3/0/0; `cargo audit --deny warnings`
escaneó 61 dependencias sin warning. Los cuatro `check_*` pasan tanto en Git
Bash como en Windows PowerShell 5.1: ocho invocaciones verdes. El contrato del
checker criptográfico acepta el fixture completo y rechaza por nombre cada una
de sus once omisiones/regresiones.

La primera ronda de ocho checkers falló en la sexta invocación:
`check_crypto_platform_evidence.ps1` usaba el `Join-Path` variádico que sólo
existe en PowerShell Core. Su contrato, ejecutado por primera vez bajo 5.1,
nació RED por la misma causa. Tras corregir rutas y hacer que el contrato use el
host actual, éste pasó. La segunda ronda llegó a 7/8 y encontró otras tres rutas
variádicas en `check_harness_isolation.ps1`; se corrigieron sin `allow` ni
fallback. La tercera ronda completa terminó 8/8. Ninguna de las dos rondas
fallidas se presenta como evidencia verde.

### Doce comprobaciones locales

| # | Comprobación | Resultado |
|---:|---|---|
| 1 | `cargo fmt --all --check` | PASS por código 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS por código 0 |
| 3 | `cargo test --workspace` | PASS Windows: 494 passed, 0 failed, 2 ignored ya existentes |
| 4 | Mutación con límite | Puerta 7 final sigue vigente: 209 mutantes, 180/0/29/0; Fase 8 no cambió Rust productivo |
| 5 | Lectura de aserciones | DECISIONS/STATUS concuerdan con los tres veredictos y las dos fronteras locales; el contrato PS distingue aceptación de once rechazos |
| 6 | Lectura de contadores | 99/18/10 derivan del ledger; 494/2 y 3/0 de `test result`; 61 de `Cargo.lock`/audit |
| 7 | La medida se ve fallar | Las dos rondas PS 5.1 fallidas prueban la incompatibilidad; el contrato provoca once fallos del checker |
| 8 | Lectura de nombres | Cada omisión del contrato nombra workflow, plataforma, harness, publicación o cruce FFI que retira realmente |
| 9 | Ledger legible | 18 abiertas; cero fichas nuevas en la fase y diez en todo 5D |
| 10 | Alcance desde `ebdffb9` | Salida prohibida vacía; STATUS/DECISIONS compartidos sólo en la consolidación mínima autorizada |
| 11 | Coherencia del informe | Informe 5D releído entero después de los cambios; inventario humano y resultados finales releídos contra código/JSON |
| 12 | `check_docs_consistency` | PASS Git Bash y PASS Windows PowerShell 5.1 dentro de la ronda final 8/8 |

## 11. Runs de CI de la rama

Todavía no existe ningún run de `codex/qyro-trust-5d`.

## 12. Qué no debe leerse como progreso

No hay red, sockets, descubrimiento, FFI nuevo, UI, selector ni política
interactiva de emparejamiento. Nada de este sprint se ha probado en hardware
físico. El mecanismo de confianza y el historial existen como módulos locales;
ninguno está conectado a una aplicación o a un transporte.
