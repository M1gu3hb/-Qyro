# R1 — Reglas no negociables

**Estas reglas no se saltan por ningún motivo, ni siquiera «sólo esta vez para
desbloquear».** Cada una está aquí porque su violación ya costó algo en este
proyecto, y en cada caso digo qué.

---

## 1. Seguridad — lo que nunca se hace

- **Nada de criptografía casera.** Ni un hash propio, ni un cifrado propio, ni un
  KDF propio, ni «esto es como HMAC pero más rápido».
- **Ningún nonce aleatorio sin diseño escrito.** El esquema actual es prefijo de
  4 bytes ‖ secuencia big-endian de 8, y **jamás se repite**. Repetir un nonce en
  ChaCha20-Poly1305 rompe la confidencialidad y la autenticación a la vez.
- **Ninguna clave privada llega nunca a Dart.** Hay una prueba de cierre
  transitivo que lo garantiza. Si una fase necesita relajarla, **eso es una
  decisión de ADR con su razonamiento escrito**, no un cambio de una línea.
- **Ninguna clave en `SharedPreferences`, en `UserDefaults`, en un `.plist`, ni en
  un log.** Nunca, en ninguna plataforma, ni «temporalmente».
- **Ninguna clave determinista en producción.** Los constructores deterministas
  son `pub(crate)` a propósito.
- **Ninguna validación se desactiva para poner CI en verde.** Si CI está rojo, el
  código está mal o la prueba está mal; nunca es la validación la que sobra.
- **Ningún advisory se ignora globalmente.** `cargo audit --deny warnings`.
- **Ningún archivo recibido se guarda con la ruta que el peer dijo, sin validar.**
  `RelativePath` valida la cadena y `safe_path::resolve_under` valida el destino;
  **los dos hacen falta**, y el segundo existe porque el symlink vive en el disco
  del receptor y el manifest no puede expresarlo.
- **Ningún enlace simbólico ni junction se acepta sin política.** La política está
  en ADR-0027 §1 y se prueba en tres sistemas.
- **Ningún archivo recibido se ejecuta, ni se abre automáticamente.**
- **Ninguna identidad se confía automáticamente por el hecho de firmar.** El
  handshake prueba posesión de una clave, **no** que sea la clave que querías. La
  confianza la decide `decide_trust` contra el almacén de peers.

## 2. Dependencias — cero externas

**`Cargo.lock` tiene 63 paquetes y todos son de primera parte.** Siete sprints sin
añadir uno.

**Si crees que necesitas una dependencia: para, escríbelo en el informe, y no la
añadas.** Di qué problema resuelve, cuántos crates transitivos arrastra **medido
con `cargo tree`, no estimado**, su licencia, el resultado de `cargo audit`, y la
alternativa sin dependencia con su coste.

Ya hay tres precedentes de cómo se decide bien:

- **ADR-0024 §1** rechazó `windows-sys` para DPAPI: tres símbolos planos
  exportados por nombre, se escriben a mano.
- **ADR-0025 §1.4** aceptó `jni-sys` para JNI: una tabla de ~233 punteros a
  función **cuyo orden es la ABI**. Transcribir tres firmas y transcribir un orden
  de 233 entradas no son el mismo riesgo.
- **La investigación del 2026-08-11** rechazó `flutter_rust_bridge` (47–60 crates,
  con `tokio`, `regex` y `backtrace`) porque `dart:ffi` ya resuelve el problema.

`cargo-mutants` es **herramienta de desarrollo**, no dependencia: no entra en
`Cargo.lock`.

## 3. Git

- **Nunca commits en `main`.** Nunca merge a `main`, nunca PR, nunca rebase de
  ramas ajenas, **nunca force-push**.
- **Nunca borrar una rama.**
- Cada fase deja `git status --short` **limpio**.
- **Nunca reescribir la historia** para «limpiar» commits.

## 4. Evidencia — el lenguaje es parte del contrato

**Nunca conviertas «compiló en Linux» en «funciona».** Las clases son distintas y
hay que nombrarlas: *compilado / probado en unidad / probado en integración /
probado entre procesos / probado en emulador / probado en simulador / probado en
hardware físico / probado por un usuario / probado en una release*.

**Una afirmación sin clase de evidencia se audita como no probada.**

- **Nada se declara terminado si sólo existe parcialmente.** «Parcial» es una
  respuesta válida; «hecho» sin evidencia no lo es.
- **Toda tabla de runs de CI es exhaustiva.** Los fallos y las cancelaciones del
  camino también. *Una lista de la que se pueden caer los fallos no es evidencia,
  es un resumen favorable.*
- **Todo número va con el comando que lo produjo**, y se vuelve a obtener si han
  pasado fases.

## 5. Pruebas — las cinco trampas que este repositorio ya produjo

No son hipotéticas. Las cinco ocurrieron aquí.

1. **Una aserción cuyos dos lados son la misma llamada.** Cinco veces. La última,
   `digest_of(&victim) == digest_of(&victim)`, en una prueba de seguridad. **Hoy
   hay una guarda que lo caza** — no la silencies con un `allow`.
2. **Un contador que registra una constante en vez de lo medido.**
   `PEAK_BUILDER_READ.fetch_max(HASH_BUFFER_LEN, ...)` seguido de
   `assert_eq!(peak, HASH_BUFFER_LEN)`. La prueba «no carga el archivo» pasaba
   aunque lo cargara entero.
3. **Una prueba cuyo nombre enuncia una propiedad y no la ejerce.**
   `a_symlink_at_the_final_component_is_refused` nunca abría un archivo, y
   desactivar `O_NOFOLLOW` entero dejaba 388 tests en verde.
4. **Una cota extrapolada de una muestra de uno.**
5. **Un `Ok` que nadie mira.** `FileSink` escribía metadatos de reanudación que
   ningún código de producción leía jamás.

**Y la regla que las mata todas:**

> Por cada propiedad que declares probada, **borra el control que la produce y
> comprueba que alguna prueba falla con nombre**. Una propiedad que sobrevive al
> borrado de su propio control no está cubierta.

**Y la sexta, que es más fina y salió en la fase de red:** una prueba cuya *forma*
no distingue un contador medido de una constante. Si una constante satisface tus
aserciones, la prueba está mal aunque el contador esté bien. La forma que sí
distingue: **dos tamaños, y una desigualdad estricta entre ellos**.

**Y la séptima, que es la mejor idea que ha producido este proyecto:**

> **Toda medición nueva viene con una prueba de que la medición podría fallar.**
> `a_descriptor_leak_would_be_visible_to_this_measurement` filtra cuatro
> descriptores a propósito para comprobar que el contador los ve. *Una medida que
> no puede ver una fuga no es evidencia de que no la haya.*

## 6. Documentación

- **Toda ADR se congela ANTES del código que decide**, y se puede comprobar en el
  historial. Si el código se escribe primero, la ADR documenta lo que pasó en vez
  de decidirlo.
- **Ninguna superficie congelada se ensancha ni se estrecha de refilón.**
  *Ensanchar un formato congelado como efecto secundario de otro sprint es cómo se
  pierde el control de un formato.* Si hace falta, es una ADR propia.
- **`STATUS.md` es la fuente de verdad ejecutable** y no puede nombrar un
  `Verified commit` a más de diez commits de HEAD.
- **Todo `QYR-00xx` citado en cualquier archivo tiene ficha en
  `BUGS_PENDING.md`.** El checker lo exige y bloquea.

## 7. El ledger — la lección que costó un P1

**Nunca vuelques la salida de una herramienta en `BUGS_PENDING.md`.**

El 2026-08-11 un barrido de `cargo-mutants` se volcó entrada por entrada: el
ledger pasó de 71 a 279 fichas y de 20 a 167 abiertas, con títulos como
«Superviviente de mutación 022 en qyro_manifest» y **doce *timeouts* registrados
como deuda P2**. La lista seguía siendo correcta y **había dejado de ser legible**,
que es lo único que la hacía útil.

**El barrido va a un informe con su tabla y su alcance declarado. Al ledger van
fichas escritas a mano, con título que una persona entienda y severidad juzgada.**
Detalle completo en `R4`.

## 8. El producto

- **Sin nube, sin cuentas, sin anuncios, sin telemetría, sin servidor central.**
- **Los botones Enviar y Recibir siguen `onPressed: null` hasta la fase 05**, y
  sólo si las fases 01–04 cumplieron lo que prometen.
- **Nada se habilita «para probar».**

## 9. El proceso

- **Una fase se termina entera o no se termina.**
- **Cada fase pasa su puerta** (`R2`) antes de que empiece la siguiente. **Si una
  puerta falla, se arregla y se repite entera**, no se parchea en la fase
  siguiente.
- **Si algo es imposible, para y escríbelo.** Ya pasó dos veces y las dos fueron
  correctas: el harness de Android que estructuralmente no podía alcanzar Keystore
  (QYR-0064), y las tres reglas contradictorias del ledger. **Parar y reportar es
  siempre mejor que improvisar un arreglo que dé verde sin probar nada.**
- **Si un documento de este directorio se contradice a sí mismo o con el código,
  eso es un hallazgo.** Regístralo y dilo. Los documentos no son sagrados; la
  evidencia sí.
