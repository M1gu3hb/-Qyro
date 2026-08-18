# FASE 12 — Cerrar la cadena. El primer archivo que viaja entre dos aparatos.

> **Es la fase más corta de las nueve y la única que no se puede saltar.** Hasta que
> esto cierre, Qyro es un motor excelente dentro de una aplicación que no funciona.

---

## 1. El defecto, con sus líneas

Auditoría independiente del 2026-08-17 sobre `d575ac85`. Cuatro piezas, y las cuatro
hay que verlas juntas:

**(a)** `apps/qyro/lib/transfer/native_transfer_service.dart:169` lee
`_listeningAddress`. El campo se declara en la línea 177 y **no se asigna en ninguna
parte del árbol** — `grep` global sobre `apps/`, `rust/` y `docs/`: dos apariciones,
ninguna es una asignación. `ownPairingString()` **sigue devolviendo `null` siempre**.
Cambió de forma en la fase 11, no de comportamiento.

**(b)** `apps/qyro/lib/transfer/transfer_screens.dart:422` liga a `'0.0.0.0:0'`.
Puerto efímero elegido por el kernel, que **nadie consulta y nadie enseña**.

**(c)** `qyro_session_local_address` existe en la superficie C y en los bindings de
Dart (`QyroTrustBindings.localAddress`) y **no tiene un solo llamante de
producción**. El informe de la fase 11 lo registró en su línea 52 y la observación
se quedó en el informe.

**(d)** El descubrimiento automático **no tiene ningún símbolo en la superficie C**.
`DiscoveryChannel.kt` está escrito y `MainActivity.kt` lo registra en el canal
`dev.qyro/discovery`, pero **ningún archivo de Dart abre ese canal**: el único
`MethodChannel` en `lib/` es `dev.qyro/file_picker`.

### Lo que eso significa en la pantalla

Los dos aparatos muestran a la vez, en los dos idiomas:

> «Escribe el código de emparejamiento que enseña el otro aparato.»
> El código de este aparato: «Sin conexión, así que no hay código que mostrar.»

**Un bucle cerrado sin salida.**

### Por qué sobrevivió

`apps/qyro/test/transfer/transfer_screens_test.dart:50` — las cuatro pantallas se
prueban contra un `FakeService` cuyo `ownPairingString()` devuelve un literal que el
test fija. **La prueba de que «la pantalla enseña el código» no puede distinguir un
código medido de una constante escrita al lado.** Antipatrón nº 6 del proyecto,
aplicado justo en la costura donde vivía el defecto.

---

## 2. La ficha que se cerró en falso, y hay que reabrir

**QYR-0322** decía, en su propio texto:

> «`open_receiver` hace `bind` y `accept` dentro de la misma llamada y no devuelve
> hasta que un peer se conecta… lo que no se puede es preguntarla **a tiempo**.»
>
> «Por qué P2: … **Sube en cuanto Dart tenga que recibir, que es la fase 05.**»

Se cerró en la fase 09 así:

> «`qyro_session_local_address` existe… Lo que la ficha describía era que no había
> forma de preguntarla; ahora la hay.»

**La ficha no describía eso.** El cierre respondió a una pregunta distinta. La fase
05 llegó, Dart tuvo que recibir, y la escalada que la propia ficha anunciaba no se
comprobó.

**Entregable:** reabrir QYR-0322 con severidad **P0** y el argumento de por qué su
cierre no la cerró. No se borra el cierre anterior: se añade debajo. Un cierre
equivocado documentado vale más que un cierre borrado.

---

## 3. La decisión que hay que congelar antes del código

`docs/adr/ADR-00XX-primer-contacto.md`. Corta. Decide:

1. **Cómo se elige el puerto de escucha.** La opción barata y correcta: **un puerto
   fijo por defecto**, conocido a priori, que se puede componer en un código de
   emparejamiento antes de que nadie se conecte. Elige el número, di por qué, y di
   qué pasa si está ocupado (**siguiente libre**, y entonces sí hace falta
   consultarlo). Esto **no** requiere separar `bind` de `accept`.
2. **Si además separas `bind` de `accept`** en la superficie de `qyro_session` —un
   `Bound` que sabe su dirección y del que sale una `Session` al aceptar—, que es lo
   que QYR-0322 pedía. **Es la opción correcta a largo plazo y la fase 14 la va a
   necesitar de todos modos.** Decide si la haces aquí o allí, y escribe por qué.
3. **Qué IP va en el código.** Un aparato tiene varias. Enumerar interfaces, excluir
   loopback y las virtuales obvias, y **enseñar todas las candidatas si hay más de
   una** en vez de adivinar. Un código con la IP de un adaptador de Hyper-V es un
   código que no funciona y no dice por qué.
4. **Quién escucha y quién conecta**, a la luz de `R8` §9: el firewall bloquea
   inbound por defecto y el perfil de una red no identificada es Public. **Sólo un
   lado debe necesitar el permiso.**

---

## 4. Entregables

1. **Reabrir QYR-0322 como P0** con el argumento de §2.
2. **La ADR de §3, congelada en su propio commit.**
3. **La pantalla de recibir liga a un puerto conocido** y **enseña la dirección
   completa en cuanto está escuchando**, antes de que nadie se conecte.
4. **`_listeningAddress` se asigna**, o desaparece y `ownPairingString()` compone la
   cadena desde la fuente real. **Un campo que se lee y no se escribe no vuelve a
   entrar en este árbol.**
5. **El descubrimiento cruza el FFI o se declara fuera de la v1.0 en los tres sitios
   que hoy lo anuncian.** Las dos salidas son legítimas; anunciarlo sin conectarlo
   no lo es. *(Si eliges declararlo fuera: la fase 14 lo conecta de verdad, y la
   decisión es sólo sobre qué dicen los documentos hoy.)*
6. **Las afirmaciones falsas, corregidas:**
   - `docs/release/v1.0.md` §1 primera viñeta y §8.
   - `README.md:22`.
   - `STATUS.md:15` — la línea de Milestone dice «en Android por Keystore» y el
     propio `STATUS.md:90` dice setenta y cinco líneas más abajo que **Keystore está
     descartado**. La cabecera del archivo canónico contradice su cuerpo.
   - `docs/release/v1.0.md` §7 dice «Dependencias externas de Rust: **una**,
     `mdns-sd`». `Cargo.lock` tiene 80 paquetes, **66 no son `qyro_*`**. Lo que se
     quería decir es «una añadida en esta release». Escríbelo así.
7. **La GitHub Release.** Ver §6.

---

## 5. La prueba que cierra la fase, y su control

**No vale una prueba con `FakeService`.** La prueba tiene que ser:

> **Dos procesos reales.** El primero abre la pantalla de recibir —o su equivalente
> programático sobre `NativeTransferService`, la clase de producción, no un fake— y
> **publica su cadena de emparejamiento**. El segundo **parsea esa cadena**, se
> conecta, y transfiere un archivo que se verifica en destino por SHA-256.

**Control de falsabilidad, obligatorio:** una segunda ejecución en la que el primer
proceso **no** está escuchando debe fallar con un error **nombrado y distinto**, y
la prueba lo exige. Sin eso, la prueba no distingue «funcionó» de «no llegó a
intentarlo».

**Y una prueba de regresión directa contra el defecto:**
`ownPairingString()` sobre `NativeTransferService` devuelve **no-nulo** en cuanto hay
listener, con un nombre que lo diga:
`own_pairing_string_is_not_null_once_this_device_is_listening`.

---

## 6. La Release, autorizada por el propietario

El propietario autorizó publicar los binarios el 2026-08-17, con estas palabras:

> «Que ya lo haga. No importa, aunque sea público… Realmente esto es un proyecto que
> quiero que funcione… si está tan preocupado por eso, que ponga ahí en el release
> una advertencia: aún no está aprobado, o algo así.»

**Instrucciones, y son exactas:**

- **Crea la GitHub Release** sobre la etiqueta que corresponda. Adjunta **el APK
  firmado, el ZIP de Windows y sus SHA-256**.
- **Cero archivos sensibles.** Nunca el keystore, nunca `key.properties`, nunca una
  clave privada. *(Verificado en la auditoría: el repositorio está limpio y
  `.gitignore` los cubre. Mantenlo así.)*
- **La advertencia va arriba del todo**, no en una nota al pie, y con estas tres
  cosas dichas:
  1. **NO APROBADO — software que funciona en las pruebas y que nadie ha usado.**
  2. **NADA DE ESTO SE HA EJECUTADO NUNCA EN HARDWARE FÍSICO.** Los escenarios que
     cerrarían ese hueco están en `docs/testing/hardware-protocol.md`, en blanco.
  3. **En Android la identidad no está en Keystore**: con Keystore, root necesitaría
     además el TEE; sin él, root basta.
- **Marca la Release como pre-release** si eso está disponible. Es la señal que el
  propio GitHub tiene para exactamente esto.
- **Corrige antes las afirmaciones falsas del §4.6.** Publicar la Release arrastra
  `docs/release/v1.0.md` como texto de presentación, y su primera viñeta hoy promete
  una capacidad que la aplicación no tiene. **Ese orden no es negociable: primero la
  verdad, después el botón.**

---

## 7. La puerta

Las quince comprobaciones de `R2` + `00-LEEME` §4, por código de salida. Con especial
atención a las dos nuevas:

- **Comprobación 14:** la tabla `capacidad | símbolo | llamante de producción |
  archivo:línea`. Para esta fase debe incluir, como mínimo,
  `qyro_session_local_address` y el símbolo de descubrimiento si lo conectas.
- **Comprobación 15:** la cadena completa escrita, desde «la persona toca Recibir»
  hasta «el socket está ligado y la cadena aparece en pantalla», y desde «la persona
  teclea el código» hasta «el archivo está en disco verificado».

---

## 8. Lo que NO hay que hacer aquí

- **No rediseñes la interfaz.** Cuatro pantallas, dos idiomas, y ya está bien.
- **No toques el motor de transferencia.** Funciona y está probado.
- **No arregles el mojibake de `session.rs`** (30 secuencias `Ã¢â‚¬â€`, UTF-8 leído
  como Latin-1). Va a `deuda-de-calidad.md`, se cierra en la 18.
- **No limpies los dos `Estado:` duplicados** de QYR-0088/0089. Misma lista.
- **No empieces el CLI.** Es la fase 13 y necesita esta cerrada.
