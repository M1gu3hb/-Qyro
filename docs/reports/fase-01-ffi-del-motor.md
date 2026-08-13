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

---

## 10. Tabla de mutación

No aplica al paso 1: no añade código de producción. El barrido con
`cargo-mutants --timeout 90` es el paso 5.

---

## 11. Tests antes y después

**527 passed, 0 failed, 2 ignored antes y después**, en Linux, con
`cargo test --workspace`. El paso 1 no añade tests: congela una decisión.

---

## 12. Delta de dependencias

**63 paquetes antes y 63 después**, con `grep -c '^\[\[package\]\]' Cargo.lock`.
`Cargo.lock` no aparece en el diff del paso (§9, comprobación 9), así que el diff
es vacío. Cero dependencias externas.

---

## 13. Archivos tocados

Base de la fase: `1023f86`.

```
$ git diff --name-only 1023f86..HEAD
BUGS_PENDING.md
docs/adr/ADR-0032-engine-ffi.md
```

Ninguno es de Codex.

---

## 14. Runs de CI

Ninguno lanzado por este paso todavía. Se listarán sin filtrar, con los fallidos y
los cancelados, en la puerta de fase.

---

## 15. Qué NO debe leerse como progreso

- **No existe una sola función `extern "C"` nueva.** `qyro_ffi` sigue exponiendo
  exactamente dos, y sigue dependiendo sólo de `qyro_core`. **Dart no puede pedir
  nada.** Lo que hay es una decisión escrita, no una superficie.
- **`qyro_session` no existe.** Es el paso 2.
- **La guarda nueva no existe y no se ha visto fallar.** Hasta que se vea fallar
  no es una guarda, es un comentario — lo dice el propio documento de fase §7.3.
- **La propiedad más antigua del proyecto sigue intacta hoy**, y la ADR decide que
  deje de estarlo en el paso 2. Que esté decidido no es que esté hecho.
- Los botones siguen `onPressed: null`.
- **Nada se ha probado en hardware físico.** Dos procesos en `127.0.0.1` no son dos
  aparatos en una Wi-Fi.

---

## 16. Ledger y handoff

| ID | Sev | Título | Al empezar | Al cerrar el paso |
|---|---|---|---|---|
| QYR-0301 | P2 | La fase 01 describe mal dos de sus tres salidas para la guarda del FFI | no existía | abierto |

**Balance: 25 abiertas antes del paso, 26 después.** Sube una.

**Documentación que queda desfasada por ADR-0032 §9:** las afirmaciones de que el
FFI no puede alcanzar la cripto dejarán de ser ciertas en el paso 2. El análisis
contó **trece archivos** que lo afirman. Corregirlos es parte del paso 2, no antes:
mientras el código no cambie, siguen siendo ciertas.

**Qué necesita saber el paso 2:**

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
