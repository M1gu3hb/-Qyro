# ADR-0027 — Leer y escribir archivos de verdad

- Estado: **congelada** antes de escribir código de filesystem.
- Fecha: 2026-08-08
- Sprint: 5B.1
- Usa, sin modificar: ADR-0017/0019 (manifest), ADR-0026 (`ContentSource`,
  `ContentSink`).
- Fuera: selectores de archivo, permisos de plataforma, FFI. Eso es 5B.2.

## Contexto

`qyro_transfer` mueve búferes. Este sprint pone disco en los dos extremos sin
tocar el motor: las dos costuras son `ContentSource` y `ContentSink`, y **si
hubiera que cambiarlas, estaban mal y eso sería el hallazgo**.

Lo que sigue decide cinco cosas. La primera es de seguridad y es la que importa.

## 1. Symlinks en el destino: se rechazan, y digo qué no cubre eso

`RelativePath` valida la cadena: sin travesía, sin absolutas, sin controles. Eso
no basta, y el motivo es que **el symlink no está en el manifest**. Un manifest
impecable con la ruta `fotos/vacaciones.jpg` escribe fuera de la raíz si `fotos/`
ya existe en el destino y es un enlace a otro sitio. El manifest no puede
expresarlo ni prohibirlo porque el enlace vive en el disco del receptor.

**Decisión: ningún componente de la ruta materializada puede ser un enlace
simbólico.** En concreto:

1. La raíz se canonicaliza **una vez**, al construir el sink.
2. Cada componente de la ruta relativa se comprueba con `symlink_metadata`
   —`lstat`, que **no** sigue enlaces—. Si existe y es un enlace: se rechaza con
   error tipado. No se sigue para ver a dónde apunta: seguirlo para juzgarlo es
   la mitad de la carrera.
3. Los directorios que falten los crea Qyro con `create_dir`, uno a uno, **nunca
   con `create_dir_all`**: crear la cadena entera delega en una función que
   atraviesa lo que no hemos mirado.
4. El archivo `.qyro-part` se abre con `O_NOFOLLOW` en Unix
   (`OpenOptionsExt::custom_flags`) y con `FILE_FLAG_OPEN_REPARSE_POINT` en
   Windows. Los dos vienen de `std::os::*`: **ninguna dependencia nueva**.
5. Después de abrir, el padre se canonicaliza otra vez y se comprueba que sigue
   dentro de la raíz.

**Qué garantiza `O_NOFOLLOW`:** que el **último** componente no se sigue. Si
alguien sustituye el nombre del `.qyro-part` por un enlace entre nuestra
comprobación y nuestro `open`, el `open` falla en vez de escribir en el destino
del enlace. Eso cierra la carrera del componente final por completo, porque la
comprobación y la apertura son la misma llamada al sistema.

**Qué NO garantiza, y hay que decirlo:** los **componentes intermedios**. Entre
comprobar que `fotos/` no es un enlace y abrir `fotos/x.qyro-part` hay una
ventana en la que un atacante con escritura en el destino podría sustituir
`fotos/` por un enlace. Cerrarla exige abrir cada directorio por descriptor y
resolver relativo a él —`openat` con `O_NOFOLLOW`, o `dirfd`—, que no está en
`std` y que en Windows es otra cosa. **No se hace en 5B.1** y queda registrado
como QYR-0072.

La ventana es real y es pequeña, y hay que ser preciso sobre a quién protege
esto: un atacante con escritura en el directorio de destino ya puede escribir lo
que quiera **ahí**. Lo que estas comprobaciones impiden es que use Qyro para
escribir **fuera** de ahí, que es el privilegio que no tiene.

## 2. Colisión en el destino: se rechaza

**Decisión: si el archivo final ya existe, la transferencia falla con error
tipado y no lo toca.**

Las tres opciones y por qué las otras dos no:

- **Sobrescribir**: pérdida de datos ajenos, en silencio, decidida por el
  emisor. El receptor no ha aceptado eso al aceptar la transferencia.
- **Renombrar** —`foto (1).jpg`—: inventa nombres que el emisor no mandó y que
  el manifest no describe, así que el archivo que llega deja de ser el archivo
  que se acordó. Además es una política de producto, y no hay producto.
- **Rechazar**: el receptor conserva lo que tenía y se entera. Es la única que no
  destruye nada.

Cuando exista UI, «sobrescribir» puede ser una elección **del receptor**, tomada
por persona y por archivo. Hasta entonces, el motor no la toma por nadie.

## 3. Metadatos de reanudación

Un archivo por transferencia, junto al destino: `.qyro-resume`.

    offset  bytes  campo          valor
    0       8      magic          "QYRO-RSM"
    8       1      version        0x01
    9       1      reserved       0x00, debe ser cero
    10      2      item_count     u16
    12      8      transfer_id    u64
    20      12×N   entradas       item_id (u32) ‖ bytes_committed (u64)

Mismo estándar que el blob de identidad: magia primero, versión por nombre, y
**una versión futura se rechaza nombrándola**, sin intentar interpretar nada. Un
formato que adivina qué quiso decir una versión que no conoce es un formato con
dos lecturas.

**Lo que NO se guarda: el estado interno del hasher.** `sha2` no expone un estado
serializable, y serializarlo a mano sería depender de un detalle interno de una
dependencia. Al reanudar, Qyro **vuelve a leer el prefijo del `.qyro-part`** y
reconstruye el hash. Cuesta una lectura secuencial y no cuesta correcciones
cuando `sha2` cambie por dentro.

Consecuencia honesta: reanudar un archivo de 1 GiB relee 1 GiB. Es un coste de
E/S, no de memoria —se relee por chunks—, y aceptarlo aquí es preferible a atar
el formato a la representación interna de una biblioteca.

## 4. `fsync`: qué se sincroniza y qué se acepta perder

Renombrar atómicamente sobre datos que el sistema todavía no ha escrito no es
atómico en la práctica. El orden congelado:

1. Escribir el contenido en el `.qyro-part`.
2. **Verificar el digest** leyendo lo escrito.
3. `sync_all()` sobre el `.qyro-part` — los datos y sus metadatos.
4. `rename` del `.qyro-part` al nombre final.
5. `sync_all()` sobre el **directorio** que lo contiene, en Unix. En Windows no
   hay equivalente directo y **no se hace**; se dice aquí en vez de fingir que
   las dos plataformas dan la misma garantía.

**Ante una caída del proceso** —el proceso muere, el sistema sigue vivo—: el
paso 3 no hace falta para la corrección. El kernel conserva lo escrito y el
`rename` es atómico. Un archivo final existe o no existe; nunca a medias.

**Ante un corte de energía**: el paso 3 es lo que hace que el archivo renombrado
tenga contenido y no un agujero de ceros. El paso 5 es lo que hace que el
`rename` mismo sobreviva. Sin el paso 5 en Windows, **un corte de energía puede
dejar el `.qyro-part` en vez del archivo final**, y ese es exactamente el caso
que el paso 8 de §5 recupera.

Lo que se acepta perder, dicho sin adornos: en un corte de energía puede
perderse la transferencia entera y quedar un `.qyro-part`. Lo que **no** se
acepta perder nunca es un archivo que ya estaba en el destino, y por eso §2
rechaza en vez de sobrescribir.

## 5. Dónde vive el `.qyro-part`, y el `.qyro-part` que sobró

**Junto al destino final, en el mismo directorio.** No en un temporal del
sistema, y la razón es dura: `rename` **no funciona entre sistemas de
archivos**. Un `/tmp` en otro volumen convierte el paso 4 en copiar y borrar,
que no es atómico y que además duplica el archivo en disco. Mismo directorio,
mismo volumen, `rename` atómico.

Nombre: `<nombre final>.qyro-part`. Va en el mismo directorio, así que hereda sus
permisos y su cuota, que es lo que el receptor eligió al elegir el destino.

**Un `.qyro-part` de una ejecución anterior**: se **reanuda si hay
`.qyro-resume` que lo describa, y se descarta si no.**

- Con metadatos: `bytes_committed` dice hasta dónde llegó. Qyro trunca el
  `.qyro-part` a esa longitud —lo que haya después nunca se confirmó— y sigue.
- Sin metadatos: un `.qyro-part` huérfano no se puede verificar contra nada, y
  quedarse con él sólo puede producir un archivo que nadie mandó. Se borra al
  empezar la transferencia que reclamaría ese nombre.

**No se pregunta.** Preguntar necesita UI, y este sprint no la tiene; inventar
aquí una política interactiva sería congelar una decisión de producto en un
motor.

## Alternativas descartadas

- Sobrescribir o renombrar en la colisión. §2.
- Guardar el estado del hasher. §3.
- `.qyro-part` en un temporal del sistema. §5, `EXDEV`.
- `create_dir_all`. §1.3.
- Seguir el enlace para juzgarlo. §1.2.

## Lo que esta decisión no promete

- **No cierra la carrera de los componentes intermedios.** §1, QYR-0072.
- **No hay selector de archivos.** La lista de archivos se la pasa el llamante.
- **No hay red.** El transporte sigue siendo un `Vec<u8>` entre dos valores.
- **No está probado en hardware físico**, y las garantías de `fsync` que §4
  describe **no se han comprobado cortando la corriente**: lo que se prueba en CI
  es la caída del proceso, que es un fallo distinto.

## Enmienda 2026-08-11 — qué metadatos describen un parcial

QYR-0101 concreta la frase «`.qyro-resume` que lo describa» de §5 sin cambiar
el formato de §3. Los metadatos describen un `.qyro-part` sólo cuando su
`transfer_id` coincide con el manifest actual y contienen una entrada para ese
`item_id`. Un `transfer_id` distinto o una entrada ausente hacen que el parcial
sea huérfano y se descarte antes de escribir. Metadatos presentes pero mal
formados siguen produciendo su error tipado; no se reinterpretan como ausencia.

## Enmienda 2026-08-11 — carrera de componentes intermedios (QYR-0072)

Se adopta la opción **(c), mitigación parcial sin dependencias**, y no se
presenta como cierre de la carrera. La raíz se canonicaliza y almacena al crear
`FileSink`. Cada apertura de un `.qyro-part` vuelve a canonicalizar su padre
**después de obtener el handle y antes de truncar, borrar o escribir**; si ya no
está bajo la raíz, devuelve `FsError::EscapesRoot`. Esto implementa por fin el
paso 5 de §1, que la ADR original exigía pero el código no ejecutaba.

Las alternativas se evaluaron así:

- **(a), resolución por descriptor:** es la única que cierra la carrera. En
  Unix necesita recorrer con `openat`/`O_NOFOLLOW` y operar después mediante
  descriptores; en Windows necesita la semántica equivalente relativa a un
  handle. No está en `std`. `libc` añadiría una dependencia —que este sprint no
  puede incorporar sin confirmación— y `extern` manual añadiría otro crate con
  `unsafe`, además de una segunda implementación de plataforma. No se finge que
  proteger sólo el `open` arreglaría los `rename`/`remove` posteriores.
- **(b), aceptar sin mitigación:** conserva cero dependencias, pero deja sin
  implementar una comprobación ya congelada en §1.5 y permite que un cambio de
  padre persistente llegue hasta la primera escritura. Se descarta.
- **(c), canonicalización post-open:** detecta el cambio simple y persistente
  antes de escribir o truncar y se expresa igual con `std` en todos los hosts.
  Es la mejora proporcionada al alcance y al modelo actuales.

Lo que sigue **sin cubrir** es concreto: un atacante que pueda cambiar el padre
puede dirigir el `open` fuera y restaurarlo antes de la canonicalización; el
handle exterior pasaría la comprobación basada en nombre. La creación del
`.qyro-part` puede dejar un archivo vacío fuera antes del rechazo. Además,
`digest`, `rename` y `remove_file` siguen resolviendo nombres y conservan sus
propias ventanas. Cerrar esas tres propiedades requiere la opción (a) completa,
no otra comprobación de ruta. QYR-0072 queda resuelta como decisión de riesgo y
mitigación, no como garantía de ausencia de TOCTOU.
