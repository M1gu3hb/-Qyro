# FASE 02 — Dart conduce una transferencia

**Estado: en curso.** Este informe se escribe durante la fase, no al final.

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

*(Se rellena por pasos, cada uno con su puerta.)*

- **Paso 0 (previo)** — línea base reproducida y auditada. Cinco de siete
  comprobaciones coinciden; dos no, y las dos son hallazgos. Detalle en §4.

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
