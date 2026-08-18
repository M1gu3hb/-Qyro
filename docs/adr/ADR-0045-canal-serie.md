# ADR-0045 — El canal serie

**Estado:** congelada · **Fecha:** 2026-08-18 · **Fase:** 16
**Fuentes medidas:** `R8` §5.1, §5.2, §5.3, §5.4 · `R7` §2

---

## 1. Qué problema resuelve, en las palabras del dueño

> «Tengo computadoras viejas… Los puertos que tiene ya no sirven, entonces no le
> puedo ni conectar USBs. Entonces, ¿de qué manera les comparto archivos?»

El canal óptico **no sirve en esa dirección**: leer un QR necesita una cámara en
la máquina que recibe, y un desktop de esa época no la tiene. Lo que sí tiene,
con seguridad, es un **DB9**.

`R8` §5.1, medido: **115 200 bps, 8N1 ⇒ 9–11 KB/s reales. 1 MB en 1,6 min, 10 MB
en 16 min.** Un orden de magnitud mejor que el QR.

---

## 2. Antes de proponerlo, Qyro pregunta lo aburrido

`R8` §5.4. **En la interfaz, no en un README.** Antes de ofrecer serie:

> ¿Esa máquina tiene lector de CD, disquetera, ranura PCMCIA o tarjeta de red?

Cualquiera de las cuatro es **entre 10 y 10 000 veces más rápida**. Un CD-R mueve
700 MB en cinco minutos; un cable de red mueve 1 MB en menos de un segundo.
**Proponer el canal lento sin descartar los rápidos es mal producto.**

---

## 3. Decisión 1 — `serialport` sin sus características por defecto

**Elegido:** `serialport` 4.9, **`default-features = false`**.
**Descartado:** `CreateFile` + `SetCommState` + `DCB` en Windows y `termios` en
Linux a mano.

| | `serialport` | Win32 + termios a mano |
|---|---|---|
| Líneas propias | ~0 | ~200 **por plataforma** |
| `unsafe` | ninguno nuestro | segunda **y tercera** excepción a `forbid(unsafe_code)` |
| Licencia | **MPL-2.0** | — |

La licencia es lo único que hacía dudar y **no es lo que parece**: MPL-2.0 es
copyleft **por archivo**. Usar la dependencia sin modificarla no obliga a nada
sobre el código de Qyro, a diferencia de GPL. Es una familia de licencia nueva en
un proyecto que hasta hoy es MIT/Apache, y por eso se escribe aquí en vez de
pasar en un `Cargo.toml`.

**Lo que decide, y está medido hoy** (`cargo tree`, 2026-08-18): con las
características por defecto arrastra **`libudev`, que es una biblioteca C** — en
un proyecto cuyo argumento entero es no depender de C. Con
`default-features = false` en Windows el árbol completo es:

```
serialport v4.9.0
├── cfg-if
├── scopeguard
└── windows-sys → windows-targets → windows_x86_64_msvc
```

**Los tres ya están en el grafo de Qyro.** El canal serie añade *un* paquete.

Se pierde la metainformación USB (fabricante y producto), y **no hace falta**: la
persona elige `COM3` o `/dev/ttyS0` por su nombre, que es lo que ve escrito.

**Confinado a un adaptador.** El protocolo habla contra `Read + Write` y no sabe
qué hay debajo, así que la dependencia se cambia sin tocar el protocolo — y es
también lo que permite probarlo entero sin un puerto.

---

## 4. Decisión 2 — **ARQ, y no el fountain de la fase 15**

**El serie tiene canal de retorno y el óptico no. Ésa es la diferencia que
decide**, y decide en contra de reutilizar el código que ya está escrito.

| | Fountain (LT) | ARQ con CRC32 |
|---|---|---|
| Necesita retorno | no | **sí, y aquí lo hay** |
| Sobrecoste con canal limpio | **5–15 % siempre** | ~0 |
| Sobrecoste con 5 % de pérdida | 5–15 % | los bloques perdidos |
| Reintentos | no existen | **se cuentan** |

Sobre un enlace limpio —que es el caso normal de un cable null-modem de un
metro— el fountain **paga su 5–15 % para nada**. A 9–11 KB/s eso son minutos en
un archivo de 10 MB.

Y hay una razón de producto encima de la aritmética: **un reintento es
observable y el sobrecoste de un fountain no**. La fase 16 tiene que poder decir
«se reintentaron 4 bloques», que es una frase que le dice a alguien que su cable
está mal. «Hicieron falta un 12 % más de frames» no se lo dice a nadie.

**Que el LT exista y no se use aquí no es desperdicio:** el óptico lo necesita
porque una pantalla no rebobina, y ese es exactamente el caso donde ARQ no puede
usarse. Dos canales, dos problemas distintos, y la diferencia es una línea de
retorno.

---

## 5. Decisión 3 — el receptor tonto, y lo que se pierde

Cuando la máquina vieja **no puede correr Qyro**, el receptor es un script que la
persona pega. Windows 7 no trae HyperTerminal pero trae **PowerShell 2.0**, que
expone `System.IO.Ports.SerialPort`: un receptor completo son ~15 líneas.

**El modo degradado, por obligación:**

| | |
|---|---|
| Marco | una línea de texto por bloque |
| Contenido | **Base64**, no binario |
| Integridad | **CRC32 por bloque**, en la misma línea |
| Recuperación | el receptor pide el bloque otra vez por número |
| Reensamblado | `certutil -decode` |

**Base64 y no binario, por dos razones que se suman:** un cable de 3 hilos no
tiene RTS/CTS y obliga a XON/XOFF, y XON/XOFF **se come los bytes 0x11 y 0x13**,
así que el binario crudo se corrompe en silencio. Y `certutil -decode` existe en
todas las Windows desde XP sin instalar nada. El coste es +33 %, y sobre 9–11
KB/s es real y se acepta: **un canal que funciona al 75 % de velocidad vale
infinitamente más que uno que corrompe.**

### 5.1 — Lo que se pierde, y se dice en pantalla antes de mandar

**El modo degradado NO está autenticado.** Un script de quince líneas no hace
X25519 ni ChaCha20-Poly1305; recibe lo que llegue por el cable y lo escribe.

Concretamente se pierde:

- **Autenticación del emisor.** Cualquiera con acceso físico al cable puede
  mandar. En un cable de un metro entre dos máquinas de la misma persona eso es
  aceptable; escrito, es una decisión, y sin escribir es un agujero.
- **Confidencialidad.** Va en claro. Quien pinche el cable lee el archivo.
- **La huella.** No hay con quién compararla.

**Lo que sí queda:** integridad por bloque (CRC32) y **el SHA-256 del archivo
completo**, que Qyro imprime y la persona compara al final. Eso detecta
corrupción; **no** detecta a alguien que sustituyó el archivo entero — que es
exactamente la lección de QYR-0359: *un hash correcto prueba que te dieron el
archivo que nombraron, no que haga lo que dijeron*.

**Va al modelo de amenazas de la fase 18.** Y va a la pantalla antes de mandar,
no en una nota al pie.

---

## 6. Decisión 4 — baudios y control de flujo

| | |
|---|---|
| Velocidad | **115 200**, 8N1 |
| Flujo, cable completo | **RTS/CTS por hardware** |
| Flujo, cable de 3 hilos | XON/XOFF, **y entonces Base64 es obligatorio** |

115 200 y no más: es la última velocidad que un UART 16550 de esa época sostiene
sin errores de framing, y la máquina de destino es precisamente de esa época.

---

## 7. El emulador de teclado PS/2 — decisión escrita, no código

`R8` §5.2. **Como canal de datos: no.** 37–375 B/s, y 1 MB tarda entre 47 minutos
y 8 horas.

**Como bootstrap: sí, y es la pieza que desbloquea el caso extremo.** Una máquina
sin PowerShell y sin HyperTerminal no tiene forma de recibir *nada* — pero teclear
2–10 KB de receptor a 375 B/s son entre 5 y 27 segundos. Un RP2040 haciéndose
pasar por teclado escribe el script y a partir de ahí el canal serie funciona.

**Necesita hardware, así que es de la fase 19.** Aquí queda el cálculo para que no
se pierda.

---

## 8. Lo que esta ADR NO decide

- **Que funcione sobre un cable.** La fase 16 se prueba sobre un par de
  pseudo-terminales o un `pipe`, y la clase de evidencia se escribe con esas
  palabras exactas: **«no sobre un UART físico ni sobre un cable null-modem»**.
  El cable es la fase 19.
- **ZMODEM.** No se implementa. Es de 1986, su historial de implementaciones está
  lleno de agujeros, y el camino que importa —PowerShell en Windows 7+— no lo
  tiene de todas formas.
- **Audio.** `R8` §5.3: 17 horas por megabyte. No.
