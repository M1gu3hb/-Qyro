# Lo que no se ha probado

**Ésta es la única página que hay que leer antes de enchufar dos aparatos.**

Es la lista honesta de lo que sigue dependiendo de un aparato físico, con los
huecos en blanco. **Un hueco en blanco es la verdad**, y esta página existe para
que nadie tenga que deducirlo del resto de la documentación.

Para ejecutarlo: [`docs/GUIA-DE-PRUEBA.md`](../GUIA-DE-PRUEBA.md).
Para anotarlo: [`docs/testing/hardware-protocol.md`](../testing/hardware-protocol.md).

---

## 1. Las seis clases de evidencia, y hasta dónde llega esto

| # | Clase | Estado |
|---|---|---|
| 1 | compilado | **lleno** |
| 2 | probado en unidad | **lleno** |
| 3 | probado en integración | **lleno** |
| 4 | probado en ejecución, dos procesos reales | **lleno** |
| 5 | probado en emulador | **lleno**, para lo que el emulador alcanza |
| 6 | **probado en hardware** | **en blanco. Ninguna casilla.** |

Cinco de seis. La sexta no la puede llenar nadie sin dos aparatos.

---

## 2. Lo que nunca ha ocurrido

- **Ningún teléfono ha ejecutado nunca esta aplicación.**
- **Ninguna transferencia ha cruzado nunca una Wi-Fi de verdad.** Todo lo que ha
  cruzado, cruzó por `127.0.0.1` entre dos procesos de la misma máquina, o por el
  bucle de red de un runner de CI.
- **Ninguna cámara ha leído nunca un QR de Qyro.** El canal óptico está probado
  de extremo a extremo con un decodificador real (`rqrr`) sobre el dibujo que la
  terminal produce, y eso demuestra que el dibujo es legible — no que un sensor
  de un teléfono, con su enfoque y su brillo, lo lea.
- **Ningún cable serie ha llevado nunca un archivo de Qyro.**
- **Ningún cable de red directo entre dos máquinas ha llevado nunca uno.**
- **Nadie ha visto nunca el diálogo del cortafuegos de Windows** delante de este
  programa.

---

## 3. Lo que sí se ha medido, y con qué números

Para que la lista de arriba no se lea como «no se sabe nada»:

| Qué | Medido |
|---|---|
| Un archivo cruza entre dos procesos, byte a byte | **Sí**, SHA-256 idéntico en origen y destino |
| Memoria por transferencia, con un archivo de **400 MB** | **Emisor 5,2 → 5,6 MB · receptor 4,9 → 5,8 MB.** O(1) por frame, medido con `VmRSS`, no razonado |
| 200 archivos en un lote | **200/200 en 292 ms**, cero lecturas vencidas |
| El puerto ocupado | **Sí**, con el 49517 tomado por otro proceso: lo dice y ofrece otro |
| El mismo archivo dos veces a la misma carpeta | **Sí**: se niega, no sobrescribe, y ahora dice por qué |
| Un nombre con un retorno de carro | **Sí**: rechazado antes de tocar el socket, y no reescribe el terminal al imprimirse |
| Un nombre con un `U+202E` (RTL override) | **Sí**: rechazado igual |
| Una huella que no coincide | **Sí**: `REFUSED`, sin «continuar de todos modos» |
| Descriptores abiertos con 200 archivos | **402 antes, 11 después** (QYR-0391). Contados en `/proc/self/fd` por un hilo muestreador **mientras la transferencia corre**, no deducidos |
| Una persona que tarda **65 s** en aceptar | **Antes**: el emisor moría a los 60,11 s con «el otro aparato no responde». **Después**: entregado a los 65,76 s (QYR-0393) |
| Quitar la comprobación de caracteres de control del validador de nombres | **3 pruebas en rojo**. Quitar la de caracteres de formato Unicode: **6**. Ejecutado, y el árbol restaurado |

**Todo eso es Linux, en un contenedor, con un binario de depuración.** Ninguna de
esas cifras dice nada sobre Windows ni sobre Android.

---

## 4. Los huecos, uno a uno

**Veintiséis escenarios**, todos en blanco, en
[`docs/testing/hardware-protocol.md`](../testing/hardware-protocol.md):

- **A1–A4** (4) — arranque y presencia: que la aplicación abra, que el `.so`
  cargue, que el manifiesto instalado pida los tres permisos y no más, que el
  `.exe` arranque.
- **B1–B4** (4) — identidad: que sobreviva a cerrar la aplicación, a reiniciar el
  teléfono, que Keystore funcione dentro de un proceso de aplicación real, y que
  las dos huellas se lean iguales en voz alta.
- **C1–C5** (5) — emparejamiento: el código tecleado, el descubrimiento en las dos
  direcciones, el rechazo de una clave cambiada, y el aislamiento de cliente.
- **D1–D5** (5) — la transferencia: foto teléfono→PC, 100 MB PC→teléfono, y las
  demás.
- **E1–E3** (3) — lo que sale mal.
- **F1b, F2b, F2c, F3b, F4b** (5) — los otros tres canales y la máquina que no
  puede instalar nada.

---

## 5. Lo que este taller **no puede** construir, y por qué importa

La tanda del 2026-08-31 corrió en **un contenedor de Linux**: sin Flutter, sin
SDK de Android, sin PowerShell y sin el objetivo `x86_64-pc-windows-msvc`.

**Consecuencias, dichas para que nadie las descubra tarde:**

1. **Los dos artefactos no se construyeron aquí.** Se construyen con los comandos
   de la guía §2, en la máquina del propietario, y quedan en
   `release/prueba-en-hardware/`. Construirlos en otra parte y decir que son ésos
   es lo que costó una retractación pública en QYR-0359.
2. **El código Dart y el Kotlin de esta tanda no se han ejecutado.** Los cambios
   de esa tanda que tocan la aplicación —los nombres de la oferta, la carpeta de
   destino de Android, la puerta del escáner, el fallo al materializar— están
   escritos, revisados y cubiertos por pruebas **que sólo CI puede correr**. El
   lado Rust sí se ejecutó, entero.
3. **`scripts/gate.ps1` no se pudo correr**, porque no hay PowerShell. Lo que sí
   se corrió son **los mismos comandos `cargo` que ese script lee de `ci.yml`**,
   que es todo lo que ese script hace en un sistema sin Windows. Está dicho, y no
   se marca como «puerta verde» algo que no es la puerta.

---

## 6. La regla, otra vez, porque es la única que importa aquí

**No se inventa evidencia de hardware.**

Un escenario sin marcar no es un aprobado. Y escribir un resultado que no ocurrió
arruina **todos los demás**, porque a partir de ahí ninguno se puede creer.
