# Estado actual — el APK de release no podia abrir un socket

**2026-08-31** · rama unica `main`

## Fase 28 en marcha — la revision antes de la primera prueba en hardware

El propietario va a probar Qyro en una PC y un telefono reales por primera vez.
Esta tanda audita, arregla y deja escrito lo que hace falta para esa prueba.

**QYR-0368, P0, arreglado.** `android.permission.INTERNET` estaba declarado
**solo** en `app/src/debug/AndroidManifest.xml` y en `app/src/profile/`. Gradle
fusiona `main` mas el sourceSet de la variante que construye, asi que ninguno de
los dos llega a release: **el APK que la gente instala no lo declaraba.** Tres de
los cuatro canales de Qyro son un socket TCP, asi que el APK construido no podia
transferir nada; lo que se ve en el telefono es `Permission denied (errno = 13)`
desde dentro de la biblioteca nativa, sin nombrar ni a Qyro ni a un permiso.

**Por que nadie lo vio:** `android_manifest_test.dart` solo comprobaba
**ausencias**. Un permiso que tiene que **estar** podia desaparecer y todo seguia
verde. Ahora hay tres guardas: una en `qyro_net::guards` sobre el manifiesto
fuente —corre en la puerta, en cada commit—, el conjunto exacto de tres permisos
en Dart, y la afirmacion sobre el manifiesto **fusionado de release**, con
`QYRO_REQUIRE_RELEASE_MANIFEST=1` en `release.yml` para que leer el de debug alli
sea rojo.

**QYR-0369, arreglado.** El `|` del codigo de emparejamiento es una **tuberia**
en PowerShell y en `cmd`, asi que `--to QYRO1|ip:puerto|huella` sin comillas no
llegaba nunca a Qyro y el error no nombraba a Qyro. Ahora `whoami`, `recv`, `qr`
y `find` lo imprimen **ya entrecomillado**, `qyro help` lo explica, y
`PairingEndpoint::parse` acepta el codigo con sus comillas -- porque un campo de
texto no las quita y el menu del CLI y la casilla del telefono son campos de
texto. Un par que abre y cierra, nunca «comillas donde las haya»: una comilla
suelta es un codigo copiado corto, y uno truncado que **parsea** es peor que uno
que no.

**QYR-0370, arreglado.** ADR-0041 §3 decia que un puerto ocupado «se dice, no se
mueve [...] y ofrece elegir otro», y el codigo no tenia palabra para decirlo:
`open_receiver` mapeaba **toda** ligadura fallida a `BadArgument`. En Windows es
el caso que pasa de verdad -- los rangos reservados por Hyper-V, WSL2 y Docker
rechazan con `WSAEACCES` (10013), que ni siquiera es «en uso». Ahora hay
`SessionError::PortUnavailable`, su codigo `-15` en la frontera C con su espejo en
Dart y su frase en dos idiomas, `qyro recv --port <n>`, y un mensaje que nombra el
puerto, el comando de `netsh` y por que Qyro **no** se mueve solo. Lo forzaron dos
guardas que ya existian y que se pusieron rojas solas.

**QYR-0371, arreglado. La duodecima capacidad muerta.** `PeersScreen` acepta un
callback `onScan` desde la fase 24B y la unica construccion de produccion no lo
pasaba, asi que en el telefono el boton «Escanear codigos» se dibujaba **apagado**
y el canal optico -- el unico que funciona sin red de ninguna clase -- no tenia
puerta. El comentario junto al boton se llama a si mismo «el llamante de
produccion del escaner» y describia un argumento que nadie daba.

Y por que nadie lo vio: `scannerAvailableOn()` ya aceptaba un sistema operativo
inyectado y la pantalla no se lo pasaba, asi que en el corredor de escritorio la
rama de Android era inalcanzable y la unica prueba sobre ese boton afirmaba que
**no existe**. Ahora hay dos pruebas de widgets y una guarda en Rust que lee
`home_screen.dart` y corre dentro de la puerta.

**QYR-0372, arreglado, y lo encontro ejecutarlo.** `qyro recv` contra un emisor
real imprimia «they have not said what they are sending yet» y preguntaba igual; y
la tarjeta del telefono ofrecia «0 archivos, 0 B». QYR-0364 esta registrada como
cerrada con la frase «una pregunta sin objeto es una formalidad, no una decision»:
se cerro en el motor y en ningun consumidor.

Medido: `open_receiver` vuelve al acabar el handshake y la oferta llega **dos**
pasos despues. El CLI no daba ninguno; la GUI daba uno, con un comentario que
decia que uno bastaba. Y `fileNames` estaba escrito a mano como lista vacia,
porque `offered_files()` no cruzaba la frontera C.

Ahora hay `Session::await_offer()` -- el numero en un solo sitio --, **ADR-0032
enmienda 6** congelada antes del codigo, y dos simbolos, **treinta y tres**. Se
mide en `two_process_pairing_test.dart`, que es la unica prueba que pone un
receptor de Dart de verdad frente a un emisor de verdad: una prueba de widgets no
habria visto nada, porque la tarjeta ya sabia dibujar nombres.

**QYR-0373, P0, arreglado.** `defaultDestination()` devolvia en Android
`Directory.current.path + '/Qyro'`, y **el directorio de trabajo de un proceso de
Android es `/`**: la respuesta era `/Qyro`, la raiz del sistema, que ninguna
aplicacion puede escribir. Recibir en el telefono lanzaba al crear la carpeta,
**antes de emitir un solo estado**, asi que pulsar Recibir no hacia nada visible.
El comentario decia «el lado Kotlin la pasa; hasta que lo haga, el directorio de
trabajo es la respuesta honesta» -- y ese lado Kotlin nunca se escribio.

Ahora existe: `PathsChannel.kt` en `dev.qyro/paths`, que devuelve
`getExternalFilesDir(null)/Qyro` -- sin permisos, visible por USB, y borrada al
desinstalar. Lo vigila una guarda en la puerta que comprueba las cuatro piezas,
incluido que los dos lados abren el mismo nombre de canal.

**QYR-0374, arreglado, y lo encontro ejecutarlo tres veces seguidas.** Mandar el
mismo archivo dos veces a la misma carpeta da una barra al 100 % y despues
`0 file(s) saved in .`, sin una palabra de por que. Negarse es lo correcto --
ADR-0027 §4, no se sobrescribe nunca -- y el defecto era que los dos consumidores
**tiraban el motivo**: el CLI con `unwrap_or(0)` y la GUI con un `catch` vacio en
un `finally`, cuyo comentario decia que «el final ya decidio lo que fue esta
transferencia». No lo habia decidido: el final es `Completed` porque la
TRANSFERENCIA termino, y `finish` se niega por una razon del sistema de archivos.
Asi que **el telefono decia «entregado» con nada en el disco** -- la misma forma
de QYR-0357 por otra puerta.

El motor no tenia nada que arreglar, y eso es parte del hallazgo.

**QYR-0375, arreglado, y el arreglo introdujo un defecto que tambien esta
arreglado.** `qyro send` decia `could not connect` para toda razon, y
`open_sender` construye el manifiesto **antes** de marcar: un nombre con un
retorno de carro se rechaza sin tocar un socket, y a la persona se le mandaba a
mirar la red. Ahora hay una frase por razon. Y la linea nueva imprimia
`path.display()` en crudo, con lo que ese mismo nombre **reescribia la linea que
lo anunciaba** -- en el unico programa que ya sabe que un terminal es un
interprete. Pasa por `safe_terminal_name`, y se imprime el nombre y no la ruta.

**QYR-0376, P0, y es el hermano mayor de QYR-0373.** `defaultIdentityPath()`
devolvia fuera de Windows `Directory.current.path + '/identity.qyro'`, o sea
**`/identity.qyro`** en Android. Escribir ahi falla, asi que `openIdentity()`
fallaba, asi que **toda sesion contestaba `identity_unreadable`**. El destino roto
impedia recibir; esto impide TODO: sin identidad no hay handshake, ni huella, ni
codigo de emparejamiento. `PathsChannel` gana `identity`, que devuelve
`getNoBackupFilesDir()/identity.qyro` -- interno, privado por el sandbox de UID, y
un directorio que el sistema nunca copia a una nube, que es la tercera cerradura
de QYR-0349.

**QYR-0377, P0.** Los comandos de construccion de Android **no podian construir**:
`cargo build --target aarch64-linux-android` falla con «linker `cc` not found»
porque a Rust hay que decirle con que enlazar, y ni el protocolo de hardware ni la
primera version de mi propia guia lo decian. Los dos llevan ahora el bloque de
PowerShell que resuelve la ruta del NDK sola y comprueba que el clang existe
antes de intentar nada. Y la DLL de Windows se construye antes de copiarse, desde
el directorio de objetivo correcto: el paso 3 copiaba una DLL que ningun paso
construia.

**QYR-0378 y QYR-0379: el canal optico tenia dos puertas mas cerradas detras de
la de QYR-0371.** `CAMERA` es un permiso peligroso y **nadie lo pedia en tiempo de
ejecucion** -- un grep de `requestPermissions` en todo el arbol de Android no
devolvia una linea --, asi que `bindToLifecycle` lanzaba `SecurityException` y la
pantalla decia «este aparato no puede mirar»: una frase sobre el aparato, cuando
faltaba una pregunta que nadie hizo. Y al completarse un escaneo, la pantalla
**leia el archivo entero y se quedaba con su longitud**: imprimia «Recibido: N
bytes» y tiraba los bytes.

Ahora se pide el permiso -- y se ofrece reintentar, sin esperar la respuesta por
el canal --, y lo que llega se escribe en la misma carpeta que el resto, con un
nombre que Qyro elige **y dice que ha elegido**: un QR no lleva el nombre dentro.

**QYR-0380, P0 del escenario D1.** Mandar desde el telefono no funcionaba por dos
motivos a la vez, y cada uno bastaba. El boton de enviar **no se encendia al
escribir** -- el `TextField` no tenia `onChanged`, asi que el estado no se
reconstruia--, y funcionaba solo si se escribia la direccion ANTES de elegir los
archivos, porque elegir si llama a setState: al reves, que es el orden natural,
se quedaba apagado. Y el campo decia «Codigo de emparejamiento» y pasaba el texto
tal cual a un motor que hace `parse::<SocketAddr>()`, asi que un codigo salia como
`bad_argument`. La pantalla de Aparatos si lo resolvia; esta, que es la que manda,
no. Ahora acepta las dos cosas y la etiqueta lo dice.

**QYR-0381.** ADR-0035 §2.1 dice que una huella que no coincide con la
autenticada refusa la sesion **sin preguntar a nadie**, y el doc-comment de
`parse_pairing` decia lo mismo en presente. **Nadie la comprobaba, y nadie
podia**: `parse_pairing` tira la huella y `qyro_pairing_parse` emite solo la
direccion. Asi que un codigo tecleado a mano -- comparado caracter a caracter por
una persona -- establecia **menos** que teclear un `ip:puerto` y anadir
`--expect`. Ahora `qyro send` usa la huella del codigo como expectativa
automatica. **En la GUI queda abierto**, con su forma escrita: la huella no cruza
la frontera C y arreglarlo es otra enmienda de ADR-0032 mas su lado Dart, sin
Flutter con el que ejecutarlo.

**QYR-0382.** `remember_peer` no tiene llamante de produccion, asi que la libreta
esta siempre vacia y **`PeerTrust::Changed` es inalcanzable**: la defensa que el
README prometia -- «si un aparato conocido presenta otra clave, Qyro se niega» --
no puede ocurrir, porque ningun aparato llega a ser conocido. El escenario C4 no
es ejecutable. Escrito en el README y en la guia; conectarlo pide una pantalla
donde una persona nombre un aparato, que es una funcion y no un cableado.

**QYR-0383, y es de los que mas duelen.** Un archivo VACIO se llevaba por delante
toda la transferencia. Medido con tres archivos y el vacio el primero: llegan
cero, y los dos llenos se quedan como `.qyro-part` -- escritos enteros,
verificados, sin renombrar. Dos defectos encadenados: la parte no se abre hasta el
primer trozo, asi que un archivo vacio salia como `DigestMismatch` -- marcado de
corrupto --, y `Session::finish` tenia **dos `return` dentro del bucle**, asi que
un solo item que fallara abandonaba todos los que venian detras. Ese es el grande;
el archivo vacio solo es la forma mas facil de alcanzarlo.

**QYR-0384.** Tres sitios con la misma forma: un fallo **antes** del primer
`yield` sale como error de stream, y la pantalla hace `await for` sin `catch`, asi
que **pulsar el boton no hace nada visible**. El drenaje de enviar no capturaba
nada -- el de recibir si, que es como los defectos sobreviven a una revision: se
mira uno y se da por hecho el otro --, `createSync` del destino lanzaba antes de
emitir, y ninguno de los dos cubria a un worker que muere por algo que no es un
`QyroSessionFailure`. El criterio, escrito una vez: un stream que la interfaz
escucha nunca termina en error, termina en un estado, aunque sea «no se que paso».

**QYR-0385.** `qyro beam` coloca el cursor arriba entre frames, y `detect_vt()`
devolvia **`Vt::Absent` en Windows siempre**, con lo que `home()` es la cadena
vacia: cada frame se **anadia**, y un QR de sesenta y siete filas subia por la
pantalla cinco veces por segundo. El canal optico, inutilizable en la unica
plataforma que dibuja. El comentario que defendia esa pesimismo pesaba color
contra pantalla rota y acertaba para el color; lo que no estaba escrito es que la
misma bandera decide si `beam` puede dibujar. Ahora `WT_SESSION` -- la marca de un
programa concreto, no una heuristica -- promete VT, y `beam` **se niega** si no la
hay en vez de dibujar algo que no se puede enfocar.

**QYR-0386.** El comodin de `_kindOf` mandaba **ocho** codigos de la frontera C a
`integrity`, o sea a la frase «llego algo que no verifico» -- una acusacion
concreta contra el otro extremo. Siete de los ocho no tienen nada que ver con el.
El peor, `identityUnreadable`: es el estado de ESTE aparato, y era exactamente el
sintoma del P0 de QYR-0376, asi que la pantalla decia «los datos llegaron mal»
cuando no habia llegado nada. Y `badArgument`, que es el unico que se corrige
escribiendo. Tres clases nuevas con sus frases en los dos idiomas.

**QYR-0387.** `sign_release_apk.ps1` re-alineaba el APK con `zipalign -p -f 4`
antes de firmarlo, y `-p` alinea a la pagina, que hasta build-tools 34 son **4
KB**. Asi que **deshacia la alineacion de 16 KB** que el NDK habia puesto --
despues de que la fase 27 la midiera, y sobre el artefacto que se publica. Y el
valor por omision de `$BuildTools` estaba clavado en `34.0.0`, donde el flag `-P`
ni existe. Ahora es `-P 16`, la version se resuelve a la mas nueva instalada
ordenando por VERSION y no alfabeticamente, se para al empezar si es anterior a
la 35, y el APK **firmado** pasa por el inspector antes de imprimir su hash:
firmar es lo ultimo que toca el paquete, asi que medir antes de firmar mide otro
archivo.

**QYR-0388.** En Android el selector devuelve **descriptores** y el motor los
ADOPTA -- los toma antes de validar nada -- asi que Rust los cierra pase lo que
pase. La pantalla no vaciaba la seleccion, asi que un segundo Enviar entregaba los
mismos numeros ya cerrados. Y el caso malo no es que falle: un descriptor cerrado
deja su numero libre y el proceso lo reutiliza -- el siguiente socket, el archivo
de identidad -- asi que entregarlo a `from_raw_fd` es mandar lo que haya ahi
ahora. Se vacian los descriptores y se conservan las rutas, que si se pueden
volver a abrir.

**QYR-0389.** El boton de Recibir no tenia guarda, y el puerto es fijo a
proposito, asi que pulsarlo dos veces arrancaba **dos receptores sobre el mismo
puerto**. Desde QYR-0370 el segundo falla con un mensaje -- «el puerto no esta
libre; lo tiene otro programa» -- siendo el otro programa Qyro. Y el segundo pisa
el estado mientras el primero sigue vivo, asi que la pantalla ensena el fallo del
segundo mientras el primero sigue escuchando de verdad.

**QYR-0390.** `_commonRoot(['D:\\video.mp4'])` devolvia **`D:`**, que en Windows
no es la raiz de la unidad sino «el directorio actual de la unidad D». El motor
hace `strip_prefix(root)` y `D:\video.mp4` no empieza por `D:` en componentes,
asi que mandar cualquier archivo de la raiz de una unidad salia como
`BadArgument` -- y desde QYR-0375 eso se explica como «el nombre fue rechazado»,
una acusacion falsa contra un `video.mp4` perfectamente normal.

**QYR-0391, y es el unico que se encontro midiendo.** Una transferencia de 200
archivos mantenia **402 descriptores abiertos a la vez** — dos por archivo: el
que lee el origen y la parte abierta del destino, ninguno cerrado hasta el final
de toda la transferencia. ADR-0047 §3 limita una transferencia a **256 archivos**
y la razon escrita ahi son los descriptores, contando **uno** por archivo; con
dos, 256 archivos son ~512, que es exactamente el techo del CRT de Windows. Un
proceso sin descriptores no falla en el archivo que los agota: falla en lo
siguiente que necesite abrir algo — el socket, la reanudacion, la identidad.

Medido antes y despues con la misma prueba, que pregunta a `/proc/self/fd`
mientras la transferencia corre:

| | Descriptores de mas, 200 archivos |
|---|---|
| Antes | **402** |
| Despues | **11** |

Y los 11 no crecen con el numero de archivos. El destino cierra la parte en
cuanto tiene los bytes que el manifiesto declara (se podia: `finish_item` ya
verificaba **por ruta**), y el origen mantiene una cache de ocho. **Sólo se
desaloja lo que tiene ruta**: en Android el selector devuelve descriptores
(ADR-0034) y cerrar uno de esos no ahorra nada, pierde el archivo. Esa guarda se
comprobo haciendo el desalojo incondicional a proposito, para ver si fallaba.

**`AGENTS.md` reescrito.** Se declaraba fuente canonica y decia que el alcance
«no incluye transferencia, transporte, LAN» y que «Qyro sigue sin transferir
archivos». Falso desde la fase 12.

---

## Lo anterior — QYR-0365 cerrada, y no era del motor

**2026-08-20**

## Lo que costo tres sesiones

`Process.start` deja la salida del hijo en una tuberia. **Un hijo que escribe a
una tuberia que nadie lee se BLOQUEA** cuando el bufer se llena — unos pocos KB.
Con 200 archivos el receptor del CLI escribe **23 349 bytes**, se bloquea, deja
de dar pasos, y el emisor espera hasta el reloj de 60 s.

| | Sin vaciar | Vaciando |
|---|---|---|
| 200 archivos | **60 295 ms** y falla | **292 ms** |
| Por archivo | — | **1,5 ms** |
| Entregados | 0 | **200/200** |

El motor, medido aparte con los mismos 200: **0,33 s, cero lecturas vencidas.**

**Las 75 esperas del emisor eran reales.** Esperaba a un receptor bloqueado
escribiendo. El sintoma apuntaba al sitio correcto y la causa estaba un proceso
mas alla.

**No se subio `IDLE_TIMEOUT`** — habria escondido esto. **No se toco el motor.**

Arreglado: `_drainChild` en los tres sitios, una celda de 200 archivos que
**fallaba antes**, y `dart_test.yaml` con `concurrency: 1`, porque ADR-0041 fija
el puerto y `flutter test` corre los archivos en paralelo.

**167 fichas, 0 abiertas.**

---

## Fase 25 en marcha — carpetas hechas, y un defecto mio que encontro un barrido

**Las carpetas vacias viajan.** `ItemKind::Directory = 2` llevaba años en el
cable —validado y con cuatro contratos— y **nadie lo emitia**: la decima
capacidad muerta. Dos ADR justificaban no mandarlas diciendo que haria falta una
version de protocolo, y el tipo ya estaba.

**Y un defecto que introduje yo:** `finish_item` sobre un directorio devolvia
error, y `Session::finish` hace `return` en ese caso — **todo lo que viniera
despues en el manifiesto se quedaba sin materializar**. Mi prueba paso por suerte
del orden. Lo encontro un barrido de nueve lectores en paralelo, y la prueba
nueva pone la carpeta **primero**.

**Lo que el barrido dejo pendiente**, verificado con archivo y linea:

1. **QYR-0317 esta mal descartada.** El progreso del receptor **nunca se
   asigna** —la unica asignacion es el brazo `Role::Sending`— y ademas `_drain`
   se traga las muestras del emisor. La barra del receptor esta congelada en
   cero por los dos lados. Hay que reabrirla, no citarla.
2. ~~`-14` no esta en el espejo de codigos~~ **HECHO.** `TooManyFiles` salia como
   «integrity» por el comodin de `_kindOf`. Ahora hay una prueba que **lee
   `abi.rs`** y exige que Dart cubra todos los codigos, con su control al reves.
   El `switch` exhaustivo de la pantalla obligo a darle su frase, en los dos
   idiomas.
3. ~~`finished()` no reconoce `Phase::Cancelled`~~ **El defecto era mas hondo y
   esta arreglado.** `Session::cancel()` solo ponia una bandera **local**: el par
   nunca se enteraba y esperaba a su reloj de 60 s para leer «el otro aparato no
   responde». `request_cancel()` —que emite el frame— existia desde la fase 04 y
   **no lo llamaba nada de produccion**: la undecima capacidad muerta. Ahora
   cancelar cruza el cable.

   **Y queda una cosa dicha, no arreglada:** el emisor se entera como
   `TransferRefused`, que la GUI lee como «el otro lado rechazo». La §5 pide
   separar «rechazo» de «cancelo a mitad», y separarlos es un codigo nuevo en la
   frontera C — calibre alto, con su enmienda a ADR-0032.
4. **`PARIDAD-GUI-CLI.md` cita lineas que no dicen lo que dice.** Cinco celdas
   verificadas, cinco mienten.
5. **Dos ADR contra la fase 25:** ADR-0033 descarta el freno de tiempo que la
   fase §1.3 prescribe, y ADR-0041 prohibe el respaldo de puerto que la §5 pide.
   Hay que enmendar o rebajar **antes** de tocar el codigo.

---


## La fase 19 esta lista, y es del propietario

`docs/testing/hardware-protocol.md` tenia veinte escenarios y **ninguno de los
tres canales nuevos**. Ahora tiene la seccion F: cable directo, canal optico,
canal serie, y la maquina que no puede instalar nada. **36 huecos, todos en
blanco.**

Cada uno trae el comando exacto. Y tres piden el numero que falta:

- **F1:** cuantos segundos tarda APIPA de verdad. Es la primera vez que se
  mediria fuera de `R8`.
- **F2:** **los fps que sostiene el telefono.** Es la medida que ADR-0048 §4 dejo
  en blanco: si son >=5, el puente esta hecho para siempre; si no, el JNI de
  copia cero tiene su argumento medido.
- **F4:** si arranca en un Windows 7 de verdad. ADR-0049 dice que **no esta
  confirmado en `msvc`**.

**No se ejecuto ninguno**, y eso es lo correcto: hace falta hardware, y un
escenario sin marcar no es un aprobado.

---

## Fase 20 — el arranque resuelto, y la decision de firma SIN tomar

- **`qyro send --self`** manda el propio binario. Es la respuesta al arranque:
  una vez hay un Qyro corriendo, se lleva a si mismo a la siguiente maquina --
  800 KB, ochenta segundos por serie. Con su control: sin `--self`, una ruta
  sigue haciendo falta, porque un `--self` que se aplicara siempre convertiria
  `qyro send informe.pdf` en `qyro send qyro.exe` en silencio.
- **`docs/release/DECISION-DE-FIRMA.md`**: los numeros y las consecuencias
  ordenados para decidir en cinco minutos. **NO decidida** — cuesta dinero.

**Lo que el implementador si dice**, y esta escrito ahi: el caso de uso empuja
hacia no firmar, porque la maquina que Qyro existe para servir recibe el archivo
por USB o por el propio Qyro, y en las dos rutas **el certificado no cambia
nada**. Firmar compra sobre todo la primera impresion de quien descarga en una
maquina normal, que es otro publico.

**Hechos tambien:** `BUILD-INFO.txt` en el artefacto de Windows —con el sha256 y
**NO FIRMADO en mayusculas**— y `docs/release/INSTALAR.md`, que son cinco pasos y
el segundo es el USB.

**Queda de la 20, y esta dicho:** la pagina de la Release **no se toco** —la
redaccion esta lista para copiar en `DECISION-DE-FIRMA.md` §6, y publicar es una
accion hacia fuera que ya lleva dos correcciones esta semana— y el
`BUILD-INFO.txt` **solo esta en el artefacto de Windows**; el job de musl tiene su
propio `upload-artifact` sin tocar.

---

## Fase 18 — la verdad, y dos frases que eran falsas

- **La Release prometia cifrado sin decir por que canal.** Es cierto por la red;
  el QR y el serie degradado **no cifran nada** — el fountain codifica, que no es
  lo mismo. Corregido a «**Por la red**...», con las excepciones nombradas.
- **`THREAT_MODEL.md` describia un canal de cuatro.** §4.bis, nueva: el optico es
  **difusion, no punto a punto** y no puede haber handshake; el serie degradado
  no autentica nada **y un cable es el canal mas privado de los cuatro**; y una
  direccion nunca es una identidad (RFC 3927 §5).
- **Deuda:** D1 y D6 cerradas. D2 ya lo estaba. Quedan cinco, **y ninguna es una
  afirmacion falsa**.

**D6 se gano el sueldo al primer intento**: tres enlaces rotos, uno de ellos
publico apuntando a un item privado. Y **entro sola en la puerta** — `gate.ps1`
lee `ci.yml`, asi que paso de 5 comandos a 6 sin tocar el script.

---

## Fase 17 — cerrada, con el binario en CI y no aqui

ADR-0049 congelada. Job `win7-builds.yml` con `-Z build-std` y los cuatro
targets, y `check_win7_imports.ps1` **con su control**: el binario normal DEBE
fallar la comprobacion, o el `[PASS]` del otro no vale nada. Visto fallar con las
tres entradas que tiene que rechazar.

**No se compilo un binario de win7 aqui**: `-Z build-std` necesita nightly y
`rust-src`, ~1,5 GB en el disco de sistema de esta maquina, que va justo. Lo
compila el runner y sube el binario y su tabla de imports.

**Y por eso ADR-0049 §3 deja la confirmacion sobre `msvc` como PENDIENTE.** `R8`
§10 midio sobre `-gnu`; el codigo de `std` es el mismo y eso es un argumento, no
una medida. Hasta que ese `dumpbin` corra, **este proyecto no afirma que
Windows 7 funcione.**

---

## QYR-0365: la medida desmiente el diagnostico

`rust/crates/qyro_session/tests/qyr_0365_measurement.rs`, con los contadores
`Session::step_tally` que la ficha pedia por su nombre. Veinte archivos de 64
bytes, dos sesiones de verdad sobre loopback:

```
  emisor:   22 pasos, 0 lecturas vencidas
  receptor: 43 pasos, 0 lecturas vencidas
  tiempo:   0.06 s   ->  0,003 s por archivo
```

**Tres milisegundos, no 1,2 segundos. Y cero esperas en los dos lados.**

La ficha decia —y yo lo repeti— que el bucle de sesion serializaba. **No
serializa.** Y `set_nodelay(true)` ya estaba desde ADR-0028, asi que Nagle
tampoco era.

**Donde queda:** los 75/1 salieron de `gui_cli_matrix_test.dart`, que es la GUI
contra el CLI — Dart conduciendo el motor por la frontera C. Esta medida es Rust
contra Rust. La diferencia entre 3 ms y 1 200 ms **esta en el lado Dart o en el
cruce**, no en el motor.

**La siguiente medida, y es una:** cronometrar por iteracion el bucle de
`native_transfer_service.dart` — `stepBlocking`, `progress`, `peerFingerprint` y
el `yield`. Si el motor hace 22 pasos en 60 ms, el coste esta entre esas cuatro
llamadas.

**No busques en `qyro_transfer` ni en `Session::advance`.** Ya esta medido y esta
limpio.

---

## El telefono ya puede mirar

`R7` prometia cuatro canales y habia tres y medio: `qyro beam` dibujaba QR y
nadie los leia. **El puente esta montado, y sin JNI:**

`ScannerChannel.kt` saca **solo el plano Y** con CameraX a **1280x720** →
`dev.qyro/scanner` → Dart → `qyro_buffer_alloc` → `qyro_scanner_look` →
`qyro_eye`. **Cero `unsafe` nuevo, cero excepcion nueva, cero paquetes de
pub.dev.**

Las tres que no se negociaban, hechas: `ResolutionSelector` pidiendo >=1280x720 ·
el de-padding fila a fila, porque `buffer.capacity()` puede ser
`rowStride*(h-1)+w` y leer de mas revienta · y la prueba del manifest en **dos
permisos exactos**, no «>=1».

## Lo que falta medir, y es una sola cifra

**Los fps que sostiene el aparato.** 921 600 bytes por frame a 720p; a 5 fps son
4,6 MB/s por un MethodChannel y otra copia por FFI. `QyroScanner.framesPerSecond`
existe para escribir ese numero. **Si sostiene >=5, hecho para siempre; si no,
entonces el cruce de copia cero por JNI tiene su argumento medido.**

**No hay aparato**, asi que el hueco esta en blanco. Fase 19.

## Lo que encontro una guarda

`promised_capabilities_test` prohibia el icono de escaner **porque no habia
camara**, y su propia razon decia como terminaba: «o se va la promesa, o llega la
camara con su plugin, su permiso, su ADR y su fila en el modelo de amenazas».
Llegaron cuatro y **faltaba la fila**. Ahora esta, y la guarda **no se debilito**:
dejo de prohibir el icono y pasa a exigir las cuatro piezas si aparece.

Y el changelog de dependencias canto que `rqrr` metia el crate `image` entero
—con `moxcms` y `pxfm`— en la biblioteca que Dart carga en el telefono.
`default-features = false` y fuera quince paquetes.

---

## El cruce JNI, cerrado con argumento a la fase 19

**No se escribe sin aparato.** Serian la **segunda** excepcion a
`forbid(unsafe_code)` de este taller —la primera costo una ADR entera— y un slot
equivocado en la vtable de JNI no da error de compilacion: da un salto a una
funcion arbitraria, y el sintoma es un proceso muerto sin traza en el aparato de
otra persona. Ninguna prueba de aqui puede tocar una `JNIEnv`.

Lo que falta es **exactamente un transporte de pixeles**, y su forma ya esta
fijada por `Eye::look(&[u8], usize, usize)`. Informe en
`docs/reports/fase-24-el-ojo.md`; decision en ADR-0048 enmienda 1.

## El hueco, en blanco

> **`R10` §8 T1 manda medir píxeles por módulo en el aparato real antes de
> escribir nada más. NO HAY APARATO.**

Lo que sí se hizo: **reproducir la aritmética** de forma independiente y dejarla
en código con prueba. Salen los dos números de `R10` idénticos — **3,07
px/módulo a 640×480 y 4,60 a 1280×720** para una v27. La decisión de ADR-0048 §3
es pedir ≥1280×720 y quedarse en v27, con la palanca escrita.

**Falta el glue de Kotlin y JNI**, que es la parte que no se puede ejercitar aquí.

## La comprobación 18, que ya cazó dos cosas

`scripts/gate.ps1` **lee `ci.yml`** y corre sus comandos más el objetivo de
Linux. Hoy cazó un `#![cfg(test)]` duplicado **antes** de empujar — el mismo error
de forma que en la fase 15, esta vez detenido por la puerta y no por CI.

---

## Los cinco arreglos

1. **`ptr_arg` en Linux** — `collect_mdns` del stub de no-Windows pedía
   `&mut Vec<FoundPeer>` sin añadir nada. Ahora `&mut [FoundPeer]`.
2. **Los cuatro enlaces de la Release daban 404** — apuntaban a la rama borrada.
   Reapuntados a `blob/main/`, y **comprobados los cuatro con `curl`: 200**.
3. **`ci.yml` decía «No `paths:` filter, deliberately»** diez líneas debajo del
   bloque `paths:` que lo desmiente.
4. **El registro de fichas tenía tres defectos, y el tercero lo encontró una
   guarda cuando yo creía haber terminado**: dos `- Estado:` en QYR-0088 y
   QYR-0089, **QYR-0089 duplicada entera** al principio del archivo, y ninguna
   cabecera. **167 fichas, 1 abierta** — antes decía «155, 0».
5. **`STATUS.md` daba un número de pruebas y son dos.** Windows **753**, medido
   hoy aquí; Linux, lo que diga CI — esta máquina compila y lintea para Linux
   pero **no ejecuta sus binarios**, y el último publicado (750) es anterior a
   los cambios de hoy. Se cita como la medida anterior, no como la actual.

**De regalo:** la prueba del enlace simbólico fallaba en cualquier consola sin
`SeCreateSymbolicLinkPrivilege` (error 1314) — indistinguible de «el resolvedor
deja pasar un enlace». Ahora **dice en voz alta que no se ejecutó**, porque
saltada no es pasada, y en `windows-latest` sigue corriendo de verdad.

## Lo siguiente

**24 → 22 → 17 → 18 → 19 → 20 → 23.** La 24 es la última capacidad que falta:
`qyro beam` dibuja QR desde la fase 15 y **nadie los lee**. `R10` ya decidió la
arquitectura y **lo primero no es código: medir píxeles por módulo en el aparato
real** (`R10` §8 T1 — 640×480 da 3,07 px/módulo, el suelo exacto de `rqrr`).

---

## 0.P0 — EL REPOSITORIO NO COMPILABA EN LINUX. Arreglado.

`qyro_net/src/lib.rs`: `dab9fa3` metió los `pub use` del beacon **entre un
`#[cfg(windows)]` y el elemento que guardaba**. El atributo se pegó al bloque
nuevo, así que fuera de Windows el beacon desapareció y `MdnsDiscovery` se
exportó sin existir. 193 ejecuciones de CI en rojo.

**Decisión congelada en ADR-0043 enmienda 2:** el beacon **es multiplataforma y
no lleva `cfg`** —sólo usa `std`, `socket2` e `if-addrs`, y es la implementación
que la §5 exige para las plataformas sin responder de mDNS—; sólo
`MdnsDiscovery` es de Windows.

**Comprobación 17, que sale de aquí:** ninguna «puerta en verde» sin
`cargo check --workspace --all-targets` contra Linux por código de salida. En
esta máquina: `rustup target add x86_64-unknown-linux-gnu` y
`--target x86_64-unknown-linux-gnu`; `check` no enlaza, no hace falta enlazador
cruzado. **Medido tras el arreglo: sale 0.**

---


## 1. Lo que se cerró en esta sesión

**El gate rojo, primero.** `check_docs_consistency` estaba en rojo en `5459a64`
—*«Stale verified commit: HEAD is 11 commits ahead»*— y se arregló actualizando
el ancla de `STATUS.md` **y volviendo a correr la puerta sobre el commit
resultante**, que es la comprobación 16 aplicada a sí misma.

## 0.ter — FASE 22, ABIERTA. Aquí se corta.

**ADR-0047 congelada** (`b5f5e97`), con los cinco números que la fase pedía. Dos
salieron de mirar en vez de suponer:

- **El desbordamiento de 4 GiB no existe.** `done` y `total` son `u64` en el
  motor y en la frontera C; el único `u32` es `item`, que vale siempre cero. La
  aritmética está bien; **la evidencia con archivos grandes sigue faltando**, y
  son dos cosas distintas.
- **`request_resume` tiene cero llamantes de producción** — sólo un test, sin
  símbolo C ni bandera de CLI. Habría sido el noveno caso. **ADR-0047 §5 la
  retira de la v1.x**, con argumento aritmético y dejando el número de mensaje
  reservado.

**Lo siguiente, en orden:**

1. **Ejecutar la retirada de §5.** Marcar `#[cfg(test)]` o borrar
   `request_resume` y el manejo de `MessageType::Resume` en
   `qyro_transfer/src/session.rs`, **y quitarla de todos los documentos que la
   mencionan** — una capacidad retirada que sigue anunciada es la misma mentira
   que una muerta.
2. **Los cinco escenarios** de `FASE-22 §4`, cada uno con su control. **El quinto
   deja de aplicar** si la reanudación se retira: se sustituye por comprobar que
   **cancelar deja el destino limpio**, sin `.qyro-part`.
3. ~~El saneado de nombres para terminal~~ **HECHO**, y con él **QYR-0364**: el
   receptor del CLI preguntaba «¿aceptas?» con una huella y nada más, mientras
   la GUI enseñaba los archivos desde siempre. `Session::offered_files` existe,
   el receptor los dibuja, y cada nombre pasa por `safe_terminal_name`.

4. ~~El techo de 256 archivos~~ **HECHO** (`3520b14`). `TooManyFiles` con código
   propio `-14` en la frontera, rechazo antes del primer descriptor, y su control:
   el techo exacto **no** se rechaza, porque un `>=` mal escrito movería el límite
   real a 255 sin que nadie lo notara.

**Queda de la fase 22: cuatro de los cinco escenarios de `FASE-22 §4`.** Todos
viven en `apps/qyro/test/transfer/gui_cli_matrix_test.dart`, que ya tiene el
arnés montado —biblioteca, binario, y el `tearDown` que espera a que el puerto
se suelte— así que cada uno es una casilla más:

1. ~~Carpeta con subcarpetas y una vacía~~ **HECHO** (`a932ec2`). Árbol comparado
   entrada por entrada; la carpeta vacía no viaja y está afirmado.
2. **200 archivos — QYR-0365, causa localizada y es peor de lo que parecía.**
   Bisecado: 10 y 50 entregan; 100 y 150 «fallan» — **pero los archivos llegan
   todos**. `IDLE_TIMEOUT` es 60 s y el corte cae entre 49,4 s y 80,3 s: es un
   reloj, no un recuento. Y debajo está el defecto real: **~1,2 s por archivo de
   64 bytes**, lineal. **No subas `IDLE_TIMEOUT`** — escondería esto y
   convertiría el fallo en veinte minutos de espera.

   **Culpé al disco y lo medí, y estaba mal.** `sync_all` cuesta **4,9 ms extra
   por archivo** en esta máquina (6,5 frente a 1,6): el 0,4 % de los 1 200 ms.
   Descartado con números.

   **Comprobado: es el reloj de lectura.** Con `READ_TIMEOUT` a 25 ms en vez de
   250, los mismos 50 archivos pasan de **49,4 s a 6,5 s** — 7,5× con el mismo
   código. La constante se revirtió: es el latido de ADR-0028 §4.1, no un botón.

   **El arreglo no es bajarla** —multiplicaría por diez los despertares de un
   hilo ocioso en máquinas viejas— sino que el bucle deje de necesitar varias
   lecturas vencidas por elemento.

   **El mecanismo ya está localizado en el código:**
   `qyro_session/src/session.rs:717` hace **un `read_frame()` por `step()`**, y
   su `Ok(None)` es «venció» — 250 ms gastados. Los dos lados hacen `step` en
   bucle, así que una ida y vuelta por elemento significa que ambos se turnan
   para esperar el reloj.

   **Medido cuál de los dos lados espera: el EMISOR.** Con 20 archivos,
   `emisor=75 receptor=1` lecturas vencidas — ~3,75 por archivo a 250 ms son
   ~0,94 s de los 1,24. El receptor no espera: trabaja y contesta.

   **El arreglo queda acotado a uno:** que el emisor no consuma un
   `READ_TIMEOUT` entero cuando todavía tiene trabajo que poner en el cable.
   `qyro_transfer` ya tiene ventana (`WINDOW_CHUNKS`, `chunks_in_flight()`), o
   sea que el protocolo ya está pensado para varias cosas en vuelo — es el bucle
   de sesión el que lo serializa. Las otras dos opciones quedan descartadas como
   primera medida y está escrito por qué.

3. **Un archivo > 4 GiB**, esparcido para no gastar disco. El control: el
   progreso del último frame **no es menor** que el del anterior. La aritmética
   ya se comprobó (`done`/`total` son `u64`, ADR-0047 §2.1); **falta la
   evidencia**, que es otra cosa.
4. **Disco lleno a mitad.** El destino no queda con ningún `.qyro-part`, y su
   contra-prueba: dejar uno a propósito y exigir que el mismo listado lo vea.
5. **Cancelar a mitad** — sustituye al escenario de reanudación, que ADR-0047 §5
   retiró. El destino tiene que quedar limpio.

---

## 0. LA RELEASE — retractada en público, y a medio rehacer

**Hecho hoy, y está vivo en
<https://github.com/M1gu3hb/-Qyro/releases/tag/v1.0.0>:**

- **Retractación pública** encabezando las notas, con los dos P0 explicados por su
  nombre, qué los causó, y **por qué no los detectó nada** — cada pieza verde y la
  cadena rota. El título dice `RETRACTADO: estos binarios no pueden enviar`.
- **`qyro-cli-windows-x64-QYR-0361-arreglado.zip` subido**, con su `LEEME.txt`,
  `SHA-256 b78199c147d93255…`. Verificado **antes** de subirlo: dos copias con
  huellas distintas, 20 000 bytes que cruzan, hash idéntico en destino.

**Lo que falta, y está dicho también en las notas públicas:**

1. **El APK — y hay un bloqueo con nombre.** `app-release.apk` sigue siendo el de
   antes y **no lleva el arreglo de QYR-0362**: la aplicación sigue sin poder
   enviar.

   **No se puede reconstruir en esta máquina y no es falta de tiempo:**
   `flutter doctor` encuentra el SDK de Android 36.0.0 pero **las licencias no
   están aceptadas** (`Android license status unknown`). Aceptarlas es aceptar un
   acuerdo legal en nombre del dueño, y eso no lo hace el implementador —
   **lo tiene que hacer una persona**, con `flutter doctor --android-licenses`, o
   hacerlo el CI con sus propias credenciales.

   Hasta entonces el hueco se queda en blanco y **está dicho en las notas
   públicas de la Release**, no sólo aquí.
2. **`qyro-windows-x64.zip`**, el paquete completo con la GUI de escritorio,
   tampoco está rehecho.

No se borró nada ni se despublicó: la nota se queda aunque el fallo esté
corregido, porque quien descargó aquello merece saber qué tenía en las manos.

---

**Fase 21 — HECHA.** Informe en `docs/reports/fase-21-las-dos-caras.md`, puerta
corrida en `52fa4d5`.

**Las cuatro casillas de la matriz pasan**, con el binario `qyro` de verdad al
otro lado y comparación byte a byte, más los dos controles. Tabla de paridad con
su script —vista fallar tres veces— y el consejero de canal en las dos caras
(`qyro_advice`, 24 → 25 símbolos con enmienda en ADR-0032).

**Lo que encontró vale más que lo que construyó:** tres defectos, los tres de la
misma forma —dos mitades probadas y el medio jamás recorrido— y ninguno lo vio
leer código.

---

**Fase 16 — HECHA.** Informe en `docs/reports/fase-16-canal-serie.md`, puerta
corrida en `5699fcd`.

| Commit | Qué |
|---|---|
| `4e88f37` | **ADR-0045 congelada**, en commit propio antes del código |
| `5699fcd` | `qyro_serial` (ARQ + CRC32 + Base64), los tres comandos, y el generador del receptor |

**El defecto que encontró ejecutar el script generado, antes de enviar nada:**
`BLOCK_BYTES` era 512, que no es múltiplo de tres, así que cada bloque codificaba
con relleno `=` y al concatenarlos el `=` quedaba en medio del flujo.
`certutil` lo rechaza —*«DecodeFile devolvió Datos no válidos. 0x8007000d»*— y la
transferencia habría informado de éxito con la otra máquina vacía. Ninguna prueba
interna lo veía: el decodificador de Qyro trabaja línea a línea y estaba de
acuerdo consigo mismo. **510**, y la invariante es un `const assert`.

**La puerta se puso en rojo y no por el código:** `rqrr` arrastra `lru` 0.12.5
con dos avisos de unsoundness y fija esa minor. Ignorados en `.cargo/audit.toml`
con qué son, por qué no llegan al producto y **qué los borra** — y con una guarda
que falla si `rqrr` deja de ser `dev-dependency`.

---

**Fase 15 — HECHA.** Informe en `docs/reports/fase-15-canal-optico.md`, puerta
corrida en `dc993d3`.

| Commit | Qué |
|---|---|
| `3633ec0` | `qyro_fountain`: Luby Transform, cero dependencias, generador congelado porque es formato de cable |
| `0125f2e` | `qyro qr` y `qyro beam`: medios bloques, invertido a propósito, 5 FPS |
| `dc993d3` | La vuelta completa: un decodificador real lee lo que dibuja la terminal |
| `ab947ab` | El informe |

**Lo que corrigió el uso y no el diseño:** el consejo de tamaño mentía (decía 37
columnas para un código de 41 — un consejo corto es peor que ninguno); un archivo
de 51 bytes dibujaba una v27 entera, el código más difícil de escanear para el
payload más pequeño; y `DrawError` no se ganaba el sueldo, porque su único
consumidor lo imprimía con `{:?}` y «TooLong» le llegaba a una persona como la
palabra TooLong.

**El receptor de CI se hizo, y no como estaba planteado.** No un directorio de
imágenes: rasterizando en memoria, con `rqrr` de dev-dependency. Un fixture
caduca y falla como «se rompió el renderizador»; esto dibuja lo que dibuja
`qyro beam`, en el momento, y lo vuelve a leer. `zune-jpeg` no hizo falta y la
trampa del MJPEG sin DHT no llega a existir.

**Coste medido:** +67 KB en el binario (1 306 624 → 1 373 696). `rqrr` no pone
ninguno: no viaja.

---

**Fase 14 — HECHA.** Informe en `docs/reports/fase-14-sin-router.md`, puerta
corrida en `07278ff`, el commit que el informe nombra.

| Commit | Qué |
|---|---|
| `f81c15a` | La cuenta atrás de APIPA (`qyro_session/src/link.rs`) y la trampa de `SocketAddrV6` |
| `b89a89a` | **ADR-0043 enmienda 1**, en commit propio antes del código |
| `dab9fa3` | El beacon por interfaz con `socket2`, y el puerto colapsado a una definición |
| `07278ff` | El lado Dart de `dev.qyro/discovery` y su llamante de producción |
| `f50ab2c` | El informe de la fase 14 |

**Dos hallazgos que no buscaba, los dos con cifra:**

- **D9** — `mdns-sd` casi dobla el binario: **666 624 → 1 295 872 bytes** al
  llegar `qyro find`. **+614 KB**, diez veces los 63 KB que este taller discutió
  para conservar el desenrollado de pila. El beacon propio hace lo mismo por
  **8 KB**. La ADR-0043 §7 citaba un presupuesto de 750–950 KB que el binario ya
  no cumple; la enmienda 1 lo corrige con la medida. **No se toca hoy** — lo
  decide la fase 19 con red de verdad.
- **D10** — el puerto que ADR-0041 congeló estaba escrito **dos veces y en
  ningún sitio del motor**, bajo un comentario que decía «no re-derivado: dos
  copias son dos puertos» siendo la segunda copia. Cerrado: `qyro_net::QYRO_PORT`
  es el original y una guarda lee el `.dart` y falla si se separan — **vista
  fallar a propósito** antes de darla por buena.

---

## 2. Lo siguiente, en orden

```
(retractar la Release) → 22 → 17 → 18 → 19 → 20 → 23
```

- **22 — lo que la gente hace de verdad.** Carpetas, tamaño, interrupción.
  **Es lo siguiente**, después de la Release.

---

## 3. Lo que sigue en blanco, y sigue en blanco a propósito

- **Cero pruebas en hardware físico.** Dos procesos en `127.0.0.1` no son dos
  máquinas. Que dos aparatos se encuentren por un cable **no está verificado**.
- **`NsdManager` no está ejercitado.** Las pruebas Dart usan un `MethodChannel`
  falso: prueban el lado Dart, no Android.
- **Ninguna cámara ha leído un QR de Qyro.** La vuelta completa la hace un
  decodificador sobre píxeles perfectos. Desenfoque, obturador rodante, moiré,
  brillo y pantalla en ángulo son fase 19.
- **El teléfono no acumula frames todavía.** El motor los produce y son legibles;
  el lado Android que los junta no existe.
- **La GUI y el CLI no se han hablado nunca.** Es la fase 21 y está a medias.
- **Ningún cable serie ha llevado un byte de Qyro.** El protocolo se probó
  sobre una cola en proceso y `certutil` sobre bytes reales; los dos puertos de
  esta máquina son endpoints Bluetooth, no un par enlazado. Fase 19.
- **El canal serie no llega a la GUI.** No hay símbolo en la frontera C, y
  ninguna pantalla lo menciona.
- **La reanudación del canal óptico no existe** (D11). ADR-0044 §5 la exige para
  sesiones largas; el límite de 20 MB es lo que hoy impide llegar a una.
- **La GUI de escritorio no tiene descubrimiento.** No hay símbolo en la
  frontera C. Lo dice con una frase, no con una lista vacía.
- El binario **no arranca en Windows 7** (`api-ms-win-core-synch-l1-2-0.dll`,
  fase 17).

---

## 4. Cuatro trampas de este entorno, para no repetirlas

1. **Heredocs de bash** destrozan `\n` y `\t` antes de que Python los vea. Usa
   `chr(92)`, escribe el script con la herramienta Write, o usa Edit.
2. **`git commit -m @'...'@` en PowerShell** se rompe si el mensaje lleva
   comillas: escribe el mensaje a un archivo y usa `git commit -F`.
3. **Flutter no está en el PATH.** Está en `D:\flutter\bin`.
4. **`verify_static.ps1` exige `-Binary`**, y el binario de la tubería es
   `target/x86_64-pc-windows-msvc/release/qyro.exe` — no `target/release`, que
   se compila con otro perfil y pesa distinto.

---

## 5. La regla que más valor dio, otra vez

**Cuando una guarda te dice que estás equivocado, tiene razón más veces de las
que crees.** En esta sesión pararon tres y acertaron las tres: el registro de
`beacon.rs`, `clippy` sobre un `assert!` entre constantes que se optimiza y no
prueba nada, y —la mejor— `qyro_session_re_exports_nothing_it_does_not_own`
rechazando `pub use qyro_net::QYRO_PORT`, porque todo lo que la fachada republica
se vuelve nombrable desde `qyro_ffi` y una excepción juzgada inofensiva de una en
una es cómo llega la primera peligrosa.

Y una cuarta cosa lo dijo sin ser una guarda: **el enlazador**. Con `beacon.rs`
escrito y sin llamante el binario no cambió ni un byte. Una capacidad sin
llamante no se envía, se compila.


## El progreso, y lo que aparecio debajo

**Arreglado:** el receptor no asignaba `done` nunca —su barra era un cero fijo
hasta el salto final, indistinguible de un cuelgue— y `Progress::item` iba
siempre a cero en los dos extremos. QYR-0318 lo habia **documentado** como
«siempre cero, porque el motor no lo asigna»: describir un defecto con precision
no es arreglarlo. Y la propia prueba de progreso llevaba escrito el hueco
—*«deliberately no assertion on the receiver's done»*— y ahi seguia.

**Lo que aparecio al ir a comprobar que la pantalla lo enseñara:** `QyroProgress`
solo existe dentro de `apps/qyro/lib/ffi/`. **Ninguna pantalla lo lee.** El camino
entero —Rust emite, la frontera despacha, Dart lo envuelve— acaba en nadie: la
**duodecima** capacidad escrita, probada e inalcanzable. Eso es fase 26 y es
exactamente su motivo.

**Lo que falta y no se hace de rebote:** la **M** de «archivo N de M» es un
parametro mas en `QyroProgressFn`, o sea la frontera C. Calibre alto, con su
enmienda a ADR-0032, no colado en un arreglo de progreso.


## El documento de paridad avalaba lo que no comprobaba

Quince citas de `PARIDAD-GUI-CLI.md` apuntaban a `};`, a `}`, a `setState(() {`,
a un comentario: **trece de catorce filas**. Y el documento decia «la comprueba
`check_parity.ps1`», cierto a medias: el guardian verificaba que el archivo
tuviera esa linea, **no que la linea dijera algo**.

**El arreglo mecanico se probo y se tiro.** Resolver cada numero al simbolo mas
cercano hacia arriba daba «Rechazar con motivo → `_drainReceive`». Precision
falsa: peor que el numero viejo, porque ya no se nota. Las citas de ahora estan
puestas a mano contra el listado de declaraciones.

El guardian pide ahora que la cita caiga en algo **nombrable**. Que corresponda a
la capacidad no es mecanizable, y esta escrito en el script que no lo es.


## El APK, construido — y la comprobacion 20 medida donde toca

Tres sesiones sin construirse. Construido hoy sobre NDK 28.2.13676358, perfil
release, `android-arm64`:

| | |
|---|---|
| `libqyro_ffi.so` (cargo) | 1 069 952 bytes |
| `libqyro_ffi.so` (**extraido del APK**) | 1 069 944 bytes |
| APK | 18.0 MB |
| Gradle | 353.3 s |

**Comprobacion 20 — PASA.** Los cuatro `PT_LOAD` del `.so` **sacado del APK**
tienen `p_align = 0x4000`. Se mide ahi y no en la salida de `cargo` porque lo que
carga Android 15 con paginas de 16 KB es lo que va dentro del paquete, y entre
una cosa y otra hay un empaquetador.

**Lo que esto NO dice:** no se ha ejecutado en un telefono. Que cargue es una
propiedad del binario y esta medida; que funcione no, y no hay aqui ninguna
evidencia de hardware.
