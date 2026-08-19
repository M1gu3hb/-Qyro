# FASE 25 — El producto completo

> Lo que hoy existe transfiere **un archivo** entre dos aparatos. Lo que la gente
> hace, medido en trece proyectos reales (`R11` §1), es mandar **muchos**, mandar
> **carpetas**, y mandar **texto**. Esta fase convierte un transferidor en una app.

---

## 0. La contradicción que hay que resolver antes de tocar código

`FASE-22` §6 dice **«no añadas una cola»**, citando `R7` §5: *Qyro no es un gestor
de descargas.* `R11` §1 dice que **13 de 13** proyectos del sector tienen varios
archivos con progreso agregado.

**No se contradicen: son dos cosas distintas y hay que separarlas por escrito.**

- **Lote** = *una* transferencia que contiene *N* archivos, con un progreso
  agregado, un botón de cancelar y un destino. Sigue siendo **una** sesión, **un**
  handshake, **una** decisión del receptor. ✅ **Esto se hace.**
- **Cola** = varias transferencias independientes esperando turno, con
  reordenación y reintentos. ❌ **Esto sigue prohibido por `R7` §5.**

Escríbelo en la ADR con esas dos frases. Es la diferencia entre completar el
producto y convertirlo en otro.

---

## 1. QYR-0365 primero, porque bloquea todo lo demás

La única ficha abierta: **más de 50 archivos y la sesión corta a los 60 s
informando fallo, mientras todos los archivos llegaron.** Un producto que dice
«falló» sobre algo que funcionó es peor que uno que falla.

**Y su diagnóstico está falsado.** La última medición: **20 archivos en 0,06 s,
cero lecturas agotadas, entre dos procesos de Rust.** El motor no es el problema.
Por descarte queda **el lado Dart o el cruce del FFI**.

**Cómo se ataca, en este orden y sin saltarse ninguno:**

1. **Reproducir con la clase de producción**, no con un arnés. `NativeTransferService`
   bajo `flutter test` contra el binario `qyro`, con 60 archivos. Si no reproduce,
   **el defecto está en la pantalla, no en el servicio**, y eso ya es media
   respuesta.
2. **Instrumentar el cruce**: cuántas llamadas al FFI por archivo, y cuánto tarda
   cada una. Si el progreso cruza la frontera C **una vez por frame** y hay 60
   archivos, son decenas de miles de cruces y ahí está el minuto.
3. **Arreglar la causa, no el síntoma.** Subir el timeout es esconderlo. Si el
   coste es por-frame, el arreglo es **agregar el progreso en Rust y cruzar como
   mucho cada 100 ms**.
4. **La prueba que cierra la ficha:** 200 archivos, y **falla si el tiempo total
   crece más que lineal** respecto a 20. Un timeout mayor no la pasa. **Ésa es la
   pregunta que la ficha hace y ésa es la que hay que responder.**

---

## 2. El aislamiento del destino — antes de aceptar la primera carpeta

`R11` §4: **Warpinator se llevó CVE-2022-42725, CVSS 7.5, por enlaces simbólicos.**
Qyro cifra y verifica el contenido de forma impecable y **eso no protege contra un
nombre**.

**La regla, y no admite matices:**

> El nombre que manda el emisor es **una sugerencia**, nunca una ruta.
> Cada ruta final se **canonicaliza** y se comprueba que **cae dentro** del
> directorio de destino. Lo que no cae, se rechaza **por nombre y con motivo**.

Lo que hay que rechazar explícitamente, y cada uno con su prueba:

| Entrada | Qué es |
|---|---|
| `../../etc/passwd` | traversal clásico |
| `..\\..\\Windows\\System32\\x` | el mismo con separador de Windows |
| ruta absoluta `/tmp/x`, `C:\x` | absoluta disfrazada de nombre |
| `CON`, `PRN`, `AUX`, `NUL`, `COM1`…`LPT9` | **nombres reservados de Windows**, con o sin extensión |
| nombre acabado en `.` o espacio | Windows lo recorta y colisiona |
| un **symlink** o **junction** en el lote | la política ya está escrita en fases anteriores: **búscala y aplícala** |
| `:` en el nombre | Alternate Data Streams en NTFS |
| ruta que pasa de 260 caracteres | Windows sin `\\?\` |
| dos archivos que **normalizan al mismo nombre** | colisión por NFC/NFD o por mayúsculas |

**El nombre viaja en UTF-8 NFC con longitud máxima en bytes.** El receptor sanea y
desambigua (`nombre (2).ext`), **nunca sobrescribe en silencio**.

**Y la contraprueba, que es la que da valor a todo lo anterior:** un test que
**quita** la canonicalización debe hacer fallar la suite. Una defensa que nadie ha
visto fallar no es una defensa.

Esto entra en `THREAT_MODEL.md` **en el mismo commit** que el código.

---

## 3. Carpetas y lotes

1. **Ruta relativa por entrada**, estructura preservada, **carpetas vacías
   incluidas** (se mandan como entrada de tipo directorio, o se decide por escrito
   que no y por qué).
2. **Progreso agregado**: bytes totales / bytes hechos, archivo N de M, y velocidad.
   **Calculado en Rust**, cruzado con freno de tiempo (§1.3).
3. **Un solo cancelar**, que para el lote entero y **no deja ni un `.qyro-part`**.
   Con su contraprueba: dejar uno a propósito y exigir que el listado lo vea.
4. **Descriptores acotados.** En Android abrir 200 a la vez es un límite duro: se
   abre **uno cada vez**. La prueba baja el límite a propósito y exige que
   **falle por nombre**, no que se agote.
5. **Un archivo > 4 GiB** (esparcido, para no gastar disco): el progreso del último
   frame **no puede ser menor** que el del anterior. Un `u32` que da la vuelta se ve
   exactamente así. **Y todo contador que cruce el FFI se revisa a mano** buscando
   `u32`/`int`.
6. **Memoria O(1) por frame.** `R11` §2.5: el líder tuvo que arreglar
   explícitamente recibir archivos mayores que la RAM. La prueba mide el pico y
   **falla si crece con el tamaño del archivo**.

---

## 4. Texto y portapapeles

`R11` §1.5: presente en 8 de 13, y es lo más barato de la lista. Un `kind=text` en
el frame. En la GUI: pegar y mandar. En el CLI: `qyro send --text "…"` y por
tubería (`stdin`). **En el receptor se enseña, se copia, y no se guarda como
archivo salvo que la persona lo pida.**

---

## 5. Los errores, con texto de persona

Cada uno con su mensaje, y **cada mensaje con una prueba que lo exige literal**:

- disco lleno · permiso denegado · ruta demasiado larga · nombre inválido en
  destino · **el receptor rechazó** · **el receptor está ocupado** · **se agotó el
  tiempo**.

Los tres últimos son de `R11` §3: **el handshake de cuatro mensajes de Qyro ya
puede distinguirlos** y confundirlos es el 80 % de los «no funciona».

**Y el diagnóstico de red de `R11` §2.1:** si en 60 s no entra nada habiendo peers
visibles, **banner con botón «diagnosticar»** que dice qué comprobó: puerto
vinculado, cortafuegos, interfaz elegida.

**Puerto de respaldo** (`R11` §2.2): `errno 10013` en Windows es real — Hyper-V,
WSL y Docker reservan rangos. **El formato `QYRO1|ip:port|huella` ya lo soporta**:
si 49517 no se puede vincular, se coge otro y **el código lo lleva dentro**. Sin él,
Qyro con puerto fijo es *más* frágil que el sector, no menos.

---

## 6. Paridad y puerta

La tabla de `docs/PARIDAD-GUI-CLI.md` **crece con las filas de esta fase**: lote,
carpeta, texto, cancelar-lote, puerto de respaldo. **Cada celda con `ruta:línea` o
con `NO -- <argumento>`.** La comprobación 14 aplica por cara. Y la 15 escribe la
cadena entera del lote: **gesto → selección → N descriptores → N hashes → destino
canonicalizado → verificación entrada por entrada**.

---

## 7. Lo que NO hay que hacer

- **No añadas una cola.** §0.
- **No subas un timeout para cerrar QYR-0365.** §1.4.
- **No confíes en un nombre.** §2.
- **No inventes un límite sin medirlo.** Un techo que nadie probó es una promesa.
