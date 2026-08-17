# LÉEME PRIMERO — Fase de implementación 2 (v1.1 → v2.0)

> **Qué es esto.** El plan de las nueve fases que quedan, escrito después de auditar
> la v1.0 sobre el repositorio real. La v1.0 está etiquetada y su motor es sólido.
> Lo que sigue no es pulir: es **construir el producto que este proyecto siempre
> quiso ser y que se había perdido de vista.**

---

## 1. Las dos cosas que cambian, y por qué

### 1.1 — La v1.0 tiene un P0 vivo

Auditoría del 2026-08-17 sobre `d575ac85`. **Dos aparatos con Qyro instalado no
pueden completar una sola transferencia usando la aplicación.**

- `native_transfer_service.dart:169` lee `_listeningAddress`, un campo declarado en
  la 177 y **que no se asigna en ninguna parte del árbol**. `ownPairingString()`
  sigue devolviendo `null` siempre.
- `transfer_screens.dart:422` liga a `'0.0.0.0:0'` — puerto efímero que nadie
  consulta ni enseña.
- El descubrimiento automático **no tiene un solo símbolo en la superficie C**, y
  `DiscoveryChannel.kt` está registrado en Kotlin pero ningún archivo de Dart abre
  el canal `dev.qyro/discovery`.

La primera pantalla dice, en los dos idiomas: «Escribe el código de emparejamiento
que enseña el otro aparato» — y el otro aparato dice «Sin conexión, así que no hay
código que mostrar». **Los dos a la vez. Es un bucle cerrado sin salida.**

Se cerró en falso en la fase 09: QYR-0322 decía *«lo que no se puede es preguntarla
a tiempo»* y *«sube en cuanto Dart tenga que recibir, que es la fase 05»*. Se cerró
diciendo que ya existe un getter. **El cierre respondió a una pregunta distinta de
la que la ficha hacía.**

### 1.2 — El objetivo real del producto se había perdido

Esto es más importante que el P0. Lee **`R7-EL-OBJETIVO-REAL.md` entero antes de
nada.** Resumen de una línea: Qyro no es una aplicación de móvil con una GUI de
Flutter. **Qyro es un binario que se ejecuta en cualquier máquina, incluso una que
no puede instalar nada, y mueve un archivo hasta ella por el canal que haya —
aunque no haya red.**

La GUI de Flutter es **una cara** del motor. Falta la otra, que es la que resuelve
el problema del que nació el proyecto: un PC viejo, sin USB que funcione, al que no
se le puede meter un archivo.

---

## 2. El mapa

| Doc | Qué es |
|---|---|
| `R7-EL-OBJETIVO-REAL.md` | **Léelo primero.** Qué es Qyro, para quién, y el criterio con el que se decide todo lo demás |
| `R8-LO-QUE-LA-INVESTIGACION-DICE.md` | Los números duros, medidos, con fuente y fecha. **No vuelvas a investigar nada de esto** |
| `FASE-12` | Cerrar la cadena. El primer archivo que viaja entre dos aparatos. Y la Release |
| `FASE-13` | `qyro` en la terminal: el binario único |
| `FASE-14` | Que se encuentren sin router |
| `FASE-15` | El canal óptico: QR animado |
| `FASE-16` | El canal serie: meter datos en una máquina que no puede leer un QR |
| `FASE-17` | Windows 7 y 32 bits |
| `FASE-18` | La verdad: modelo de amenazas y documentos de los canales nuevos |
| `FASE-19` | Hardware: los escenarios de los cuatro canales |
| `FASE-20` | Distribución |

Las reglas `R1`–`R6` de `docs/fase-implementacion/` **siguen vigentes sin cambios**.
No las copio aquí. Lo que se añade son dos comprobaciones de puerta, en §4.

---

## 3. El orden, y por qué es ése

**12 primero, y no se discute.** Hasta que un archivo viaje de un aparato a otro
por la aplicación, todo lo demás son cimientos de una casa sin puerta. Es además la
fase más corta de las nueve.

**13 antes que 14, 15 y 16.** El binario de terminal es el *envase* de los tres
canales nuevos. Construirlos primero significaría construirlos dos veces, o
construirlos dentro de Flutter, donde no sirven para el caso de uso que los pidió.

**15 y 16 son hermanos, no alternativas.** El QR resuelve «la máquina puede
mostrarme algo». El serie resuelve «la máquina no puede leerme nada». Son las dos
mitades del mismo problema y ninguna sustituye a la otra.

**17 va después de 16 y no antes:** compilar para Windows 7 es un *pipeline*, no
una funcionalidad. Hacerlo antes de que exista el binario que compilar es orden
inverso.

---

## 4. Las dos comprobaciones que se añaden a la puerta

La puerta de `R2` pasa de trece a **quince** comprobaciones. Las dos nuevas salen
directamente de cómo se rompió la v1.0.

### Comprobación 14 — **el llamante de producción**

> Por cada capacidad que tu informe declara hecha, **nombra el llamante de
> producción: archivo y línea.** Si el llamante es una prueba, un arnés o nadie, la
> capacidad **no existe** y el informe no puede decir que existe.

Es la pregunta que abrió la fase 11 (*«¿quién llama a `KeystoreWrapper`? Nadie»*) y
es la que habría evitado los tres defectos que sobrevivieron: `KeystoreWrapper` sin
llamante, `qyro_session_local_address` sin llamante, `MdnsDiscovery` sin llamante.

**Forma mecánica:** una tabla en el informe de fase, `capacidad | símbolo | llamante
de producción | archivo:línea`. Una fila con «ninguno» es un bloqueo, no una nota.

### Comprobación 15 — **la cadena completa desde el gesto**

> Para la capacidad principal de la fase, escribe la cadena entera **desde el gesto
> de una persona hasta el byte**: qué toca o teclea, qué función de UI se dispara,
> qué símbolo del FFI cruza, qué hace el motor. **Sin saltos.** Si un eslabón no
> existe, la fase no cierra.

La v1.0 tenía los dos extremos de esa cadena y le faltaba el medio, y nada lo miró
porque cada extremo tenía sus propias pruebas en verde.

### Y una regla sobre cerrar fichas

> **Una ficha se cierra respondiendo a la pregunta que hace, no a una parecida.** Si
> la ficha contiene la palabra «sube», «escala», «cuando», o nombra una fase futura
> como condición, **no se puede cerrar sin comprobar si esa condición ocurrió** y
> escribir el resultado de esa comprobación en el cierre.

---

## 5. Lo que NO cambia

- **La regla del carril.** Sólo un P0 detiene. Todo lo demás a
  `docs/reports/deuda-de-calidad.md`, que vuelve a abrirse, y se vacía en la 18.
  Excepción: lo que impide construir lo siguiente es bloqueo, no deuda.
- **Dos destinos para una ficha**: HECHA, o cerrada/descartada con argumento.
  Nunca «pendiente».
- **ADR congelada antes del código, en su propio commit.**
- **`docs/reports/ESTADO-ACTUAL.md`**, 120 líneas, reescrito y commiteado tras cada
  paso. Cuando el contexto se acabe: ese archivo primero, después **sólo** el
  documento del paso que toca. **Nunca releas las reglas completas con el depósito
  vacío.**
- **Nunca `main`. Nunca force-push. Nunca reescribir historia. Nunca borrar una
  rama.**
- **No se inventa evidencia de hardware.** Es lo único que arruinaría el proyecto.

---

## 6. Autonomía

**Decide tú. No preguntes.** Están pre-autorizados: commit y push en tu rama,
instalar herramientas, modificar CI, editar cualquier archivo, crear crates nuevos
en el workspace, las dependencias nombradas en `R8` §7, elegir nombres, y **toda
decisión de alcance**. Si dudas entre dos opciones, elige, escribe por qué en una
línea, y sigue.

**Las únicas cuatro excepciones:** `main`, dinero, un aparato físico, y una segunda
persona.

Si no encuentras por dónde: **investiga con agentes**, decide con lo que encuentres,
y congela la decisión en una ADR con la fuente y la fecha. No te quedes esperando.
