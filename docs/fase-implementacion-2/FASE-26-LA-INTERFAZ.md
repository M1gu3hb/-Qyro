# FASE 26 — La interfaz

> El propietario pidió: **azul eléctrico, aire Matrix, glassmorphism / liquid glass,
> minimalista tipo Apple, y que parezca hecha por una institución de
> desarrolladores profesionales.** `R12` ya tiene los colores, los ratios medidos y
> las reglas del vidrio. **Esta fase los implementa; no los reabre.**

---

## 1. El sistema, antes que las pantallas

Nada de esto se toca hasta que exista `lib/design/`:

1. **`ThemeExtension<QyroTokens>`** con los ~48 tokens de `R12` §2 y §3, y un
   **`ColorScheme` escrito a mano**. `fromSeed` prohibido, con el motivo en la ADR.
2. **La guarda que hace que dure:** un `grep` de `Color(0x` fuera de `lib/design/`
   **rompe la build**. Sin ella, dentro de dos sesiones hay literales por todas
   partes. **Comprobación 19 de la puerta.**
3. **La prueba de contraste**: recorre los pares (color, fondo) declarados,
   calcula el ratio con la fórmula de WCAG 2.1 y **falla por debajo del umbral**.
   Y **su contraprueba**: bajar un token a propósito la pone en rojo.
4. **La prueba de semántica**: recorre el árbol de widgets y **falla si un nodo
   interactivo no tiene `Semantics.label`**. `R11` §2.10 — el líder de la categoría
   tiene esa issue abierta y sin respuesta. Es el diferenciador más barato que hay.
5. **Los primitivos**: `QyroSurface`, `QyroGlass`, `QyroButton`, `QyroField`,
   `QyroBadge`, `QyroProgress`, `QyroMono`, `QyroFingerprint`, `QyroPeerRow`.
   **Las pantallas componen, no dibujan.**
6. **Las fuentes** Inter y JetBrains Mono con **su licencia SIL OFL 1.1 copiada al
   repo** y subsetting activado.

---

## 2. Las pantallas, y qué las hace distintas

### 2.1 — Inicio: quién eres tú

`R11` §3: PairDrop pone en pantalla, permanente, **«Te conocen como: …»**.
Para una app cuyo argumento entero es *saber con quién hablas*, decir primero
**quién eres y quién te ve** es traducir la tesis a la interfaz.

- **Tu alias**, derivado de forma determinista de la huella Ed25519 — estable, no
  editable sin cambiar de identidad, sin registro. Y **tu huella en monoespaciado**.
- **«Se te puede descubrir: en esta red / no»**, con el estado real.
- Dos acciones grandes: **Enviar** y **Recibir**. Nada más en esta pantalla.

### 2.2 — Una fila, no dos listas

`R11` §3, y es el patrón que más simplifica: un aparato **descubierto** y uno
**tecleado** se pintan igual. Lo que cambia es un **badge**.
Y durante la transferencia **la barra de progreso ocupa el sitio del badge** — eso
**elimina la pantalla de progreso separada**, que es una pantalla menos que
diseñar, traducir y mantener.

Estados por fila: descubierto · tecleado · emparejado · **huella cambiada** ·
enviando · recibiendo · hecho · fallo. **Cada uno con icono, no sólo color**
(SC 1.4.1).

### 2.3 — La verificación: dieciséis iconos

`R11` §3. Se combinan las huellas de **los dos** extremos, SHA-256, los primeros
128 bits, mapeados a **16 iconos** de un alfabeto de 256 con siluetas
distinguibles, en rejilla 4×4, **con el hash en texto al lado**.
La pregunta, literal: **«¿Se ve igual en el otro aparato?»**

Qyro ya tiene huella comparable en voz alta. Con iconos es **más rápido, menos
propenso a error, no necesita idioma común y funciona por videollamada**.

**Y la decisión de aceptar, con tres salidas:** **Aceptar · Rechazar · Aceptar y
recordar.** El `Y / N / P` de `R11` §3 — **la única forma de tener favoritos sin
romper «nada se acepta solo»**. El alfabeto de iconos se congela en una ADR con
los 256 dibujos listados: si cambia después, dos versiones de Qyro se ven distinto
y la comparación deja de valer.

> **Lo que NO se copia:** LocalSend activa auto-accept de favoritos **por defecto**
> desde 1.18.0. Eso viola la regla del producto. **En Qyro nunca hay auto-accept.**

### 2.4 — Lo que hace el aire Matrix

`R12` §2.2, y se resume en una frase: **cero lluvia de caracteres, cero terminal
falsa, cero verde fosforito.** Lo que da el aire es la tipografía y la rejilla:

- El código `QYRO1|ip:puerto|huella` en monoespaciado **con los cuatro campos en
  colores distintos**. Un formato que se explica solo al mirarlo.
- Las cifras de velocidad con **ancho tabular**, para que no bailen.
- La regla que decide cada caso sin discusión: **si una persona lo va a comparar
  carácter a carácter con otra pantalla, va en monoespaciado.**

### 2.5 — Windows

No es Android con ratón. **`R11` §2** más lo obvio:

- **Arrastrar y soltar** sobre la ventana. Es *el* gesto de escritorio.
- **Teclado entero**: `Tab` recorre todo en orden, `Enter` acepta, `Esc` cancela,
  y el **anillo de foco de `R12`** —cian, **dibujado fuera del componente** para
  que se mida contra el fondo (11.15:1) y no contra el relleno azul, donde daría
  2.07 y suspendería SC 1.4.11.
- **Bandeja** con «recibir en segundo plano», y el fallo del sector que hay que
  evitar (`R11` §2.9): **arrancar en bandeja y no poder recibir hasta abrir la
  ventana una vez.** La prueba: **sin abrir la ventana nunca, mandar un archivo.**
- **Selector de interfaz de red visible**, no escondido (`R11` §2.3: el líder tardó
  dos años en añadirlo).

---

## 3. Rendimiento, medido y no supuesto

`R12` §7.1 nombra el riesgo: **Impeller en Windows con `BackdropFilter` apilado.**

- **Máximo dos regiones con blur a la vez.** Las tarjetas de lista **no llevan
  blur**: relleno sólido más el borde de `R12` §4.1. Se ve igual y no cuesta nada.
- **`BackdropFilter.grouped`** para superficies contiguas — una pasada en vez de N.
- **`ImageFilter.blur(tileMode: TileMode.decal)`**, no el modo por defecto: el
  `clamp` estira el borde y produce el halo sucio del glassmorphism mal hecho.
- **Se mide con `--profile` en la pantalla más pesada y se apunta el número en el
  informe.** Si baja de 60 fps, **esa pantalla pasa a sólido**. No se discute: se
  mide y se decide.
- **`MediaQuery.disableAnimations` respetado en un solo sitio.**

---

## 4. La prueba que cierra la fase

1. **Contraste**: automática, con contraprueba.
2. **Semántica**: automática, con contraprueba.
3. **Sin literales de color** fuera de `lib/design/`: guarda de build, y **se
   comprueba que falla** metiendo uno a propósito.
4. **Golden tests** de los nueve primitivos y de las tres pantallas, en las dos
   plataformas. Un golden que nadie ha visto fallar no vale: **se cambia un token y
   se exige que rompan.**
5. **fps medido** y escrito.
6. **La fila con sus ocho estados**, cada uno con su golden.

---

## 5. Lo que NO hay que hacer

- **No uses `fromSeed`.**
- **No pongas texto secundario ni azul sobre vidrio con contenido ajeno detrás.**
  `R12` §4: a α 0.72 dan 3.13 y 3.32. Suspenden. O α sube a 0.84, o no van ahí.
- **No pongas la etiqueta del botón primario en blanco.** 3.75:1. Va casi negro.
- **No metas un shader de refracción el primer día.** `R12` §4.3: primero el borde
  en gradiente, que no puede romper Impeller.
- **No rediseñes el CLI.** El propietario dijo explícitamente que esto es la app.
