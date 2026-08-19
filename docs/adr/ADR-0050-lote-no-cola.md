# ADR-0050 — Un lote no es una cola

- **Estado:** **congelada** antes de una sola línea de código.
- **Fecha:** 2026-08-20
- **Fase:** 25
- **Depende de:** ADR-0026 (protocolo), ADR-0027 (rutas), ADR-0028 (transporte), ADR-0047 (límites y carpetas).
- **Gobernada por:** `R7` §5 y `R11` §1.

---

## 1. La contradicción, y no lo es

Dos documentos de este taller parecen decir cosas opuestas:

- **`FASE-22` §6:** *«no añadas una cola»*, citando `R7` §5 — **Qyro no es un
  gestor de descargas.**
- **`R11` §1:** **trece de trece** proyectos del sector mueven varios archivos con
  un progreso agregado.

No se contradicen. **Son dos cosas distintas y este documento existe para
separarlas por escrito**, porque la diferencia es la que hay entre completar el
producto y convertirlo en otro.

---

## 2. Las dos frases

> **Lote** = *una* transferencia que contiene *N* archivos, con un progreso
> agregado, un botón de cancelar y un destino. Sigue siendo **una** sesión,
> **un** handshake, **una** decisión del receptor. ✅ **Esto se hace.**

> **Cola** = varias transferencias independientes esperando turno, con
> reordenación y reintentos. ❌ **Esto sigue prohibido por `R7` §5.**

---

## 3. Cómo se nota la diferencia desde fuera

La prueba de si algo es un lote o una cola no es cuántos archivos hay. Es **qué
puede hacer una persona con ello**:

| | Lote | Cola |
|---|---|---|
| Handshakes | uno | uno por transferencia |
| Decisiones del receptor | **una**, sobre el conjunto | una por cada |
| Cancelar | **uno**, y para todo | por elemento, y hay que elegir |
| Reordenar | no existe | existe |
| Reintentar uno solo | no existe | existe |
| Sobrevive a cerrar la aplicación | **no** | sí, o no sería una cola |

**Si alguien pide reordenar, ya no es un lote.** Ésa es la línea, y está aquí
para que dentro de seis meses nadie la cruce sin darse cuenta.

---

## 4. Lo que el lote sí trae, y por qué cada cosa

1. **Progreso agregado**: bytes hechos de bytes totales, **archivo N de M**, y
   velocidad. Calculado **en Rust**, porque dos consumidores que lo calcularan
   por su cuenta darían dos cifras distintas para la misma transferencia
   (ADR-0046 §4).
2. **Un solo cancelar**, que para el lote entero y **no deja ni un
   `.qyro-part`**. Un cancelar por archivo obligaría a una persona a decidir
   cincuenta veces lo que decidió una.
3. **Ruta relativa por entrada**, con la estructura preservada.

---

## 5. Las carpetas vacías: **ya estaba decidido, y se confirma**

`FASE-25` §3.1 pide incluirlas **o decidir por escrito que no**. La segunda mitad
de esa frase ya estaba cumplida antes de escribir esta ADR: **ADR-0047 §4 lo
decidió**, con este argumento —

> *«El manifiesto lista archivos, y una carpeta vacía no es un archivo. Se dice
> aquí en vez de descubrirse: quien manda una carpeta con un directorio vacío
> dentro no lo encontrará al otro lado, y eso es una pérdida de información
> aunque no sea una pérdida de bytes.»*

**Y lo encontró una prueba, no yo.** El primer borrador de esta sección presentaba
la decisión como nueva; al ir a citar la prueba que la respalda —`a folder keeps
its shape, and an empty subfolder does not travel`— resultó que su mensaje de
fallo ya apunta a **ADR-0047 §4**, y dice literalmente: *«si esto empieza a
fallar, la decisión cambió y hay que cambiar el documento, no la prueba»*.

**Aquí no se decide nada, entonces. Se confirma y se añade una razón que ADR-0047
no tenía:** para que una carpeta vacía viajara haría falta una entrada de tipo
directorio en el manifiesto, y **todo receptor tendría que entenderla** — incluido
el de quince líneas de PowerShell del canal serie (ADR-0045), que sólo sabe
escribir bytes. Es una versión de protocolo en los cuatro canales, para siempre,
a cambio de una carpeta vacía.

---

## 6. Texto: un `kind`, no un archivo con nombre falso

`R11` §1.5: presente en **8 de 13**, y es lo más barato de la lista.

**Decisión:** el texto viaja como un **tipo de entrada**, no como un archivo
llamado `pegado.txt`. En el receptor **se enseña y se copia**, y **no se guarda
como archivo salvo que la persona lo pida.**

**Por qué no un archivo:** porque entonces el receptor tendría que adivinar si
`nota.txt` es un archivo que alguien quería guardar o un texto que alguien quería
leer, y adivinaría mal la mitad de las veces. Un tipo lo dice.

Y **no contradice la §5**: una carpeta vacía es forma sin contenido; un texto es
contenido sin archivo. El primero se descarta, el segundo se nombra.

---

## 7. Lo que esta ADR NO decide

- **Cómo se dibuja.** Eso es la fase 26.
- **Si el lote sobrevive a cerrar la aplicación.** No lo hace, y eso es lo que lo
  mantiene siendo un lote. Si algún día tiene que sobrevivir, **es una cola** y
  hay que reabrir `R7` §5, no esta ADR.
- **El número de archivos.** Lo fija ADR-0047 en 256, y no se toca aquí.
