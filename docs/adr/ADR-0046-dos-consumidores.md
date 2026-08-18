# ADR-0046 — Dos consumidores del motor

**Estado:** congelada · **Fecha:** 2026-08-18 · **Fase:** 21
**Fuentes:** ADR-0042 §2 · `R7` §2 · `R8` §4 y §5.1

---

## 1. El hueco que esta ADR cierra

Desde la fase 13 el motor tiene **dos consumidores**: la GUI de Flutter, que
cruza el FFI, y el CLI de Rust, que llama a `qyro_session` directamente.
ADR-0042 §2 lo aceptó y escribió la consecuencia — *«una capacidad no está hecha
hasta que los dos la alcanzan»*.

**Y la costura entre ellos no se ha ejercitado nunca.**

Es exactamente el hueco que produjo los cinco defectos de este proyecto:
`KeystoreWrapper`, `qyro_session_local_address`, `Session::finish`, `history()` y
el descubrimiento. **Los cinco tenían las dos mitades probadas y el medio jamás
recorrido.** `Session::finish` se encontró porque alguien puso por primera vez un
receptor de Dart contra un emisor real; no lo encontró leer código.

---

## 2. Decisión 1 — qué significa «una capacidad existe»

> **Una capacidad existe cuando los dos consumidores la alcanzan, o cuando está
> escrito, en un documento de producto, que es de uno solo y por qué. Nada queda
> en el medio.**

Se adopta la propuesta del documento de fase sin cambiarla, porque el estado que
prohíbe —«existe a medias y nadie lo dijo»— es literalmente el estado en el que
estuvieron las cinco capacidades muertas. Ninguna estaba decidida; todas estaban
**sin decidir**, que es lo que las hizo invisibles.

**Una celda vacía en la tabla de §3 no es un olvido: es un incumplimiento.** Se
llena o se argumenta.

---

## 3. Decisión 2 — la tabla vive en un archivo y la comprueba un script

`docs/PARIDAD-GUI-CLI.md`, y `scripts/check_parity.ps1` la lee y **sale distinto
de cero** si alguna celda está vacía o dice «sí» sin apuntar a un llamante.

**Por qué un script y no prosa:** una tabla en prosa se desincroniza del código en
la primera semana y sigue leyéndose como verdad. Este taller ya tiene el
precedente — la fase 11 anotó en su informe que `qyro_session_local_address` no
tenía llamante y **la observación se quedó ahí** hasta que la fase 12 tropezó con
ella. Un informe no comprueba nada; un código de salida sí.

**La tabla se ve fallar antes de creerla.** Con una fila borrada a propósito, el
script tiene que ponerse rojo. Una comprobación que nadie ha visto fallar no es
una comprobación (QYR-0304).

---

## 4. Decisión 3 — el consejero de canal vive en `qyro_session`

Las fases 14, 15 y 16 le dicen cada una algo distinto al usuario sobre qué camino
usar. **Son tres interfaces contradictorias esperando a existir.**

**Un solo módulo decide, y las dos caras lo llaman.** Vive en `qyro_session`
porque es lo único que las dos alcanzan: la GUI por el FFI y el CLI directamente.
Ponerlo en el CLI lo dejaría fuera de la GUI; ponerlo en Dart lo dejaría fuera
del CLI; escribirlo dos veces es la definición del problema.

**El orden, y es fijo:**

| # | Canal | Cuándo | Velocidad (`R8`) |
|---|---|---|---|
| 1 | **Red compartida** | hay una | ~10 MB/s, 1 MB «al instante» |
| 2 | **Cable directo** | hay dos máquinas y un cable ethernet | igual, tras la espera de APIPA |
| 3 | **Serie** | la otra máquina no puede leer un QR | **9–11 KB/s** |
| 4 | **Óptico** | no hay cable de ninguna clase | **~8 KB/s** |

Serie **por delante** de óptico aunque los dos sean lentos: es un orden de
magnitud más rápido y no necesita que nadie sostenga un teléfono durante minutos.
El óptico es el último porque es el único que **no necesita cable ninguno**, y
por eso es el que queda cuando no queda nada.

**Y antes de proponer cualquiera de los dos lentos, lo aburrido** (`FASE-16` §2):
¿esa máquina tiene CD, disquetera, PCMCIA o tarjeta de red? Cualquiera es entre
10 y 10 000 veces más rápida. **Proponer el canal lento sin descartar los rápidos
es mal producto**, por bien que funcione el lento.

**La estimación va con la propuesta, no después.** Un canal que dice «6–17
minutos» antes de empezar es un canal que alguien puede rechazar; uno que lo
descubre a los diez minutos ya le gastó diez minutos.

---

## 5. Decisión 4 — el motor devuelve la frase, no un código

**Un error que en la GUI dice «la clave de este aparato ha cambiado» y en el CLI
dice `code -7` son dos productos.**

| Qué | Dónde vive | Por qué |
|---|---|---|
| **Decisiones** — rechazos, consejo de canal, estimaciones | **el motor, ya formadas** | las dos caras tienen que decir lo mismo |
| **Cromo** — títulos, botones, navegación | `.arb` de Flutter | una terminal y una pantalla táctil no se parecen y no deben |

Es el precedente de `HumanFingerprint::to_grouped_hex`, que ya existe por esta
razón exacta: dos aparatos que dibujaran la misma huella distinta harían que
leerla en voz alta no significara nada.

**Lo que esto NO decide:** el idioma. El motor devuelve la frase en inglés y la
GUI la traduce si quiere; traducir en el núcleo obligaría a meter un catálogo en
el binario portátil. Queda anotado como deuda con dueño, no como olvido.

---

## 6. Lo que esta ADR NO decide

- **Que el CLI cruce el FFI.** ADR-0042 §2 decidió lo contrario con argumento.
  Esta fase mide la consecuencia; no la revierte.
- **Unificar las interfaces.** Se unifican **las decisiones y los textos**, no la
  forma.
- **Añadir capacidades para llenar la tabla.** Una celda que dice «la GUI no
  necesita esto, y por qué» es una respuesta completa.
