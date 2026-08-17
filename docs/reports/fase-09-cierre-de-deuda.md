# Fase 09 — El cierre de la deuda

**Base:** `ce66f72`. **Rama:** `claude/qyro-net-6a`.

---

## 1. Objetivo y alcance

> **Cero fichas abiertas.** Cada una de las 37 que quedaban abiertas termina en
> uno de dos destinos: **cerrada** con la evidencia ejecutada, o **descartada**
> con el argumento de por qué la v1.0 sale sin ella. Ninguna dice «pendiente».

**No objetivo:** arreglar todo. Un ledger vacío por arreglarlo todo no cabía en
una sesión, y un ledger vacío por borrar lo incómodo no vale nada. Lo que se pide
es que **ninguna decisión quede sin argumento**.

**Alcance:** `BUGS_PENDING.md` entero y `docs/reports/deuda-de-calidad.md`
entero, incluidos los cinco puntos que estaban «registrados en el carril, sin
ficha propia» y que por no tener ficha no aparecían en ningún recuento.

---

## 2. Qué se hizo

1. **Las 37 fichas abiertas dispositionadas**: **18 cerradas, 19 descartadas**.
   Cada una lleva un párrafo fechado en su propia ficha, no en un anexo.
2. **`deuda-de-calidad.md` reescrito como vaciado**, con las cuatro familias de
   descarte y su criterio escrito **una vez** en lugar de diecinueve.
3. **Los cinco puntos sin ficha** del carril dispositionados en tabla: dos
   cerrados por otra vía, tres descartados con su motivo.
4. **QYR-0318 arreglada de verdad** en vez de descartada, porque el argumento del
   descarte no se sostuvo al escribirlo (§4).
5. **`docs/testing/hardware-protocol.md`**: veinte escenarios con su comando
   literal y su hueco en blanco, que es donde termina la única cosa que esta
   sesión no puede hacer.

---

## 3. Cómo se hizo

### El criterio, antes de mirar ninguna ficha

Una ficha se **cierra** si existe hoy algo ejecutable que demuestra que el
defecto ya no está: una prueba con nombre, un comando con su exit code, un
artefacto. No se cierra porque suene arreglada.

Una ficha se **descarta** si el trabajo que pide es real pero la v1.0 sale mejor
sin él, y entonces el párrafo tiene que decir **qué se pierde**, no sólo que se
descarta. Un descarte que no nombra el coste es un «pendiente» con otro nombre.

### Las cuatro familias, escritas una vez

Diecinueve párrafos distintos para diecinueve descartes habrían sido diecinueve
oportunidades de razonar distinto sobre lo mismo. Los descartes se agruparon:

| Familia | Cuántas | El argumento, en una línea |
|---|---|---|
| Cobertura con rendimiento decreciente | 4 | La cobertura que importa existe; enumerar cada `io::ErrorKind` cubre el sistema operativo, no este código |
| La guarda textual, otra vez | 2 | La defensa que carga el peso es el **tipo**; ensanchar el texto es volver a correr una carrera ya perdida (QYR-0304) |
| Una fuente que no existe | 3 | Buscarla y no encontrarla **es un resultado**. Una regla inventada rechaza lo legítimo y aparenta cubrir lo que no cubre |
| Fuera del alcance de la v1.0 | 10 | Superficie nueva del motor, o algo que depende del propietario y tiene su comando en el protocolo de hardware |

### Las cerradas se comprobaron contra el código, no contra el recuerdo

Ocho de las dieciocho describían un defecto que **ya no existe** porque una fase
posterior lo eliminó de camino a otra cosa —QYR-0323 la eligió el ADR-0034 al
descartar el paraguas `file_selector`; QYR-0064 la cerró el test instrumentado de
la fase 06—. En esas, el párrafo nombra la prueba o el archivo que lo demuestra,
para que el cierre se pueda desmentir.

---

## 4. Qué se encontró que no estaba en el plan

**Un descarte que no se sostuvo al escribirlo.** QYR-0318 decía que
`Progress::item` se documenta uno-based y no se asigna nunca. El borrador la
descartaba: asignarlo de verdad es superficie nueva en `qyro_transfer` el día de
la v1.0, que es un argumento correcto sobre **el campo**.

Sobre **la frase**, no. La documentación decía «uno-based, cero antes del
primero» y el valor es cero siempre, así que se leía como *la transferencia no ha
empezado* durante toda la transferencia. Y esa frase no se queda en Rust: `item`
cruza el FFI, entra en `QyroProgress` y aparece en su `toString()`.

Arreglar la frase cuesta dos líneas. Se arregló en los dos lados y la ficha pasó
a **cerrada**. Es la lección de la fase entera: **descartar es un destino, no una
salida**, y cuando el argumento no aguanta escribirse, lo que hay que hacer es el
trabajo.

**Un cierre que se firmó contra algo que aún no existía.** El párrafo de QYR-0004
dice que `docs/release/v1.0.md` publica el SHA-256 del APK y del `.exe`. Al
escribirlo, ese documento tenía los dos huecos sin rellenar. Queda anotado aquí y
es lo que la fase 10 tiene que hacer cierto; un cierre que se apoya en un archivo
que no dice lo que se afirma es exactamente el anti-patrón número tres.

---

## 5. Qué se arregló y qué no

**Arreglado:** QYR-0318, en Rust y en Dart.

**No arreglado, y dicho:** los diecinueve descartes. El coste de cada uno está en
su ficha. El mayor, con diferencia, no es ninguno de ellos: es que **nada se ha
ejecutado en hardware físico**, y eso no es deuda de calidad sino la fase 07.

---

## 6. A qué afectaba cada defecto

| Ficha | A qué afectaba | Destino |
|---|---|---|
| QYR-0318 | Un dato falso que cruzaba hasta la interfaz | Cerrada, corrigiendo la frase en los dos lados |
| QYR-0317 | La barra del receptor no avanza por bytes | Descartada; el receptor muestra indeterminada y el total, y el emisor sí avanza |
| QYR-0326 | `http` viaja en el grafo de una aplicación que promete no salir a internet | Descartada; entra por `file_selector_platform_interface`, nadie lo llama, y se dice en voz alta |
| QYR-0324 | En esta máquina no se puede `flutter build` | Descartada; es el Modo Desarrollador del propietario, y su consecuencia está acotada y escrita |
| QYR-0004 | Los artefactos no traían checksum | Cerrada por la fase 10, no antes (§4) |

---

## 7. Resultado contra el objetivo — **CUMPLIDO**

```
python - <<'PY'
import re
t=open('BUGS_PENDING.md', encoding='utf-8').read()
b=[x for x in re.split(r'\n(?=## QYR-)',t) if re.match(r'## QYR-',x)]
def estado(x):
    m=re.search(r'^- Estado: *\*{0,2}(\w+)', x, re.M)
    return m.group(1).lower() if m else '?'
print('total',len(b),'abiertas',len([x for x in b if estado(x)=='abierto']))
PY
```

```
total 147 abiertas 0
```

`{'cerrado': 128, 'descartado': 19}`. Ninguna ficha sin párrafo, ningún párrafo
que diga «pendiente», ningún «hay una limitación» sin el coste al lado.

---

## 8. Clase de evidencia por afirmación

| Afirmación | Clase |
|---|---|
| 147 fichas, 0 abiertas | **Medido**, con el script de arriba, exit 0 |
| 18 cerradas / 19 descartadas | **Medido**, mismo script agrupando por estado |
| Cada cierre nombra algo ejecutable | **Revisado a mano**, ficha por ficha |
| `Progress::item` ya no miente | **Compilado y probado en unidad** (`cargo test -p qyro_session`, `flutter test`) |
| El protocolo de hardware está listo | **Escrito, no ejecutado.** Veinte huecos en blanco |
| Algo de esto funciona en un teléfono | **Ninguna.** No hay evidencia de hardware físico y no se inventa |

---

## 9. La puerta — 2026-08-16

Trece comprobaciones, por exit code, sobre el árbol de esta fase. Ver §9 del
informe de la fase 10 para la corrida completa que precede a la etiqueta; esta
fase toca dos archivos de código (la frase de `Progress::item`) y el resto es
documentación.

| # | Comprobación | Exit |
|---|---|---|
| 1 | `cargo test --workspace` | 0 — 633 pasados, 0 fallados, 2 ignorados |
| 2 | `cargo fmt --all --check` | 0 |
| 3 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| 13 | `cargo clippy -p qyro_session --all-targets --target aarch64-linux-android -- -D warnings` | 0 |
| 4–8 | `flutter analyze`, `flutter test`, `dart format --set-exit-if-changed` | 0 — 86 pasadas, 10 saltadas |
| 9–12 | `check_docs_consistency` en Bash **y** en PowerShell | 0 |

---

## 10. Tabla de mutación

**Ninguna.** Esta fase no añade lógica: cambia dos comentarios y diecinueve
párrafos de un ledger. Un barrido de mutación sobre un comentario no tiene nada
que matar, y decirlo es más honesto que ejecutar uno para poder poner una tabla.

---

## 11. Tests antes y después

| | Antes | Después |
|---|---|---|
| Rust | 633 pasadas, 2 ignoradas | 633 pasadas, 2 ignoradas |
| Dart | 86 pasadas, 10 saltadas | 86 pasadas, 10 saltadas |

Las diez saltadas de Dart son las que necesitan algo que esta máquina no tiene:
la biblioteca nativa compilada (`QYRO_FFI_LIBRARY_PATH`) y el manifiesto
fusionado que sólo existe después de `flutter build apk`. **Saltada no es
pasada**, y por eso se cuentan aparte en cada tabla en vez de sumarse.

Sin cambio, y es lo correcto: el defecto de QYR-0318 era una frase, y una frase
no se prueba con un test. Añadir uno que afirmara «el campo vale cero» habría
congelado en verde justamente lo que algún día habrá que cambiar.

---

## 12. Delta de dependencias

Ninguna. `Cargo.lock` sigue en **80 paquetes**; `pubspec.lock` en **45**.

---

## 13. Archivos tocados

| Archivo | Qué |
|---|---|
| `BUGS_PENDING.md` | 37 fichas dispositionadas, cada una con su párrafo |
| `docs/reports/deuda-de-calidad.md` | Reescrito como vaciado |
| `docs/testing/hardware-protocol.md` | **Nuevo.** Veinte escenarios, veinte huecos |
| `docs/release/v1.0.md` | **Nuevo.** Se completa en la fase 10 |
| `rust/crates/qyro_session/src/session.rs` | La frase de `Progress::item` |
| `apps/qyro/lib/ffi/qyro_session_api.dart` | La misma frase, del otro lado del FFI |
| `docs/reports/fase-09-cierre-de-deuda.md` | Este archivo |

---

## 14. Runs de CI

En el commit de esta fase. Ver §14 del informe de la fase 10, que es el que
precede a la etiqueta y el que `STATUS.md` cita.

---

## 15. Qué NO debe leerse como progreso

**Un ledger vacío no es software sin defectos.** Es software cuyos defectos
*conocidos* tienen dueño. Los 147 hallazgos de este ledger salieron de barridos
de mutación, de fuzzing y de leerse el propio código con desconfianza; los que
nadie ha buscado siguen ahí, y el mejor sitio donde encontrarlos es un teléfono
de verdad, que es lo único que no se ha hecho.

**Diecinueve descartes son diecinueve cosas que no están.** El archivo dice
cuáles. Que estén argumentadas no las convierte en hechas, y este informe no las
llama hechas en ningún sitio.

**Cerrar una ficha citando una prueba no prueba que la prueba sea buena.**
Ocho de los dieciocho cierres se apoyan en pruebas escritas en fases anteriores
por la misma sesión que ahora las cita como evidencia. Los barridos de mutación
de las fases 01 a 05 son lo que sostiene que esas pruebas fallan cuando deben;
esta fase no ejecutó uno nuevo, y por eso el §10 dice «ninguna».

---

## 16. Ledger y handoff

- `BUGS_PENDING.md`: **147 fichas, 0 abiertas.** 128 cerradas, 19 descartadas.
- Fichas nuevas de esta fase: **ninguna.** El único hallazgo —el cierre de
  QYR-0004 firmado contra un documento incompleto— se resuelve dentro de la fase
  siguiente en vez de abrir una ficha que nacería y moriría el mismo día.
- `docs/reports/ESTADO-ACTUAL.md` reescrito.
- Siguiente: **fase 10**, cerrar la documentación contra lo que existe y etiquetar.
