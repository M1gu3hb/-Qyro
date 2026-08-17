# FASE 16 — El canal serie: meter datos en una máquina que no puede leer un QR

> El nivel 4 de la escalera, y **la respuesta literal a la escena de `R7` §2.** Un PC
> de sobremesa viejo puede *mostrar* un QR pero no puede *leer* uno: no tiene cámara.
> Esta fase es cómo se le mete un archivo de todos modos.

---

## 1. Por qué existe

Palabras del propietario:

> «Tengo computadoras viejas… Los puertos que tiene ya no sirven, entonces no le
> puedo ni conectar USBs. Entonces, ¿de qué manera les comparto archivos?»

El canal óptico no sirve en esa dirección: hace falta una cámara en la máquina que
recibe, y no la hay. Lo que **sí** hay en un desktop de esa era, con total
seguridad, es un **puerto serie DB9**.

`R8` §5.1, medido: **115 200 bps, framing 8N1 ⇒ 9–11 KB/s reales. 1 MB en 1,6
minutos. 10 MB en 16 minutos.** Un orden de magnitud mejor que el QR.

**Y la pieza que lo hace posible sin instalar nada:**

- **Windows XP trae HyperTerminal de serie**, con XMODEM/YMODEM/ZMODEM/Kermit.
- **Windows 7 no lo trae, pero trae PowerShell 2.0**, que expone
  `System.IO.Ports.SerialPort` directamente. **Un receptor completo son ~15 líneas.**

**Qyro genera ese script y lo enseña en pantalla.** Ésa es la función. El usuario lo
teclea o lo pega, y la máquina vieja pasa a ser un receptor sin haber instalado nada.

---

## 2. Antes que nada: preguntar lo aburrido

`R8` §5.4. **Antes de proponer serie, Qyro pregunta:**

> ¿La máquina tiene lector de CD, disquetera, ranura PCMCIA o tarjeta de red?

Porque cualquiera de las cuatro es **entre 10 y 10 000 veces más rápida**. Un CD-R
mueve 700 MB en cinco minutos. Un cable de red mueve 1 MB en menos de un segundo.
**Proponer el canal lento sin haber descartado los rápidos es mal producto**, y va en
la interfaz, no en un README.

---

## 3. La decisión que hay que congelar

`docs/adr/ADR-00XX-canal-serie.md`. Decide:

1. **Cómo se habla con el puerto.** `serialport` es una dependencia externa con
   licencia distinta a la del proyecto; hablar con `CreateFile` + `SetCommState` +
   `DCB` en Windows y `termios` en Linux son ~200 líneas y el proyecto ya tiene
   precedente de Win32 crudo confinado en un crate (`qyro_win_dpapi`, ADR-0024 §1).
   **Elige, escribe por qué, y si eliges Win32 crudo, el `unsafe` va confinado y
   declarado como allí.**
2. **El protocolo de línea.** No uses ZMODEM. Necesitas: framing, CRC por bloque,
   ARQ o FEC, y reanudación. **Decide si reutilizas el fountain LT de la fase 15** —
   que resolvería la pérdida sin necesitar canal de retorno — o si haces ARQ clásico
   aprovechando que el serie **sí** es bidireccional. **El serie tiene retorno; el
   óptico no. Es la diferencia que decide.**
3. **Qué se le pide al receptor tonto.** El script de PowerShell/HyperTerminal es
   simple por obligación: no puede hacer criptografía ni fountain decoding. Decide el
   **modo degradado**: probablemente `bloque + CRC32 + reintento`, con el binario
   reensamblado por `certutil -decode` desde Base64. **Y escribe qué garantía se
   pierde en ese modo** — casi seguro la autenticación, y eso va al modelo de
   amenazas.
4. **Baudios y handshake de flujo.** 115 200 y RTS/CTS por hardware si el cable lo
   lleva; si es un cable de 3 hilos, XON/XOFF y **no** se puede mandar binario crudo.
   Ésa es otra razón para Base64 en el modo degradado.

---

## 4. Entregables

1. **La ADR de §3, congelada.**
2. **Enumerar puertos serie** y enseñarlos con su nombre (`COM3`,
   `/dev/ttyUSB0`), porque el usuario tiene que saber cuál eligió.
3. **`qyro send <archivo> --serial <puerto>`** con el protocolo completo, y
   **`qyro recv --serial <puerto>`** para cuando las dos máquinas pueden correr Qyro.
4. **El generador del receptor tonto.** `qyro serial-bootstrap` imprime, listo para
   copiar:
   - Para **Windows 7+**: el script de PowerShell 2.0 con `System.IO.Ports.SerialPort`,
     el bucle de lectura, y la línea de `certutil -decode` que reconstruye el binario.
   - Para **Windows XP**: las instrucciones de HyperTerminal —velocidad, paridad,
     control de flujo, `Transfer → Receive File`— y el protocolo que hay que elegir.
   - Para **Linux viejo**: la línea con `cat < /dev/ttyS0 > salida` y `stty`.
   **Con los valores concretos rellenados**, no con `<puerto>` genérico.
5. **El emulador de teclado PS/2, escrito como nota, no como código.** `R8` §5.2:
   como canal de datos es de 37–375 B/s y 1 MB tarda entre 47 min y 8 h — **no**.
   Como **bootstrap** para teclear 2–10 KB del receptor en una máquina que ni siquiera
   tiene PowerShell, **sí**, y es la pieza que desbloquea el caso extremo. Necesita
   hardware (un RP2040), así que **es de la fase 19**. Aquí sólo se documenta la
   decisión y el cálculo, para que no se pierda.

---

## 5. La prueba que cierra la fase

**Sin hardware, y aun así real:**

> Dos procesos, un **par de pseudo-terminales** (`socat -d -d pty,raw,echo=0
> pty,raw,echo=0` en Linux; en Windows, un par de puertos virtuales o un `pipe` que
> implemente el mismo trait). Un archivo cruza, se verifica por SHA-256.

**Controles:**
1. **Inyectando un 5 % de bytes corruptos**, la transferencia **sigue completándose**
   (si elegiste FEC) o **se recupera con reintentos contados** (si elegiste ARQ) — y
   el número de reintentos se **mide y se asevera**, no se asume.
2. Inyectando **20 %**, falla con un error nombrado en un tiempo acotado. **No se
   cuelga.**
3. **El script de bootstrap generado se ejecuta de verdad**: en CI, `pwsh` corre el
   PowerShell que Qyro imprimió, contra el pty, y recibe el archivo. **Un script
   generado que nadie ha ejecutado es un script que no funciona** — y este proyecto ya
   pagó por esa lección.

**La clase de evidencia, escrita con precisión:** «probado sobre pseudo-terminales,
no sobre un UART físico ni sobre un cable null-modem». **No la subas de categoría.**
El cable real es la fase 19.

---

## 6. La puerta

Quince comprobaciones. En la 15, la cadena completa incluye al humano:
**«la persona conecta el cable» → «Qyro enumera los puertos» → «Qyro imprime el
script» → «la persona lo pega en la máquina vieja» → «los bytes cruzan» →
«`certutil -decode` reconstruye» → «el hash coincide».** Escríbela entera, con el
paso humano incluido y marcado como tal.

---

## 7. Lo que NO hay que hacer

- **No implementes ZMODEM.** Es un protocolo de 1986 con implementaciones
  históricamente llenas de agujeros, y no lo necesitas: HyperTerminal lo tiene, pero
  el camino de PowerShell no, y ése es el que importa en Windows 7+.
- **No añadas audio.** `R8` §5.3, con sus números: 17 horas por megabyte.
- **No prometas que el modo degradado está autenticado.** Si el receptor es un script
  de 15 líneas, no lo está. **Dilo en pantalla, antes de mandar.**
- **No inventes que se probó sobre un cable.** Es lo único que arruinaría el proyecto.
