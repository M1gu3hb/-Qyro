# R2 — El protocolo de puerta

**Una puerta es la auto-auditoría que cierra un paso.** No pasas al siguiente
hasta que las doce comprobaciones pasan.

**Cada fase tiene al menos una puerta al final, y las fases largas tienen puertas
intermedias** — cada documento de fase dice dónde están las suyas.

---

## 1. Las doce comprobaciones

### 1 — Formato

```
cargo fmt --all --check
```

**Lee el código de salida del proceso, no la salida de texto.** Este proyecto ya
produjo dos falsos verdes por leer mal la salida de un comando: `&&` después de
una tubería lee el estado de `tail`, no el de `rustfmt`; y un `grep -c` que
devolvía «4» se leyó como informativo cuando el código de salida real era 101.

### 2 — Clippy

```
cargo clippy --workspace --all-targets -- -D warnings
```

Por código de salida. **En Linux siempre; en Windows también**, que el job
`rust-windows` ya existe.

### 3 — Tests

```
cargo test --workspace
```

PASS, **sin ignorados nuevos**. Un test ignorado es un test que no existe.

### 4 — Barrido de mutación de la fase

Por cada propiedad que la fase declare probada: **aplica la mutación que debería
romperla, confirma que falla un test con nombre, restaura.**

- Usa `cargo-mutants`, que ya está adoptado. **Con límite de tiempo por mutante.**
- **Si un mutante cuelga en vez de fallar, eso es un resultado que investigar**,
  no un fallo del barrido. La pregunta siempre es: *¿puede un peer producir esta
  condición?*
- **Si un control sobrevive a su propio borrado, la fase no está terminada.**
- **La salida del barrido va al informe** (`docs/reports/fase-NN-*.md`), con su
  alcance declarado: cuántos mutantes de cuántos, caught / missed / unviable /
  timeout, por crate. **Nunca al ledger** (`R4`).

### 5 — Lectura de aserciones

Lee cada `assert!`/`assert_eq!`/`assert_ne!` nuevo y comprueba que **los dos lados
pueden diferir**. La guarda `assert_no_assertion_compares_a_call_to_itself` caza
el caso literal; léelas igualmente, porque la guarda no ve
`f(x) == g(x)` cuando `f` y `g` acaban en la misma llamada.

### 6 — Lectura de contadores

Si la fase añadió un contador bajo `cfg(test)`, comprueba dos cosas:

1. **Registra un valor derivado de la operación**, no una constante.
2. **La forma de la prueba distingue un contador medido de una constante.** Si una
   constante satisface tus aserciones, la prueba está mal aunque el contador esté
   bien. La forma que sí distingue: dos tamaños y una desigualdad estricta.

### 7 — La medida se ve fallar

**Por cada medición nueva, una prueba que provoque a propósito lo que la medición
debería detectar.**

El modelo es `a_descriptor_leak_would_be_visible_to_this_measurement`: filtra
cuatro descriptores adrede y comprueba que el contador los ve. *Una medida que no
puede ver una fuga no es evidencia de que no la haya.*

### 8 — Lectura de nombres

Por cada test nuevo o renombrado: **¿el cuerpo ejerce lo que el nombre dice?**
`a_symlink_at_the_final_component_is_refused` no abría ningún archivo.

### 9 — Coherencia del informe

**Relee las secciones del informe que esta fase pudiera haber invalidado** —
conteos, tablas, listas de archivos, clases de evidencia, cualquier «pendiente» de
algo ya hecho — y **corrígelas contra el código actual, no contra tu memoria**.

Un informe donde §4 dice 63 y §12 dice 62 es un informe en el que no se puede
confiar. Ya pasó.

### 10 — El ledger sigue legible

Cuenta las entradas abiertas:

```
python3 - <<'PY'
import re
t=open('BUGS_PENDING.md', encoding='utf-8').read()
b=[x for x in re.split(r'\n(?=## QYR-)',t) if re.match(r'## QYR-',x)]
def estado(x):
    m=re.search(r'^- Estado: *\*{0,2}(\w+)', x, re.M)
    return m.group(1).lower() if m else '?'
print('total',len(b),'abiertas',len([x for x in b if estado(x)=='abierto']))
PY
```

*(Corregido el 2026-08-13, QYR-0313. Tenía **tres** defectos en ocho líneas.)*

1. *El patrón era `- Estado: *abierto`, que no casa con `- Estado: **abierto**`.
   Cuatro fichas lo escriben en negrita, así que devolvía 32 donde la verdad
   eran 36.*
2. *`open()` sin `encoding` usa la página de códigos del sistema, así que en
   Windows moría sobre un ledger lleno de acentos: la comprobación 10 no se podía
   correr en la misma plataforma donde la 11 resultó estar rota.*
3. *Y buscaba por **subcadena en todo el bloque** en vez de leer el campo. Una
   ficha que cita el texto `- Estado: abierto` dentro de su prosa —por ejemplo,
   la ficha que documenta este mismo defecto— se contaba a sí misma como abierta
   estando cerrada. Ahora lee el campo, anclado a principio de línea con `re.M`,
   y se queda con la primera coincidencia.*

*El tercero apareció al cerrar QYR-0312 y QYR-0313: el conteo bajó de 45 a 44
cuando tenían que ser 43, y la ficha que sobraba era la que hablaba del script.
**Un contador que se equivoca al contar su propia corrección es la forma más
barata que hay de descubrir que buscaba la cosa equivocada.***

**Si la fase añadió más de diez fichas, mira por qué antes de aceptarlo.** Ese
techo existe **para que nadie vuelque salida de herramienta en el ledger**, que
es lo que costó un P1 el 2026-08-11. **No es un techo para hallazgos de auditoría
escritos a mano.** Si auditando encuentras doce defectos reales, son doce fichas:
regístralas todas y escribe en la puerta que pasa el criterio de `R4` §2, que
supera el de aquí, y por qué. **Lo que no vale es dejar de buscar para no pasar
de diez.**

Y toda ficha tiene que tener un título que una persona entienda sin abrir una
herramienta.

### 11 — Coherencia documental

```
bash scripts/check_docs_consistency.sh
pwsh scripts/check_docs_consistency.ps1
```

Los dos. Acuérdate de las dos reglas que muerden: `Verified commit` a **diez
commits o menos** de HEAD, y **todo `QYR-00xx` citado tiene ficha**.

### 12 — Escribir el resultado

**Escribe el resultado de la puerta en el informe de la fase antes de empezar el
paso siguiente.** Fecha, las doce comprobaciones con su veredicto, la tabla de
mutación del paso, y lo que encontraste.

---

## 2. Qué hacer cuando una puerta falla

**Arréglalo y repite la puerta entera.** No la parchees en el paso siguiente, y no
declares el paso cerrado «con una salvedad».

**Si no puedes arreglarlo:**

1. **Para.**
2. Escribe en el informe **qué comprobación falló, por qué, y qué hace falta**.
3. **Deja la rama en verde** — revierte lo que haga falta para que el árbol
   compile y pase.
4. Sigue con la fase siguiente **sólo si es independiente**; si no, para del todo.

**Parar y reportar es una respuesta correcta y ya se usó dos veces con acierto en
este proyecto.** Improvisar un arreglo que dé verde sin probar nada, no.

---

## 3. La distinción que hay que saber hacer

Cuando una mutación no produce un fallo limpio, hay **tres** resultados posibles y
no son lo mismo:

| Resultado | Qué significa | Qué hacer |
|---|---|---|
| **Muerto** | Un test falló con nombre | Nada. La propiedad está cubierta |
| **Superviviente** | La suite quedó en verde | **La propiedad no está cubierta.** Escribe la prueba o registra la ficha |
| **Cuelgue** | El comportamiento cambió de forma observable, pero no como un fallo | **No es un superviviente.** La propiedad está cubierta y lo que falla es la forma de fallar. Investiga si el bucle es alcanzable desde entrada de un peer; si lo es, es un hallazgo de seguridad, y si no, es una guarda de progreso de cinco líneas |

Confundir «cuelgue» con «superviviente» infla el trabajo. Confundirlo con «muerto»
esconde un posible bucle infinito.

---

## 4. Puerta de fase contra puerta de paso

- **Puerta de paso** — dentro de una fase, al cerrar un bloque de trabajo. Las
  doce comprobaciones, resultado escrito en el informe.
- **Puerta de fase** — al final. Las doce **más**:
  - los criterios de aceptación del documento de la fase, uno a uno, con su
    veredicto;
  - **todos los workflows en verde sobre el commit final**, con sus IDs, y la
    tabla de runs exhaustiva;
  - `STATUS.md`, `HANDOFF.md`, `NEXT_STEPS.md`, `CHANGELOG.md` y
    `BUGS_PENDING.md` al día;
  - el informe de la fase completo según `R5`.

**Sólo después de una puerta de fase se abre el documento de la fase siguiente.**
