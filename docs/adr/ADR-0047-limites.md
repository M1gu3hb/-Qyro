# ADR-0047 — Los límites, con números

**Estado:** congelada · **Fecha:** 2026-08-18 · **Fase:** 22
**Fuentes:** `R7` §R7.5 y §5 · `R8` §4 y §5.1 · ADR-0027 · ADR-0044 §5

> **Un límite sin número es un límite que nadie respeta.** Todo lo aquí escrito
> es una cifra o una regla que un test puede comprobar.

---

## 1. Lo que hoy está probado, y es poco

Un archivo de **256 KiB** entre dos procesos que no fallan. Todo lo demás —una
carpeta, cien archivos, 8 GB, un disco que se llena, cerrar la tapa a mitad— es
una suposición. `R7` §R7.5 promete *«notas, texto, fotos, vídeos, documentos,
carpetas enteras»*.

---

## 2. Decisión 1 — techos de tamaño

| | Número | Por qué ése |
|---|---|---|
| Por archivo, red | **sin techo propio** | Ver §2.1: los contadores son de 64 bits, medido |
| Por archivo, óptico | **20 MB**, y se niega diciendo cuánto tardaría | ADR-0044 §5, ya implementado |
| Por archivo, serie | **sin techo duro**, con la estimación delante | ADR-0045; a 9–11 KB/s la estimación es el techo real |
| Por transferencia | **el mismo que por archivo** | `R7` §5: una transferencia cada vez, sin cola |

### 2.1 — El desbordamiento que se temía **no existe**, y está medido

El documento de fase avisa: *«todo `u32` en el camino es una bomba… el FFI de
progreso usa enteros. Hay que mirarlo, no suponerlo.»* Mirado:

```
qyro_session::Progress { done: u64, total: u64, item: u32 }
qyro_session_progress(handle: u64, out_done: *mut u64,
                      out_total: *mut u64, out_item: *mut u32)
```

**`done` y `total` son de 64 bits en el motor y en la frontera C.** El único
`u32` es `item`, que cuenta elementos del manifiesto y **vale siempre cero**
porque el motor nunca lo asigna (D4). Un archivo de 8 GB no da la vuelta a
ningún contador.

Lo que **no** está comprobado es que el camino entero funcione con esos tamaños,
y por eso el escenario 3 de la fase existe. **La aritmética es correcta; la
evidencia falta.** Son dos cosas distintas y ésta ADR no las confunde.

---

## 3. Decisión 2 — techo de número de archivos: **256**

Y la razón es Android, no el gusto: el límite de descriptores abiertos por
proceso es duro, y el selector de archivos entrega **descriptores**, no rutas
(ADR-0034). Doscientos cincuenta y seis deja margen ancho bajo cualquier
`RLIMIT_NOFILE` razonable y es más de lo que nadie selecciona a mano.

**Al pasarlo se niega por nombre, antes de abrir nada.** Un fallo por
agotamiento de descriptores llega como un error del sistema en mitad de la
transferencia, con el archivo a medias; una negativa contada llega antes de
empezar y dice el número.

---

## 4. Decisión 3 — carpetas: la política ya existe, se aplica

**No se reinventa.** ADR-0027 ya decidió, y lo que falta es comprobar que se
aplica por este camino:

- **La estructura se preserva**, relativa a la raíz común.
- **Cada componente se comprueba con `symlink_metadata`** al materializar
  (ADR-0027 §2). Un symlink **no está en el manifiesto**, así que no puede
  materializarse: la comprobación existe porque un enlace colocado durante la
  transferencia convertiría una escritura dentro del destino en una escritura
  fuera.
- **Carpetas vacías: no viajan.** El manifiesto lista archivos, y una carpeta
  vacía no es un archivo. Se dice aquí en vez de descubrirse: quien manda una
  carpeta con un directorio vacío dentro no lo encontrará al otro lado, y **eso
  es una pérdida de información aunque no sea una pérdida de bytes**. Si alguna
  vez importa, el manifiesto necesita entradas de directorio y eso es una
  versión de protocolo, no un parche.

---

## 5. Decisión 4 — la reanudación: **se retira de la v1.x**

`qyro_transfer::request_resume` existe, emite `MessageType::Resume`, y **su único
llamante es un test**. No hay símbolo en la frontera C ni bandera en el CLI.
**Ninguna de las dos caras puede invocarla.** Sería el noveno caso de este
proyecto.

Las dos salidas valían. Se elige **retirar**, y el argumento es aritmético:

| Canal | 1 GB | Lo que ahorra reanudar |
|---|---|---|
| Red (`R8` §4, ~10 MB/s) | ~100 s | **segundos** |
| Serie (`R8` §5.1, 9–11 KB/s) | ~28 h | horas — pero ADR-0045 §5 ya limita por estimación |
| Óptico (`R8` §4, ~8 KB/s) | ~36 h | horas |

**Sobre una red, reanudar ahorra segundos y cuesta la pregunta más difícil que
hay aquí:** qué pasa si el archivo de origen cambió entre el corte y la
reanudación. Contestarla mal no da un error — da **un archivo corrupto que
verifica su propio hash porque el hash se recalculó**. Un mecanismo cuyo modo de
fallo es «entrega silenciosamente algo que nunca existió» no entra en una v1.x
para ahorrar segundos.

**Lo que se hace en concreto:**

1. `request_resume` y el manejo de `MessageType::Resume` **se marcan
   `#[cfg(test)]` o se borran**, y el mensaje del protocolo queda **reservado**
   —no se reutiliza el número— para que una v2 pueda añadirlo sin romper nada.
2. **Se borra de todos los documentos que la mencionan.** Una capacidad retirada
   que sigue anunciada es la misma mentira que una capacidad muerta.
3. **La reanudación que sí importa es la del canal óptico** (D11, ADR-0044 §5), y
   es **otro mecanismo**: allí no se retransmite nada porque el fountain no tiene
   piezas numeradas; lo que hace falta es un punto de control de los bloques ya
   decodificados. Queda con dueño y fecha, y no se confunde con ésta.

---

## 6. Decisión 5 — nombres en una terminal

La GUI tiene `safeDisplayName` y su prueba. **Una terminal es otro problema:** un
nombre con `\r` reescribe la línea que la persona está leyendo, y uno con
secuencias VT puede mover el cursor o cambiar colores.

**La regla, y es una sola:** al dibujar un nombre en una terminal, **todo
carácter de control C0 y C1** (`U+0000`–`U+001F`, `U+007F`–`U+009F`) se sustituye
por `U+FFFD`. No se recorta, no se escapa con barras, no se interpreta: se
sustituye, uno por uno, para que la longitud siga siendo comparable y el nombre
siga siendo reconocible.

**Se sustituye y no se elimina** por lo mismo que ADR-0036 decidió para la GUI:
un nombre que era sólo controles no puede convertirse en una fila vacía, porque
una fila vacía es una fila que nadie ve.

**Y el saneado es sólo para dibujar.** El nombre que va al manifiesto y el que se
escribe en disco pasan por las reglas de ADR-0027, que son otras y más estrictas.

---

## 7. Lo que esta ADR NO decide

- **Una cola.** `R7` §5: una transferencia cada vez.
- **Compresión automática**, salvo donde `R8` §4 la exige.
- **Que ninguno de estos límites esté comprobado.** Esta ADR fija los números;
  los cinco escenarios de la fase son los que dan la evidencia, y hasta que
  existan **estos límites son decisiones, no medidas**.

---

## Enmienda 1 (2026-08-31, fase 28) — el techo de 256 estaba calculado contra un descriptor por archivo, y eran dos

La §3 dice que 256 «deja margen ancho bajo cualquier `RLIMIT_NOFILE` razonable».
**El margen no era ancho: era la mitad de lo que decía**, y ésta es la primera
vez que alguien lo mide en vez de razonarlo.

Una transferencia de 200 archivos mantenía **402 descriptores abiertos a la
vez** — dos por archivo, no uno:

```
[measure] 200 files: 4 descriptors before, 406 at the peak, 402 extra
```

- El origen (`FileSource::try_read`) abría en el primer trozo y no cerraba
  nunca.
- El destino (`FileSink::part_for`) abría el `.qyro-part` en el primer trozo y
  sólo lo soltaba en `finish_item`, que corre al final — cuando ya están todos
  abiertos.

Con dos por archivo, **256 archivos son ~512**, que es exactamente el techo por
omisión del CRT de Windows. El número de la §3 seguía siendo defendible; la
aritmética que lo defendía, no.

**Lo que cambia y lo que no.** El techo **sigue siendo 256** y se sigue negando
por nombre antes de abrir nada: el número era razonable y lo que estaba mal era
la cuenta debajo. Lo que cambia es el consumo (QYR-0391): el destino cierra la
parte en cuanto tiene los bytes que el manifiesto declara —se podía, porque
`finish_item` ya verificaba **por ruta**— y el origen mantiene una caché de
ocho. Medido igual después: **11 de más**, y no crecen con el número de
archivos.

**La excepción que esta enmienda escribe para que no se pierda:** el desalojo
**sólo toca lo que tiene ruta**. En Android el selector entrega descriptores
(ADR-0034) y cerrar uno de ésos no ahorra un descriptor — pierde el archivo,
porque no hay forma de reabrirlo. Es la misma razón por la que existe el techo,
y habría sido la forma más fácil de romper el arreglo mientras se arreglaba.
