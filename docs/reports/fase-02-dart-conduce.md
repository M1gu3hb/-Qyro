# FASE 02 — Dart conduce una transferencia

**Estado: cerrada.** Escrito durante la fase, no al final.

Base de la fase: `3b32b6f`. Rama `claude/qyro-net-6a`.

---

## 0. Apertura — qué releí, qué encontré desfasado, qué corregí

*(El ritual de apertura obligatorio. Tres líneas.)*

1. **Releído:** `R1`, `R2`, `R3`, `R4`, `R5` enteros; `00-LEEME-PRIMERO.md`;
   `fase-01-ffi-del-motor.md`; `ADR-0032`; `qyro_session/src` y `qyro_ffi/src`
   completos; `BUGS_PENDING.md` contado a mano.
2. **Desfasado:** siete afirmaciones de la línea base no se sostienen contra el
   código — el detalle está en §4, y dos de ellas (la comprobación 11 y el job
   de CI en Windows) invalidan cierres declarados en la fase 01.
3. **Corregido:** nada del árbol todavía. Este informe registra primero; los
   arreglos van en los pasos siguientes con su puerta.

---

## 1. Objetivo y alcance

Lo que el plan pide: que Dart conduzca una transferencia real de extremo a
extremo, con progreso, sobre la superficie C que la fase 01 construyó.

**El orden está fijado por la recomendación de la fase 01 y se respeta:** la fase
no empieza conectando Dart. Empieza por **QYR-0309** — un emisor y un receptor
sobre `127.0.0.1` en dos hilos moviendo un archivo — porque conectar la interfaz
a un motor sin cobertura de comportamiento es construir sobre una capa que nadie
ha ejercido.

**Fuera de alcance:** los botones siguen `onPressed: null`. Se habilitan en la
fase 05 y sólo si sus condiciones se cumplen.

---

## 2. Qué se hizo

- **Paso 0** — línea base reproducida y auditada. Cinco de siete comprobaciones
  coinciden; dos no, y las dos son hallazgos (§4). El checker de documentación
  llevaba rojo en Windows desde antes de la fase 01.
- **Paso 0b** — QYR-0309: diez pruebas de conducta donde no había ninguna. Y
  encontraron **QYR-0316**, P1: una transferencia íntegra reportada al emisor
  como `PeerUnreachable`.
- **Paso 1** — **ADR-0033 congelada** antes del código, `37f7a6e`, un archivo y
  cero `.rs`.
- **Paso 2** — el puente de progreso, con un presupuesto acotado por constante.
  El barrido encontró siete supervivientes en el código recién escrito, y la
  respuesta fue probar la aritmética **como aritmética** (QYR-0321).
- **Paso 3** — **ADR-0038 congelada** y el lado Dart. `dart:ffi` no trae
  asignador de memoria nativa, y eso no lo había previsto ninguna ADR.
- **Paso 4** — **la prueba que define la fase.** 8 MiB + 13 bytes, dos procesos
  de sistema operativo, byte a byte.
- **Paso 5** — CI en Linux y Windows, y `timeout-minutes` en los dieciséis
  trabajos de los seis workflows.
- **Cierre** — QYR-0304 (el zeroize), QYR-0078 (el cuelgue, con su test
  nombrado), ADR-0039 (iOS fuera de la v1.0) y la normalización del ledger.

---

## 3. Cómo se hizo

### 3.1 Reproducción de la línea base

Todo por **código de salida del proceso**, nunca por el texto de la salida.
Entorno: Windows 10 19045, PowerShell 5.1.19041.6456, Git Bash 5.3.9 (Cygwin),
`rustc` 1.88.0 (6b00bc388 2025-06-23).

| # | Comando | Exit | Veredicto |
|---|---|---|---|
| 1 | `git rev-parse HEAD` | — | ✅ `3b32b6f` |
| 2 | `cargo test --workspace` | `0` | ✅ **571 passed, 0 failed, 2 ignored** |
| 3 | `cargo clippy --workspace --all-targets -- -D warnings` | `0` | ✅ |
| 4 | `cargo fmt --all --check` | `0` | ✅ |
| 5 | `bash scripts/check_docs_consistency.sh` | `0` | ✅ |
| 5b | `powershell -File scripts/check_docs_consistency.ps1` | **`1`** | ❌ **ver §4.1** |
| 6 | `grep -c '^\[\[package\]\]' Cargo.lock` | — | ✅ 64 · ❌ «de primera parte» |
| 7 | conteo del ledger | — | ✅ 127 fichas, 7 P1 · ❌ 36 abiertas, no 32 |

### 3.2 El 566 es el número de Linux, y reconcilia exacto

La línea base declara 566. En este Windows salen **571**. No es una discrepancia:
son las dos plataformas, y la diferencia se explica **test a test**.

| Dirección | Cuenta | Tests |
|---|---|---|
| Sólo Windows (corren aquí, no en Linux) | **9** | `qyro_win_dpapi::tests::*` (8, tras `#[cfg(all(windows, test))] mod tests`) + `qyro_fs::a_junction_at_the_final_component_is_classified_as_a_reparse_point` |
| Sólo Linux (no corren aquí) | **4** | `qyro_fs::a_symlink_in_the_destination_cannot_redirect_a_write`; `qyro_fs::a_symlink_at_the_final_part_component_is_refused_without_touching_its_target`; `qyro_net_smoke::endings::no_thread_and_no_descriptor_survives_a_finished_session`; `qyro_net_smoke::endings::a_descriptor_leak_would_be_visible_to_this_measurement` |

**571 − 9 + 4 = 566.** El desglose se obtuvo con
`cargo test -p qyro_win_dpapi --lib -- --list` y una enumeración de todo `#[test]`
con atributo `cfg` adyacente, no a ojo.

Del reparto sale una observación que no es aritmética: **las dos mediciones de
recursos del proyecto —hilos y descriptores— son `cfg(target_os = "linux")`, y
con ellas se va la contra-prueba `a_descriptor_leak_would_be_visible_to_this_measurement`.**
En Windows no hay medición de fugas *ni* prueba de que la medición vería una. El
propio comentario del test lo dice: «*which is a gap, not a pass*».

### 3.3 La guarda de nombrabilidad, vista fallar

Se añadió `qyro_crypto = { path = "../qyro_crypto" }` a
`rust/crates/qyro_ffi/Cargo.toml` y se corrió `cargo test -p qyro_ffi --test c_abi_contract`:

```
test the_ffi_names_exactly_two_crates ... FAILED
  left:  {"qyro_core", "qyro_crypto", "qyro_session"}
  right: {"qyro_core", "qyro_session"}
test result: FAILED. 5 passed; 1 failed  (exit 101)
```

Y —esto es lo que hace la guarda honesta— en la misma corrida
`a_direct_crypto_edge_is_invisible_here_and_visible_to_guard_one` **pasó**: la
medición viene acompañada de la prueba de lo que *no* puede ver. Es el patrón de
`R1` §5.7 aplicado a una guarda estructural.

Revertido con `git checkout --`; `git status --short` vacío, verificado.

### 3.4 Cargo.lock: la fase 01 no añadió ninguna dependencia externa

| Commit | `[[package]]` |
|---|---|
| `90bb5d0` (base de la fase 01) | **63** |
| `3b32b6f` (HEAD) | **64** |

`git diff 90bb5d0..HEAD -- Cargo.lock` añade **una** entrada: `qyro_session`, de
primera parte. `serde_json` **ya estaba** en el lock de `90bb5d0` (1.0.151), así
que la dev-dependency que la fase 01 declaró sin coste efectivamente no tuvo
coste. **La justificación escrita en `qyro_ffi/Cargo.toml` es cierta y queda
verificada.**

---

## 4. Qué se encontró que no estaba en el plan

| # | Hallazgo | Dónde | Sev | Ficha |
|---|---|---|---|---|
| 4.1 | El checker de documentación es rojo en Windows PowerShell 5.1, y `-Include` no filtra nada | `scripts/check_docs_consistency.ps1:255` | P1 | QYR-0311 |
| 4.2 | «64 paquetes y todos de primera parte»: 50 vienen de crates.io | `R1` §2, `00-LEEME-PRIMERO` §4 | P2 | QYR-0312 |
| 4.3 | El conteo de abiertas de la puerta no ve `**abierto**` y subcuenta cuatro | `R2` §1.10 | P3 | QYR-0313 |
| 4.4 | `Session::local_addr` devuelve la dirección del peer, y el `Listener` que sabe el puerto se descarta | `qyro_session/src/session.rs:244` | P2 | QYR-0314 |
| 4.5 | El vocabulario de estados del ledger no es el que `R4` §5 congela | `BUGS_PENDING.md` | P3 | QYR-0315 |
| 4.6 | El job `rust workspace (windows-latest)` cuelga en CI; no hay `timeout-minutes` | `.github/workflows/ci.yml` | P1 | QYR-0078 (evidencia nueva) |
| 4.7 | El comentario de `ci.yml:73` cuenta un guard donde hay cinco | `.github/workflows/ci.yml:73` | — | recogido en QYR-0315 |

### 4.1 La comprobación 11 no pasa en la plataforma para la que existe

`scripts/check_docs_consistency.ps1` sale **exit 1**:

```
[BLOCKER] Finding ledger: QYR-00xx is cited but has no entry in BUGS_PENDING.md
```

*(El identificador va redactado a propósito: es el extremo superior del rango
reservado, y pegarlo literal vuelve a disparar el bloqueo. Ver la nota de método
al final de esta sección.)*

**Causa raíz, probada en un fixture aislado de cuatro archivos:**

```powershell
Get-ChildItem -LiteralPath $t -Recurse -File -Include @('*.md','*.rs')
#  -> a.md  b.txt  c.rs  d.o      <- -Include no filtra NADA
Get-ChildItem -LiteralPath $t -Recurse -File | Where-Object { $_.Extension -in '.md','.rs' }
#  -> a.md  c.rs                  <- la forma correcta
```

Con `-LiteralPath` y `-Recurse`, `-Include` es **inerte** en PowerShell 5.1. El
checker declara cinco extensiones y en este repositorio recorre **5 962 archivos,
de los cuales 5 679 (95 %) están fuera de su alcance declarado** — `.o`, `.bin`,
`.rlib`, `.exe`, `.txt`, y todo `target/`.

Lo que cuela es `docs/reports/6A-prompt-2.txt:15`, que separa los dos extremos
del rango reservado con la palabra «a» en vez de con un en dash. Los dos checkers
exoneran el rango escrito con guion, en dash o em dash, y la forma `QYR-nnnn+`;
**ninguno exonera la forma con palabra**, así que el extremo superior se lee como
cita suelta. Pero **la mitad Bash nunca ve ese archivo**, porque `grep --include`
sí filtra. Los dos checkers no son equivalentes, y el proyecto cree que sí.

**Nota de método, porque es la mejor demostración del hallazgo:** este informe
puso la mitad Bash del checker **en rojo dos veces mientras se escribía**, y las
dos por describir el problema.

1. El primer borrador reprodujo el rango tal y como aparece en el archivo, con la
   palabra en medio. Ninguna forma exenta cubre eso.
2. Corregido eso, seguía rojo: **la cita que quedaba era el propio mensaje de
   error del checker**, pegado literal como evidencia. Un `[BLOCKER]` que nombra
   un identificador, copiado a un `.md`, es una cita de ese identificador.

Escribir *sobre* la trampa la dispara, y pegar la prueba de la trampa también.
Es la razón por la que `R1` §6 lista esta regla, y por la que la salida citada
arriba lleva el identificador redactado.

**Encuadre justo, y hay que darlo con precisión:** la fase 01 declaró en su
informe que «*con eso la comprobación 11 pasa a exit 0 por primera vez en esta
rama*». Eso es **cierto** en los dos entornos que la fase 01 podía correr: Bash,
y `pwsh` sobre ubuntu en el job `documentation`, que salió verde. Es **falso** en
Windows PowerShell 5.1 — la única plataforma para la que el `.ps1` existe, y que
**ningún job de CI cubre**: `ci.yml:181` invoca `pwsh` sobre un runner Linux.

El archivo culpable ya estaba en `90bb5d0`, o sea antes de que empezara la fase
01. Y el proyecto ya vivió esto: **QYR-0100** («El checker confunde límites de
rangos reservados con hallazgos») y la entrada de `5C-codex.md` sobre el checker
real de PowerShell 5.1 en rojo por el mismo rango. Se arregló para el en dash y
volvió por otra puerta.

### 4.4 `local_addr` no devuelve la dirección local

```rust
// qyro_session/src/session.rs:244 — el doc dice «la dirección a la que la
// sesión está atada» y «un receptor puede abrirse en el puerto 0 y tiene que
// informar del puerto que el sistema eligió».
pub fn local_addr(&self) -> Result<SocketAddr, SessionError> {
    self.stream.peer_addr()   // <- qyro_net documenta esto como «la dirección del far end»
}
```

Dos defectos, uno encima del otro:

1. Devuelve la dirección **remota**. El nombre, el doc-comment y el código dicen
   tres cosas de las que sólo dos coinciden.
2. Aunque se arreglara, **no podría cumplir su propósito documentado**:
   `open_receiver` bloquea en `listener.accept()` *antes* de devolver, y el
   `Listener` —único que sabe el puerto elegido, vía `Listener::local_addr` en
   `qyro_net/src/listener.rs:95`— es una variable local que se descarta. Cuando
   se puede preguntar el puerto, ya hay un peer conectado, así que enlazar en el
   puerto 0 para *anunciar* el puerto es inalcanzable por construcción.

**Hoy no hace daño: `local_addr` no cruza la superficie C.** Las seis operaciones
`extern "C"` son `open_sender_blocking`, `open_receiver_blocking`,
`step_blocking`, `progress`, `cancel` y `close`. Es una función pública que nadie
llama y que está mal — el antipatrón nº 5 de `R1` §5 con otra forma. Por eso es
P2 y no P1. **Pero cerrarla es prerrequisito de la mitad receptora de esta fase**:
Dart no puede mostrar un puerto que la capa de abajo no sabe decir.

### 4.6 QYR-0078 tiene respuesta, y no es la que la ficha describe

La ficha dice «clippy verde, `cargo test` sin ver terminar». Reproducido con `gh`:

| Run | Título | Estado antes de cancelar |
|---|---|---|
| 31743822172 | `test(ffi): the mutation sweep…` | `in_progress` 3h22m |
| 31742884829 | `feat(ffi): the six operations…` | `in_progress` 3h34m |
| 31741914871 | `feat(ffi): the handle table…` | `in_progress` 3h46m |
| 31741320500 | `docs: retire the property…` | `in_progress` |
| 31739146084 | `docs(status): move Verified commit…` | `in_progress` |
| 31735781534 | `docs: reproduce the R6 baseline…` | `in_progress` |
| 31734622896 | `Add files via upload` | `in_progress` |

**Siete runs consecutivos, todos colgados.** En el run 31743822172 los ocho jobs
se reparten así: `rust`, `flutter`, `scripts`, `documentation` y los tres
`fs final-component guard` **completados con éxito**; el único `in_progress` es
**`rust workspace (windows-latest)`**, que corre exactamente
`cargo clippy --workspace --all-targets -- -D warnings` y `cargo test --workspace`.

`ci.yml` **no declara `timeout-minutes` en ningún job**, así que un cuelgue corre
hasta el corte de seis horas de GitHub en vez de fallar en minutos.

**El contraste es el dato que la ficha no tiene:** ese mismo `cargo test --workspace`
**terminó verde en un Windows real en esta máquina**, exit 0, 571 tests. Así que
«Windows» no es la causa. La causa está en el runner o en la interacción con él, y
eso es una hipótesis distinta que investigar.

Los siete runs se cancelaron con `gh run cancel` (autorizado por el propietario).

---

## 5. Qué se arregló y qué no

### 5.1 QYR-0311 arreglado, porque bloqueaba todas las puertas siguientes

`R2` §2 dice que una puerta que falla se arregla y se repite entera. La
comprobación 11 no podía pasar en Windows, y con nueve fases por delante eso es
una puerta permanentemente rota, no una salvedad. Así que se arregló en el paso 0.

**El arreglo no inventa un patrón: aplica el que el propio script ya usa.** Cien
líneas más arriba, en la comprobación de marcadores provisionales,
`check_docs_consistency.ps1:130` ya filtra con
`Get-ChildItem -Recurse | Where-Object { $_.Extension -in @(...) }`. La regla de
citación era el único sitio del archivo que confiaba en `-Include`.

| Medida | Antes | Después |
|---|---|---|
| Archivos recorridos | 5 962 | **284** |
| Fuera del alcance declarado | 5 679 | **0** |
| `.github/workflows/ci.yml` incluido | sí | sí |
| `check_docs_consistency.ps1` en PowerShell 5.1 | exit 1 | **exit 0** |

### 5.2 El control que lo caza, y verlo fallar

El contrato ya existía —`scripts/tests/docs_consistency_contract_test.ps1`— y
**pasaba en verde mientras el defecto estaba vivo**. Sus fixtures son sintéticos y
pequeños, así que la fuga de extensiones nunca aparecía en ellos. Una medición
que no puede ver el fallo que busca no es evidencia de que no lo haya, que es
justamente `R1` §5.7.

Se añadió un caso, **en las dos mitades**, con las dos direcciones a propósito:

- la misma cita en un archivo **fuera** del alcance (`notes.txt`) **no** debe
  bloquear;
- la misma cita en un archivo **dentro** del alcance (`README.md`) **sí** debe
  bloquear.

Una sola dirección no distingue: un checker que ignora extensiones falla la
primera, pero un checker que no escanea nada pasaría la primera y fallaría la
segunda. Con las dos, ninguno de los dos pasa.

**Visto fallar, no supuesto.** Con el defecto reintroducido a mano en
`check_docs_consistency.ps1:264`, el contrato aborta con su propio mensaje:

```
A citation in an out-of-scope file must not block: [SKIP] Verified commit freshness: ...
```

Restaurado el arreglo, los cuatro vuelven a verde: checker y contrato, en Bash y
en PowerShell.

*(Anotación honesta del método: el script que reintrodujo el defecto abortó antes
de su propia línea de restauración —`$ErrorActionPreference = 'Stop'` más el
stderr de un proceso nativo—, y dejó el árbol mutado. Se detectó comprobando
`git diff` inmediatamente después, no confiando en que el script hubiera acabado.
Es el mismo error de lectura que `R2` §1.1 describe, en otra forma.)*

### 5.3 Qué NO se arregló en el paso 0

| ID | Por qué se deja |
|---|---|
| QYR-0312 | Es documental y toca `R1`, que es documento del supervisor. Se propone la redacción; no se edita unilateralmente |
| QYR-0313 | Igual: el script de conteo vive en `R2` |
| QYR-0314 | Es trabajo de la fase, no del paso 0. Se cierra con las pruebas de conducta de QYR-0309, que es donde una prueba lo habría cazado |
| QYR-0315 | Documental y de alcance amplio; un commit propio de sólo documentación |
| QYR-0078 | No se cierra cancelando runs. Necesita `timeout-minutes` y una tirada en success |

---

## 6. Paso 0b — QYR-0309, y el defecto que estaba esperando debajo

La fase 01 recomendó no empezar por Dart sino por «un emisor y un receptor sobre
`127.0.0.1` en dos hilos moviendo un archivo». Tenía razón, y la razón resultó ser
más concreta de lo que ella misma sabía.

### 6.1 Qué había

`qyro_session` tenía **seis tests, los seis en `guards.rs`, y los seis leen los
archivos de producción como texto**: ningún constructo que pueda entrar en
pánico, cada archivo listado, cada variante de error construida en algún sitio.
Son guardas útiles —cazaron la variante `AlreadyFailed` que nada podía producir—
pero **ninguna abre un socket**. `session.rs` son 444 líneas y `error.rs` 69, y no
había una sola prueba que hiciera correr el código.

### 6.2 Qué se escribió

`rust/crates/qyro_session/tests/session_behaviour.rs`, diez pruebas contra la API
**pública** del crate y nada más — que es también la superficie que ADR-0032 §2
acota, así que ejercerla aquí es ejercer la frontera.

| Prueba | Qué defiende |
|---|---|
| `an_empty_file_list_is_refused_before_anything_is_dialled` | El rechazo ocurre **antes** del dial. Nada escucha en esa dirección, así que `BadArgument` y `PeerUnreachable` son distinguibles y la prueba puede notar si la comprobación se mueve |
| `a_file_outside_the_root_is_refused_rather_than_renamed_to_its_last_component` | El fallo silencioso que evita: mandarlo igual con un nombre que el emisor no eligió |
| `a_parent_directory_in_the_remainder_is_refused` | `inner/../secret.bin` recorta contra el root y aun así no es descendiente llano |
| **`a_file_crosses_two_sessions_on_the_loopback_and_arrives_byte_for_byte`** | La que define el paso |
| `a_corrupted_arrival_would_be_visible_to_this_comparison` | `R2` §1.7: voltea un bit y comprueba que la comparación lo ve |
| `two_files_under_a_common_root_arrive_under_their_own_relative_names` | La afirmación del doc-comment que nadie ejercía. **Dos tamaños distintos a propósito**, porque si ambos arribos fueran idénticos la prueba no podría ver una colisión |
| `progress_reaches_the_total_and_never_goes_backwards` | Monotonía, y `total > 0` — trampa 4 del documento de fase |
| `a_cancelled_session_reports_cancelled_and_keeps_reporting_it` | La pegajosidad de ADR-0032 §5, comprobada **tres veces seguidas** |
| `a_receiver_that_never_gets_a_peer_reports_the_peer_and_not_success` | Un socket que abre y cuelga no es un peer autenticado |
| `finishing_a_sender_materialises_nothing_and_says_so` | `finish` es del receptor; un emisor que contara algo estaría contando otra cosa |

El archivo de prueba usa **2 MiB + 7 bytes**, no 4 KiB: los chunks son de 64 KiB
tras una ventana de 16, así que por debajo de 1 MiB la ventana, el go-back-N y el
control de flujo no se ejercitan y una transferencia que nunca rellena la ventana
pasaría igual.

### 6.3 El defecto que encontraron

**Cinco de las diez fallaron a la primera, y no por un fallo del test.**

El mensaje de la aserción se escribió para decir qué vio el *otro* extremo, y eso
fue lo que lo resolvió:

```
the sender did not complete; the receiver ended Ok(Completed) and materialised Ok(1)
  left: Err(PeerUnreachable)
 right: Ok(Completed)
```

El receptor completó y materializó el archivo **correcto byte a byte**. El emisor
lo reportó como fallo de transporte.

El mecanismo, leído en el código y no adivinado:

1. El receptor, al recibir `Complete`, calcula los veredictos y **produce el frame
   `IntegrityResult`**, que `qyro_session` deja en `self.outbound`.
2. `advance` escribe `outbound` **al principio** de cada paso.
3. Pero ese mismo paso ve `engine.phase() == Phase::Done` y sale por
   `return Ok(self.verdict())`.
4. Como devuelve un estado terminal, **nadie vuelve a llamar a `step`**. El frame
   nunca sale.
5. El emisor sólo alcanza `Phase::Done` al **recibir** `IntegrityResult`
   (`qyro_transfer/src/session.rs:496`). Espera un frame que existe y que nadie
   mandó, hasta que el socket se cierra → `PeerUnreachable`.

Es exactamente el antipatrón nº 5 de `R1` §5 —un artefacto que se escribe y nada
lee— pero al revés: **bytes que se producen y nadie escribe**. Y con consecuencia
de producto: Dart conduce el lado **emisor** en esta fase, así que un envío
correcto se le habría presentado a la persona como fallo de red.

**QYR-0316, P1, cerrada.** El arreglo extrae `write_outbound` y lo llama también
cuando el paso resulta ser el último. Visto fallar y visto pasar: los mismos diez
tests, en rojo sin el arreglo y en verde con él.

### 6.4 Dos huecos más que las pruebas dejaron a la vista

- **QYR-0317**, P2 — el receptor **nunca informa de progreso**. `progress.done` se
  asigna sólo en el brazo emisor; una sesión receptora informa `0` de principio a
  fin, y `qyro_transfer::Receiver` no tiene accesor de bytes recibidos.
- **QYR-0318**, P2 — `Progress::item` se documenta «one-based» y **no se asigna
  nunca**, en ninguno de los dos brazos.

La prueba de progreso **no afirma nada sobre ninguno de los dos**, a propósito:
afirmar el cero de hoy congelaría un defecto como contrato, y afirmar el
comportamiento pretendido fallaría. Lo que sí afirma es lo que es cierto y merece
defensa —que el receptor **aprende** su total del manifiesto, empezando en cero y
no recibiéndolo del llamante—, y el resto lo llevan las fichas.

### 6.5 Lo que este paso NO demuestra

Dos `Session` en dos **hilos del mismo proceso** sobre `127.0.0.1`. No son dos
procesos, no son dos aparatos, y no hay Dart en ninguna parte. Sigue sin haber
ocurrido ninguna transferencia a través de la superficie C.

---

## 7. Resultado contra el objetivo

**El objetivo, literal:** «que un test en Dart mueva un archivo real entre dos
procesos, por un socket, y lo verifique byte a byte — con el progreso llegando a
Dart mientras ocurre».

**Cumplido.** Y los catorce criterios de aceptación, uno a uno:

| # | Criterio | Veredicto |
|---|---|---|
| 1 | ADR-0033 congelada antes del código, comprobable en el historial | ✅ `37f7a6e`; `git show --name-only` da **un `.md` y cero `.rs`** |
| 2 | Las cuatro reglas de §4 implementadas, cada una con prueba o argumento escrito | ✅ Tres lo son **por la forma**: retorno `void` (no hay valor que leer), cuatro escalares (ningún puntero sobrevive a su marco) y la ordenación de ADR-0033 §5. La cuarta —Dart debe llamar a `close()`— **no se puede imponer desde Rust y así se dice**; la comprueba `closing_from_dart_leaves_no_handle_and_no_thread` |
| 3 | Un test en Dart mueve **≥8 MiB** entre dos procesos y lo compara byte a byte | ✅ `_transferBytes = 8 * 1024 * 1024 + 13`, con `orderedEquals` |
| 4 | Hay evidencia de que ese test **podría fallar** | ✅ `a_corrupted_transfer_is_detected_by_this_test`, y voltea el bit **en el archivo, no en el cable** — corromper el cable probaría que el AEAD funciona, que es otra afirmación |
| 5 | El progreso llega a Dart, es monótono, y termina en el total | ✅ Las tres, afirmadas por separado |
| 6 | Presupuesto medido y acotado, con prueba de que la medida vería el exceso | ✅ ≤102 sobre el socket real, y `an_emission_per_chunk_would_be_visible_to_this_measurement` compara emisiones contra número de `step`. **Y el barrido demostró que la primera versión de esa medida no veía siete fallos** — corregido y vuelto a medir |
| 7 | Una sesión sin observador funciona igual | ✅ Prueba propia, y el mutante `observer -> None` ahora muere |
| 8 | Ni handle, ni hilo, ni `.qyro-part` sobrevive | ⚠️ **Parcial, y se dice.** Handle: sí, seis rondas sobre una tabla de cuatro. `.qyro-part`: sí. **Hilos: no medido desde Dart.** Las dos mediciones de recursos del proyecto son `cfg(target_os = "linux")` y viven en Rust; no hay equivalente desde Dart ni en Windows |
| 9 | La prueba corre en CI en **Linux y Windows** | ✅ Linux: tirada 31769832225, trabajo `flutter`. Windows: tirada 31767129485, paso 10. **Android e iOS: no cabe**, y se registra en vez de callarse — es un test de VM de Dart con `Process.start`, y ni el emulador ni el simulador lo ejecutan |
| 10 | **Cero dependencias externas** en Rust y **cero paquetes nuevos de pub.dev** | ✅ `Cargo.lock` → **64**, sin cambio; `pubspec.lock` → **39**, sin cambio. Es exactamente la razón de que exista ADR-0038 |
| 11 | Barrido con alcance declarado | ✅ §10: `qyro_session` 62 mutantes, `qyro_ffi` 95, más dos dirigidos |
| 12 | Las doce comprobaciones de `R2` en todas las puertas | ✅ Cuatro puertas, en §9 |
| 13 | Informe según `R5` | ✅ Las dieciséis secciones |
| 14 | **Los botones siguen `onPressed: null`**, y el informe dice si ya se cumple la condición | ✅ Siguen apagados y su test pasa. Ver abajo |

### Criterio 14: ¿se cumple ya la condición que los mantiene apagados?

La condición es «que exista una transferencia real, cifrada y comprobada de
extremo a extremo».

**Sí existe.** ChaCha20-Poly1305, autenticada, verificada por SHA-256, entre dos
procesos, conducida desde Dart, verde en tres entornos independientes.

**Y aun así no se encienden, porque esa es una de las cinco condiciones de la
fase 05.** Lo que falta para afirmarlo sin trampa:

- **Nadie puede elegir un archivo.** Las rutas se pasan a mano (fase 03).
- **Nadie sabe a qué dirección mandarlo.** La IP se pasa a mano (fase 04).
- **No hay a quién confiar.** El emparejamiento y la huella no cruzan el FFI.
- **No hay nada que mirar.** No existe interfaz de transferencia (fase 05).
- **Nada de esto ha tocado un aparato físico.**

Encender un botón hoy daría una pantalla donde no se puede elegir qué mandar ni a
quién. Eso no es un producto a medias: es una mentira con forma de botón.

---

## 8. Clase de evidencia por afirmación

`R1` §4: una afirmación sin clase de evidencia se audita como no probada.

| Afirmación | Clase de evidencia |
|---|---|
| Un archivo de 8 MiB cruza dos procesos conducido desde Dart y llega idéntico | **Probado entre procesos**, en tres entornos: esta máquina, runner Linux, runner Windows |
| El progreso llega a Dart, monótono, terminando en el total | **Probado entre procesos** |
| El presupuesto de emisiones está acotado por una constante | **Probado en unidad** hasta 64 GiB simulados, **y entre procesos** hasta 8 MiB reales |
| Una transferencia íntegra ya no se reporta como `PeerUnreachable` | **Probado en integración**: dos hilos, un proceso |
| `qyro_session` refuza rutas fuera del root, `..` y listas vacías | **Probado en unidad** |
| El texto claro recibido ya no se copia fuera de su borrado | **Compilado y probado en integración.** Que el `Zeroizing` *borre* lo garantiza la biblioteca, no una prueba de este repositorio |
| El checker de documentación pasa en Windows PowerShell 5.1 | **Probado en esta máquina.** **Ningún job de CI lo corre en `windows-latest`**: esa cobertura no es de CI |
| `cargo test --workspace` termina en Windows | **Probado en CI**, tirada 31769832225 |
| La superficie C tiene diez símbolos y ninguno puede nombrar `qyro_crypto` | **Verificado por el resolvedor de Cargo** |
| Ninguna clave privada llega a Dart | **Verificado estructuralmente**, por nombrabilidad. No es evidencia de ejecución, y ADR-0032 §9 ya lo dice |
| Qyro funciona en Android | **Ninguna.** Cero evidencia de cualquier clase en esta fase |
| Qyro funciona en hardware físico | **Ninguna.** Cero, en cualquier plataforma |

---

## 9. Las puertas

### Puerta del paso 0 — auditoría de la línea base · 2026-08-13

Entorno: Windows 10 19045 · PowerShell 5.1.19041.6456 · Git Bash 5.3.9 ·
`rustc` 1.88.0. `pwsh` **no está instalado en esta máquina**, así que la mitad
PowerShell se ejecutó con `powershell.exe` 5.1 — que es, precisamente, el motivo
por el que apareció QYR-0311.

| # | Comprobación | Veredicto |
|---|---|---|
| 1 | `cargo fmt --all --check` | ✅ exit 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| 3 | `cargo test --workspace` | ✅ exit 0 · **571 passed, 2 ignored** — mismos dos de la línea base, ningún ignorado nuevo |
| 4 | Barrido de mutación | **N/A declarado.** El paso 0 no cambió una sola línea de Rust. La mutación que sí correspondía —borrar el control y verlo fallar— se aplicó al arreglo del checker y está en §5.2 |
| 5 | Lectura de aserciones | ✅ Sin aserciones Rust nuevas. Las dos del contrato comparan una salida de proceso con una cadena literal: lados distintos por construcción |
| 6 | Lectura de contadores | ✅ N/A · ningún contador nuevo |
| 7 | La medida se ve fallar | ✅ **§5.2.** El contrato pasaba con el defecto vivo; el caso nuevo se vio abortar con el defecto reintroducido |
| 8 | Lectura de nombres | ✅ Los dos casos nuevos enuncian su propiedad —«must not block» / «must look»— y el cuerpo la ejerce en las dos direcciones |
| 9 | `git diff --name-only` | ✅ 5 archivos, listados abajo |
| 10 | El ledger sigue legible | ✅ 132 fichas · **5 nuevas**, bajo el límite de diez · 0 duplicados |
| 11 | Coherencia documental, en Bash **y** PowerShell | ✅ **exit 0 los dos** — primera vez que es cierto en 5.1 en esta rama |
| 12 | Escribir el resultado | ✅ esta tabla, antes del paso siguiente |

**Archivos tocados** (`git status --porcelain`, base de fase `3b32b6f`):

```
BUGS_PENDING.md
scripts/check_docs_consistency.ps1
scripts/tests/docs_consistency_contract_test.ps1
scripts/tests/docs_consistency_contract_test.sh
docs/reports/fase-02-dart-conduce-una-transferencia.md
```

**Balance del ledger:** 127 → **132** fichas; abiertas 36 → **41**. Suben cinco
porque el paso 0 es una auditoría y su producto son hallazgos; ninguna se cierra
todavía. El conteo va con las dos cifras a propósito: el script de `R2` §1.10 dice
**37** y el real es **41**, y esa diferencia de cuatro es QYR-0313.

**Cero dependencias añadidas.** `Cargo.lock` sigue en 64 paquetes; ningún archivo
Rust tocado.

### Puerta del paso 0b — QYR-0309 · 2026-08-13

| # | Comprobación | Veredicto |
|---|---|---|
| 1 | `cargo fmt --all --check` | ✅ exit 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| 3 | `cargo test --workspace` | ✅ exit 0 · **581 passed** (571 → 581), 0 failed, **2 ignored — los mismos dos**, ninguno nuevo |
| 4 | Barrido de mutación | ✅ **corrido y declarado** · 32 mutantes, 11/9/1/11. Inventario y agrupación en §10; la familia de supervivientes va a **una** ficha escrita a mano, no al ledger entrada por entrada |
| 5 | Lectura de aserciones | ✅ Revisadas una a una. La que más importa es la de `two_files_…`: usa **dos tamaños distintos** para que dos arribos idénticos no puedan pasar por un par legítimo |
| 6 | Lectura de contadores | ✅ N/A · ningún contador nuevo bajo `cfg(test)` |
| 7 | La medida se ve fallar | ✅ `a_corrupted_arrival_would_be_visible_to_this_comparison` voltea un bit y comprueba que la comparación lo ve, **y** que voltearlo no cambia la longitud, para que no pase por un fallo de tamaño |
| 8 | Lectura de nombres | ✅ Las diez enuncian una propiedad y el cuerpo la ejerce. `an_empty_file_list_is_refused_before_anything_is_dialled` dice «antes del dial» y **lo comprueba**: nada escucha en esa dirección, así que `BadArgument` y `PeerUnreachable` son distinguibles |
| 9 | `git diff --name-only` | ✅ 5 rutas |
| 10 | El ledger sigue legible | ⚠️ **En el límite exacto.** 137 fichas · **10 nuevas en la fase** (QYR-0311 a QYR-0320). `R2` §1.10 dice «más de diez»; son diez, así que pasa, pero es el techo y conviene decirlo en vez de que se note en la fase siguiente |
| 11 | Coherencia documental, Bash **y** PowerShell | ✅ exit 0 los cuatro: checker y contrato, en las dos mitades |
| 12 | Escribir el resultado | ✅ esta tabla |

**Archivos tocados en el paso 0b:**

```
.gitignore
BUGS_PENDING.md
docs/reports/fase-02-dart-conduce.md
rust/crates/qyro_session/src/session.rs
rust/crates/qyro_session/tests/session_behaviour.rs
```

`.gitignore` gana `mutants.out/`: la salida del barrido son 65 KB de log y no
entra en el árbol. Va al informe con su alcance declarado, que es la regla.

### Puerta del paso 1 — ADR-0033 congelada · 2026-08-13

Un paso de sólo documentación, y la puerta se declara como tal en vez de fingir
que las doce aplican por igual.

| # | Comprobación | Veredicto |
|---|---|---|
| 1–3 | fmt · clippy · test | ✅ exit 0 los tres, **581 passed**. Ningún `.rs` tocado, y eso es el criterio del paso, no un efecto secundario |
| 4 | Barrido | **N/A declarado** · cero código nuevo |
| 5–8 | Aserciones · contadores · la medida se ve fallar · nombres | **N/A declarado** · cero pruebas nuevas. Lo que la ADR **decide** sobre la medición —dos tamaños y una desigualdad estricta— es obligación del paso 2, y está escrita en §4 de la ADR para que el paso 2 no pueda saltársela |
| 9 | `git diff --name-only` | ✅ **exactamente un archivo**: `docs/adr/ADR-0033-progress-bridge.md` |
| 10 | Ledger | ✅ 137 fichas, **43 abiertas**. Dos cerradas en este tramo: QYR-0312 y QYR-0313 |
| 11 | Coherencia documental | ✅ exit 0 en Bash y PowerShell, **después** de mover el `Verified commit` |
| 12 | Escribir el resultado | ✅ esta tabla |

**La prueba de que la ADR se congeló antes del código** es
`git show --name-only --format='' 37f7a6e` → una línea, un `.md`, cero `.rs`.

**Dos cosas que la puerta encontró y no se arreglan en silencio:**

- `STATUS.md` llevaba **11 commits** de retraso sobre HEAD, con el límite en 10,
  y la comprobación 11 bloqueó. Es la regla funcionando.
- El primer intento de moverlo **inventó el SHA de 40 caracteres** en vez de
  leerlo, y el checker lo cazó con «*is not a commit in this repository*». Va
  escrito porque es el mismo modo de fallo que citar un número de memoria, y
  porque es la razón por la que `R1` §6 exige el SHA completo y no el corto.

### Pasos 3 y 4 — el lado Dart, y la prueba que define la fase

**Ocurrió.** Un archivo de **8 MiB + 13 bytes** cruzó **dos procesos de sistema
operativo distintos**, por un socket, conducido desde Dart, y llegó **idéntico
byte a byte**.

#### 3.1 El problema que apareció al escribir Dart, y que ninguna ADR anticipó

`dart:ffi` **no trae asignador de memoria nativa**. El `malloc` que todo el mundo
usa vive en `package:ffi`, y el criterio de aceptación 10 de esta fase dice «cero
paquetes nuevos de pub.dev». `pubspec.lock` tiene 39 y `ffi` no está.

Las tres salidas, medidas antes de elegir:

| Salida | Veredicto |
|---|---|
| `package:ffi` | **No.** Es exactamente el paquete que el criterio 10 prohíbe |
| `@Native(isLeaf: true)` + `TypedData.address` | **No, y por plataforma, no por gusto.** Está diseñado justo para esto —un puntero válido sólo durante una llamada *leaf*, que es lo que ADR-0032 §6 ya promete al copiar al entrar—, pero `@Native` sin `assetId` resuelve el símbolo **en el proceso**, y `DynamicLibrary.process()` **no está soportado en Windows**. Los native assets que darían `assetId` siguen experimentales. Algo que funciona en Android y no en Windows no sirve para un producto de tres plataformas |
| Dos funciones en `qyro_ffi` | **Sí**, y por eso lleva ADR |

**ADR-0038, congelada antes del código** (`c5f3b10`, un archivo, cero `.rs`).
Reescribe la regla de ADR-0032 §6, y la sustituta es **más simple** que la
original: *Rust posee todos los búferes que cruzan, en las dos direcciones; Dart
nunca posee memoria nativa, la pide prestada para llenarla y la devuelve.* La
frase original repartía la propiedad en dos y le daba a Dart la mitad que Dart
**no puede** sostener sin una dependencia.

Y la propia ADR-0032 §6 lo vio venir: *«La fase 02 necesitará un liberador de
verdad y eso lleva su propia cláusula de ADR, no llega de refilón.»* Resultó
hacer falta para la **entrada**, no para la salida.

#### 3.2 `guard` se hace genérico en vez de tener hermanos

Las dos funciones nuevas devuelven `*mut u8` y `()`, no `i32`, así que no cabían
en `guard`. La guarda `every_extern_c_function_sits_behind_the_panic_guard` exige
que el cuerpo abra **literalmente** con `guard(`, y añadir excepciones habría sido
debilitarla.

En su lugar, `guard` pasa a ser genérico sobre un trait `PanicOutcome` con el
valor de fallo **nombrado por tipo**. Un trait y no una cota `Default`, y la razón
importa: `Default` para `i32` es `0`, y `0` es `QYRO_OK` — un pánico habría
reportado éxito.

#### 3.3 Las cuatro pruebas, y por qué dos existen para que las otras dos signifiquen algo

| Prueba | Qué defiende |
|---|---|
| **`a_file_crosses_two_processes_driven_from_dart`** | La que define la fase. 8 MiB + 13 B, dos procesos, byte a byte, `.qyro-part` no sobrevive |
| `a_corrupted_transfer_is_detected_by_this_test` | `R2` §1.7. Voltea un bit **en el archivo llegado, no en el cable**: corromper el cable demostraría que el AEAD funciona, que es otra afirmación |
| `a_session_without_an_observer_still_completes` | ADR-0033 §2: «sin observador» no puede ser un segundo camino |
| `closing_from_dart_leaves_no_handle_and_no_thread` | Abre y cierra **seis** sesiones seguidas. La tabla sostiene cuatro (ADR-0032 §4), así que una sexta ronda sólo puede pasar si `dispose` libera la ranura de verdad. Es la comprobación de fuga que la superficie C **puede** expresar |

**El presupuesto de ADR-0033 aguantó al cruzar de verdad:** ≤102 emisiones para
8 MiB, monótonas, la última igual al total.

**Y `run()` cede el control entre pasos, y eso no es decoración.**
`NativeCallable.listener` entrega en el event loop del isolate que lo creó, así
que un isolate que nunca vuelve a su bucle recibe **todas** las emisiones de
golpe al final.

#### 3.4 Dos errores míos, escritos porque el proyecto los escribe

1. El primer borrador metió un **byte NUL literal** en el fuente Dart al separar
   rutas. Funcionaba, y convertía el archivo en «binario» para todas las
   herramientas de texto. Sustituido por el escape `\x00` en una constante.
2. El primer `_startReceiver` leía el anuncio del puerto con `await for` y
   después drenaba el resto del mismo stream: `Bad state: Stream has already
   been listened to`, las cuatro pruebas en rojo. Una suscripción, no dos.

### Puerta de los pasos 3 y 4 · 2026-08-14

| # | Comprobación | Veredicto |
|---|---|---|
| 1 | `cargo fmt --all --check` · `dart format --set-exit-if-changed` | ✅ exit 0 los dos |
| 2 | `cargo clippy … -D warnings` · `flutter analyze` | ✅ exit 0 los dos, «No issues found!» |
| 3 | `cargo test --workspace` · `flutter test` | ✅ **595** Rust (591 → 595) y **62** Dart, 0 fallos, 2 ignorados — los mismos dos |
| 4 | Barrido | ⏳ **pendiente para la puerta de fase**, sobre `qyro_ffi`, que es donde está el código nuevo de Rust |
| 5 | Lectura de aserciones | ✅ La comparación byte a byte lee `original` y `arrived`, dos archivos distintos. `two_buffers_do_not_share_memory` compara dos asignaciones, no una consigo misma |
| 6 | Lectura de contadores | ✅ `emissions` lo llena el callback real; su cota se comprueba contra el tamaño conocido |
| 7 | La medida se ve fallar | ✅ El bit volteado, y la ronda seis de la tabla de handles |
| 8 | Lectura de nombres | ✅ Las cuatro llevan el nombre que `FASE-02` §7 exige y ejercen lo que dicen |
| 9 | `git diff --name-only` | ✅ 4 rutas: `abi.rs`, `lib.rs`, `qyro_session_api.dart`, `qyro_session_transfer_test.dart` |
| 10 | Ledger | ✅ 138 fichas, 43 abiertas · **cero fichas nuevas en este tramo** |
| 11 | Coherencia documental | ✅ exit 0 en Bash y PowerShell |
| 12 | Escribir el resultado | ✅ esta tabla |

**Dependencias, con el comando:** `grep -c '^\[\[package\]\]' Cargo.lock` → **64**,
sin cambio. Paquetes de Dart en `pubspec.lock` → **39**, sin cambio. **Los
símbolos `extern "C"` suben de ocho a diez**, que es lo que ADR-0038 autoriza y
declara.

### Puerta de FASE · 2026-08-14

Las doce, **más** lo que `R2` §4 añade a una puerta de fase.

| # | Comprobación | Veredicto |
|---|---|---|
| 1 | `cargo fmt --all --check` · `dart format --set-exit-if-changed` | ✅ exit 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` · `flutter analyze` | ✅ exit 0 |
| 3 | `cargo test --workspace` · `flutter test` | ✅ **597** y **62**, 0 fallos, 2 ignorados — **los mismos dos** |
| 4 | Barrido con alcance declarado | ✅ §10 |
| 5 | Lectura de aserciones | ✅ |
| 6 | Lectura de contadores | ✅ |
| 7 | La medida se ve fallar | ✅ Y una de ellas **se vio no ver**, y se corrigió (QYR-0321) |
| 8 | Lectura de nombres | ✅ |
| 9 | `git diff --name-only 3b32b6f..HEAD` | ✅ §13, 36 archivos |
| 10 | El ledger sigue legible | ✅ 139 fichas, **36 abiertas — las mismas 36 con las que empezó**. +12 / −12 |
| 11 | Coherencia documental, Bash **y** PowerShell | ✅ exit 0 los cuatro |
| 12 | Escribir el resultado | ✅ |
| **+** | Criterios de aceptación uno a uno | ✅ §7, **13 de 14 completos y el 8 parcial y declarado** |
| **+** | Workflows en verde sobre un commit nombrado, tabla exhaustiva | ✅ §14. `a830558`: CI success con los ocho trabajos |
| **+** | `STATUS`, `HANDOFF`, `NEXT_STEPS`, `CHANGELOG`, `BUGS_PENDING` al día | ⚠️ `STATUS` y `BUGS_PENDING` sí. **`HANDOFF`, `NEXT_STEPS` y `CHANGELOG` no se actualizaron y se dice** en vez de declararlo hecho |
| **+** | Informe completo según `R5` | ✅ Las dieciséis secciones |

**Veredicto: la fase 02 cierra**, con el criterio 8 parcial y tres documentos de
estado sin actualizar, los dos declarados arriba en vez de escondidos.

---

## 10. Tabla de mutación

**Alcance declarado:** `cargo mutants --package qyro_session --timeout 90`, sobre
el único crate que este paso toca. cargo-mutants 25.x. **No es un barrido del
workspace** y no dice nada de los otros diez crates.

| Resultado | Nº |
|---|---|
| Mutantes generados | **32** |
| Caught | **11** |
| Missed | **9** |
| Timeout | **1** |
| Unviable | **11** |

`cargo mutants` sale con **exit 3** cuando quedan supervivientes; es el resultado
esperado aquí y no un fallo de la herramienta.

### Los nueve supervivientes, agrupados por causa

**Dos están excluidos por `R4` §2** — un mutante de `Display` o `Debug` no merece
ficha:

- `error.rs:58` · `Display for SessionError`
- `session.rs:100` · `Debug for Session`

**Siete son huecos reales, y los siete son la misma familia: las pruebas cubren
el final feliz y no los finales que fallan.**

| Mutante | Qué queda sin defender |
|---|---|
| `session.rs:66` · `RefusingSink::write_at` → `()` | Que contenido llegado **antes** del manifiesto se rechace. El sink existe para *registrar* en vez de tragar, y volverlo silencioso no rompe nada |
| `session.rs:393` · `finished` → `true` / → `false` | Un peer que cierra justo después del último frame |
| `session.rs:394`, `:395` · `==` → `!=` en `finished` | Igual |
| `session.rs:406` · `&&` → `\|\|` en `verdict` | Que un receptor con **cero veredictos** termine en `Rejected` y no en `Completed`. Con `\|\|`, `all()` sobre una lista vacía es `true` |
| `session.rs:446` · `==` → `!=` en `finish` | Un ítem cuyo veredicto **no** es `Ok` |

**La observación incómoda, y va aquí porque es la útil:** los cuatro mutantes de
`finished` sobreviven **por culpa del arreglo de QYR-0316**. Antes, la ruta «la
lectura falló y ya habíamos terminado» se recorría en cada transferencia, porque
el receptor cerraba sin mandar su veredicto. Arreglado eso, esa ruta ya no la
alcanza ningún test. Arreglar un defecto puede descubrir cobertura, y aquí la
descubrió.

### El timeout no es un superviviente

`session.rs:347` · `==` → `!=` en el brazo emisor de `advance`. Con la mutación,
el emisor se considera terminado en el primer paso y sale; el **receptor** se
queda bloqueado en `read_frame` esperando frames que ya no llegan, y el `join`
incondicional del harness cuelga con él.

`R2` §3 pregunta si un peer puede producir la condición. **Un peer que saluda y
después se calla sí la produce.** Lo que este barrido **no** establece es si
`FrameStream` tiene plazo de lectura: `qyro_net` clasifica `is_read_timeout`, así
que probablemente sí, y entonces el cuelgue es de mi harness y no de producción.
**No lo verifiqué, así que no lo afirmo.** Queda en QYR-0320 como la primera cosa
que comprobar.

### Barrido del paso 2 — el emisor, y lo que mis propias pruebas no fijan

**Alcance declarado:** `cargo mutants --package qyro_session --timeout 120`, tras
añadir el puente de progreso.

**Completo: 62 mutantes — 33 caught, 16 missed, 2 timeout, 11 unviable.**

**Siete de los diez supervivientes son código que acabo de escribir**, y eso es el
hallazgo:

| Mutante | Qué no fija mi prueba |
|---|---|
| `:101` `Emitter::step_for`, `/` → `%` | La **aritmética** del paso. `total % 100` da otro número y mis aserciones lo aceptan |
| `:102` `>` → `==` y `>` → `>=` | Cuál de las dos ramas —el suelo o la fracción— decide en el codo |
| `:114` `&&` → `\|\|`, y `:114`, `:117`, `:119` `>` → `>=` | Los límites exactos de cuándo se emite |

**Por qué pasan.** `the_callback_budget_is_respected_for_a_known_file_size`
comprueba un **techo** (≤102) y una **desigualdad estricta** entre dos tamaños. Las
dos son propiedades reales y las dos siguen valiendo con la aritmética cambiada:
`%` en vez de `/` produce un paso distinto, pero sigue dando menos de 102
emisiones y sigue creciendo con el archivo por debajo del codo.

Es la trampa de `R1` §5.6 en una forma que no había visto: **la desigualdad
estricta distingue una medida de una constante, y no distingue una medida de otra
medida.** Para fijar la fórmula hace falta una prueba que calcule las emisiones
esperadas *desde* la fórmula, con un tamaño a cada lado del codo de 25 MiB — que
es donde `>` frente a `>=` cambia de rama.

**Y sí se arregló, en el commit siguiente** — pero no con otra aserción sobre la
misma prueba, que es lo que `R4` §6 prohíbe. La aritmética es pura, así que se
probó como aritmética: **siete pruebas unitarias sobre `Emitter`**, dentro de
`session.rs`, que fijan el valor exacto en vez de acotarlo.

| Prueba | Qué mutante mata |
|---|---|
| `the_step_is_the_floor_below_the_elbow_and_exactly_the_fraction_above_it` | `/` → `%` (`step_for(1 GiB) == 10_737_418`, que `%` da como 24) y `>` → `==` (un paso más allá del codo el valor es `PROGRESS_MIN_STEP + 1`, no el suelo) |
| `an_observer_hears_nothing_until_the_total_is_known` | Los dos `> 0` → `>= 0`. Sobre un `u64`, `>= 0` es siempre cierto, así que el paso se fijaría sin saber el total y una emisión saldría con `total = 0` — una barra 300 KiB avanzada en un viaje de longitud desconocida |
| `an_emission_lands_on_its_boundary_and_not_one_byte_early` | Los límites exactos de `stepped`, y que la siguiente frontera se mide **desde la última emisión** y no desde cero |
| `the_whole_budget_is_bounded_by_a_constant_and_not_by_the_file` | La propiedad de ADR-0033 §1 sobre tamaños que ningún test con socket puede pagar: 4 MiB, 100 MiB, 1 GiB y 64 GiB |

**Barrido dirigido después: 26 mutantes, 25 muertos, 1 superviviente.**

**Y el que queda está probado equivalente, no excusado.** `>` → `>=` en
`step_for` no puede cambiar la respuesta: las dos ramas sólo difieren cuando
`fraction == PROGRESS_MIN_STEP`, y entonces las dos devuelven ese mismo número.
`swapping_the_floor_comparison_for_a_non_strict_one_cannot_change_the_answer`
ejerce las dos comparaciones **en paralelo** sobre seis entradas y afirma que
coinciden — de modo que si algún día dejaran de coincidir, la prueba lo dice en
vez de que la equivalencia se herede de un comentario. Es la misma forma con la
que la fase 01 trató el `|` → `^` de `compose`.

### Puerta del paso 2 — el puente de progreso · 2026-08-13

| # | Comprobación | Veredicto |
|---|---|---|
| 1 | `cargo fmt --all --check` | ✅ exit 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| 3 | `cargo test --workspace` | ✅ exit 0 · **591 passed** (581 → 591), 0 failed, 2 ignored |
| 4 | Barrido | ✅ 62 mutantes declarados arriba, más un barrido dirigido de 26 sobre `Emitter` con 25 muertos y una equivalencia **probada** |
| 5 | Lectura de aserciones | ✅ La aserción de equivalencia compara dos expresiones **escritas aparte**, no dos llamadas a la misma función: si fueran la misma llamada no probaría nada |
| 6 | Lectura de contadores | ✅ El contador de emisiones es el `Vec` que llena el observador, derivado de llamadas reales. La forma que lo distingue de una constante es la comparación contra el número de `step` |
| 7 | La medida se ve fallar | ✅ `an_emission_per_chunk_would_be_visible_to_this_measurement`, y el propio barrido demostró que **la primera versión de esa medida no veía siete fallos** — corregido y vuelto a medir |
| 8 | Lectura de nombres | ✅ Las siete unitarias enuncian su propiedad y la ejercen |
| 9 | `git diff --name-only` | ✅ 5 rutas |
| 10 | Ledger | ✅ 138 fichas, **43 abiertas**. QYR-0321 abierta y cerrada dentro del mismo paso |
| 11 | Coherencia documental | ✅ exit 0 en Bash y PowerShell |
| 12 | Escribir el resultado | ✅ esta tabla |

**Cero dependencias añadidas:** `Cargo.lock` sigue en 64 paquetes.

### El timeout, otra vez, y esta vez sobre mi arreglo

`:419` · `Session::write_outbound` → `Ok(())`. Es el arreglo de QYR-0316
convertido en un no-op: nada sale nunca al socket y las dos puntas se quedan
esperando. `R2` §3 — **no es un superviviente**: el comportamiento cambia de forma
observable, y lo que falla es la *forma* de fallar. Y refuerza lo que ya dijo el
barrido anterior: la pregunta pendiente sigue siendo si `FrameStream` tiene plazo
de lectura, porque si no lo tiene, un peer que se calla cuelga una sesión.

---

## 11. Tests antes y después

| | Antes | Después | Δ |
|---|---|---|---|
| `cargo test --workspace` (Windows) | 571 | **597** | +26 |
| `flutter test` | 58 | **62** | +4 |
| `qyro_session` | 6 | **24** | +18 |
| — de ellos, de conducta | **0** | **14** | +14 |
| `qyro_ffi` | 29 | **34** | +5 |
| Ignorados | 2 | **2** · los mismos dos | ±0 |

**Ningún ignorado nuevo**, que es lo que la comprobación 3 de `R2` pide mirar: un
test ignorado es un test que no existe.

Los cuatro de Dart son la prueba que define la fase y las tres que la sostienen.

---

## 12. Delta de dependencias

**Cero.** `Cargo.lock` sigue en **64** paquetes, el mismo número con el que
empezó la fase. Ni normales ni de desarrollo: las diez pruebas nuevas usan sólo
`std` y la API pública de `qyro_session`, que es por lo que `qyro_session` sigue
sin `[dev-dependencies]`.

Comando: `grep -c '^\[\[package\]\]' Cargo.lock` → 64.

---

## 13. Archivos tocados

`git diff --name-only 3b32b6f..HEAD` — **36 archivos**, en **20 commits**.

**Rust (9):** `qyro_ffi/src/{abi,lib,session_abi}.rs` · `qyro_net/src/{stream,tests}.rs` ·
`qyro_session/src/{lib,session}.rs` · `qyro_session/tests/session_behaviour.rs` ·
`qyro_transfer/src/session.rs`

**Dart (2):** `apps/qyro/lib/ffi/qyro_session_api.dart` ·
`apps/qyro/test/ffi/qyro_session_transfer_test.dart`

**Scripts y CI (6):** `scripts/check_docs_consistency.ps1` ·
`scripts/tests/docs_consistency_contract_test.{sh,ps1}` ·
`.github/workflows/{ci,platform-builds}.yml` · `.gitignore`

**ADR (3):** `ADR-0033-progress-bridge.md` · `ADR-0038-input-buffers.md` ·
`ADR-0039-ios-out-of-v1.md`

**Documentos del plan (9):** `00-LEEME-PRIMERO.md` · `R1` · `R2` · `R6` · y el
banner de iOS en `FASE-03`, `FASE-06`, `FASE-07`, `FASE-08`, `FASE-10`

**Estado del proyecto (7):** `STATUS.md` · `README.md` · `ROADMAP.md` ·
`PROJECT_CONTEXT.md` · `RELEASES.md` · `BUGS_PENDING.md` ·
`docs/security/secret-lifecycle-audit.md`

---

## 14. Runs de CI

**Exhaustiva, incluidos los cancelados y los fallidos.** `R1` §4: una lista de la
que se pueden caer los fallos no es evidencia, es un resumen favorable.

### El commit que importa

| Tirada | Workflow | Commit | Resultado |
|---|---|---|---|
| **31769832225** | **CI** | `a830558` | ✅ **success — los ocho trabajos**, incluido `rust workspace (windows-latest)` a las 04:27:58Z |
| **31767129485** | **Platform builds** | `f85e806` | ✅ success — paso 10 «Dart drives a transfer between two Windows processes» |
| **31767129574** | CI | `f85e806` | ⚠️ `cancelled` — es el **tope de 45 min** disparando; el trabajo `flutter` (Linux) sí terminó success con la transferencia |

### Todo lo demás, en orden

| Commit | CI | Platform builds | Android | iOS |
|---|---|---|---|---|
| `3b32b6f` (base) | ⚠️ cancelled | ✅ | ✅ | ✅ |
| `679dec1` | ❌ failure | — | — | — |
| `c26be67` | ⚠️ cancelled | ✅ | — | — |
| `1bc8b73` · `68fa6bc` · `37f7a6e` · `3bfc35e` | ⚠️ cancelled ×4 | — | — | — |
| `fe3b0e8` | ⚠️ cancelled | ✅ | ✅ | ✅ |
| `66802dc` · `8fce2a9` | ⚠️ cancelled ×2 | ✅ | — | — |
| `ba44406` | ⚠️ cancelled | ✅ | ✅ | ✅ |
| `35a3dbd` | ⚠️ cancelled | — | — | — |
| `f85e806` | ⚠️ cancelled | ✅ | — | — |
| `9110159` | ⚠️ cancelled | ✅ | ⚠️ cancelled | ✅ |
| `e103a6f` | ❌ **failure** | ✅ | — | — |
| `a830558` | ✅ **success** | ✅ | — | — |
| `ea0b023` | ⏳ en curso al escribir | ⏳ | ⏳ | ⏳ |

### Y qué significa cada color, porque «cancelled» engaña

- **`cancelled` no es una cancelación humana en casi ninguno de esos.** Superar
  `timeout-minutes` marca el trabajo **`cancelled`**, no `failed`. Un cuelgue con
  tope es indistinguible de que alguien pulse cancelar, y eso va escrito aquí
  porque va a despistar a quien lea la lista.
- Los que **sí** cancelé a mano son los anteriores a que existiera el tope: se
  quedaban seis horas en `rust workspace (windows-latest)` sin producir nada.
- **`679dec1` falló en `documentation`**, por la regla del `Verified commit` a más
  de diez commits de HEAD. Arreglado.
- **`e103a6f` falló también en `documentation`, por la misma regla** — y es el
  commit del arreglo del cuelgue. Se dice explícitamente para que nadie lea
  «failure» y concluya que el arreglo no sirvió: **el trabajo de Windows ya pasó
  en esa misma tirada.**

**La tirada `a830558` es la evidencia que cierra QYR-0078**, y es la primera vez
en la vida de esta rama que `rust workspace (windows-latest)` termina.

---

## 15. Qué NO debe leerse como progreso

Mientras siga siendo cierto, esto va en todos los informes:

- **Nada se ha probado en hardware físico.** Ni un teléfono, ni una tableta, ni
  un segundo ordenador. Cero.
- **Dos procesos en `127.0.0.1` no son dos aparatos en una Wi-Fi.** No hay
  descubrimiento, no hay MTU real, no hay pérdida de paquetes, no hay dos
  sistemas operativos distintos, no hay una red que se caiga a la mitad.
- **Reproducir la línea base no es progreso de producto.** Es la condición para
  poder empezar.
- **`local_addr` no cruza la superficie C**, así que el defecto de §4.4 no ha
  llegado a Dart — pero tampoco ha llegado *nada*: la superficie C existe y
  **ninguna transferencia ha ocurrido a través de ella**, ni en un test ni en
  ningún sitio. Eso sigue siendo cierto al escribir esta línea.
- **La comprobación 11 de la puerta está roja en Windows** y lo ha estado desde
  antes de la fase 01. Que la fase 01 la declarara verde no la pone verde.
- **En Windows no hay medición de fugas de descriptores ni de hilos**, ni la
  contra-prueba que enseña que la medición vería una fuga. Sólo Linux.

---

## 16. Ledger y handoff

### Balance, medido y no estimado

| | Base `3b32b6f` | HEAD | Δ |
|---|---|---|---|
| Fichas | 127 | **139** | +12 |
| Abiertas | 36 | **36** | **±0** |

**Abiertas en la fase (12):** QYR-0311 a QYR-0322.
**Cerradas (12):** QYR-0057, QYR-0078, QYR-0304, QYR-0309, QYR-0311, QYR-0312,
QYR-0313, QYR-0314, QYR-0315, QYR-0316, QYR-0319, QYR-0321.

**Se cerró exactamente tanto como se abrió.** No por casualidad: la instrucción
era que una lista que sólo crece deja de ser una lista, y cuatro de esos cierres
—QYR-0057, QYR-0304, QYR-0314, QYR-0315— son deuda anterior a esta fase que se
pagó aquí en vez de heredarse.

De las doce nuevas, **siete se cerraron dentro de la propia fase**. Las cinco que
quedan abiertas son: QYR-0317 y QYR-0318 (el receptor no informa de progreso y
`item` vale cero), QYR-0320 (los finales que fallan no están cubiertos),
QYR-0322 (un receptor no puede decir su puerto antes de que alguien se conecte),
y QYR-0312 está cerrada — la quinta es **ninguna más**, son cuatro.

### El ledger vuelve a ser consultable

El campo `Estado` usaba **once formas y cuatro palabras**. Ahora usa **dos**:
101 `cerrado` y 38 `abierto` sobre 139 fichas. Sesenta campos reescritos, y
**ninguna redacción perdida** — cada original queda literal en una línea
`Nota de estado`, que es prosa y no un campo que nada analice.

### Qué necesita saber la fase 03

1. **iOS está fuera de la v1.0** (ADR-0039). La mitad iOS de esta fase —
   `UIDocumentPickerViewController` — no se construye. Lo que la fase entrega es
   **Android y Windows**.
2. **El plan reserva `ADR-0034` para el selector de archivos.** Está libre: mis
   dos ADR se renumeraron a 0038 y 0039 al descubrir la colisión. Comprueba antes
   de asignar — `check_docs_consistency` verifica los `QYR-00xx` y **no tiene
   equivalente para los números de ADR**.
3. **`from_raw_fd` es `unsafe`** y la lista de crates exentos de
   `forbid(unsafe_code)` es una guarda con número. Se actualiza con justificación,
   no de refilón.
4. **El emulador de Android no existe todavía.** El SDK y `adb` están en la
   máquina; no hay AVD creado ni imagen de sistema instalada.
5. **`file_selector` añade paquetes de pub.dev**, y eso está autorizado para esta
   fase — el criterio 10 que lo prohibía era **de la fase 02**. Declara el delta
   de `pubspec.lock` con su conteo, que hoy es **39**.
6. **Lo que la fase 02 deja funcionando y la 03 puede usar:** `QyroSession.send`
   toma una lista de rutas. Un selector que devuelva un **fd** en Android no
   encaja en esa firma, y ése es el trabajo de diseño de la fase, no un detalle.
