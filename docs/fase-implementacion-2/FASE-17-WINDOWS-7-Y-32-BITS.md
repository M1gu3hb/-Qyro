# FASE 17 — Windows 7 y 32 bits

> No es una funcionalidad: es un **pipeline**. Por eso va después de que el binario
> exista, no antes.

---

## 1. El hecho que decide toda la fase

**Un binario de Rust compilado con los targets normales no arranca en Windows 7.**
No va lento, no degrada: **el loader falla antes de `main`.**

Verificado compilando e inspeccionando imports (`R8` §10): un binario stock importa
**estáticamente** `WaitOnAddress`, `WakeByAddressAll` y `WakeByAddressSingle` de
`api-ms-win-core-synch-l1-2-0.dll`, que es **Windows 8 mínimo**
(https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitonaddress).
Al ser import estático no hay degradación posible; sólo «falta la DLL».

El piso oficial de Rust es **Windows 10** desde la **1.78** (2024-05-02,
https://blog.rust-lang.org/2024/02/26/Windows-7/).

**La solución existe y está verificada:** los targets `x86_64-win7-windows-msvc` /
`i686-win7-windows-msvc` (y sus gemelos `-gnu`) sustituyen esas APIs por **SRW locks
(Vista+)** y `Fls*`, y producen un import set limpio: ADVAPI32, KERNEL32, WS2_32,
bcrypt, msvcrt, ntdll — **todas existen en Windows 7**.

**El precio:** son **Tier 3**. *«Official builds are not available.»* `rustup target
add` **falla** para ellos. Requiere **nightly + `-Z build-std`**.

---

## 2. La decisión que hay que congelar

`docs/adr/ADR-00XX-windows-7.md`:

1. **Si se sube el pin de `rust-toolchain.toml`.** Está en 1.88.0; el stable vigente
   al 2026-08-17 es 1.97.1. Esta fase necesita nightly de todos modos. **Decide si
   el proyecto entero sube, o si sólo el job de win7 usa nightly.** La segunda opción
   es más conservadora y probablemente la correcta: el resto del árbol sigue en un
   stable fijo y auditado, y el pipeline Tier 3 queda aislado y declarado.
2. **`msvc` o `gnu`.** La medición de `R8` §10 se hizo sobre `-gnu` (no había Windows
   SDK). El código de `std` es el mismo, pero **hay que confirmarlo en `msvc` con
   `dumpbin /imports` antes de comprometerse**. Escríbelo como lo que es: una
   verificación pendiente, no un supuesto.
3. **Qué se declara y qué no.** Un target Tier 3 **no tiene CI upstream ni garantías
   del proyecto Rust**. La Release tiene que decirlo: «compilado para Windows 7 con
   un target no soportado oficialmente; funciona en nuestras pruebas».
4. **Windows XP: descartado, y con la respuesta correcta escrita al lado.** No hay
   target y no lo va a haber. **A una XP no se le lleva Qyro: se le lleva un archivo
   por serie desde HyperTerminal, que ya está en la máquina.** Eso es la fase 16 y es
   la respuesta, no una excusa.

---

## 3. Entregables

1. La ADR de §2.
2. **Un job de CI aparte** que compile con
   `cargo +nightly build -Z build-std=std,panic_abort --target x86_64-win7-windows-msvc`
   y su gemelo de 32 bits, con `rust-src` instalado.
3. **La comprobación de imports como paso de puerta, por código de salida:** el
   binario resultante **no puede importar** nada de `api-ms-win-core-synch-l1-2-0.dll`
   ni requerir `vcruntime140.dll`. **Con su control:** el binario compilado con el
   target normal **debe fallar** esa comprobación. Sin el control, el paso no
   distingue nada.
4. **Los cuatro targets de 32 bits** para hardware viejo: `i686-pc-windows-msvc`,
   `i686-win7-windows-msvc`, `i686-unknown-linux-musl`, y el de Windows 7 de 64 bits.
5. **Tamaños medidos de cada uno**, en la tabla del informe. Referencias de `R8` §6:
   ~723 KB (win7 x64), ~812 KB (win7 x86), ~899 KB (musl i686).

---

## 4. La prueba que cierra la fase

Sin una máquina con Windows 7, la evidencia máxima honesta es:

> **Los imports del binario, verificados por código de salida, no contienen ninguna
> API posterior a Windows 7**, y la lista completa de DLLs importadas está escrita en
> el informe.

**Clase de evidencia, literal:** «verificado por análisis estático de imports; **no
ejecutado nunca en una máquina con Windows 7**». **No la subas de categoría.**
La ejecución real es la fase 19 y necesita la máquina del propietario.

---

## 5. Lo que NO hay que hacer

- **No intentes Windows XP.** No hay target. Registrarlo como descartado con el
  argumento de §2.4 es cerrar la ficha; intentarlo es perder una sesión.
- **No subas todo el proyecto a nightly** sin escribir por qué. Un árbol auditado
  sobre un stable fijo es un activo.
- **No declares «funciona en Windows 7»** hasta que alguien lo haya arrancado en uno.
