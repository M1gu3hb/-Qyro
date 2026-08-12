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

Pendiente.

## 5. Puerta 3 — supervivientes importantes

Pendiente.

## 6. Puerta 4 — ADR-0031

Pendiente.

## 7. Puerta 5 — confianza implementada

Pendiente.

## 8. Puerta 6 — historial local

Pendiente.

## 9. Puerta 7 — barrido y guardas

Pendiente.

## 10. Puerta 8 — documentación y CI final

Pendiente.

## 11. Runs de CI de la rama

Todavía no existe ningún run de `codex/qyro-trust-5d`.

## 12. Qué no debe leerse como progreso

No hay red, sockets, descubrimiento, FFI nuevo, UI, selector ni política
interactiva de emparejamiento. Nada de este sprint se ha probado en hardware
físico. El mecanismo de confianza y el historial aún no existen al abrir este
informe.
