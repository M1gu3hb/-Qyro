# R12 — El diseño: azul eléctrico, vidrio, y los números que lo hacen legal

> Medido el 2026-08-19 con la fórmula de contraste de **WCAG 2.1** (relative
> luminance, SC 1.4.3 texto ≥4.5:1, SC 1.4.11 componentes ≥3:1).
> **Todos los ratios de este documento están calculados, no estimados.**
> No los reinventes: cópialos.

---

## 1. La regla que gobierna todo lo demás

El propietario pidió *Matrix / tecnológico, azul eléctrico, glassmorphism, liquid
glass, minimalista tipo Apple, que parezca hecho por una institución de
desarrolladores profesionales.*

Las cuatro cosas que hacen que un diseño así **parezca amateur** son conocidas y
las cuatro son medibles:

1. **Vidrio con texto encima que no contrasta.** Es el fallo nº1 del
   glassmorphism y §4 tiene los números exactos donde ocurre.
2. **Neón sobre negro puro.** `#000000` con acentos saturados produce *halation*
   en OLED y cansa. Por eso la base de Qyro es **`#05070C`**, no negro.
3. **Blur en todas partes.** Cada `BackdropFilter` es una pasada de GPU sobre la
   región. Cuatro apilados en una lista que hace scroll es el motivo por el que
   «la app va lenta» en un móvil de gama media.
4. **`ColorScheme.fromSeed`.** Genera 30 tonos por algoritmo y ninguno está
   verificado contra WCAG. **Se escribe a mano. Siempre.**

---

## 2. La paleta, con sus ratios reales

Base oscura única. **No hay tema claro en v2.0** — decidirlo y escribirlo es más
honesto que un tema claro sin verificar.

| Token | Hex | vs `#05070C` | vs `#0B111C` | Uso |
|---|---|---|---|---|
| `bg.base` | **`#05070C`** | — | — | fondo de la app |
| `bg.surface` | **`#0B111C`** | — | — | tarjeta, hoja, barra |
| `text.primary` | **`#E6EDF7`** | **17.10** | **16.04** | todo el texto normal |
| `text.muted` | **`#9AA7BC`** | **8.28** | **7.76** | secundario **sólido** |
| `text.dim` | **`#8B98AC`** | **6.89** | **6.47** | terciario, mínimo permitido |
| `accent.blue` | **`#2F81F7`** | **5.38** | 5.04 | relleno de botón primario |
| `accent.blueBright` | **`#5FB0FF`** | **8.77** | 8.23 | texto/icono azul sobre fondo |
| `accent.cyan` | **`#22D3EE`** | **11.15** | 10.46 | **anillo de foco**, progreso |
| `state.ok` | **`#3FD68C`** | **10.76** | 10.09 | verificado, completado |
| `state.warn` | **`#FFB454`** | **11.43** | 10.72 | huella nueva, degradado |
| `state.error` | **`#FF6B6B`** | **7.26** | 6.81 | rechazado, fallo |

### 2.1 — El error que se comete siempre

> **Texto blanco sobre `accent.blue` da 3.75:1. Suspende SC 1.4.3.**
> La etiqueta del botón primario **es `#05070C` (5.38:1)**, casi negro sobre azul.

Es contraintuitivo, se ve mejor, y es la diferencia entre un botón que pasa una
auditoría y uno que no. Escríbelo en el token: `on.accent = #05070C`.

### 2.2 — El aire Matrix, sin disfraz

Matrix no es «verde». Es **monoespaciado, alineación de rejilla y densidad de
información**. En Qyro eso ya existe y sólo hay que **enseñarlo**:

- La huella Ed25519 en **JetBrains Mono**, agrupada de 4 en 4 — ya hay
  `HumanFingerprint::to_grouped_hex`.
- El código de emparejamiento `QYRO1|…` en monoespaciado, **con los cuatro campos
  coloreados distinto**: prefijo `text.dim`, ip `text.muted`, puerto `accent.cyan`,
  huella `accent.blueBright`. Un formato que se explica solo al mirarlo.
- Los bytes/segundo en monoespaciado con **ancho tabular**, para que el número no
  baile mientras sube.
- **Cero lluvia de caracteres, cero terminal falsa, cero verde fosforito.** Eso es
  disfraz. Lo que da el aire es la tipografía y la rejilla.

### 2.3 — Tipografía

**Inter** (SIL OFL 1.1) para interfaz, **JetBrains Mono** (SIL OFL 1.1) para todo
lo que sea un dato verificable: huellas, códigos, rutas, hashes, tamaños.
Las dos licencias permiten empaquetado; **la licencia se copia al repo** junto a
los `.ttf`, y se declara en `pubspec.yaml` con subsetting activado.

**La regla semántica, y es la buena:** *si una persona lo va a comparar carácter a
carácter con otra pantalla, va en monoespaciado.* Esa frase decide cada caso sin
discusión.

---

## 3. Escala, ritmo y forma

- **Rejilla de 4 px.** Espaciados: `4 8 12 16 20 24 32 40 56`. Nada fuera.
- **Radios:** `8` control pequeño · `12` botón/campo · `16` tarjeta · `24` hoja ·
  `999` píldora. Cuatro valores y un caso especial.
- **Tipos:** `display 32/38 w600` · `title 22/28 w600` · `body 15/22 w400` ·
  `label 13/18 w500` · `mono 13/20 w400`.
- **Objetivo táctil ≥48×48 dp** en Android (SC 2.5.8 pide 24; Material pide 48 y
  gana el más estricto). En Windows, ≥32×32 px con puntero.
- **Movimiento:** `120 ms` estados, `220 ms` transiciones, `340 ms` hoja. Curva
  única `Curves.easeOutCubic`. **Y `MediaQuery.disableAnimations` respetado en
  un solo sitio**, no en cada widget.

---

## 4. El vidrio, y dónde exactamente deja de ser legal

Aquí están los números que casi nadie calcula. Vidrio = relleno `bg.surface`
con alpha α + `BackdropFilter`. Se mide el **peor caso**: fondo blanco detrás.

| α del relleno | Compuesto | `text.primary` | `text.muted` | `accent.blueBright` |
|---|---|---|---|---|
| 0.60 | `#6D7077` | 4.21 ❌ | 2.04 ❌ | 2.16 ❌ |
| 0.72 | `#4F545C` | **6.47** ✅ | 3.13 ❌ | 3.32 ❌ |
| 0.84 | `#323740` | **10.15** ✅ | **4.91** ✅ | **5.21** ✅ |

**Las tres reglas que salen de esa tabla:**

1. **α mínimo global = 0.72.** Por debajo ni el texto primario pasa.
2. **Sobre vidrio con contenido arbitrario detrás (una foto, una miniatura, la
   pantalla de otra app) sólo se pone `text.primary`.** `text.muted` y el azul
   **están prohibidos ahí**. Si el diseño los pide, α sube a **0.84**.
3. **Sobre el propio fondo de la app el vidrio es gratis:** compuesto `#090E18`,
   texto 16.39, muted 7.93. Todo pasa. Es decir: **el vidrio decorativo interno no
   tiene problema; el vidrio sobre contenido ajeno sí.** Y el segundo caso es
   exactamente la UI flotante de recepción de la FASE 27.

### 4.1 — El borde del vidrio

Un borde blanco al 22 % sobre `bg.surface` da **1.97:1** — **suspende SC 1.4.11**
si es el único indicador del borde del componente. Hacen falta **α 0.34**
(3.08:1). Solución adoptada: **borde a `#FFFFFF` α 0.34 en el lado superior e
izquierdo** (el gradiente de luz que hace que parezca vidrio) y α 0.10 en el
resto, decorativo. El que cuenta para la norma es el de arriba.

### 4.2 — El coste, y el único blur permitido

`BackdropFilter` en Flutter guarda la capa detrás y la vuelve a componer. En una
lista, **cada tarjeta con blur es una pasada**. Regla dura:

> **Como máximo DOS regiones con blur simultáneas en pantalla**, y las tarjetas de
> una lista **no llevan blur**: llevan relleno sólido `bg.surface` más el borde de
> §4.1. Se ve igual y no cuesta nada.
> Cuando haya varias superficies contiguas con blur, **`BackdropFilter.grouped`**
> (Flutter 3.35+, y el proyecto está en 3.44.8) las compone en **una sola** pasada.

Y **`ImageFilter.blur(tileMode: TileMode.decal)`**, no el modo por defecto: el
`clamp` estira el borde y produce el halo sucio del glassmorphism mal hecho.

### 4.3 — «Liquid glass»

Lo que hace que un vidrio parezca líquido no es más blur: es **la refracción del
borde** — un realce de 1 px que sigue la curva y se aclara donde la luz pega. En
Flutter se consigue con un `ShaderMask` o, más barato y sin shader, con un
`Container` de borde en gradiente. **Empieza sin shader.** Si el propietario lo ve
y quiere más, hay un `FragmentProgram` después; pero un shader que no compila en
el backend de Impeller de Windows es un riesgo que no hace falta correr el primer
día.

---

## 5. Cómo se implementa, para que no se pudra

1. **`ThemeExtension<QyroTokens>`** con los ~48 tokens de §2 y §3. **Ningún color
   literal fuera de ese archivo.** La puerta lo comprueba: un `grep` de
   `Color(0x` fuera de `lib/design/` **falla la build**. Es la única forma de que
   esto siga siendo cierto dentro de tres sesiones.
2. **`ColorScheme` escrito a mano.** `fromSeed` prohibido, con el motivo en la ADR.
3. **Widgets primitivos** en `lib/design/`: `QyroSurface`, `QyroGlass`,
   `QyroButton`, `QyroField`, `QyroBadge`, `QyroProgress`, `QyroMono`,
   `QyroFingerprint`. Las pantallas **no dibujan**, componen.
4. **Prueba de contraste automatizada.** Una prueba de Dart que recorre los pares
   (color, fondo) declarados en los tokens, calcula el ratio con la fórmula de
   WCAG y **falla si alguno baja del umbral**. Es 60 líneas y convierte esta tabla
   en algo que no se puede romper por accidente.
   **Y su contraprueba:** bajar un token a propósito debe poner la prueba en rojo.

---

## 6. Accesibilidad: el diferenciador barato

`R11` §2.10 lo dice: **el líder de la categoría tiene una issue de accesibilidad
abierta y sin respuesta.** Lo que cuesta poco y cierra el hueco:

- `Semantics` con etiqueta en **cada** control; nada que se toque sin nombre.
- **El estado no se comunica sólo por color** (SC 1.4.1): rechazado lleva icono,
  verificado lleva icono, en curso lleva icono. El color acompaña, no informa.
- Se anuncia el cambio de estado de la transferencia con `SemanticsService.announce`
  — TalkBack y NVDA lo leen.
- **Prueba:** un test de widget que recorre el árbol y **falla si un nodo
  interactivo no tiene `Semantics.label`.** Igual que la de contraste: automática o
  no existe.

---

## 7. Los dos riesgos, nombrados

1. **Impeller en Windows.** El backend de Flutter para Windows cambió; `BackdropFilter`
   apilado con `saveLayer` es el camino donde más regresiones ha habido. **Mitigación:
   medir con `--profile` y el overlay de rendimiento en la pantalla más pesada, y
   apuntar el número.** Si baja de 60 fps, el vidrio de esa pantalla pasa a sólido.
   No se discute: se mide y se decide.
2. **El vidrio sobre contenido ajeno** de §4.2. La UI flotante de recepción es
   literalmente eso. Ahí α = 0.84 y sólo `text.primary`. **Está decidido aquí para
   que la FASE 27 no lo reabra.**
