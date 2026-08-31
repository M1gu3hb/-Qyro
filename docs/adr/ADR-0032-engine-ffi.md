# ADR-0032 — El FFI del motor, y qué pasa con la guarda más antigua del proyecto

- Estado: **congelada** antes de una sola línea de código de la fase 01.
- Fecha: 2026-08-12
- Fase: 01 — El FFI del motor
- Usa, sin modificar: ADR-0026 (motor), ADR-0027 (filesystem), ADR-0028 (red).
- Fuera: callbacks empujados (fase 02), Dart, UI, selector, descubrimiento.

## Contexto

`qyro_ffi` expone hoy dos funciones y depende sólo de `qyro_core`. Todo lo
construido en siete meses es inalcanzable desde la aplicación. Esta ADR decide
cómo deja de serlo.

El obstáculo no es técnico sino de garantía. `c_abi_contract.rs:149-157` afirma
que el cierre de dependencias de `qyro_ffi` es **exactamente**
`{qyro_core, qyro_ffi}`, y `:114-144` lista catorce nombres prohibidos. Es la
propiedad más antigua del proyecto: *si Dart nunca puede pedir una clave, no hay
forma de que una clave se escape por ahí.*

---

## 1. La medición que decide, y que ninguna de las tres salidas del plan anticipa

Replicando exactamente el recorrido del test —`cargo metadata --format-version 1`,
aristas no-dev, sin `--filter-platform`—:

| Grafo | Cierre | Nombres prohibidos alcanzados |
|---|---|---|
| `qyro_ffi` hoy | **2** | 0 / 14 |
| (a) ffi → {net, transfer, fs, crypto} | **50** | 12 / 14 |
| (b) ffi → session → {net, transfer, fs} | **51** | 12 / 14 |
| (c) ffi → fs *(sólo motor y disco)* | **49** | 12 / 14 |

Y el resultado que gobierna esta ADR:

```
cierre bajo (b):                                            51 nombres
cierre bajo (b) + qyro_crypto directo en qyro_ffi:          51 nombres
DIFERENCIA: []
```

Bajo (c), idéntico: `[]`.

> **Una vez `qyro_crypto` está dentro del cierre, un test con forma de cierre es
> estructuralmente ciego a que `qyro_ffi` tome una arista directa hacia él.**

Esa arista directa —una línea en un `Cargo.toml`— es exactamente lo que la guarda
existe para impedir. Después de esta fase, **ninguna** afirmación sobre el cierre
puede verla, en ninguna de las tres salidas. No es que la guarda haya que
retocarla: es que hace falta **otra clase** de guarda.

## 2. Decisión: la salida (b), un crate `qyro_session`

`qyro_ffi` depende de `qyro_core` y de `qyro_session`, y de nada más.
`qyro_session` es lo único que el FFI ve; expone operaciones y ningún tipo que
lleve material de clave.

**El motivo no es el que da el plan.** No es que (b) conserve la propiedad sin
debilitarla —§1 mide que no—, sino:

> **(b) es la única salida en la que la guarda escrita a mano tiene un límite
> verificado por la máquina.**

La resolución de nombres de Rust (edición 2018+) resuelve `qyro_crypto::X` sólo si
`qyro_crypto` está en el *extern prelude*, y eso ocurre **únicamente** por una
dependencia directa en el `Cargo.toml` de ese crate. No hay forma de nombrar una
dependencia transitiva. Por tanto:

- Bajo (b), la afirmación `dependencias_directas(qyro_ffi) == {qyro_core, qyro_session}`
  —preguntada al resolvedor, **sin ninguna lista que nadie mantenga**— demuestra
  que todo el material criptográfico que `qyro_ffi` puede alcanzar está acotado
  por la API pública de `qyro_session`: una superficie nueva, pequeña y congelada.
- Bajo (a), esa afirmación es **imposible por construcción**: `qyro_crypto` pasa a
  ser dependencia directa y nombrada del crate que Dart carga, y la cota pasa a ser
  la API pública entera de `qyro_crypto` + `qyro_net` + `qyro_transfer` + `qyro_fs`.

La diferencia es categórica, no de grado: (a) y (b) meten la pila criptográfica en
el `.so` igual, pero sólo (b) deja una pregunta de mundo cerrado que una máquina
puede responder.

### Por qué no (a)

Además de lo anterior, el listón que el propio plan le pone —«sólo si demuestras
que la guarda nueva es tan difícil de romper por accidente como la vieja»— no se
puede alcanzar: toda guarda de (a) es una lista de nombres de tipo, y una lista de
nombres falla abierta. Hay precedente vivo en este repositorio, no hipotético:
`AuthenticatedFrame::payload(&self) -> &[u8]` devuelve los mismos bytes secretos
que `into_zeroizing_payload`, y `BYTE_RETURN_MARKERS` no contiene `&[u8]`, así que
la guarda de egreso de `qyro_crypto` **ya** tiene un hueco abierto por esa forma.

### Por qué no (c)

No sólo deja el producto a medias: bajo (c) el motor es **inconstruible**.
`Sender::new` y `Receiver::new` exigen un par `FrameSealer`/`FrameOpener`, y los
dos únicos productores son `into_frame_crypto` de `qyro_crypto` y
`qyro_net::Session::into_parts` — y (c) prohíbe `qyro_net`. Un contribuyente
bloqueado así tiene exactamente un camino más corto a un build que compile:
añadir `qyro_crypto` a `qyro_ffi`, que compila, **no añade ni un paquete** a
`Cargo.lock` y —medido en §1— no cambia el cierre en nada. (c) fabrica justo el
cambio que la guarda existe para impedir y a la vez retira el instrumento que lo
vería.

## 3. La guarda nueva, en tres piezas

Ninguna sustituye a la vieja por sí sola. Las tres juntas acotan lo que queda.

**3.1 — Dependencias directas, sin lista.** `qyro_ffi` declara exactamente
`{qyro_core, qyro_session}`. Se pregunta al resolvedor, no al texto del manifiesto.
Es la única de las tres que no envejece, porque no contiene ningún nombre que
alguien tenga que acordarse de añadir.

**3.2 — La fachada de `qyro_session` no reexporta nada ajeno.** Ningún
`pub use` de otro crate, y la superficie pública es un conjunto congelado.
Incluye la forma sin nombres: `pub use qyro_net::Session;` no menciona
`qyro_crypto` y sin embargo entrega `into_parts() -> (.., FrameSealer, FrameOpener)`
por inferencia.

**3.3 — El cierre pasa a ser un registro de cambios, y se dice que lo es.** La
afirmación de 51 nombres **no impide nada** —§1 lo mide— y se adopta
deliberadamente como bitácora: si el conjunto cambia, alguien lo mira. Venderla
como sucesora de la vieja sería mentir.

Las tres se ven fallar, con pruebas negativas que empalman una arista directa
`qyro_ffi → qyro_crypto` en la salida real del resolvedor y comprueban que 3.1 la
denuncia por nombre mientras 3.3 es ciega a ella. Esa prueba es la que demuestra
que mover la guarda a profundidad uno no fue cosmético.

## 4. Handles

**Entero de 64 bits, no puntero.** Un puntero es una dirección que se
desreferencia: uno corrupto, repetido o fabricado hace que Rust toque memoria que
el llamante influye — comportamiento indefinido y uso después de liberar. Un
índice entero sólo puede fallar una búsqueda, y un fallo de búsqueda es un error
tipado.

**Composición: `generación: u32` en los 32 bits altos ‖ `ranura: u32` en los
bajos.** El plan ofrece un contador que no reutiliza índices *o* una etiqueta en
los bits altos; se toman las dos, porque cada una sola es defectuosa —un contador
sin reutilización crece sin cota durante la vida del proceso, y una ranura sola no
distingue un handle vivo de uno rancio apuntando a una ranura reutilizada.

**Resolución, en este orden, sin pánico y sin desreferenciar:** ranura fuera de
rango → error; ranura vacía → error; generación distinta → error.

**El handle `0` nunca es válido, por construcción y no por un caso especial:** las
generaciones empiezan en **1**, así que `handle >> 32 == 0` no puede coincidir con
ninguna ranura viva. El `0` es justamente el valor accidental más probable desde
Dart. Aun así lleva prueba propia: una propiedad que sólo existe por construcción
es exactamente la que hay que verificar borrando el control.

**Doble cierre = la comprobación de generación.** `close` incrementa la generación
y vacía la ranura; la segunda llamada ya no coincide. No es un código distinto:
«este handle no está vivo» es un solo hecho.

**Agotamiento de generación:** tras 2³² cierres sobre una ranura la generación
daría la vuelta y un handle muy viejo volvería a ser válido. Al desbordar, la
ranura se **retira permanentemente**.

**Un handle que Dart pierda sin cerrar se filtra hasta el fin del proceso. No hay
barrido, y es una decisión.** Un barrido necesita una señal de vida que Dart no
puede dar, y uno que cierre una sesión que Dart todavía usa es peor que la fuga.
La fuga está **acotada**: `MAX_ESTABLISHED_SESSIONS` es 4, así que la tabla tiene
tope y abrir de más es un error tipado, no crecimiento. El finalizador de Dart es
la fase 02.

## 5. Errores

**Toda función `extern "C"` devuelve `i32`. `0` es éxito, los negativos son
errores. Los valores salen por parámetros de salida.** No hay `last_error` por
hilo como canal principal: se lee a posteriori, y Dart no garantiza que la llamada
siguiente llegue por el hilo que falló.

**El estado es un canal distinto del error, y esto corrige el §10 del documento de
fase.** El boceto `while (qyro_step(handle) == QYRO_IN_PROGRESS)` mezcla los dos:
un fallo de transporte y «sigue en curso» llegarían por el mismo `int`. Se congela
en su lugar un parámetro de salida `out_state` con `IN_PROGRESS`, `COMPLETED` y
`REJECTED` — los estados de la tabla del plan menos `error`, que no es un estado
sino el código de retorno.

**Un error en un hilo de Rust que Dart no creó: Dart se entera en la llamada
siguiente.** El plan pide que se diga sin rodeos, y se dice. El trabajador guarda
su error terminal en el registro de la sesión y la siguiente llamada sobre ese
handle lo devuelve. **El error es pegajoso**: una vez fallada, toda llamada
posterior devuelve el mismo código hasta `close`. Lo contrario es el fallo sutil —
un segundo `Ok` deja creer a Dart que la sesión se recuperó cuando su trabajador
está muerto.

**Ninguna cadena de error cruza la frontera en esta fase.** Si hace falta forma
legible, bytes **estáticos** con su longitud, como el par
`qyro_protocol_version_ptr`/`_len` que ya existe: prestados, nunca liberados. Eso
borra el problema de propiedad de cadenas en vez de resolverlo.

## 6. Cadenas y búferes

**Dart posee todo lo que pasa; Rust posee todo lo que devuelve. Nunca mezclado.**

**De entrada** —direcciones, rutas, la lista de archivos— pares
`(*const u8, usize)`, **delimitados por longitud, no terminados en NUL**: una ruta
POSIX es bytes y no tiene por qué ser UTF-8; una longitud no se puede desbordar
por un terminador ausente; y es la convención que este crate ya usa. **Rust copia
al entrar y no retiene el puntero del llamante más allá de la llamada**, lo que
elimina entera la clase «Dart lo liberó mientras Rust lo tenía».

**De salida, en esta fase, nada que haya que liberar — así que no hay
`qyro_free_*` y no hay doble liberación que probar.** Es un estrechamiento
deliberado del §5.3 del plan: esa regla no es incorrecta, es **vacía** en esta
fase, y la ADR lo dice en vez de enviar un par asignador/liberador sin uso. El
progreso son parámetros de salida; el nombre del elemento va a un búfer que asigna
el llamante, con truncamiento reportado y `cap == 0` como forma legal de preguntar
el tamaño. La fase 02 necesitará un liberador de verdad y **eso lleva su propia
cláusula de ADR**, no llega de refilón.

## 7. Hilos

**Toda llamada es segura desde cualquier hilo. Llamadas sobre handles distintos no
compiten. Llamadas sobre el mismo handle se serializan.** La tabla se toma
brevemente para clonar dos `Arc` y se suelta, así que un `step` largo nunca
retiene el cerrojo de la tabla ni bloquea a un handle ajeno.

**`cancel` no toma el cerrojo de la sesión.** Levanta una bandera atómica que
alcanza por la tabla. Tomar el cerrojo lo haría esperar al mismísimo `step` que
intenta interrumpir, es decir: no cancelaría nada.

**Se dice en el nombre.** Exactamente una llamada puede bloquear sin cota y lleva
el sufijo: **`qyro_session_step_blocking`**. Las demás son acotadas. El sufijo es
la señal que Dart necesita para saber qué no puede correr en el isolate de UI.

**Cerrar durante un `step` en vuelo:** `close` levanta la bandera, sube la
generación —con lo que todo handle pendiente queda rancio de inmediato— y luego
toma el cerrojo para el desmontaje, que espera a que el `step` retorne.

## 8. Pánico

**Cada función `extern "C"` es exactamente un `catch_unwind`, el más externo,
envolviendo todo incluida la validación de argumentos.** El trabajo real vive en
una función privada, así que la cáscara `extern "C"` es fina y comprobable
mecánicamente. La validación va **dentro**, para que un pánico validando también
se atrape.

**Un pánico atrapado envenena la sesión, no la devuelve a un estado usable.** Es la
parte que se suele hacer mal: un pánico a mitad de `step` deja la máquina de
estados en un estado desconocido, y devolver el código dejando la sesión utilizable
invita a Dart a reanudar desde ahí.

**Dos cosas se afirman, no se suponen:**

1. **`panic = "abort"` no se pone nunca.** Hoy ningún perfil lo pone, lo que
   significa que el diseño entero descansa sobre una ausencia no escrita. Bajo
   `abort`, todo `catch_unwind` es código muerto y el mandato del plan se evapora
   en silencio. Lleva guarda que lee el manifiesto y falla.
2. **El conjunto de funciones `extern "C"` es exacto**, y cada una contiene
   `catch_unwind`. La igualdad de conjuntos hace doble trabajo: impone el punto
   anterior y es el mecanismo del «seis operaciones; si necesitas la séptima,
   escribe por qué». La lista de archivos se deriva del fuente, o un `.rs` nuevo
   nunca se escanea y la guarda pasa vacía.

## 9. Lo que esta decisión NO promete

- **La propiedad más antigua del proyecto muere en esta fase, y ninguna guarda de
  aquí la devuelve.** Hasta hoy, «el camino no existe» era literalmente cierto: el
  código que filtraría una clave no se podía hacer compilar. Después de (b), la
  pila criptográfica está enlazada en el `cdylib` que Dart carga, 12 de los 14
  nombres prohibidos están dentro del cierre, y el sujeto de la guarda pasa de
  *alcanzabilidad*, que decidía el compilador, a *nombrabilidad más una superficie
  auditada*, que deciden personas y transcribe un test.
- **Lo que sobrevive es más pequeño que lo que trece archivos de este repositorio
  afirman hoy.** Esas frases quedan desfasadas y hay que corregirlas.
- El accidente que la guarda nueva **no** puede cazar es un ensanchamiento
  *correcto de aspecto* de la fachada: alguien añade una operación legítima, la
  igualdad se pone roja, y la reparación honesta es extender la lista. Esa guarda
  se repara editándola, categoría a la que la vieja nunca perteneció.
- **No hay callbacks empujados**, ni Dart, ni UI, ni selector, ni descubrimiento.
- **Nada de esto se ha probado en hardware físico**, y dos procesos en `127.0.0.1`
  no son dos aparatos en una Wi-Fi.
- Los botones Enviar y Recibir siguen `onPressed: null`.

## Alternativas descartadas

- **(a) Precisar la guarda a tipos de material de clave.** §2. Hace `qyro_crypto`
  dependencia directa del crate que Dart carga y deja toda guarda posible como una
  lista de nombres que falla abierta; hay un hueco vivo de esa forma ya en el
  repositorio.
- **(c) No cruzar la red esta fase.** §2. Paga el coste entero de la guarda —49
  paquetes, 12 de 14 prohibidos— para recuperar **un** nombre de crate, y deja el
  motor inconstruible.
- **Un proceso auxiliar** que aísle la cripto del espacio de direcciones de Dart.
  Es lo único que conservaría la propiedad de verdad, y iOS prohíbe lanzar
  procesos auxiliares. Descartado por la plataforma, no por el coste.
- **Fundir `qyro_ffi` y `qyro_session` en un crate.** Estrictamente peor: la
  división en dos es precisamente lo que crea el límite comprobable de §3.1.


---

## Enmienda 1 (2026-08-14) — la superficie crece de once a diecinueve, y ninguna cruza un tipo

Las fases 04a y 05 necesitan cuatro cosas al otro lado de la frontera que la
superficie de la fase 01 no tenía: la **huella** del peer, el **veredicto de
confianza**, el **rechazo** del receptor y la **cadena de emparejamiento**. Sin
ellas el motor las tiene y la aplicación no puede pedirlas, que es la definición
exacta de una función que no existe.

### La regla que no cambia

**Ninguna función nueva cruza un tipo.** Todas devuelven `i32`, y los valores
salen por out-parámetros: enteros, o **texto en un búfer que el llamante prestó**
(ADR-0038). No hay struct, no hay puntero a memoria de Rust que Dart tenga que
liberar, y no hay enum de otro crate: `PeerTrust` y `RejectReason` son de
`qyro_session`, que es lo único que `qyro_ffi` puede nombrar además de
`qyro_core`.

### El contrato del texto, escrito una vez para las cinco que lo usan

```
qyro_*(..., out: *mut u8, capacity: usize, out_len: *mut usize) -> i32
```

- `out_len` recibe **siempre** la longitud que hacía falta, quepa o no.
- Si no cabe: `QYRO_ERR_BAD_ARGUMENT`, **nada escrito**, y `out_len` con lo que
  se necesita. El llamante reserva y repite. *Un búfer a medio escribir junto a
  un código de error es cómo se lee media huella y se compara en voz alta.*
- `capacity == 0` con `out` nulo es legítimo: es la forma de **preguntar** el
  tamaño sin reservar nada.

### Las ocho operaciones

| Símbolo | Qué devuelve |
|---|---|
| `qyro_session_peer_fingerprint` | La huella del peer **ya formateada** por el core |
| `qyro_session_local_address` | La dirección que este extremo ligó, para poder anunciarla |
| `qyro_session_peer_trust` | 0 conocido · 1 **cambiado** · 2 nuevo |
| `qyro_session_remember_peer` | — |
| `qyro_trust_forget_peer` | 1 si había algo que olvidar, 0 si no |
| `qyro_trust_list_peers` | Los nombres, separados por NUL |
| `qyro_session_reject` | — |
| `qyro_session_rejection` | 0 declinó · 1 sin sitio · 2 manifiesto · 3 sin especificar · −1 no rechazó |
| `qyro_pairing_parse` | La dirección de una cadena válida, o un código si no lo es |

*(Nueve filas para «las ocho»: `qyro_pairing_parse` valida y extrae a la vez, y
contarla aparte sería contar dos veces la misma operación.)*

### El libro de confianza es del proceso, y es una decisión

`TrustBook` vive en un `Mutex` estático de `qyro_ffi`, no en una segunda tabla de
handles. **Una aplicación tiene un libro.** Una tabla de handles para un objeto
único es una tabla que sólo se puede usar mal, y la que ya existe está ahí porque
las sesiones son varias.

**No persiste**, y no es un olvido: `seal_known_peers` necesita un
`SecretWrapper`, que en Android no existe hasta la fase 06. Lo que funciona hoy
es la decisión —una clave conocida que cambia se refuta por nombre— dentro de una
ejecución.

### Lo que esta enmienda NO promete

- **No promete descubrimiento.** `qyro_pairing_parse` lee una cadena que una
  persona escribió o escaneó. Nada busca nada.
- **No promete que la confianza sobreviva a un reinicio.** Eso es la fase 06.

---

## Enmienda 2 (2026-08-16) — veinte no era diecinueve, y ahora son veintitrés

**La enmienda 1 dice «de once a diecinueve». Eran veinte.** El error se copió a
`STATUS.md`, a `docs/release/v1.0.md`, a `ESTADO-ACTUAL.md`, a dos informes de
fase y al propio doc-comment de `qyro_ffi/src/guards.rs`, y **nada lo contaba**
(QYR-0352). El mensaje del guard de pánico seguía diciendo «nineteen» mientras la
constante doscientas líneas más arriba decía veinte.

Ahora lo cuenta `the_c_surface_is_exactly_the_symbols_that_are_written_down`, que
lee la fuente de producción con el análisis compartido —el mismo que salta
comentarios y `#[cfg(test)]`, porque su primer borrador contó
`qyro_test_panicking_boundary` y una callback de prueba llamada `record`— y se ha
visto fallar al quitar un símbolo de la lista y pasar al devolverlo.

**Un matiz que la cifra única escondía:** `qyro_session_open_sender_fd_blocking`
es `#[cfg(unix)]`, así que una `cdylib` de Windows exporta uno menos que una de
Android. La constante cuenta lo declarado en la fuente, que es lo que se revisa.

**ADR-0040 añade tres**: `qyro_identity_open_blocking`,
`qyro_identity_set_wrapper` y `qyro_identity_fingerprint`. **Veintitrés.**
Ninguno cruza un tipo: una ruta y una huella por búfer prestado, dos punteros a
función de escalares y un `uintptr_t`. El precedente del puntero a función es
`QyroProgressFn` de ADR-0033.

---

## Enmienda 3 (2026-08-17) — el símbolo veinticuatro, y la pregunta que faltaba

**`Session::finish()` no tenía símbolo, y sin él un archivo recibido nunca
llega.** Es lo que verifica el digest y renombra el `.qyro-part` a su nombre
definitivo (ADR-0027 §4). La superficie tenía veintitrés símbolos, ninguno lo
alcanzaba, y el resultado es el peor de los posibles: **el producto decía
«entregado» y dejaba una parte** (QYR-0357).

```c
/* Materialises what arrived and releases what did not. Returns the number of
   items that reached their final name through `out_count`. */
int32_t qyro_session_finish(uint64_t handle, uint32_t *out_count);
```

Un entero por out-parámetro, como todo lo demás. **Veinticuatro.**

### Por qué no se vio, y qué se cambia además del código

`qyro_net_smoke serve` —el receptor de Rust— llama a `finish` desde el sprint 6A.
Ninguna prueba de este proyecto había puesto nunca un **receptor de Dart** frente
a un emisor real: `qyro_session_transfer_test.dart` prueba Dart-como-emisor. La
mitad receptora de la frontera nunca se ejercitó de extremo a extremo.

**La regla que sale de aquí, y vale más que el símbolo:** cuando una operación
existe en los dos lados —emitir y recibir, sellar y abrir, envolver y
desenvolver— **las dos mitades necesitan su prueba de extremo a extremo, y con
el producto en cada rol.** Las tres veces que este proyecto ha enviado una
capacidad inalcanzable han sido costuras que ninguna prueba cruzaba, y las tres
se habrían visto preguntando quién llama.

---

## Enmienda (2026-08-18, fase 21) — un símbolo más: `qyro_advice`

**La superficie C pasa de 24 a 25 símbolos.** No se añade uno a esta frontera sin
escribir por qué, y ésta es la razón.

### Qué es

```c
int32_t qyro_advice(int32_t has_network, int32_t peer_discovered,
                    int32_t has_serial_port, int32_t other_has_camera,
                    uint64_t payload_len,
                    uint8_t *out, size_t capacity, size_t *out_len);
```

Cuatro hechos entran, una frase sale. Como todo lo demás en esta superficie, el
texto se escribe **en un buffer prestado por quien llama** y no se escribe nada
si no cabe. **No cruza ningún tipo.**

### Por qué tiene que cruzar la frontera

ADR-0046 §4 decide que un solo módulo elige el canal —red, cable directo, serie,
óptico— porque las fases 14, 15 y 16 tenían cada una algo que decir y tres
interfaces inventando su propio orden es como un producto acaba
contradiciéndose. Ese módulo vive en `qyro_session`.

**Y el CLI lo alcanzaba y la GUI no.** Eso es literalmente la celda vacía que
ADR-0046 §2 prohíbe: una capacidad que existe en una cara, no existe en la otra,
y nadie lo ha escrito. Las opciones eran tres y dos son malas:

| | |
|---|---|
| Que la GUI no lo tenga | Es la decisión que la tabla de paridad marcaba «TODAVÍA», y era pendiente, no decisión: **la GUI sí debería tenerlo** |
| Que Dart lo reimplemente | **Dos implementaciones del mismo orden** es exactamente el problema que ADR-0046 §4 existe para evitar |
| **Un símbolo** | Un `int32` de retorno y texto a un buffer prestado, la forma que esta superficie ya usa nueve veces |

### Por qué la frase y no un código

ADR-0046 §5, y el precedente está en esta misma superficie:
`qyro_identity_fingerprint` devuelve el texto ya agrupado en vez de bytes,
**porque dos aparatos que dibujaran la misma huella distinta harían que leerla en
voz alta no significara nada.** Un consejo que cruzara como un entero se
convertiría en «canal 3» en una cara y un párrafo en la otra, y eso son dos
productos.

### Lo que no cambia

Sigue sin cruzar un tipo. Sigue sin haber asignación que el otro lado deba
liberar. `qyro_ffi` sigue pudiendo nombrar exactamente `qyro_core` y
`qyro_session`, que es lo que la guarda 1 acota — y el consejero está en
`qyro_session`, así que no abre ninguna arista nueva en el grafo.

---

## Enmienda 5 (2026-08-19, fase 24B) — seis símbolos para el ojo, y ninguno cruza un tipo

La superficie pasa de **veinticinco a treinta y uno**. Es el crecimiento más
grande desde la enmienda 1, así que el argumento va entero.

### Qué se añade

| Símbolo | Qué cruza |
|---|---|
| `qyro_scanner_open(out_handle) -> i32` | un `u64` por parámetro de salida |
| `qyro_scanner_look(handle, luma, w, h) -> i32` | bytes prestados y dos enteros |
| `qyro_scanner_tally(handle, out_seen, out_read) -> i32` | dos `u64` de salida |
| `qyro_scanner_result_len(handle, out_len) -> i32` | un `usize` de salida |
| `qyro_scanner_result(handle, out, cap) -> i32` | bytes a un búfer prestado |
| `qyro_scanner_close(handle)` | nada |

**Ninguno cruza un tipo**, que es la invariante que esta ADR existe para
mantener. Un escaneo vive detrás de un `u64` en la misma clase de tabla que las
sesiones, y lo único que se mueve son enteros y bytes en búferes que el otro lado
ya sabe reservar (`qyro_buffer_alloc`, ADR-0038).

### Por qué seis y no uno

La tentación era un símbolo que hiciera todo. Cada uno de los seis existe porque
**responde a una pregunta que se hace en un momento distinto**:

- `look` llega a 30 por segundo; `tally` se dibuja en cada repintado, que son
  más; `result_len` se pregunta una vez, y `result` una sola vez después.
  Meterlos juntos obligaría a copiar novecientos mil bytes para dibujar una barra
  de progreso.
- `result_len` va **separado de** `result` para que quien llama reserve el tamaño
  exacto. Es la misma forma que el resto de esta superficie ya usa.

### `tally` devuelve dos números por la misma llamada, a propósito

«300 mirados, 2 leídos» y «300 mirados, 280 leídos» son **la misma barra de
progreso y dos situaciones opuestas**: la primera dice que hay que acercar el
teléfono, la segunda que va bien. Dos símbolos distintos dejarían dibujar una sin
la otra, y una pantalla que sólo enseñara la mitad estaría escondiendo justo el
dato que sirve para actuar.

### Un código de error nuevo: `QYRO_ERR_NOT_READY` (−15)

«Todavía faltan bloques» es el estado normal de un escaneo durante casi todo su
tiempo de vida. Devolverlo como `BAD_ARGUMENT` haría que una pantalla enseñara un
error mientras todo va bien, que es la clase de mentira que esta ADR §5 ya
prohibió una vez.

### Lo que no cambia

`qyro_ffi` sigue nombrando exactamente `qyro_core` y `qyro_session`. El ojo vive
en `qyro_eye` y **se alcanza envuelto por `qyro_session::Scanner`**, que es la
misma forma que `browse` usa con `qyro_net` — no una reexportación, que la guarda
2 rechazaría con razón.

Sigue sin haber asignación que el otro lado deba liberar: `result` escribe en un
búfer que quien llama reservó y libera.

### Lo que esto **no** promete

**Que ningún teléfono haya leído un QR de Qyro.** Estos seis símbolos son el
camino; que por él pasen píxeles de una cámara real es la fase 19, y el hueco
sigue en blanco.

---

## Enmienda 6 (2026-08-31, fase 28) — dos símbolos, y la pregunta que se hacía sin objeto

**El teléfono preguntaba «¿aceptas 0 archivos, 0 B?».** Literalmente eso: la
tarjeta de oferta dibuja `receiveOfferFrom(fileCount, humanBytes(totalBytes))` y
recibía cero y cero.

**Medido, no razonado.** `Session::open_receiver` vuelve en cuanto el handshake
termina, y la oferta y el manifiesto llegan después —**dos pasos después**, no
uno—. Está medido en
`session_behaviour::what_is_offered_is_unknown_until_await_offer_and_known_after_it`:

| tras | `offered_files()` | `progress().total` |
|---|---|---|
| paso 1 | vacío | **0** |
| paso 2 | el manifiesto | el total real |

El worker de Dart daba **un** `stepBlocking()` y mandaba `progress().total` junto
a la oferta, así que el número era 0. Y `fileNames` estaba **escrito a mano como
lista vacía**, porque no había símbolo que la trajera: `Session::offered_files()`
existía desde QYR-0364, tenía un llamante en el CLI y **no cruzaba la frontera**.

QYR-0364 está registrada como cerrada con la frase «una pregunta sin objeto es una
formalidad, no una decisión». Se cerró en el motor y en ningún consumidor.

### Los dos símbolos

```c
/* Steps until the offer and its manifest have arrived, and no further.
   Bounded: a peer that connects and says nothing ends at the read deadline. */
int32_t qyro_session_await_offer_blocking(uint64_t handle);

/* What is being offered: name, size, name, size ..., separated by NUL. */
int32_t qyro_session_offered_files(uint64_t handle, uint8_t *out,
                                   size_t capacity, size_t *out_len);
```

**Treinta y tres.** Ninguno cruza un tipo nuevo.

- El primero lleva `_blocking` porque bloquea, y §7 prohíbe que un `_blocking`
  corra donde se dibujan frames. Va en el worker, como los otros cuatro.
- El segundo usa el contrato de texto de la enmienda 1 —`emit_text`, capacidad
  cero para preguntar el tamaño— y el separador **NUL**, que es el mismo que
  `qyro_trust_list_peers` y `qyro_session_open_sender_blocking` usan y por la
  misma razón: es el único byte que un nombre no puede contener, así que partir
  por él es exacto y ningún nombre necesita escaparse.

### Por qué dos y no uno

Meter la espera dentro del captador lo convertiría en una función que bloquea sin
decirlo en su nombre, y **§7 es exactamente esa regla**. Un captador que a veces
tarda tres segundos es un captador que alguien llamará desde el hilo que dibuja.

### Por qué no se resolvió duplicando el bucle en Dart

Era la alternativa barata: dar dos `stepBlocking()` en vez de uno. Se descarta
porque el número **dos** es una propiedad del protocolo, y una propiedad del
protocolo escrita en los dos lados es la forma exacta del defecto que este taller
ya pagó con el puerto —tres copias y ningún original—. Vive en
`Session::await_offer`, y `qyro_session_await_offer_blocking` es su puerta.

### Lo que esta enmienda **no** promete

**Que ningún teléfono haya enseñado esta tarjeta.** Los dos símbolos son el
camino; que por él pase la oferta de un emisor real, en un aparato real, es la
fase 19 y el hueco sigue en blanco.

---

## Enmienda 7 (2026-08-31, fase 28) — un símbolo, porque la comparación que esta ADR describe era imposible desde el otro lado

El comentario de `qyro_pairing_parse` dice, palabra por palabra, que lo que hace
el llamante con una cadena válida es «marcar la dirección y después comparar
`qyro_session_peer_fingerprint` contra **lo que escaneó**».

**La frontera no tenía forma de devolver «lo que escaneó».** El único símbolo de
emparejamiento entrega la dirección y tira la huella —a propósito, y el propósito
era bueno: una huella devuelta se dibuja, y dibujarla la hace parecer
establecida—. Pero devolverla *para comparar* y devolverla *para enseñar* son dos
cosas, y al no separarlas se perdieron las dos.

### Lo que eso costaba

QYR-0381 arregló esto en la terminal, que llama a `qyro_session` directamente y
tiene `pairing_fingerprint` a mano. **La otra cara no tenía por dónde**, así que
en el teléfono —que es donde está la cámara, y donde el QR es la forma normal de
emparejar— escanear un código ataba la sesión a **una dirección y a ninguna
clave**: el trabajo caro del emparejamiento se hacía y no servía de nada, que es
literalmente lo que QYR-0381 dice de la mitad que sí se arregló.

ADR-0035 §2.1 no es ambigua: *«si la cadena llevaba una huella y no coincide con
la autenticada, la sesión se rechaza **sin preguntar a nadie**»*. Era cierto en
un consumidor de dos.

### El símbolo

```c
/* La expectativa que promete una cadena de emparejamiento: 32 hex.
   No es una huella autenticada y no establece nada (ADR-0035 §2.1). */
int32_t qyro_pairing_fingerprint(const uint8_t *text, size_t text_len,
                                 uint8_t *out, size_t capacity,
                                 size_t *out_len);
```

**Treinta y cuatro.** No cruza ningún tipo nuevo: es el contrato de texto de la
enmienda 1, el mismo `emit_text` y la misma llamada de capacidad cero para
preguntar el tamaño.

### Por qué separado y no un par

Por lo mismo que en `qyro_session`: quien sólo quiere marcar sigue pidiendo lo
mismo que pedía, y una tupla obligaría a cada llamante existente a decidir qué
hace con un valor que no pidió. Y por una segunda razón que esta enmienda escribe
para que no se pierda: **las dos mitades tienen que aceptar y rechazar
exactamente lo mismo**. Una cadena que una acepta y la otra no es una sesión sin
expectativa que nadie ve, y hay una prueba que lo comprueba sobre la misma lista
de cadenas rotas.

### Por qué no se analizó la cadena en Dart

Era más barato: `QYRO1|dirección|huella` se parte con un `split('|')`. Se
descarta porque entonces el formato de ADR-0035 §2 viviría en dos sitios, y una
versión futura del formato tendría que arreglarse en los dos —o peor, en uno—.
Es la misma razón por la que existe la enmienda 6.

### Lo que esta enmienda **no** promete

**Que nadie haya escaneado nunca un código con una cámara de verdad.** El símbolo
existe, la GUI lo llama y hay una guarda que lo comprueba desde el gate. Que un
teléfono lea un QR dibujado por una terminal es la fase 19 y el hueco sigue en
blanco.
