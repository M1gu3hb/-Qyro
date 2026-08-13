# Fase 01 — El FFI del motor

**Estado: Paso 1 cerrado (ADR-0032 congelada). Pasos 2–5 pendientes.**

Se escribe durante la fase. Las secciones que dicen «pendiente» lo están de verdad.

---

## 1. Objetivo y alcance

> Que Dart pueda pedirle a Rust que envíe o reciba archivos por un socket, y que
> las claves privadas sigan sin poder llegar a Dart.

No objetivos declarados por el documento de fase §11: callbacks empujados (fase
02), Dart, UI, selector, descubrimiento, emparejamiento por el FFI, Keystore,
Keychain, empaquetado.

---

## 2. Qué se hizo

- **Paso 0 (previo)** — línea base reproducida, cinco de seis números coinciden.
  Detalle en `fase-00-linea-base.md`.
- **Paso 1** — la decisión de §4 tomada y **ADR-0032 congelada antes de una sola
  línea de código**, en commit propio `d282319` con **cero archivos `.rs`**.
- Pasos 2 a 5: **no empezados**.

---

## 3. Cómo se hizo

La decisión de §4 se sometió a análisis adversarial: una fase de fundamentos, tres
evaluaciones independientes —una por salida, cada una instruida para atacar la
suya—, tres refutaciones escépticas de esas evaluaciones, y una síntesis. Ocho
análisis, todos re-midiendo los números en lugar de heredarlos.

**La medición que decide**, replicando el recorrido del propio test:

| Grafo | Cierre | Prohibidos |
|---|---|---|
| `qyro_ffi` hoy | 2 | 0 / 14 |
| (a) | 50 | 12 / 14 |
| (b) | 51 | 12 / 14 |
| (c) | 49 | 12 / 14 |

Y la que gobierna la ADR: con `qyro_crypto` ya dentro del cierre, **añadir una
arista directa `qyro_ffi → qyro_crypto` cambia el cierre en nada** — diferencia
`[]`, bajo (b) y bajo (c). Esa arista es exactamente lo que la guarda existe para
impedir.

**Elegida la salida (b)**, por un motivo que el plan no da: es la única en la que
la guarda escrita a mano tiene un límite verificado por la máquina, porque Rust
sólo resuelve `qyro_crypto::X` si `qyro_crypto` está en el *extern prelude* del
propio crate, y eso sólo lo pone una dependencia directa.

Alternativas descartadas y por qué: en la ADR. Un proceso auxiliar sería lo único
que conservaría la propiedad de verdad, y iOS lo prohíbe.

---

## 4. Qué se encontró que no estaba en el plan

| Hallazgo | Dónde | Gravedad | Cómo se descubrió |
|---|---|---|---|
| La línea base declara verde una comprobación que el plan pone en rojo | `R4` §3 y §4 | P2 · QYR-0300 | Paso 0 |
| §4 describe mal dos de sus tres salidas | `FASE-01` §4 | P2 · QYR-0301 | Paso 1, midiendo antes de decidir |
| Un test con forma de cierre es ciego a la arista directa que existe para impedir | `c_abi_contract.rs` | — · en ADR-0032 §1 | Análisis adversarial |
| `AuthenticatedFrame::payload` devuelve los mismos bytes que `into_zeroizing_payload` y la guarda de egreso no lo ve | `qyro_crypto` | pendiente de ficha | Evaluación de la salida (a) |
| `panic = "abort"` no lo pone nadie, y nada lo afirma | perfiles del workspace | pendiente de ficha | Análisis de §5.5 |
| `qyro_ffi` tiene la única excepción de guardas mínimas, así que el análisis anti-pánico no corre sobre el crate que va a ganar cinco funciones `extern "C"` | `qyro_identity_store/src/guards.rs` | pendiente de ficha | Síntesis |

Los tres últimos se registran al empezar el paso 2, que es donde se actúa sobre
ellos. Registrarlos ahora sin tocarlos sólo llenaría el ledger.

---

## 5. Qué se arregló y qué no

| ID | Qué | Estado |
|---|---|---|
| QYR-0300 | La línea base no reproduce del todo | Abierto — dos de sus tres causas son decisión del supervisor |
| QYR-0301 | §4 describe mal dos salidas | Abierto — corregido *en* ADR-0032, que decide con las descripciones arregladas |

**No arreglado, y por qué no:** la cita huérfana de `R4` §4 exige un identificador
fuera de mi rango y evidencia ejecutada que no tengo. Bloquea la comprobación 11 de
toda puerta. Detalle en `fase-00-linea-base.md`.

---

## 6. A qué afectaba cada defecto

**QYR-0301.** Qué se rompía: la decisión central de la fase se habría tomado sobre
dos descripciones falsas, las dos empujando hacia la salida que el plan recomienda.
Para quién: para las fases 02, 03, 05 y 07, que heredan esta superficie — el
documento dice que si queda mal diseñada se rehace cuatro veces. En qué escenario:
el normal, si se hubiera aceptado que (b) «conserva la guarda intacta» y escrito la
ADR sobre eso. La ADR habría documentado una garantía que el código no da, que es
justo lo que `R1` §6 prohíbe.

**El cierre ciego.** Qué se rompe: nada hoy; todo a partir del paso 2. Para quién:
para cualquiera que lea el test y crea que sigue impidiendo lo que impedía. En qué
escenario: alguien añade `qyro_crypto` a `qyro_ffi` para desbloquearse, el test
pasa, y la propiedad más antigua del proyecto se pierde sin que salte nada.

---

## 7. Resultado contra el objetivo

| Objetivo del documento §9 | Resultado |
|---|---|
| 1. ADR-0032 congelada antes del primer commit de código | **Cumplido** — `d282319`, cero `.rs` |
| 2. Decisión de §4 tomada, argumentada, implementada, guarda vista fallar | **Parcial** — tomada y argumentada; implementar y verla fallar es el paso 2 |
| 3–7, 9, 10, 12 | **No hecho** — pasos 2 a 5 |
| 8. Cero dependencias externas | **Cumplido hasta aquí** — §12 |
| 11. Las doce comprobaciones en todas las puertas | **Parcial** — §9 |
| 13. Informe según `R5` | **En curso** — esto |
| 14. Los botones siguen `onPressed: null` | **Cumplido** — no se ha tocado Dart |

---

## 8. Clase de evidencia por afirmación

| Afirmación | Clase | Plataforma | Evidencia |
|---|---|---|---|
| 527 passed, 0 failed, 2 ignored | Probado en unidad e integración | Linux | `cargo test --workspace`, exit 0 |
| `clippy -D warnings` y `fmt --check` limpios | Compilado | Linux | exit 0 del proceso |
| El cierre de `qyro_ffi` es hoy `{qyro_core, qyro_ffi}` | Comprobado | — | `cargo tree -p qyro_ffi -e normal` |
| Los cierres de (a) 50, (b) 51, (c) 49, con 12/14 prohibidos | Comprobado, re-medido por cuatro análisis independientes | — | Réplica del recorrido de `c_abi_contract.rs:39-102` sobre `cargo metadata` |
| Una arista directa `qyro_ffi → qyro_crypto` no cambia el cierre | Comprobado | — | Misma réplica, diferencia `[]` |
| La ADR está congelada antes del código | Comprobado en el historial | — | `git show --stat d282319`: cero `.rs` |
| Cualquier cosa sobre Windows, Android, iOS, hardware físico | **Ninguna** | — | Nada se ejecutó fuera de Linux |

---

## 9. Las puertas

### Puerta del Paso 1 — 2026-08-12 — **PASADA CON UNA SALVEDAD DECLARADA**

| # | Comprobación | Veredicto |
|---|---|---|
| 1 | `cargo fmt --all --check` | PASS — exit 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS — exit 0 |
| 3 | `cargo test --workspace`, sin ignorados nuevos | PASS — 527 / 0 / 2, los mismos dos |
| 4 | Barrido de mutación del paso | **No aplica** — el paso no añade código de producción |
| 5 | Lectura de aserciones | **No aplica** — cero aserciones nuevas |
| 6 | Lectura de contadores | **No aplica** — cero contadores nuevos |
| 7 | La medida se ve fallar | **No aplica** — cero mediciones nuevas |
| 8 | Lectura de nombres de test | **No aplica** — cero tests nuevos |
| 9 | `git diff --name-only 1023f86..HEAD` | PASS — `BUGS_PENDING.md` y `docs/adr/ADR-0032-engine-ffi.md`. Ningún archivo de Codex |
| 10 | El ledger sigue legible | PASS — 118 fichas, 26 abiertas. Este paso añadió **una** |
| 11 | `check_docs_consistency` | **FALLA** — un BLOCKER, heredado. Salvedad abajo |
| 12 | Resultado escrito antes del paso siguiente | PASS — esto |

**La salvedad, dicha y no escondida.** La comprobación 11 falla por una cita
huérfana en `R4` §4, heredada y ajena: ya fallaba antes de que yo tocara nada
(cinco BLOCKER sobre `6de0af7`, uno ahora). No la puedo cerrar sin acuñar un
identificador fuera de mi rango o editar el documento del supervisor, y `R3` §8
prohíbe inventar un arreglo para un plan imposible.

Se sigue bajo `R2` §2.4 —*«sigue con la fase siguiente sólo si es independiente»*—
porque una cita en un `.md` no condiciona el diseño ni el código del FFI. **La
puerta de fase no podrá declararse pasada mientras siga así**, y eso también queda
escrito.

**Cerrada el 2026-08-13, en el paso 2.** La salvedad ya no existe: la cita
resuelve y las dos comprobaciones dan exit 0. Ficha QYR-0302, commit `fb4ecb9`.
Lo que había supuesto la salvedad —que hacía falta un identificador fuera de mi
rango— era falso, y lo dice la propia ficha: el incidente no se había perdido en
la consolidación de 5D, se había **renumerado**. La cita estaba a uno.

---

### Puerta del Paso 2 — 2026-08-13 — **PASADA**

| # | Comprobación | Veredicto |
|---|---|---|
| 1 | `cargo fmt --all -- --check` | PASS — exit 0. **Primero dio exit 1**; ver abajo |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS — exit 0 |
| 3 | `cargo test --workspace`, sin ignorados nuevos | PASS — 537 / 0 / 2, los mismos dos generadores |
| 4 | Barrido de mutación del paso | **Diferido al paso 5**, que es donde `FASE-01` §6 lo pone. Y hoy mediría poco: los seis tests de `qyro_session` son guardas estructurales, no de comportamiento (§15) |
| 5 | Lectura de aserciones | PASS — leídas las de los seis tests de `c_abi_contract.rs`. Ninguna compara una llamada consigo misma; el guard 2 lleva además dos controles positivos, sin los cuales un comprobador que lo rechazara todo pasaría sus dos negativos |
| 6 | Lectura de contadores | PASS — dos números y su comando: cierre 51 (`CLOSURE`) y conjunto directo 2, ambos de `cargo metadata`, no del manifiesto |
| 7 | **La medida se ve fallar** | PASS — es el centro de este paso. Detalle abajo |
| 8 | Lectura de nombres de test | PASS — los seis dicen lo que comprueban; `a_direct_crypto_edge_is_invisible_here_and_visible_to_guard_one` nombra la ceguera en vez de esconderla |
| 9 | `git diff --name-only 1023f86..HEAD` | PASS — catorce archivos, ninguno de Codex, ninguno de `main` (§13) |
| 10 | El ledger sigue legible | PASS — 119 fichas, 26 abiertas. El paso añadió **una**, cerrada |
| 11 | `check_docs_consistency` (bash y pwsh) | PASS — exit 0 **las dos**. Primera vez en esta rama desde que llegaron los documentos del plan |
| 12 | Resultado escrito antes del paso siguiente | PASS — esto |

**Comprobación 1, dicha porque falló.** `cargo fmt --all -- --check` devolvió
exit 1 sobre dos cadenas de `qyro_session/src/session.rs`. No se descubrió leyendo
la salida —que era un diff, no la palabra «error»— sino leyendo `$?`, que es
exactamente la lección que QYR-0083 dejó escrita en `R2` §1. Se aplicó
`cargo fmt --all` y se reverificó por exit code.

**Comprobación 7 — la medida se ve fallar, y con qué.** La puerta del paso 2 pide
una cosa concreta: *la prueba de cierre transitivo, sea cual sea su forma nueva,
tiene que fallar cuando la violas a propósito*. Se violó de verdad, con la línea
real en el manifiesto, no con metadatos falsificados:

1. Con `qyro_crypto = { path = "../qyro_crypto" }` añadido a `qyro_ffi/Cargo.toml`:
   `the_ffi_names_exactly_two_crates` **FALLA**, exit 101, y el diff que imprime es
   exactamente `{"qyro_core", "qyro_crypto", "qyro_session"}` contra
   `{"qyro_core", "qyro_session"}`.
2. En esa **misma** ejecución, `the_dependency_closure_matches_its_changelog`
   **pasa**. Es la confirmación de ADR-0032 §1 contra el resolvedor real, y dice
   algo incómodo que conviene no suavizar: **la guarda que este paso sustituyó
   habría seguido en verde por encima de la arista que filtra la clave.**
3. Y la propiedad que sostiene toda la guarda, medida en vez de argumentada. Con
   una función sonda idéntica, `fn _probe() -> Option<qyro_crypto::DeviceIdentity>`:
   con la arista, `cargo build -p qyro_ffi` da exit 0; sin la arista, exit 101 con
   `error[E0433]: failed to resolve: use of unresolved module or unlinked crate
   qyro_crypto`. **`qyro_crypto` está en el cierre en los dos casos.** Alcanzable y
   no nombrable: eso es la frontera.

El manifiesto y la sonda se revirtieron. `git diff --stat` contra HEAD sobre
`rust/crates/qyro_ffi/src/lib.rs` es vacío: el archivo quedó idéntico al commit,
byte a byte, no «equivalente».

---

### Puerta del Paso 3 — 2026-08-13 — **PASADA**

| # | Comprobación | Veredicto |
|---|---|---|
| 1 | `cargo fmt --all -- --check` | PASS — exit 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS — exit 0. Primero dio 12 errores; abajo |
| 3 | `cargo test --workspace`, sin ignorados nuevos | PASS — 547 / 0 / 2, los mismos dos |
| 4 | Barrido de mutación del paso | PASS — cinco mutaciones dirigidas, tabla en §10. El barrido completo con `cargo-mutants` sigue siendo el paso 5 |
| 5 | Lectura de aserciones | PASS — y produjo un hallazgo, QYR-0307: una aserción no cubría lo que su nombre prometía |
| 6 | Lectura de contadores | **No aplica** — el paso no añade contadores |
| 7 | **La medida se ve fallar** | PASS — las cinco. Una sobrevivió primero, y eso es el hallazgo |
| 8 | Lectura de nombres de test | PASS — y uno estaba mal puesto; ver QYR-0307 |
| 9 | `git diff --name-only` | PASS — `handle.rs` y `abi.rs` nuevos, `lib.rs`, `BUGS_PENDING.md`, este informe. Ninguno de Codex |
| 10 | El ledger sigue legible | PASS — 124 fichas, 31 abiertas. El paso añadió **una** |
| 11 | `check_docs_consistency` (bash y pwsh) | PASS — exit 0 las dos |
| 12 | Resultado escrito antes del paso siguiente | PASS — esto |

**Comprobación 2, dicha porque falló.** `cargo clippy -p qyro_ffi --all-targets`
dio **12 errores** a la primera, de dos causas distintas y las dos reales: el
bloque `#![deny(...)]` de módulo alcanzaba también a los `mod tests`, que usan
`expect` a propósito; y con los módulos privados y sin llamantes de producción, la
tabla entera se leía como código muerto. Se arregló cada una por su lado —un
`#![allow(...)]` con motivo dentro de cada `mod tests`, y `pub mod` para las dos
piezas, que además es lo que el paso 4 necesita— y no silenciando el lint.

**Comprobación 7 — las cinco medidas, vistas fallar.** Cada control de §5.1 roto a
propósito, y el test que debe cazarlo, ejecutado:

| # | Mutación | Test dirigido | Resultado |
|---|---|---|---|
| H1 | Las generaciones empiezan en `0` y no en `1` | `the_handle_zero_is_refused_because_generations_start_at_one` | exit 101 |
| H2 | `get` no compara la generación | `a_handle_from_another_session_...` | exit 101 |
| H3 | `close` no incrementa la generación | `a_double_close_is_an_error_and_not_a_crash` | **exit 0 — SOBREVIVIÓ** |
| H4 | Al desbordar, la ranura se recicla en vez de retirarse | `a_slot_whose_generation_would_wrap_is_retired_rather_than_reused` | exit 101 |
| H5 | `guard` llama al cuerpo sin `catch_unwind` | `a_panic_inside_the_c_boundary_becomes_an_error_code` | exit 101 |

**H3 es el hallazgo del paso, y es mío.** La mutación que borra el incremento de
generación **sobrevivió** al test del doble cierre. El motivo está en QYR-0307: la
ADR-0032 §4 dice que el doble cierre *es* la comprobación de generación, y no lo
es. Una ranura recién cerrada está **vacía**, así que la resolución falla en la
comprobación de vacío antes de mirar ninguna generación. Lo que la generación
protege es la **reutilización de ranura**, no el doble cierre.

Consecuencias, las dos escritas:

1. Lanzada contra la suite entera, H3 **sí muere** — la caza
   `a_handle_from_another_session_...`. O sea que el control está cubierto; lo que
   estaba mal era mi puntería, el mismo error de clase que M4/QYR-0080 en 6A.
2. Pero muere por la aserción de **precondición** de ese test, la que comprueba
   que el escenario sigue siendo significativo, antes que por la sustantiva. Un
   test que caza una regresión de seguridad por su andamiaje es un test que puede
   dejar de cazarla en cuanto alguien reescriba el andamiaje.

Por eso `a_double_close_is_an_error_and_not_a_crash` se reforzó para afirmar que la
generación avanzó, que es el mecanismo que su nombre promete. Tras el refuerzo, H3
dirigida a ese test da exit 101 con «close must advance the generation», left 1,
right 2. **La otra mitad de QYR-0307 sigue abierta**: la frase de la ADR describe
un mecanismo que no es el que opera, y una ADR congelada se enmienda en su propio
commit, no de paso.

---

### Puerta del Paso 4 — 2026-08-13 — **PASADA**

| # | Comprobación | Veredicto |
|---|---|---|
| 1 | `cargo fmt --all -- --check` | PASS — exit 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS — exit 0. Falló antes, por un `mut` sin uso que destapó un test mal nombrado |
| 3 | `cargo test --workspace`, sin ignorados nuevos | PASS — 563 / 0 / 2. Falló antes; abajo |
| 4 | Barrido de mutación del paso | PASS — cuatro dirigidas, §10 |
| 5 | Lectura de aserciones | PASS — y destapó que la regla pegajosa no tenía ninguna |
| 6 | Lectura de contadores | PASS — ocho símbolos `no_mangle`: 2 en `lib.rs` y 6 en `session_abi.rs`, con `grep -c 'unsafe(no_mangle)'` |
| 7 | **La medida se ve fallar** | PASS — las cuatro, una tras arreglar lo que destapó |
| 8 | Lectura de nombres de test | PASS — y uno estaba mal puesto, corregido |
| 9 | `git diff --name-only` | PASS — ninguno de Codex |
| 10 | El ledger sigue legible | PASS — 125 fichas, 30 abiertas. Dos cerradas, una nueva |
| 11 | `check_docs_consistency` (bash y pwsh) | PASS — exit 0 las dos |
| 12 | Resultado escrito antes del paso siguiente | PASS — esto |

**Las seis operaciones**, ni una más: abrir emisora, abrir receptora, avanzar,
progreso, cancelar, cerrar. Con las dos de versión que ya existían son ocho
símbolos `extern "C"`, y una guarda cuenta que son ocho.

**Una corrección al alcance, hecha y no callada.** La tabla del paso 4 dice que la
sesión emisora recibe **directorio raíz**, y `qyro_session::Session::open_sender`
no lo tenía: derivaba el nombre de cada archivo con `file_name()`, o sea aplanando.
Dos archivos llamados `a.txt` en carpetas distintas viajaban los dos como `a.txt` y
le dejaban al receptor una colisión que el emisor había fabricado. Se implementó el
raíz —`strip_prefix`, y refuse si el resto no es un descendiente llano— en vez de
exponer una superficie más estrecha en silencio.

**Comprobación 3, dicha porque falló.** `cargo test --workspace` dio exit 101 con
«qyro_ffi declares SessionError but its structural guards do not check every
variant». No lo declara. La guarda de workspace decide quién declara un enum
partiendo la fuente **en bruto** por `"pub enum "`, sin descartar tests ni cadenas
literales, y una guarda mía contiene esa frase entre comillas para vigilar mi
propio brazo `_`. QYR-0308. Se evitó localmente con `concat!`; la guarda compartida
**no se tocó**, porque cambiar cómo decide quién declara qué afecta a varios crates
y tiene alcance propio.

**Comprobación 7 — las cuatro medidas.**

| # | Mutación | Test dirigido | Resultado |
|---|---|---|---|
| G1 | `qyro_session_close` sin `guard(` | `every_extern_c_function_sits_behind_the_panic_guard` | exit 101 |
| G2 | `panic = "abort"` en un perfil | `no_cargo_profile_sets_panic_abort` | exit 101 |
| G3 | Un brazo borrado del mapa de errores | `every_session_error_variant_has_its_own_code_...` | exit 101 |
| G4 | La regla pegajosa borrada | *(ninguno la cubría)* | **exit 0 — SOBREVIVIÓ** |

**G4 es el hallazgo del paso, y no es puntería: es un hueco.** Borrar la regla que
ADR-0032 §5 congela —que una sesión fallada devuelve **el mismo código** para
siempre— **no rompía ni un test de este crate**. Lanzada contra la suite entera
tampoco. Y el motivo por el que no lo cubría nadie es el que hace peligroso el
hueco: envenenar una sesión de verdad exige una sesión de verdad, y eso exige un
peer, así que el test natural sería de integración disfrazado de unitario.

La política se extrajo a una función pura, `sticky`, y se probó ahí: que el fallo
se pega, que un `Ok` posterior sigue devolviendo el código viejo —«un segundo `Ok`
deja creer a Dart que la sesión se recuperó»—, que un fallo posterior no
sobrescribe al primero, que el éxito **no** envenena, y que un parámetro de salida
nulo no envenena una sesión que no llegó a tocarse. Con eso, G4 dirigida a su test
da exit 101: left 0, right -6.

**Dos fichas cerradas por este paso**, las dos con la evidencia de haberlas visto
fallar: QYR-0305 —la guarda de `panic = "abort"`, que sin ella deja todo
`catch_unwind` de adorno— y QYR-0306 —`qyro_ffi` ya lleva el mínimo estructural
compartido y la lista de excepciones quedó vacía—.

**Y dos guardas que fallaron mientras se escribían**, dichas porque el informe se
escribe durante y no después: la de pánico marcaba tres funciones que sí estaban
guardadas —leía «la línea siguiente al nombre», y estas firmas ocupan siete
líneas—, y la de sitios de construcción pedía que `HandleError` se construyera
fuera del archivo que lo declara, que es la pregunta correcta para `SessionError` y
no para un enum que sólo la tabla puede producir. La segunda se resolvió eximiendo
las dos variantes **con el argumento escrito**, que es la salida que la propia
guarda ofrece, y dejando su alcance real —el suelo de parseo— en pie.

---

## 10. Tabla de mutación

No aplica al paso 1: no añade código de producción. El barrido con
`cargo-mutants --timeout 90` es el paso 5.

---

## 11. Tests antes y después

**527 passed, 0 failed, 2 ignored antes y después**, en Linux, con
`cargo test --workspace`. El paso 1 no añade tests: congela una decisión.

**Paso 2: 527 → 537**, en Linux, misma orden. La cuenta, que suma exacta: **+6**
de `qyro_session`, todos guardas estructurales, y **+4** de `c_abi_contract.rs`,
que pasa de dos tests a seis. Los 2 ignorados son los mismos dos generadores de
vectores de siempre; **cero ignorados nuevos**.

Y lo que esos 537 **no** cubren: los seis de `qyro_session` son de estructura
—que los archivos estén listados, que cada variante de error tenga sitio de
construcción, que ninguna ruta de producción pueda entrar en pánico—. **Ninguno
abre una sesión.** `open_sender`, `open_receiver`, `step`, `progress`, `cancel` y
`finish` no los ejerce todavía nada. Eso es el paso 4, y por eso el barrido de
mutación es el paso 5 y no éste.

---

## 12. Delta de dependencias

**63 paquetes antes y 63 después**, con `grep -c '^\[\[package\]\]' Cargo.lock`.
`Cargo.lock` no aparece en el diff del paso (§9, comprobación 9), así que el diff
es vacío. Cero dependencias externas.

**Paso 2: 63 → 64.** El paquete que entra es `qyro_session`, **de primera parte**,
un miembro nuevo del workspace. Misma orden:
`git show 1023f86:Cargo.lock | grep -c '^\[\[package\]\]'` da 63 y sobre el árbol
de hoy da 64. **Cero dependencias externas de Rust añadidas**, que es lo que la
regla prohíbe; el número sube porque el workspace tiene un crate más, no porque
haya llegado nada de fuera.

Lo que sí cambió de tamaño es lo que queda **enlazado bajo el FFI**: de 2 crates a
51. Ninguno es nuevo en el repositorio — todos estaban ya bajo `qyro_transfer` y
`qyro_net` — pero antes no estaban bajo `qyro_ffi`, y ahora sí. La constante
`CLOSURE` de `c_abi_contract.rs` los fija por nombre, declarada como registro de
cambios y no como guarda, por el motivo que ADR-0032 §3.3 mide.

---

## 13. Archivos tocados

Base de la fase: `1023f86`.

```
$ git diff --name-only 1023f86..HEAD    # tras el paso 1
BUGS_PENDING.md
docs/adr/ADR-0032-engine-ffi.md
```

Tras el paso 2, catorce:

```
$ git diff --name-only 1023f86..HEAD
BUGS_PENDING.md
CHANGELOG.md
Cargo.lock
Cargo.toml
NEXT_STEPS.md
STATUS.md
.github/scripts/android_crypto_smoke.sh
docs/adr/ADR-0032-engine-ffi.md
docs/fase-implementacion/R4-COMO-REGISTRAR-BUGS.md
docs/reports/fase-01-ffi-del-motor.md
rust/crates/qyro_ffi/Cargo.toml
rust/crates/qyro_ffi/tests/c_abi_contract.rs
rust/crates/qyro_session/**  (5 archivos)
rust/tools/qyro_crypto_smoke/src/lib.rs
```

**Ninguno es de Codex** y ninguno es de `main`. El único ajeno es
`R4-COMO-REGISTRAR-BUGS.md`, del supervisor, y el cambio es **un identificador**:
la cita de §4 pasa a apuntar a la ficha que sí existe (QYR-0302). Se hace porque
dejar la rama en rojo está prohibido y porque la evidencia identifica el referente
sin ambigüedad; queda dicho aquí para que se vea, no escondido en un diff.

`rust/crates/qyro_ffi/src/lib.rs` **no** está en la lista, y eso es deliberado:
se le añadió una sonda para la comprobación 7 y se le quitó. `git diff` sobre él
es vacío.

---

## 14. Runs de CI

Ninguno lanzado por este paso todavía. Se listarán sin filtrar, con los fallidos y
los cancelados, en la puerta de fase.

---

## 15. Qué NO debe leerse como progreso

Tras el paso 1 se escribió esto, y tres de las cuatro primeras han caducado. Se
dejan, con lo que hoy es cierto al lado, porque un informe que se reescribe para
parecer coherente deja de ser un registro:

- **No existe una sola función `extern "C"` nueva.** `qyro_ffi` sigue exponiendo
  exactamente dos, y sigue dependiendo sólo de `qyro_core`. **Dart no puede pedir
  nada.** Lo que hay es una decisión escrita, no una superficie.
  → *Tras el paso 2: la segunda mitad ya no vale, `qyro_ffi` depende de
  `qyro_session`. **La primera sigue igual de cierta**: siguen siendo exactamente
  dos funciones, `qyro_protocol_version_ptr` y `qyro_protocol_version_len`, y
  ninguna abre una sesión. Dart sigue sin poder pedir nada.*
- **`qyro_session` no existe.** Es el paso 2. → *Ya existe: cinco archivos, seis
  guardas. Ninguna de las seis lo **ejecuta**.*
- **La guarda nueva no existe y no se ha visto fallar.** Hasta que se vea fallar
  no es una guarda, es un comentario — lo dice el propio documento de fase §7.3.
  → *Se ha visto fallar, con la arista real en el manifiesto y con `E0433` en el
  compilador. §9, comprobación 7.*
- **La propiedad más antigua del proyecto sigue intacta hoy**, y la ADR decide que
  deje de estarlo en el paso 2. Que esté decidido no es que esté hecho.
  → *Hecho está. **La propiedad murió en este paso**: la pila criptográfica está
  enlazada en el `cdylib` que Dart carga. Lo que la sustituye es más pequeño y hay
  que decirlo con esas palabras — antes lo decidía el compilador sobre
  alcanzabilidad, ahora lo decide una superficie pública que revisan personas, y
  un test transcribe. Trece archivos afirmaban lo viejo; QYR-0303.*
- **Y una nueva, que el paso 2 crea:** `qyro_session` compila y está guardado, pero
  **no está probado**. Sus seis tests miran la forma del código, no su conducta.
  Que el workspace esté en 537 verdes no dice nada sobre si una sesión transfiere
  un archivo.
- Los botones siguen `onPressed: null`.
- **Nada se ha probado en hardware físico.** Dos procesos en `127.0.0.1` no son dos
  aparatos en una Wi-Fi.

---

## 16. Ledger y handoff

| ID | Sev | Título | Al empezar | Al cerrar el paso |
|---|---|---|---|---|
| QYR-0301 | P2 | La fase 01 describe mal dos de sus tres salidas para la guarda del FFI | no existía | abierto |
| QYR-0302 | P2 | `R4` §4 citaba un identificador que la consolidación de 5D renumeró | no existía | **cerrado** |
| QYR-0303 | P2 | Trece archivos afirmaban una propiedad que la fase 01 derogó | no existía | abierto |
| QYR-0304 | **P1** | El motor deshace el zeroize del texto claro recibido en la línea siguiente | no existía | abierto |
| QYR-0305 | P2 | Nada impide que un perfil ponga `panic = "abort"` y anule el `catch_unwind` | no existía | abierto |
| QYR-0306 | P2 | `qyro_ffi` es la única excepción al mínimo de guardas | no existía | abierto |
| QYR-0078 | P1 | `qyro_net` no se ejecuta ni se compila en Windows | abierto | abierto, **media contestada** |

**Balance: 25 abiertas antes del paso 1, 26 tras el paso 1, 30 tras el paso 2.**
Sube cuatro: las cinco nuevas menos QYR-0302, que nace cerrada.

**El P1 es QYR-0304 y merece leerse antes que el resto de este informe.** No lo
introduce esta fase — está en `qyro_transfer` desde 5A — pero lo encuentra, y es de
la peor clase: `into_zeroizing_payload().to_vec()`. Se llama al accesor que
protege y se deshace la protección en la misma expresión. El doc-comment del
método, tres archivos más allá, describe exactamente la conducta que esa línea
impide. Justificación del P1 en la ficha, con por qué no es P0 y por qué no es P2.

**QYR-0078, contestada a medias y no cerrada.** El trabajo `rust workspace
(windows-latest)` existe y su paso de clippy pasó, así que «`qyro_net` no se
compila siquiera en Windows» es falso. El paso `cargo test --workspace` seguía en
curso al consultarlo, así que «no se ejecuta» **no** está contestado. No se cierra:
`R4` §5 pide evidencia ejecutada, y un paso sin terminar no es un verde. Es
literalmente la regla de no convertir «compiló» en «funciona».

**Documentación que queda desfasada por ADR-0032 §9:** las afirmaciones de que el
FFI no puede alcanzar la cripto dejarán de ser ciertas en el paso 2. El análisis
contó **trece archivos** que lo afirman. Corregirlos es parte del paso 2, no antes:
mientras el código no cambie, siguen siendo ciertas.

**Qué necesita saber el paso 3** (tabla de handles):

- La frontera está puesta y comprobada. Lo que el paso 3 no puede hacer sin
  romperla es nombrar `qyro_crypto` en `qyro_ffi`, y no hará falta: la tabla de
  handles guarda `qyro_session::Session`, que no expone nada de la pila cripto.
- ADR-0032 §4 congela el handle como `generation||slot` en un `u64`, con el 0
  inválido por construcción. Los tests que el paso 3 debe traer están nombrados en
  `FASE-01` §6: doble cierre, handle inválido, handle de otra sesión, handle cero,
  y un pánico dentro de la frontera C convertido en código de error.
- **QYR-0305 se activa en el paso 4, no antes.** Cuando exista el primer
  `catch_unwind` habrá código que confía en que `panic = "abort"` no está puesto, y
  la guarda tiene que llegar con él.
- **QYR-0306 también.** `qyro_ffi` es hoy la única excepción al mínimo de guardas, y
  el paso 4 lo lleva de dos funciones a ocho. La exención se vacía ahí.
- `qyro_session` no tiene un solo test de conducta. Si el paso 3 añade la tabla de
  handles sin ejercer una sesión, el barrido del paso 5 va a tener poco que matar.

**Qué necesitaba saber el paso 2** (cumplido; se deja como registro):

- La estructura elegida es (b). El crate se llama `qyro_session` y es lo único que
  `qyro_ffi` ve.
- La guarda de §3.1 de la ADR —dependencias directas exactas, preguntadas al
  resolvedor— es la que hay que escribir primero, porque es la única sin lista.
- La prueba negativa que la ve fallar tiene que empalmar la arista directa en la
  salida real del resolvedor y comprobar **dos** cosas: que el cierre es ciego a
  ella, y que la guarda de profundidad uno la denuncia por nombre.
- Tres hallazgos esperan ficha al empezar el paso 2: el hueco de
  `AuthenticatedFrame::payload`, la ausencia no afirmada de `panic = "abort"`, y
  que `qyro_ffi` es el único crate con excepción de guardas mínimas justo antes de
  ganar cinco funciones `extern "C"`.
