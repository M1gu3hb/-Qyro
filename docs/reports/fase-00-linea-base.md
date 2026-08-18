# Fase 00 — Reproducción de la línea base

Los pasos 0 a 3 del encargo: mover el plan, leer las reglas, y reproducir la línea
base de `R6` §1. No es una de las diez fases numeradas; es la condición previa a
abrir la 01.

**Estado: PARADO en el paso 3, por instrucción expresa de `R6` §1.**

---

## 1. Objetivo y alcance

Mover los diecisiete documentos del plan a `docs/fase-implementacion/`, leer los
seis de reglas, y reproducir los seis números de `R6` §1. `R6` §1 dice, literal:

> **Tu primera tarea es reproducirlos.** Si no coinciden, para y repórtalo:
> significa que el árbol que tienes no es el que se planificó, y todo lo demás se
> apoya en eso.

No objetivos: nada de la fase 01. No se ha abierto `FASE-01-FFI-DEL-MOTOR.md`.

---

## 2. Qué se hizo

1. **Paso 0** — los diecisiete documentos movidos con `git mv`, para que la
   historia de cada uno siga al archivo. Commit `749df9a`. Los cuatro `.md` de
   `docs/` que no son del plan se quedaron donde estaban.
2. **Pasos 1 y 2** — leídos `00-LEEME-PRIMERO`, y `R6`, `R1`, `R2`, `R3`, `R4`,
   `R5` en ese orden, enteros, una vez.
3. **La fusión que faltaba** — `R6` §6 dice que la rama es la de red **con
   `codex/qyro-trust-5d` fusionada**, y que si no lo está hay que hacerlo primero.
   No lo estaba. Commit `90bb5d0`, un solo conflicto en `STATUS.md`, exactamente
   como `R6` §6 predijo.
4. **Paso 3** — los seis números, reproducidos por comando.

---

## 3. Cómo se hizo

El conflicto de `STATUS.md` se resolvió conservando mi cabecera: esta rama es el
destino de la fusión y su milestone —8 MiB entre dos procesos por un socket—
contiene al de la otra —5 MiB entre dos directorios—. Las secciones del cuerpo de
la otra rama auto-fusionaron y siguen ahí.

---

## 4. Qué se encontró que no estaba en el plan

| Hallazgo | Dónde | Gravedad | Cómo se descubrió |
|---|---|---|---|
| `check_docs_consistency` está en rojo sobre el árbol que `R6` §1 declara en PASS | `R4` §3 y §4 | P2 (QYR-0300) | Paso 3, al reproducir la línea base |
| La fusión de `codex/qyro-trust-5d` no estaba hecha | rama | — | `R6` §6, comprobado con `git rev-parse` |

El segundo no es un defecto del plan: `R6` §6 lo anticipa y da la instrucción.

---

## 5. Qué se arregló y qué no

| ID | Qué | Estado |
|---|---|---|
| — | La fusión que faltaba | Hecha, `90bb5d0` |
| QYR-0300 | La primera de las tres causas del rojo: la plantilla de `R4` §3 usa un identificador real como encabezado. **Cerrada por el hecho de que esta ficha exista** | Ficha abierta por las otras dos causas |

**No arreglado, y por qué no:** las otras dos causas son decisiones del
supervisor. La ficha QYR-0300 las detalla. En corto: una exige un identificador
fuera de mi rango y evidencia ejecutada que no tengo, y la otra es una corrección
al texto de `R4` §3.

---

## 6. A qué afectaba

**QYR-0300.** Qué se rompe: `ci.yml` ejecuta `check_docs_consistency` en Bash y en
PowerShell, así que la rama no puede pasar una puerta mientras siga rojo — y la
comprobación 11 de `R2` es exactamente ésa. Para quién: para cualquier fase que
intente cerrar su puerta, es decir, para todas. En qué escenario: el normal, desde
el primer commit.

Vale la pena decir la forma del defecto, porque es la tercera vez que este
proyecto la encuentra: **el comprobador no distingue citar un hallazgo de escribir
sobre un hallazgo.** Un documento que explica cómo se registran los bugs tiene que
nombrar identificadores para dar ejemplos, y al hacerlo los cita. Las dos veces
anteriores fueron QYR-0076 y QYR-0092.

---

## 7. Resultado contra el objetivo

| Objetivo | Resultado |
|---|---|
| Paso 0: mover los diecisiete | **Cumplido** |
| Pasos 1–2: leer las siete reglas | **Cumplido** |
| Paso 3: reproducir la línea base | **Parcial — cinco de seis** |
| Paso 4: abrir la fase 01 | **No hecho, a propósito.** `R6` §1 ordena parar |

---

## 8. Clase de evidencia por afirmación

| Afirmación | Clase | Plataforma | Evidencia |
|---|---|---|---|
| 527 passed, 0 failed, 2 ignored | Probado en unidad e integración | Linux | `cargo test --workspace`, exit 0 |
| `clippy -D warnings` limpio | Compilado | Linux | exit 0 del proceso |
| `cargo fmt --all --check` limpio | Compilado | Linux | exit 0 del proceso |
| 63 paquetes, todos de primera parte | Comprobado | — | `grep -c '^\[\[package\]\]' Cargo.lock` |
| 116 fichas, 24 abiertas | Comprobado | — | `grep -c '^## QYR-'` y `grep -c '^- Estado: abierto'` |
| `check_docs_consistency` PASS | **No reproducida.** Falla | Bash y PowerShell | exit 1, un BLOCKER |
| Cualquier cosa sobre Windows, Android, iOS o hardware físico | **Ninguna en este paso.** No se ejecutó nada fuera de Linux | — | — |

### Los seis números, lado a lado

| Comprobación | `R6` §1 declara | Medido aquí | ¿Coincide? |
|---|---|---|---|
| Tests | 527 passed, 0 failed, 2 ignored | 527 passed, 0 failed, 2 ignored | **Sí** |
| Clippy | PASS | exit 0 | **Sí** |
| Formato | PASS | exit 0 | **Sí** |
| Paquetes | 63 | 63 | **Sí** |
| Ledger | 116 entradas, 24 abiertas | 116 entradas, 24 abiertas | **Sí** |
| Coherencia docs | PASS | **exit 1** | **No** |

---

## 9. Las puertas

**Ninguna.** Este tramo no cierra una puerta: `R6` §1 ordena parar antes, y la
comprobación 11 de `R2` —`check_docs_consistency`— no pasaría de todos modos.
Declarar una puerta aquí sería exactamente lo que `00-LEEME-PRIMERO` §2 llama una
fase declarada cerrada que no lo está.

---

## 10. Tabla de mutación

No aplica. Este tramo no añade código de producción: dos fusiones, un movimiento de
archivos y una ficha.

---

## 11. Tests antes y después

Sin cambio por trabajo propio. **Antes de la fusión de `codex/qyro-trust-5d`: 468**
(`cargo test --workspace` sobre `749df9a`). **Después: 527**, que es el número que
`R6` §1 declara. Los 59 de diferencia son de la otra rama, no míos.

---

## 12. Delta de dependencias

**63 antes y 63 después**, con `grep -c '^\[\[package\]\]' Cargo.lock`. Ninguna
dependencia externa nueva: este tramo no toca `Cargo.toml`.

---

## 13. Archivos tocados

Base del tramo: `6de0af7`, el commit que trajo el plan.

```
$ git diff --name-only 6de0af7..HEAD -- . ':!docs/fase-implementacion'
```

Se listan aparte los diecisiete renombrados del paso 0 y lo que llega de la fusión,
porque tras una fusión esta lista deja de ser prueba de no solapamiento y hay que
separarla a mano.

---

## 14. Runs de CI

Ninguno lanzado por este tramo todavía. Se listarán, sin filtrar y con los fallidos
y cancelados incluidos, en la primera fase que cierre puerta.

---

## 15. Qué NO debe leerse como progreso

- **No se ha abierto la fase 01.** El FFI del motor sigue siendo dos funciones que
  devuelven la versión del protocolo. Dart no puede pedir nada.
- **No hay producto.** Los botones Enviar y Recibir siguen `onPressed: null`.
- **No se ha probado nada en hardware físico.** Ni un teléfono, ni una tablet, ni
  una máquina Windows que no sea un runner. Sigue siendo cierto y hay que decirlo
  cada vez.
- **Dos procesos en `127.0.0.1` no son dos aparatos en una Wi-Fi**: no hay pérdida
  de paquetes, ni MTU, ni suspensión de radio, ni aislamiento de cliente.
- **La identidad sigue sin persistir en Android ni en iOS.**
- Reproducir la línea base **no es haber avanzado**. Es haber comprobado desde
  dónde se sale.

---

## 16. Ledger y handoff

| ID | Sev | Título | Estado al empezar | Estado al cerrar |
|---|---|---|---|---|
| QYR-0300 | P2 | La línea base del plan declara verde una comprobación que el propio plan pone en rojo | no existía | abierto |

**Balance: 24 abiertas antes, 25 después.** Sube una, y es la que documenta por qué
este tramo para.

**Documentación desfasada:** `STATUS.md` tiene `Verified commit` movido a este
tramo. `R4` y `R6` son las que el hallazgo señala.

**Qué necesita saber la fase 01:** nada de este tramo la condiciona técnicamente.
Lo único que la bloquea es administrativo: hasta que la cita huérfana de `R4` §4 se
resuelva, **ninguna puerta puede pasar la comprobación 11 de `R2`**, y por tanto
ninguna fase puede cerrarse en regla.
