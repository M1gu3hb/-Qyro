# R4 — Cómo registrar y resolver hallazgos

`BUGS_PENDING.md` es **el instrumento con el que este proyecto sabe qué está
pendiente**. `STATUS.md`, `HANDOFF.md` y `check_docs_consistency` se apoyan en él.

Su valor entero es que **una persona lo lee de un tirón y sabe qué queda**. Todo
lo de abajo protege eso.

---

## 1. La lección que costó un P1

El 2026-08-11 se volcó un barrido de `cargo-mutants` entrada por entrada. El
ledger pasó de **71 a 279 fichas** y de **20 a 167 abiertas**, con títulos como
«Superviviente de mutación 022 en qyro_manifest» y **doce *timeouts* archivados
como deuda P2**.

Nada de eso era falso. **Y aun así el ledger dejó de servir**, porque nadie —ni el
supervisor— podía leerlo para saber qué quedaba. Una lista correcta que ha dejado
de ser útil es una garantía que ya no garantiza.

Se reparó: hoy son **116 fichas, 24 abiertas**, y el barrido vive en
`docs/reports/mutation-sweep-2026-08-11.md` con 1 172 líneas y su alcance
declarado.

**Regla que sale de ahí, y no se negocia:**

> **La salida de una herramienta va a un informe. Al ledger van fichas escritas a
> mano, con título que una persona entienda y severidad juzgada.**

---

## 2. Qué merece una ficha

**Sí:**

- Un defecto de comportamiento, alcanzable, en código que existe.
- Un control de seguridad o integridad **sin prueba que lo cubra**.
- Una divergencia entre la documentación y el código.
- Una decisión aplazada a propósito, con su motivo.
- Una garantía que sólo está verificada en una plataforma, cuando se afirma en
  varias.
- Una familia de supervivientes de mutación **agrupada por causa**.

**No:**

- Un mutante de `Display`, `Debug` o formateo.
- Un mutante equivalente o `unviable`.
- Un `timeout` **por sí solo** (ver `R3` §4): va una ficha por conclusión, no una
  por timeout.
- Una tarea del plan. Las fases están en este directorio, no en el ledger.
- Una idea de mejora sin defecto detrás.

---

## 3. El formato

```markdown
## QYR-0300 — Un título que se entiende sin abrir nada

- Plataforma: Linux; qyro_net
- Severidad: P2
- Esperado: el listener rechaza una conexión cuando el presupuesto está agotado
- Actual: el contador de pendientes no baja si el hilo muere antes del `Drop`
- Resolución: pendiente; requiere un contador basado en RAII sin `Drop` implícito
- Estado: abierto
- Fecha: 2026-08-12
- Evidencia: cargo-mutants, mutante `qyro_net/src/listener.rs:88:9`; reproducido
  con `cargo test -p qyro_net a_peer_that_opens_connections...`
```

**El título es lo que más importa.** Si necesita una herramienta para entenderse,
está mal escrito.

**Los identificadores son consecutivos y no se reutilizan nunca.** El siguiente
libre es **`QYR-0300`**. Comprueba antes de asignar:

```
grep -oE '^## QYR-[0-9]{4}' BUGS_PENDING.md | sort -u | tail -3
```

---

## 4. Las severidades

**No las exageres.** El proyecto ya sufrió 31 P1 asignados por regla en un solo
sprint, y eso destruye la escala.

| Sev | Criterio | Ejemplos reales |
|---|---|---|
| **P0** | Bloquea el milestone actual **o crea una garantía de seguridad falsa** que alguien podría creerse hoy | Un bucle infinito alcanzable desde el cable; una clave que llega a Dart |
| **P1** | Deuda grave: un control de seguridad sin cobertura, una plataforma sin evidencia cuando se afirma que la hay, algo que bloquea la fase siguiente | QYR-0073 (`O_NOFOLLOW` sin ninguna prueba); QYR-0064 (el harness de Keystore); QYR-0288 (el ledger ilegible) |
| **P2** | Mejora importante o hueco de cobertura sin consecuencia de seguridad | QYR-0089 (`TransferReject` que nadie emite) |
| **P3** | Menor, cosmético, o deuda de documentación | QYR-0057 (tres fichas con un `Estado` que no es un estado) |

**Un P1 se justifica por escrito en la propia ficha.** Si no puedes explicar en una
frase por qué es grave, es P2.

---

## 5. Los estados

Sólo tres, y **son estados, no narraciones**:

- `abierto` — no está resuelto.
- `cerrado` — está resuelto **y hay prueba que lo demuestra**.
- `descartado` — se decidió no arreglarlo, con el motivo en `Resolución`.

**Nunca** «abierto al inicio de este tramo» ni variantes. Eso ya es QYR-0057.

**Y `cerrado` exige evidencia ejecutada.** Una ficha cerrada dice **qué mutación se
aplicó y qué test falló**, o no está cerrada.

---

## 6. Cómo se resuelve un hallazgo

1. **Reprodúcelo primero.** Un arreglo cuyo defecto no reprodujiste es un arreglo
   que no sabes si arregla algo. Anota el comando y el resultado.
2. **Escribe la prueba que falla**, con nombre que enuncie la propiedad.
3. **Arregla el código.**
4. **Comprueba que la prueba pasa** — y que **falla** si vuelves a introducir el
   defecto.
5. **Cierra la ficha** con la evidencia de los pasos 1 y 4.

**Lo que no vale como «arreglado»:**

- Un test que pasa, sin más.
- Renombrar una prueba sin cambiar lo que ejerce.
- Un `assert!` más en la misma prueba defectuosa. **Bórrala y escribe otra.**
- Un `#[allow(...)]` para que una guarda no se queje.
- «No pude probar Windows» sin registrar que la garantía queda sin verificar allí.

---

## 7. Cuándo NO se arregla

**Registrar y seguir es una respuesta correcta y frecuente.** Lo que no vale es
callarlo.

Se registra y no se arregla cuando:

- está fuera del alcance de la fase **y no la bloquea**;
- arreglarlo exige ensanchar una superficie congelada, y eso es una ADR propia;
- arreglarlo exige una dependencia nueva, y eso es una decisión con su
  justificación;
- la evidencia para arreglarlo bien no existe todavía —por ejemplo, hace falta
  hardware físico—.

**En los cuatro casos la ficha dice explícitamente qué haría falta para
cerrarla.**

---

## 8. La higiene del ledger, por puerta

En cada puerta (`R2` §1.10):

- **Cuenta las abiertas.** Si la fase añadió más de diez fichas, **la fase está
  mal hecha**, no el ledger.
- **Ninguna ficha ajena editada.** Sólo se cierran las que tú resolviste.
- **Ningún identificador repetido.** `check_docs_consistency` lo comprueba, pero
  míralo tú.
- **Todo `QYR-00xx` que cites en cualquier archivo tiene ficha.** Incluye los
  informes, los comentarios de código y las ADR.

---

## 9. Al final de cada fase

Escribe en el informe una tabla, no una prosa:

| ID | Sev | Título | Estado al empezar | Estado al cerrar |
|---|---|---|---|---|

Y una línea con el balance: **abiertas antes, abiertas después, y por qué subió o
bajó.**
