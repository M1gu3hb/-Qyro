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
