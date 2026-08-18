# ADR-0049 — Windows 7 y 32 bits

- **Estado:** **congelada** antes de una sola línea de código.
- **Fecha:** 2026-08-19
- **Fase:** 17
- **Depende de:** ADR-0042 (CLI), ADR-0045 (canal serie).
- **Gobernada por:** `R7` §2 — la máquina que no puede instalar nada — y `R8` §10.

---

## 1. El hecho que decide toda la fase

**Un binario de Rust compilado con los targets normales no arranca en Windows 7.**

`std` importa **estáticamente** `WaitOnAddress`, `WakeByAddressAll` y
`WakeByAddressSingle` de `api-ms-win-core-synch-l1-2-0.dll`, que es **Windows 8
mínimo**. No es una llamada que se pueda evitar en tiempo de ejecución: está en la
tabla de importaciones, así que el cargador falla **antes** de que corra una sola
instrucción del programa.

Esto no se dedujo: lo destapó una guarda de este repositorio —`verify_static.ps1`—
al inspeccionar los imports en vez de suponerlos, y lo lleva diciendo desde la
fase 13 como `[NOTE]`.

---

## 2. Decisión 1 — **el pin del proyecto no se mueve; sólo el job de win7 usa nightly**

`rust-toolchain.toml` se queda en **1.88.0**.

Los targets `x86_64-win7-windows-msvc` e `i686-win7-windows-msvc` son **Tier 3** y
necesitan `-Z build-std`, que es nightly. Subir el proyecto entero a nightly por
eso sería cambiar el compilador de **todo** —la criptografía incluida— para arreglar
una plataforma.

**Un job aparte, con su propio toolchain, y declarado como aislado.** El resto del
árbol sigue en un stable fijo y auditado.

---

## 3. Decisión 2 — **`msvc`, y la verificación está pendiente**

`R8` §10 midió sobre `-gnu`, porque no había Windows SDK en aquella máquina. El
código de `std` es el mismo, **y eso es un argumento, no una medida.**

**Se elige `msvc`** porque es lo que el resto de la tubería ya usa
(`.cargo/config.toml` fija `+crt-static` para `x86_64-pc-windows-msvc` e
`i686-pc-windows-msvc`) y mezclar cadenas de enlazado por plataforma es cómo
aparece un binario que nadie sabe reproducir.

> **PENDIENTE, y escrito como pendiente:** confirmar con `dumpbin /imports` que el
> binario de `x86_64-win7-windows-msvc` **no** importa
> `api-ms-win-core-synch-l1-2-0.dll`. Hasta que ese comando se haya ejecutado y su
> salida esté pegada en un informe, **esta ADR no afirma que Windows 7 funcione.**

---

## 4. Decisión 3 — **la comprobación de imports es un paso de puerta, con su control**

`scripts/check_win7_imports.ps1`, por código de salida:

1. El binario del target de win7 **no puede importar** nada de
   `api-ms-win-core-synch-l1-2-0.dll` ni requerir `vcruntime140.dll`.
2. **Y el binario del target normal DEBE fallar esa misma comprobación.**

**El punto 2 es el que hace que el 1 signifique algo.** Sin él, una comprobación
que no encontrara nunca ese import —porque el patrón está mal escrito, porque
`dumpbin` no está, porque el archivo no existe— pasaría en verde para siempre y
diría exactamente lo mismo que una que funciona.

Es la misma forma que salvó a este proyecto en la fase 13: fue el **fallo** del
control de `+crt-static` lo que destapó este bloqueo.

---

## 5. Decisión 4 — **lo que la Release declara**

Tier 3 significa, en palabras del proyecto Rust, que *«no hay builds oficiales»* y
que no hay CI upstream ni garantías.

La Release lo dice **así**, sin adornos:

> Compilado para Windows 7 con un target **no soportado oficialmente** por el
> proyecto Rust. Funciona en nuestras pruebas. No hay garantía de nadie más.

Un binario que promete Windows 7 sin decir eso está vendiendo una garantía que no
tiene.

---

## 6. Decisión 5 — **Windows XP se descarta, y la respuesta correcta va al lado**

No hay target de Rust para XP y no lo va a haber.

**Y eso no deja a esa máquina fuera del producto.** A una XP no se le lleva Qyro:
se le lleva un archivo **por el puerto serie desde HyperTerminal**, que ya está
instalado en esa máquina desde el día que salió. Eso es la fase 16, ya está hecho,
y `qyro serial` imprime el receptor para pegar allí.

La respuesta a «¿y XP?» es un procedimiento que funciona, no una excusa.

---

## 7. Lo que esta ADR NO promete

- **Que Qyro arranque en un Windows 7.** Ninguno lo ha ejecutado. Fase 19, y el
  hueco sigue en blanco.
- **Que el target Tier 3 compile aquí.** `-Z build-std` necesita nightly y
  `rust-src`, ~1,5 GB en el disco de sistema de esta máquina, que va justo. **Se
  compila en CI**, que es donde el coste no es de nadie.
- **32 bits probado.** Se compila; que funcione en hardware de 32 bits es la misma
  fase 19.
