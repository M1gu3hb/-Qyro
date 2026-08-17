# Deuda de calidad — **vaciada**

**Qué era este archivo.** Desde el 2026-08-14 rigió la regla del carril: sólo un
P0 detenía una fase, y todo lo demás se registraba aquí para arreglarse en la
fase 09. La regla funcionó: tres sesiones seguidas se habían consumido enteras en
hallazgos de calidad reales mientras el producto no se movía, y con el carril el
producto se movió.

**La fase 09 vacía la lista. No la hereda.** Este archivo ya no tiene entradas
abiertas, y `BUGS_PENDING.md` tampoco: **147 fichas, 0 abiertas**, contadas con el
script de `R2` §1.10.

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

---

## Cómo se vació

**Dos destinos y ninguno más:** `cerrado` con la evidencia ejecutada, o
`descartado` con el argumento de por qué la v1.0 sale sin ello. **18 cerradas, 19
descartadas.** Cada una lleva su párrafo en su propia ficha, fechado, y ninguno
dice «pendiente».

Una de ellas cambió de destino al escribirla. QYR-0318 —«`Progress::item` se
documenta uno-based y no se asigna nunca»— iba a descartarse porque asignarlo de
verdad es superficie nueva del motor. Escribir el argumento enseñó que el defecto
**no era el campo, era la frase**: decía «cero antes del primero» y valía cero
siempre, así que leía «la transferencia no ha empezado» durante toda la
transferencia, y esa frase cruza el FFI hasta Dart. Arreglar la frase cuesta dos
líneas. **Descartar es un destino, no una salida**, y cuando el argumento no se
sostiene lo que hay que hacer es el trabajo.

Las cuatro familias de descarte, con el criterio escrito una vez:

| Familia | Cuántas | El argumento |
|---|---|---|
| **Cobertura con rendimiento decreciente** | 4 — QYR-0290, 0292, 0294, 0296 | Un contrato de frontera por constante sobre un decodificador que ya resiste seis targets de fuzzing y 281 mutantes. La cobertura que importa existe; enumerar cada `io::ErrorKind` es cubrir el sistema operativo, no este código |
| **La guarda textual, otra vez** | 2 — QYR-0056, 0090 | Una guarda de texto siempre pierde contra la sintaxis. La defensa que carga el peso es el **tipo** (`VerifiedPayload`), y ensanchar el texto sería volver a correr una carrera ya perdida una vez (QYR-0304) |
| **Una fuente que no existe** | 3 — QYR-0029, 0034, 0065 | Buscar la fuente primaria y no encontrarla **es un resultado**. Adivinar la regla sería peor que su ausencia: una regla inventada rechaza lo legítimo y aparenta cubrir lo que no cubre |
| **Fuera del alcance de la v1.0** | 10 | Superficie nueva del motor, herramientas que ningún hallazgo ha necesitado, o cosas que dependen del propietario y tienen su comando escrito en el protocolo de hardware |

---

## Lo que quedaba «registrado en el carril, sin ficha propia»

| Qué | Disposición |
|---|---|
| La guarda textual de `into_zeroizing_payload` no cubre `deref` ni variables intermedias | **Descartado.** `VerifiedPayload` es la defensa; la guarda textual es cosmética y se dijo así al escribirla |
| `cargo doc -D warnings` no está en la puerta | **Descartado.** Un enlace intra-doc roto no cambia comportamiento, y la puerta tiene ya trece comprobaciones que sí |
| No hay job que corra `check_docs_consistency.ps1` en `windows-latest` | **Cerrado por otra vía.** La edición PowerShell se ejecuta en **cada puerta de esta sesión**, en un Windows real, con PowerShell 5.1 — que es una plataforma más hostil que `windows-latest` y donde QYR-0311 apareció |
| `assert_analysis_reached_the_end` compara la última línea, que en un `.rs` es `}` | **Cerrado.** Lo que escondía era QYR-0328, y eso se arregló en su causa: `item_end` salta ahora los literales de carácter. La comprobación vacua sigue siendo cierta y ya no es la única |
| El chequeo de formato de Dart no cubre `tools/branding_generator` | **Descartado.** `branding_generator` es una herramienta de compilación que genera un archivo que sí está formateado y sí está comprobado por `branding_generator_test.dart`. Formatear la herramienta no cambia su salida |

---

## Lo que este archivo prueba, y lo que no

**Prueba** que ninguna decisión quedó sin argumento: cada una de las 37 fichas
que estaban abiertas al empezar la fase 09 tiene hoy un párrafo que dice qué se
hizo o por qué no.

**No prueba** que el software no tenga defectos. Prueba que **los defectos que
este proyecto conocía están resueltos o descartados a propósito**, que es lo
único que un ledger puede prometer.

Y sigue habiendo una cosa sin evidencia, dicha aquí porque es la más importante
de todas: **nada se ha ejecutado en hardware físico.** Eso no es deuda de
calidad; es la fase 07, necesita dos aparatos y una persona, y está lista para
ejecutarse en `docs/testing/hardware-protocol.md`.
