# FASE 22 — Lo que la gente hace de verdad

> Todo lo que este proyecto ha probado son transferencias limpias de un archivo
> pequeño entre dos procesos que no fallan. **Nadie ha mandado una carpeta, ni un
> archivo de 8 GB, ni ha llenado un disco a mitad, ni ha cerrado la tapa del portátil
> con una transferencia corriendo.**

---

## 1. Por qué existe

`R7` §R7.5 dice: *«Notas, texto, fotos, vídeos, documentos, carpetas enteras.»* Hoy
la evidencia cubre **un archivo de 256 KiB**. Todo lo demás es una suposición.

Y la lista de lo que no se ha ejercitado es la lista de lo que rompe un producto el
primer día que alguien lo usa en serio:

- Una **carpeta** con subcarpetas. ¿Se preserva la estructura? ¿Qué pasa con una
  carpeta vacía? ¿Con un enlace simbólico? — hay política escrita desde hace fases y
  nadie ha comprobado que se aplique por este camino.
- **Cien archivos** de golpe. ¿Se abren cien descriptores a la vez? En Android eso es
  un límite duro.
- Un archivo de **más de 4 GB**. Todo `u32` en el camino es una bomba: `MAX_PAYLOAD_LEN`
  es 1 MiB por frame, pero el tamaño total, el progreso y los contadores cruzan el
  FFI, y **el FFI de progreso usa enteros**. Hay que mirarlo, no suponerlo.
- El **disco de destino se llena** a mitad. `SessionError::StorageRefused` existe.
  ¿Llega a la pantalla con un texto que una persona entienda? ¿Se limpian los
  `.qyro-part`?
- **Cancelar y volver a empezar.** El motor tiene metadatos de reanudación desde el
  sprint 5A y **nadie ha comprobado nunca que reanudar reanude** en vez de reenviar
  todo.
- **Nombres hostiles** por el camino del CLI: la GUI tiene `safeDisplayName` y una
  prueba; una terminal es otro problema — un nombre con `\r` reescribe la línea, uno
  con secuencias VT puede mover el cursor.

---

## 2. La decisión que hay que congelar

`docs/adr/ADR-00XX-limites.md`. Decide y escribe los números, porque **un límite sin
número es un límite que nadie respeta**:

1. **El techo de tamaño por archivo y por transferencia**, y qué pasa al pasarlo.
2. **El techo de número de archivos**, y por qué ése — en Android el límite de
   descriptores abiertos es la razón, no el gusto.
3. **La política de carpetas**: estructura preservada, carpetas vacías, y qué se hace
   con symlinks y junctions. **Esto último ya tiene política** de fases anteriores:
   búscala y **aplícala**, no la reinventes.
4. **Reanudación**: qué se persiste, dónde, cuánto vive, y **qué pasa si el archivo de
   origen cambió** entre el corte y la reanudación. Esa última es la pregunta que
   decide si la reanudación es segura o es una forma elegante de corromper un archivo.
5. **El nombre en una terminal**: qué se sanea y con qué regla.

---

## 3. Entregables

1. La ADR de §2.
2. **Carpetas de verdad**, en las dos caras, con la estructura preservada.
3. **La reanudación, alcanzable y probada** — o **retirada del producto y de todos
   los documentos que la mencionan**. Las dos salidas valen; lo que no vale es un
   `resume` que existe en el motor y que nadie puede invocar. Sería el quinto caso.
4. **Los errores feos, con texto de persona.** Disco lleno, permiso denegado, ruta
   demasiado larga (Windows: 260 caracteres sin `\\?\`), nombre inválido en el
   destino aunque fuera válido en el origen.
5. **El saneado de nombres para terminal**, con prueba.

---

## 4. La prueba que cierra la fase

Cinco escenarios, todos mecanizados y todos con control:

| # | Escenario | Control |
|---|---|---|
| 1 | Una carpeta con subcarpetas y una vacía | El árbol de destino se compara **entrada por entrada** con el de origen; sobrar un archivo falla igual que faltar |
| 2 | 200 archivos pequeños | Con el límite de descriptores bajado a propósito, **falla por nombre** en vez de agotarse |
| 3 | Un archivo **> 4 GiB** (esparcido, para no gastar disco) | El progreso reportado en el último frame **no es menor** que en el anterior — un `u32` que da la vuelta se ve exactamente así |
| 4 | Disco lleno a mitad | El destino **no queda con ningún `.qyro-part`**, y su contra-prueba: dejar uno a propósito y exigir que el mismo listado lo vea |
| 5 | Cortar a mitad y reanudar | **Se miden los bytes que cruzan la segunda vez**, y la prueba exige que sean **menos** que la primera. Sin esa medida, «reanudó» y «reenvió todo» son indistinguibles |

**El escenario 5 es el que importa.** Es el único que puede fallar de forma
silenciosa y plausible, y su forma —comparar dos medidas— es la única que lo
distingue.

---

## 5. La puerta

Dieciséis comprobaciones. En la 15, la cadena de la reanudación se escribe entera:
**«la persona cancela» → qué se escribe en disco → «la persona vuelve a mandar» →
qué se lee → qué NO se retransmite → el archivo final verifica.**

---

## 6. Lo que NO hay que hacer

- **No añadas una cola.** `R7` §5: Qyro no es un gestor de descargas. Una
  transferencia cada vez.
- **No añadas compresión automática** salvo donde `R8` §4 la exige (el canal óptico
  con texto). Comprimir un JPEG gasta CPU para nada y lo dice el propio `R8`.
- **No inventes un límite «generoso» sin medirlo.** Un techo que nadie probó es una
  promesa.
