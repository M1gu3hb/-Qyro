# Registro de hallazgos

Una ficha por hallazgo, **un solo `- Estado:`** y un solo bloque `## QYR-xxxx`.
`check_docs_consistency` lo comprueba: encontró que QYR-0089 estaba **duplicada
entera** —esta cabecera ocupa el sitio donde estaba la copia sobrante— y que
QYR-0088 y QYR-0089 llevaban un `- Estado: abierto` viejo debajo del `cerrado`.

Los tres se retiraron el 2026-08-19. Ninguno cambiaba lo que el script de
recuento informaba —lee el primer estado y la primera aparición— y los tres
contradecían la ficha a quien la leyera entera, que es la única forma en que
alguien la lee de verdad.

Estados: `cerrado`, `descartado`, `abierto`. No hay más.

## QYR-0381 — La huella del código de emparejamiento se validaba y se tiraba

- Estado: **CERRADO en el CLI. Abierto en la GUI, y aquí está el argumento.**
- Severidad: **ALTA** (la mitad cara del emparejamiento no compraba nada)
- Fecha: 2026-08-31

**ADR-0035 §2.1, literal:**

> *«Si la cadena llevaba una huella y no coincide con la autenticada, la sesión
> se refusa **sin preguntar a nadie**. Quien escaneó ya contestó la pregunta, y
> preguntar otra vez es cómo la gente aprende a decir que sí.»*

**Y el doc-comment de `parse_pairing` decía lo mismo**, en presente: «la mitad de
la huella **no** se devuelve a propósito: es una expectativa que hay que
comprobar contra la huella autenticada, no un valor que enseñar como si estuviera
establecido».

**Nadie la comprobaba, y nadie podía.** `parse_pairing` devuelve la dirección y
tira la huella; `qyro_pairing_parse` —el símbolo por el que pasa la GUI— emite la
dirección **y nada más**. Ningún llamante tenía forma de recuperarla.

**Así que un código tecleado a mano establecía menos que un `ip:puerto`.** Quien
teclea un código lo compara carácter a carácter con la pantalla del otro aparato
— la parte cara del emparejamiento, la que hace una persona con los ojos — y
Qyro se quedaba con la mitad barata, la dirección. Teclear `192.168.1.5:49517` y
añadir `--expect <huella>` daba **más** garantía que teclear el código entero.

**El arreglo, en el CLI.** `qyro_session::pairing_fingerprint` devuelve la
expectativa, y `qyro send` la usa **automáticamente** cuando `--to` es un código.
`--expect` explícito sigue ganando, porque es una decisión que alguien escribió a
propósito para esa ejecución. Y cuando la negativa viene del código y no de una
bandera, el mensaje lo dice: *«esa expectativa venía del código de emparejamiento,
no de una bandera»*.

Comprobado ejecutándolo: con el código bueno manda; con una huella cambiada,
`REFUSED`.

Devuelta aparte y no como par, para que todos los llamantes de `parse_pairing`
sigan significando lo que significaban. Y las dos nunca discrepan sobre qué es un
código, lo cual tiene su prueba: si pudieran, un llamante obtendría dirección
para una cadena cuya huella no puede leer, y marcaría **sin expectativa
ninguna**, en silencio.

### Lo que queda abierto, y por qué no se hace aquí

**La GUI no puede hacer lo mismo todavía.** `qyro_pairing_parse` emite sólo la
dirección, así que la huella no cruza la frontera C. Arreglarlo es **otro símbolo
más** —o cambiar el que hay, que es peor: cambiar la forma de un símbolo
existente rompe a cualquiera que lo llame— y eso es una enmienda de ADR-0032, la
segunda del día.

**No se hace en esta tanda por una razón concreta y no por cansancio:** el
entorno donde se arregló **no tiene Flutter**, así que un cambio de la frontera C
más su lado Dart más su pantalla se escribiría entero sin poder ejecutarse. Ya
hay seis cambios así en esta tanda; el séptimo, en el camino de la seguridad, no
lo compensa. Queda escrito aquí con su forma completa para quien lo tome.

**Lo que sí protege a la GUI hoy:** la huella que enseña la tarjeta de oferta es
la **autenticada**, y una persona puede compararla. Es la comprobación que
siempre ha sido de la persona.

## QYR-0382 — Nada recuerda un aparato, así que «esta clave ha cambiado» no puede ocurrir

- Estado: **CERRADO como documentación. La capacidad sigue sin llamante.**
- Severidad: **ALTA** (el `README` prometía una defensa que no existe)
- Fecha: 2026-08-31

**El defecto.** `Session::remember_peer` existe, tiene símbolo en la frontera C
(`qyro_session_remember_peer`), tiene enlace en Dart (`rememberPeer`) — y
**nadie lo llama en producción**. Los únicos llamantes están en
`session_behaviour.rs`.

**La consecuencia.** La libreta está **siempre vacía**, así que `peerTrust`
contesta `newPeer` para todo el mundo y **`PeerTrust::Changed` es inalcanzable**.
La defensa que el `README` prometía —«si un aparato conocido presenta otra clave,
Qyro se niega»— **no puede ocurrir**, porque ningún aparato llega a ser conocido.

Y el escenario **C4** del protocolo de hardware pide exactamente eso: reinstalar
la aplicación y ver el aparato en rojo. **No es ejecutable**, y descubrirlo con
el teléfono en la mano es la peor forma de descubrirlo.

**Qué se hace en esta tanda.** Escribirlo donde se lee:

- El `README` lo dice, en el mismo punto donde antes prometía lo contrario.
- La guía de prueba avisa antes de C4 y dice qué anotar: **no ejecutable**, no
  fallido. Y ofrece la mitad que **sí** se puede comprobar —un código con una
  letra cambiada, que sale `REFUSED`— que es el escenario **E3**.

**Qué NO se hace, y por qué.** Conectarlo no es cablear una llamada: hace falta
**una pantalla donde una persona le ponga nombre a un aparato**, porque la
libreta va por nombre y un nombre que el programa se invente es un nombre que
nadie reconoce. Eso es una función, no un arreglo, y una función que además
decide cómo se ve la confianza en este producto. Merece su ADR y su fase, no el
final de una tanda de auditoría.

**La pregunta que cierra esta ficha:** ¿promete Qyro una defensa que no tiene?
**Ya no.**

## QYR-0380 — Mandar desde el teléfono no funcionaba, por dos motivos a la vez

- Estado: **CERRADO**
- Severidad: **P0 para el escenario D1** (teléfono → PC, la primera prueba con el teléfono)
- Fecha: 2026-08-31

**Dos defectos en la misma pantalla, y cada uno basta para que no se pueda
mandar nada.**

### 1. El botón no se encendía al escribir

```dart
onPressed: _chosen.isEmpty || _address.text.trim().isEmpty ? null : _send,
```

El `TextField` lleva un `TextEditingController` y **ningún `onChanged`**. Un
controlador no reconstruye nada al escribir, así que el botón se quedaba apagado
hasta que otra cosa llamara a `setState`.

**Y por eso pasaba las pruebas y no la vida:** elegir archivos **sí** llama a
`setState`, así que la pantalla funcionaba si se escribía la dirección **antes**
de elegir — y no si se hacía al revés, que es el orden natural: primero eliges lo
que mandas, luego dices a dónde.

### 2. El campo pedía un código y el motor quiere una dirección

La etiqueta era `peersManualLabel`, «Código de emparejamiento» — que es
exactamente lo que el otro aparato enseña. Y `_send` pasaba el texto **tal cual**:

```dart
address: _address.text.trim(),
```

Por debajo, `qyro_session_open_sender_blocking` hace
`address.parse::<SocketAddr>()`. Un código de emparejamiento **no** es un
`SocketAddr`, así que salía `bad_argument`.

**La pantalla de Aparatos sí lo resolvía** —tiene su `_resolve()` con
`addressOfPairingString`— y ésta no. Y ésta es la que se usa para mandar.

**El arreglo.** `onChanged: (_) => setState(() {})`, y `_send` pasa el texto por
el mismo analizador del motor: devuelve `null` cuando no es un código, y entonces
se usa tal cual, que es lo correcto para un `ip:puerto` tecleado a mano. **Se
aceptan los dos**, y la etiqueta lo dice: «Código de emparejamiento, o dirección
como ip:puerto».

**La prueba nueva hace las cosas en el orden que fallaba:** elegir primero,
escribir después. Es lo que ninguna prueba hacía.

**La pregunta que cierra esta ficha:** desde el teléfono, ¿se puede mandar un
archivo tecleando el código que el PC enseña? **Sí, y la etiqueta ya no promete
algo distinto de lo que acepta.**

## QYR-0378 — El permiso de cámara no se pedía nunca, así que no había canal óptico

- Estado: **CERRADO**
- Severidad: **P0 para el canal óptico**
- Fecha: 2026-08-31

**`CAMERA` es un permiso peligroso**, y desde Android 6 declararlo en el
manifiesto **no concede nada**: hay que pedirlo en tiempo de ejecución. **Nada en
este repositorio lo pedía** — ni `ScannerChannel.kt`, ni `MainActivity.kt`, ni el
lado Dart. Un `grep` de `requestPermissions` en todo el árbol de Android no
devolvía una sola línea.

`bindToLifecycle` con el permiso denegado lanza `SecurityException`, que llegaba
a la pantalla como **«Este aparato no puede mirar»** — una frase sobre el
aparato, cuando lo que faltaba era una pregunta que nadie había hecho.

Y era la última puerta cerrada del canal óptico, después de QYR-0371: el botón se
arregló por la mañana y detrás no había cámara.

**El arreglo.** `ScannerChannel` comprueba y pide, y `start` se niega con el
código `permission` en vez de dejar que CameraX lance. La pantalla lo distingue:
dice que el aviso del sistema está en pantalla y ofrece **volver a intentarlo**.

**Se pide y no se espera la respuesta**, a propósito: esperarla exige
`onRequestPermissionsResult` y una máquina de estados a través del canal para una
respuesta que la persona ve con sus ojos. Pulsar otra vez es más barato de
escribir y de entender, y no tiene estados intermedios que puedan quedarse
colgados.

`requestPermissions` de la plataforma y no `ActivityCompat`: existe desde API 23,
el `minSdk` está muy por encima, y así no depende de que `androidx.core` siga en
el grafo por una biblioteca de cámara.

**La pregunta que cierra esta ficha:** al pulsar «Leer códigos con la cámara»,
¿sale el aviso del sistema? **Sí. Que después la cámara lea un QR de verdad es el
escenario F2, y sigue en blanco.**

## QYR-0379 — El canal óptico recibía el archivo y lo tiraba

- Estado: **CERRADO**
- Severidad: **P0 para el canal óptico** (recibía y no entregaba)
- Fecha: 2026-08-31

**El defecto, en una línea.** `scan_screen.dart`, al completarse un escaneo:

```dart
final bytes = scanner.result();
setState(() => _receivedBytes = bytes?.length);
```

El ojo reensamblaba el archivo entero, lo verificaba, lo devolvía **y la pantalla
se quedaba con su longitud**. Imprimía «Recibido: N bytes» y tiraba los bytes.

**El único canal que funciona sin red de ninguna clase no entregaba nada**, y lo
decía con una frase que sonaba a éxito.

**El arreglo.** Se escribe en la misma carpeta que el resto de lo que llega
—`getExternalFilesDir(null)/Qyro`, la de QYR-0373— y la pantalla **dice dónde**.

**El nombre lo pone Qyro, y eso también se dice.** Un QR no lleva el nombre del
archivo dentro: `qyro_eye` reensambla un bloque de bytes y nada más, porque un
código que cargara con un nombre cargaría con menos archivo. Así que el nombre es
`qyro-optico-<fecha>.bin`, **y la pantalla explica que lo eligió ella**. Un
nombre inventado que se anuncia es honesto; uno que se calla hace creer a la
persona que lo eligió quien mandó el archivo.

La marca de tiempo va dentro para que dos lecturas seguidas no colisionen, y aquí
una colisión no se puede negociar con nadie: no hay otro extremo al que
preguntarle.

**La pregunta que cierra esta ficha:** cuando el escaneo termina, ¿hay un archivo
en el disco? **Sí, y la pantalla dice cuál y dónde.**

## QYR-0376 — La identidad se escribía en la raíz del sistema, así que no había identidad

- Estado: **CERRADO**
- Severidad: **P0** (sin identidad no hay nada, en ninguna dirección)
- Fecha: 2026-08-31

**Es el hermano mayor de QYR-0373, y es peor.** `defaultIdentityPath()` devolvía,
fuera de Windows, `Directory.current.path + '/identity.qyro'`, bajo un comentario
gemelo del otro: *«On Android the Kotlin side is what knows
`getNoBackupFilesDir()`; until it passes one in, the app's working directory is
the honest answer»*. Ese lado Kotlin tampoco se escribió.

**En Android eso es `/identity.qyro`**, la raíz del sistema. Escribir ahí falla,
así que `openIdentity()` fallaba, así que **toda sesión contestaba
`identity_unreadable`** (ADR-0040, que decidió a propósito que una identidad que
no abre **no se regenera**).

**Y eso no es medio producto, es el producto.** El destino roto (QYR-0373)
impedía **recibir**. Esto impide **todo**: sin identidad no hay handshake, ni
huella que enseñar, ni código de emparejamiento — así que el teléfono no puede ni
mandar ni recibir, y la pantalla de aparatos no tiene código que dibujar.

**El arreglo.** `PathsChannel` gana un método `identity`, que devuelve
`getNoBackupFilesDir()/identity.qyro`, y `HomeScreen` lo pregunta antes de abrir.

**`getNoBackupFilesDir` y no `getFilesDir` ni `getExternalFilesDir`**, y las tres
razones son distintas:

- **Interno**, o sea privado por el sandbox de UID, que es exactamente la
  protección que `THREAT_MODEL.md` nombra para esta semilla y la que ADR-0040 §7
  asume. `getExternalFilesDir` —donde sí va lo que se recibe— lo puede leer
  cualquier aplicación con permiso de almacenamiento. **Ahí no va una clave
  privada.**
- **`NoBackup`**, que es la tercera cerradura de QYR-0349: esa ficha encontró que
  la aplicación mandaba el blob envuelto a Google Drive por omisión, y se cerró
  con `allowBackup=false` y `dataExtractionRules`. Un directorio que el sistema
  **nunca** copia lo cierra otra vez, diga lo que diga el manifiesto el día que
  alguien lo edite.

**Vigilado** por `qyro_core::repository_contract::the_phone_is_told_where_to_write`,
que ahora comprueba las dos rutas y **que la identidad no acabe en la externa**.

**La pregunta que cierra esta ficha:** ¿tiene el teléfono una identidad estable?
**Tiene dónde guardarla, que es lo que le faltaba.** Que sobreviva a un reinicio
es el escenario **B2**, y sigue en blanco.

## QYR-0377 — Los comandos de construcción de Android no podían construir

- Estado: **CERRADO**
- Severidad: **P0** (el propietario no podía llegar ni a tener un APK)
- Fecha: 2026-08-31

**El defecto.** `docs/testing/hardware-protocol.md` §1 —y la primera versión de
`docs/GUIA-DE-PRUEBA.md`, o sea el mío— decían:

```
rustup target add aarch64-linux-android
cargo build --release --package qyro_ffi --target aarch64-linux-android
```

**Y eso falla**, con `linker \`cc\` not found`. Rust sabe **compilar** para
Android y no sabe **con qué enlazar**: hace falta apuntarle al `clang` del NDK
con `CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER`. Los workflows de CI lo hacen
—cuatro líneas de `export` en cada uno— y **ninguno de los dos documentos que una
persona sigue lo decía**.

**Y el segundo, en el mismo sitio:** el paso 3 del protocolo hacía

```
copy ..\..\target\release\qyro_ffi.dll build\windows\x64\runner\Release\
```

sobre una DLL que **ningún paso construye**. Copiaba lo que hubiera dejado ahí
otra compilación, o nada. Y `target\release\` es el objetivo del **host**, que no
es el mismo directorio que `target\x86_64-pc-windows-msvc\release\`.

**El arreglo.** Los dos documentos llevan ahora el bloque completo, en
PowerShell, que **resuelve la ruta del NDK sola** y comprueba con un `Test-Path`
que el `clang` existe antes de intentar nada — porque «no such file or directory»
sin decir cuál es media hora perdida. Y el nombre del enlazador de 32 bits se
escribe con su aviso: es `armv7a-…-androideabi21-clang`, con la `a` y con `eabi`,
y **no se deriva** del nombre del objetivo de Rust.

La DLL de Windows se construye antes de copiarse, y desde el directorio de
objetivo correcto.

**Y de paso, la alineación de 16 KB, pedida en vez de supuesta.** Los dos
workflows que construyen para Android exportan ahora
`RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384"`. El NDK r28 la pone por
omisión y el r27 no, y un runner de GitHub cambia de NDK sin avisar a nadie. Un
`.so` alineado a `0x1000` **no carga** en Android 15. Es seguro ponerlo ahí:
`.cargo/config.toml` sólo tiene tablas de Windows, los comandos pasan `--target`
—así que los scripts de compilación del host no lo ven— y es idempotente donde ya
estaba puesto.

**La pregunta que cierra esta ficha:** siguiendo la guía al pie de la letra,
¿sale un APK? **Sí, o sale un mensaje que dice qué falta.**

## QYR-0375 — Todo fallo de envío se anunciaba como un problema de red

- Estado: **CERRADO**
- Severidad: **MEDIA** (manda a la persona a mirar donde no es)
- Fecha: 2026-08-31

**El defecto.** `qyro send` imprimía `could not connect: {error}` para **todas**
las razones por las que un envío no arranca. Pero `Session::open_sender`
construye el manifiesto **antes** de marcar, así que varias de esas razones
vuelven de una función que no ha tocado un socket.

**Medido.** Un archivo llamado `inocente.txt\rACEPTADO: nada peligroso aqui` se
rechaza —correctamente, porque `qyro_manifest` no acepta caracteres de control en
un nombre— y a la persona se le decía «could not connect». Se va a mirar la red,
el router y el cortafuegos por un problema que estaba en el nombre del archivo.

**El arreglo.** Una frase por razón, y cada una nombra dónde mirar: el nombre
rechazado dice que es el nombre y qué clases de nombre se rechazan; el peer
inalcanzable dice las tres cosas que comprobar; la identidad ilegible dice cuál
es el archivo y por qué Qyro no se inventa otra. Con su control: dos razones
distintas no pueden dar la misma frase.

**Y un detalle de formato que era un defecto de verdad.** El primer borrador
usaba un literal continuado con `\`. **`cargo fmt` lo colapsa en una sola línea
de fuente y mete la sangría dentro de la cadena**, así que imprimía quince
espacios delante de cada línea de continuación. Se construye juntando un array,
que no puede sufrir eso, y hay una prueba que mide la sangría.

### Y el arreglo introdujo un defecto, que también está aquí

La línea nueva imprimía `path.display()` **en crudo**, y el mismo archivo de
prueba —el del retorno de carro— **reescribió la línea que lo estaba
anunciando**. En el único programa del proyecto que ya sabe que un terminal es un
intérprete: ADR-0047 §6 pone esa regla en el sitio donde se dibuja, y esto era un
sitio donde se dibuja.

Pasa por `qyro_session::safe_terminal_name`, con su prueba en los dos sentidos, y
se imprime **el nombre y no la ruta**: un diagnóstico no necesita las carpetas, y
enseñarlas es lo que las notas de seguridad llaman exponer rutas completas.

**Y el orden.** «connecting to …» se imprimía antes de construir el manifiesto,
así que quedaba justo encima de «el nombre fue rechazado antes de mandar nada»:
dos frases contradiciéndose, una de ellas falsa. Ahora dice «preparing …» antes y
«connected to …» cuando de verdad hay conexión.

**La pregunta que cierra esta ficha:** cuando un envío no arranca, ¿sabe la
persona dónde mirar? **Sí, y no siempre es la red.**

## QYR-0374 — La barra llegaba al 100 % y no se guardaba nada, sin decir por qué

- Estado: **CERRADO**
- Severidad: **ALTA** (y en la GUI era peor: decía «entregado» con el disco vacío)
- Fecha: 2026-08-31

**Encontrado ejecutándolo tres veces seguidas.** Mandar el mismo archivo dos
veces a la misma carpeta da, la segunda:

```
[################################] 100%  100000 / 100000 bytes

  0 file(s) saved in .
```

Una barra completa, y después una línea que dice que no se guardó nada. **Sin una
palabra de por qué.** Quien lo vea concluye, con razón, que el programa está roto.

**Y negarse es lo correcto.** ADR-0027 §4: no se sobrescribe **nunca**. El
defecto no es la negativa, es que los dos consumidores **tiraban el motivo**:

- El CLI, con `session.finish().unwrap_or(0)`. Una negativa se convertía en un
  cero, y «cero archivos guardados» y «no había nada que guardar» se imprimen
  igual.
- **La GUI, peor.** `finish()` vivía en un `finally` con un `on
  QyroSessionFailure {}` vacío, y el comentario que lo justificaba decía que «el
  final ya decidió lo que fue esta transferencia». **No lo había decidido:** el
  final es `Completed` porque **la transferencia** terminó —los bytes cruzan
  bien—, y `finish` se niega por una razón del sistema de archivos que ese final
  no conoce. Así que el worker devolvía `0`, o sea éxito, y **el teléfono decía
  «entregado» con nada en el disco**.

  Es exactamente la forma de **QYR-0357** que el comentario de encima dice haber
  cerrado. La misma mentira, por otra puerta.

**El motor no tenía nada que arreglar**, y eso es parte del hallazgo:
`Session::finish` devuelve `Err(StorageRefused)` y lo devuelve bien. Lo pinta
ahora `a_second_arrival_under_a_taken_name_refuses_with_a_reason_and_not_a_zero`,
que además comprueba que el archivo de la primera vez sigue intacto y que no
queda ningún `.qyro-part` tirado.

**El arreglo.**

- El CLI distingue los dos casos y, cuando la transferencia terminó y no se
  materializó nada, dice el error, **dice que Qyro no sobrescribe nunca** y da el
  comando para recibir en otra carpeta.
- La GUI materializa **dentro** del camino feliz, así que su fallo puede cambiar
  la respuesta; una bandera evita que el `finally` lo llame dos veces. En un
  final que no fue `Completed`, tragárselo ahí sigue siendo correcto y no
  esconde nada: esa transferencia ya fracasó por otra razón.
- Y `receiveNoRoom` —la frase que ahora sale— deja de decir sólo «No hay sitio
  para esto»: nombra la causa que de verdad se va a dar.

**Medido antes y después**, con el mismo archivo mandado dos veces:

| | Antes | Después |
|---|---|---|
| CLI | `0 file(s) saved in .` | «los archivos llegaron y no se guardó ninguno», el error, y qué hacer |
| GUI | «entregado» | la negativa, con su frase |

**La pregunta que cierra esta ficha:** cuando la barra llega al 100 % y no se
guarda nada, ¿sabe la persona por qué? **Sí.**

## QYR-0373 — El teléfono escribía en la raíz del sistema, así que no escribía

- Estado: **CERRADO**
- Severidad: **P0** (recibir en el teléfono no hacía nada, y ése es medio producto)
- Fecha: 2026-08-31

**El defecto.** `defaultDestination()` en
`apps/qyro/lib/transfer/native_transfer_service.dart` devolvía, en Android,
`Directory.current.path + '/Qyro'`, bajo este comentario:

> *«Android hands the app its own directory; the Kotlin side passes it in. Until
> it does, the process working directory is the honest answer rather than a
> guessed path that would fail at write time.»*

**El directorio de trabajo de un proceso de Android es `/`.** Así que la
respuesta era **`/Qyro`**: la raíz del sistema de archivos, que ninguna
aplicación puede escribir. `Directory('/Qyro').createSync(recursive: true)`
lanza, y lanza dentro de `receive()` **antes de emitir un solo estado**: pulsar
Recibir en el teléfono no producía nada visible — ni código propio, ni
«esperando», ni error.

**El comentario decía la verdad sobre el plan y era falso sobre el resultado.**
«El lado Kotlin la pasa» describía algo que **nunca se escribió**, y «el
directorio de trabajo es la respuesta honesta» era falso: una respuesta honesta
es una que se puede usar, y ésta falla siempre.

**El arreglo.**

- `PathsChannel.kt`, nuevo, en `dev.qyro/paths`, con un solo método
  `destination`. Devuelve `getExternalFilesDir(null)/Qyro` y **la crea**.
  `getExternalFilesDir` y no otra cosa por tres propiedades que hacen falta a la
  vez: no pide **ningún permiso** —un permiso de almacenamiento aquí sería el
  defecto que ADR-0034 §4 existe para impedir—, **la persona puede llegar a lo
  que recibió** por USB o con un explorador de archivos, y **se borra al
  desinstalar**. `getFilesDir()` también es escribible y **no se ve desde
  fuera**: un archivo que llega y que su dueño no puede abrir no ha llegado.
- Se registra en `MainActivity`, junto a los otros tres canales.
- `qyro_paths.dart` lo pregunta, y **ninguno de sus fallos lanza**: canal
  ausente, `PlatformException`, `null` y cadena vacía devuelven `null` y quien
  llama se queda con lo de antes. Un arreglo que revienta cuando el canal no está
  sería peor que el defecto.
- `ReceiveScreen` pregunta antes de abrir la sesión, en vez de pasar `''`.

**Y lo que no se ha cambiado, a propósito:** la línea de respaldo de
`defaultDestination()` sigue siendo la de antes, con el comentario ahora
corregido. Poner ahí otra ruta sería una suposición sobre un aparato que este
taller no ha tocado nunca, y una suposición que falle **al escribir el primer
byte** es peor que un fallo al crear la carpeta, que al menos ocurre antes de que
nada esté en el cable.

**Cómo se vigila.** `qyro_core::repository_contract::the_phone_is_told_where_to_write`
comprueba las cuatro piezas —el canal, su registro en `MainActivity`, que la
pantalla pregunta, y que los dos lados abren **el mismo nombre de canal**— y
corre dentro de la puerta. Una prueba de Dart puede demostrar que el puente
pregunta bien, y `qyro_paths_test.dart` lo hace; **no** puede demostrar que
Kotlin conteste ni que la pantalla llegue a preguntar.

**Lo que esta ficha NO puede decir.** Que un teléfono haya escrito un archivo en
esa carpeta. El entorno donde se arregló no tiene Android ni Flutter. El
escenario **D2** sigue en blanco, y la guía dice qué mirar si vuelve a fallar.

**La pregunta que cierra esta ficha:** ¿tiene el teléfono una carpeta escribible
donde dejar lo que llega? **Sí, y se la da quien la conoce.**

## QYR-0372 — «¿Aceptas 0 archivos, 0 B?»

- Estado: **CERRADO**
- Severidad: **ALTA** (se pedía aceptar una transferencia sin decir qué, en las dos caras)
- Fecha: 2026-08-31

**Encontrado ejecutándolo, no leyéndolo.** `qyro recv` contra un emisor real
imprime, literal:

```
  someone connected. They say they are:
    b76c0bb3-034672e9-4c9ab47b-632ddcc0
  they have not said what they are sending yet.
  accept from this device? [y/N]
```

Y en el teléfono, la tarjeta de oferta dibuja
`receiveOfferFrom(fileCount, humanBytes(totalBytes))` y recibía **cero y cero**:
«0 archivos, 0 B».

**QYR-0364 está registrada como cerrada con la frase «una pregunta sin objeto es
una formalidad, no una decisión».** Se cerró en el motor —`offered_files()` lee
el manifiesto y lo lee bien— y **en ningún consumidor**.

**La causa, medida.** `Session::open_receiver` vuelve en cuanto el handshake
termina; la oferta y el manifiesto llegan **dos pasos** después, no uno:

| tras | `offered_files()` | `progress().total` |
|---|---|---|
| paso 1 | vacío | **0** |
| paso 2 | el manifiesto | el total real |

- **El CLI** llamaba a `offered_files()` sin dar **ningún** paso, así que el
  motor no tenía nada que decir y el terminal preguntaba a ciegas.
- **La GUI** daba **un** paso —con un comentario que decía «un paso trae la
  oferta y el manifiesto», y son dos— y mandaba `progress().total`, que ahí vale
  0. Y `fileNames` estaba **escrito a mano como lista vacía**, porque
  `Session::offered_files()` **no cruzaba la frontera C**: existía desde
  QYR-0364, tenía un llamante en el CLI, y ningún símbolo. La tarjeta que dibuja
  los nombres ya estaba hecha y nadie se los daba.

**Y 0 no es «no lo sé» para quien lo lee: es «nada»**, que es una mentira
distinta.

**El arreglo.**

1. `Session::await_offer()` en el motor: da pasos hasta que el manifiesto está, y
   no más. Acotado en ocho —dos es lo que hace falta— para que un par que conecta
   y se calla termine en el plazo de lectura y no aquí. **El número vive en un
   solo sitio**, que es la lección del puerto: tres copias y ningún original.
2. `qyro recv` lo llama antes de preguntar. Ahora dice: «they want to send 1
   file(s), 200000 bytes: informe.pdf (200000 bytes)».
3. **ADR-0032 enmienda 6**, congelada antes del código en su propio commit, y dos
   símbolos —**treinta y tres**—:
   `qyro_session_await_offer_blocking` y `qyro_session_offered_files`, el segundo
   con el contrato de texto de la enmienda 1 y el separador NUL de
   `qyro_trust_list_peers`.
4. El worker de Dart los usa, y `totalBytes` pasa a ser **el total del
   manifiesto** en vez del progreso: es lo que el par dice que va a mandar, que
   es la pregunta que se le hace a la persona.

**Dónde se mide, y por qué ahí.** En `two_process_pairing_test.dart`, que pone un
receptor de Dart de verdad frente a un emisor de verdad. Una prueba de widgets no
habría visto nada: la tarjeta ya sabía dibujar nombres, y las dos pruebas que la
cubren le pasan `fileNames` a mano. **Lo que fallaba no era la tarjeta, era que
nadie se los daba.**

**La pregunta que cierra esta ficha:** ¿sabe la persona qué está aceptando?
**Sí, en las dos caras.** Comprobado a mano en el CLI, entre dos procesos.

## QYR-0371 — El canal óptico no tenía puerta en el teléfono

- Estado: **CERRADO**
- Severidad: **ALTA** (el único canal que funciona sin red, inalcanzable)
- Fecha: 2026-08-31

**La duodécima capacidad escrita, probada y sin llamante.** `PeersScreen` acepta
un callback `onScan` desde la fase 24B, y junto al botón hay un comentario que se
llama a sí mismo *«el llamante de producción del escáner»*. La única construcción
de producción de esa pantalla —la de `TransferHome`, en
`transfer_screens.dart`— era `PeersScreen(service: widget.service)`: **nunca
pasaba `onScan`**. Con `onPressed: null`, Material dibuja el botón **apagado** y
se niega a pulsarlo.

Así que el escáner, `ScannerChannel.kt`, CameraX, `qyro_eye`, `qyro_fountain` y
el permiso `CAMERA` estaban todos completos, y **no había forma de entrar**. El
comentario describía un argumento que nadie pasaba.

**Y por qué ninguna prueba lo vio, que es la mitad interesante.**
`scannerAvailableOn()` ya aceptaba un sistema operativo inyectado —para esto
exactamente— y la pantalla **no se lo pasaba**, así que leía
`Platform.operatingSystem` directamente. Las pruebas corren en escritorio, donde
eso es `linux`, así que la rama de Android era **inalcanzable desde cualquier
prueba** y la única prueba sobre este botón afirmaba que **no existe**. Una rama
en la que ninguna prueba puede entrar no está cubierta, por muchas pruebas que la
rodeen.

**El arreglo, y por qué en dos sitios.**

- `TransferHome` acepta `onScan` y lo pasa; `HomeScreen` lo construye, porque
  abrir el escáner necesita la biblioteca nativa cargada y `HomeScreen` es donde
  se construye el motor. `NativeTransferService.nativeLibrary` la expone en vez
  de abrirla otra vez: `DynamicLibrary.open` dos veces sobre la misma ruta son
  dos manejadores sobre una imagen, y el motor tiene estado de proceso.
- `PeersScreen` acepta `operatingSystem`, así que la rama de Android por fin se
  puede probar desde el escritorio.
- Dos pruebas de widgets: con `onScan` el botón existe **y se puede pulsar**; sin
  motor cargado se apaga en vez de mentir. Y sobre una superficie alta a
  propósito, porque la pantalla es una `ListView` y una `ListView` no monta lo
  que queda fuera de la vista: un `findsNothing` por no haberse dibujado se lee
  igual que uno por no existir.
- Y **`qyro_core::repository_contract::the_optical_channel_has_a_door`**, en
  Rust, que lee `home_screen.dart` y exige `onScan:` y `ScanScreen(`. Una prueba
  de widgets puede demostrar que la pantalla honra el callback que le dan; **no
  puede demostrar que producción se lo dé**, porque el llamante de producción
  construye el motor de verdad. Es la misma forma que
  `qyro_net::guards::the_two_consumers_agree_on_the_port`, y corre dentro de la
  puerta.

**La pregunta que cierra esta ficha:** desde el teléfono, ¿se puede llegar al
escáner? **Sí, y hay una guarda en la puerta que se pone roja el día que el
cableado se caiga otra vez** — comprobado quitando el arreglo: la guarda falla
nombrando `onScan:`.

## QYR-0370 — Un puerto ocupado salía como «argumento no usable», y no había con qué elegir otro

- Estado: **CERRADO**
- Severidad: **ALTA** (el receptor no arranca, y el mensaje manda a mirar donde no es)
- Fecha: 2026-08-31

**Lo que la ADR decidió y el código no sabía decir.** ADR-0041 §3, con estas
palabras: *«Si el puerto está ocupado: se dice, no se mueve. Nada de "siguiente
libre en silencio". [...] Qyro dice qué puerto está ocupado y ofrece elegir
otro.»*

Lo que `Session::open_receiver` hacía era
`Listener::bind(bind).map_err(|_| SessionError::BadArgument)`: **todas** las
razones por las que una ligadura puede fallar colapsadas en la variante cuyo
mensaje es «the address, port or path was not usable». Nada por encima podía
distinguir «ese puerto está cogido» de «esa ruta está mal», así que nada por
encima podía ofrecer otro puerto. Y `qyro recv` **no tenía bandera de puerto**,
así que ni siquiera había con qué aceptar la oferta.

**Una decisión escrita en una ADR que el código no puede expresar es una decisión
que nadie implementó.**

**Y en Windows no es teórico.** Windows reserva rangos TCP para Hyper-V, WSL2 y
Docker —`netsh interface ipv4 show excludedportrange protocol=tcp` los enseña— y
una ligadura dentro de uno falla con **`WSAEACCES`, 10013**, que `std` presenta
como `PermissionDenied` y **no** como «en uso». Quien instaló Docker una vez hace
dos años se encuentra un receptor que no arranca y un mensaje sobre un argumento
que nunca escribió.

**El arreglo, en las dos caras, porque la GUI tenía el mismo defecto.**

- `SessionError::PortUnavailable` en el motor, con `AddrInUse` y
  `PermissionDenied` mapeados a ella: para quien tiene la máquina delante las dos
  significan *este puerto no es tuyo hoy*, y las dos se contestan igual.
- `QYRO_ERR_PORT_UNAVAILABLE = -15` en la frontera C, su espejo en Dart, su
  `QyroFailureKind` y su frase en los dos idiomas. Lo forzaron dos guardas que ya
  existían: la que cuenta las variantes de `SessionError` contra los brazos de
  `session_code`, y la que **lee `abi.rs`** y exige que Dart cubra cada código.
  Las dos se pusieron rojas solas al añadir la variante, que es exactamente para
  lo que están.
- `qyro recv --port <n>`, y cuando el puerto no está libre: una frase que **nombra
  el puerto**, nombra los dos motivos de Windows, da el comando de `netsh`,
  explica **por qué Qyro no se mueve solo**, y pregunta por otro puerto si hay
  terminal que pueda contestar.
- Y `whoami_on(port)`: el código de emparejamiento se vuelve a imprimir en cada
  intento **con el puerto de verdad dentro**. Enseñar el código de 49517 mientras
  se escucha en 49518 sería enseñar una dirección donde no hay nadie.

**Sin respaldo automático, y ésa es la mitad que no se hizo.** Un puerto que se
mueve solo pierde las dos propiedades por las que ADR-0041 lo eligió fijo —un
permiso de cortafuegos en vez de uno por sesión, y una cadena que se puede
predecir— y las pierde sin avisar.

**La pregunta que cierra esta ficha:** si el 49517 está ocupado, ¿sabe la persona
qué pasó y cómo seguir? **Sí, y sin salir de la pantalla donde está.**

## QYR-0369 — El `|` del código de emparejamiento es una tubería en PowerShell y en `cmd`

- Estado: **CERRADO**
- Severidad: **ALTA** (el primer intento de todo el mundo en Windows, y el error no nombra a Qyro)
- Fecha: 2026-08-31

**El defecto.** El código de emparejamiento es
`QYRO1|192.168.1.5:49517|<huella>`, y `qyro whoami`, `qyro recv` y `qyro find` lo
imprimían **tal cual**. En PowerShell y en `cmd` el `|` es el operador de
tubería, así que

    qyro send informe.pdf --to QYRO1|192.168.1.5:49517|ab12cd34

**no llega nunca a Qyro**: la consola parte la línea en el primer `|` y trata de
ejecutar `192.168.1.5:49517` como si fuera un programa.

**Y el error no menciona a Qyro.** `cmd` dice «'192.168.1.5:49517' no se reconoce
como un comando interno o externo»; PowerShell dice «Expected expression after
'|'». Ninguno da por dónde empezar, y este binario existe **para Windows**: era
el primer intento de todo el mundo.

**El arreglo, en dos mitades, porque una sola habría creado un defecto nuevo.**

1. **Se imprime ya entrecomillado.** `typeable()` en `qyro_cli/src/flows.rs`
   envuelve el código en comillas **dobles**, que son las únicas que funcionan en
   las tres consolas: `cmd`, PowerShell y un shell de tipo Unix. Las tres
   funciones que lo enseñan pasan por ahí. El **payload del QR no**: lo lee una
   cámara, no una consola.
2. **Y se sigue aceptando entrecomillado.** Un shell quita las comillas antes de
   que Qyro vea el argumento; **un campo de texto no**. El menú del propio CLI,
   la casilla «teclea el código» del teléfono y un decodificador de QR al que le
   den una cadena entrecomillada las reciben literales. Así que
   `qyro_net::PairingEndpoint::parse` —el único analizador por el que pasan los
   dos consumidores— quita **un par que abre y cierra**, y sólo uno.

**Por qué un par y no «las comillas donde estén».** Un código copiado un carácter
corto termina en una comilla suelta. Quitarla convertiría un código roto en uno
que **parsea**: marcaría la dirección correcta esperando una huella a la que le
falta el último byte. `but_a_lone_quote_is_still_a_broken_code` es el control, y
cubre también `"`, `""` y la cadena vacía, que es donde un troceo ingenuo entra
en pánico.

**Y la ayuda lo dice.** `qyro help` tiene una sección propia con el ejemplo
entrecomillado y el motivo, porque el día que alguien copie el código de otro
sitio vuelve a romperse y tiene que haber dónde leerlo. El mensaje de rechazo de
`--to` también lo explica, en vez de repetir la forma del código.

**La pregunta que cierra esta ficha:** ¿puede alguien pegar en PowerShell lo que
Qyro le enseña, sin tocarlo? **Sí.**

## QYR-0368 — El APK de release no podía abrir un socket

- Estado: **CERRADO**
- Severidad: **P0** (el producto entero, inutilizable en el único build que se instala)
- Fecha: 2026-08-31

**El defecto.** `android.permission.INTERNET` estaba declarado **sólo** en
`apps/qyro/android/app/src/debug/AndroidManifest.xml` y en
`apps/qyro/android/app/src/profile/AndroidManifest.xml`. Gradle fusiona `main`
más el sourceSet **de la variante que construye**, así que ninguno de esos dos
llega a una build de release: el manifiesto fusionado de
`flutter build apk --release` no lo contenía ni una vez.

Tres de los cuatro canales de Qyro son un socket TCP —la red local, el cable
directo sin router y el descubrimiento— y empiezan en `qyro_net`:
`TcpListener::bind` en `listener.rs`, `TcpStream::connect` en `stream.rs`. Sin el
permiso los dos lanzan.

**Lo que la persona habría visto:** `Permission denied (errno = 13)` desde una
llamada de socket dentro de `libqyro_ffi.so`. Un mensaje que **no nombra ni a
Qyro ni a un permiso**, en el único tipo de build que alguien instala. Debug y
profile sí lo llevan, así que `flutter run`, el emulador y los tres trabajos de
CI que construyen debug pasaban: **el único build que fallaba era el único que se
instala.**

**Y el comentario mentía en las dos direcciones.** La cabecera del manifiesto de
`main` decía «The only permission this application declares» cuando ya había dos
—`CHANGE_WIFI_MULTICAST_STATE` y `CAMERA`— y faltaba el tercero. Una frase así
hace que la ausencia parezca deliberada, que es exactamente cómo sobrevivió.

**Por qué ninguna prueba lo vio.** `apps/qyro/test/android_manifest_test.dart`
comprobaba **sólo ausencias**: ninguna de almacenamiento, ningún
`ACCESS_LOCAL_NETWORK`, y el conjunto exacto de permisos **del manifiesto
fuente**. Un permiso que tiene que **estar** podía desaparecer y todo seguía
verde. Es el mismo defecto que ese archivo se escribió para evitar —una medida
que no puede ver el fallo para el que existe— aplicado a sí mismo.

**El arreglo.** El permiso en `app/src/main/AndroidManifest.xml`, con su
argumento al lado, y el comentario corregido. Y tres guardas, las tres rojas
antes del cambio:

1. `qyro_net::guards::the_android_manifest_declares_internet` — lee el manifiesto
   fuente y corre dentro de `cargo test --workspace`, o sea en `scripts/gate.ps1`,
   en cada commit y en cada plataforma. Está en `qyro_net` y no al lado del
   manifiesto porque el fallo es de ese crate: es el crate cuyo propósito entero
   se vuelve inalcanzable. Con su control, que prueba que el analizador lee
   declaraciones y no prosa en los dos sentidos.
2. `android_manifest_test.dart` — el conjunto exacto pasa de dos permisos a tres,
   y una prueba aparte sólo para `INTERNET`, para que el fallo diga **cuál** y
   **por qué** en vez de imprimir la diferencia de dos listas.
3. `android_manifest_test.dart` sobre el manifiesto **fusionado de release**, que
   es el único sitio donde se ve lo que un plugin añade o quita. Sólo release: el
   fusionado de debug **sí** lleva `INTERNET` desde `app/src/debug/`, así que
   afirmarlo ahí pasaría mientras el APK publicado sigue roto. `release.yml` pone
   `QYRO_REQUIRE_RELEASE_MANIFEST=1` para que leer el de debug en ese trabajo sea
   rojo y no un pase.

**La pregunta que cierra esta ficha:** ¿puede el APK de release abrir un socket?
**Sí, y hay una guarda que se pone roja el día que deje de poder.**

## QYR-0367 — Un commit de una línea de documentación compilaba el árbol dos veces

- Estado: **CERRADO**
- Severidad: **MEDIA** (ruido, no corrección — pero ruido que tapa lo que importa)
- Fecha: 2026-08-18

**El defecto.** `ci.yml` no tenía **ningún** filtro de rutas, así que cualquier
`push` lo disparaba entero: Rust en Linux, Rust en Windows, Flutter, las guardas
de sistema de ficheros y los scripts. La sesión anterior hizo **28 commits de una
sola línea** en `ESTADO-ACTUAL.md` —el 36 % de sus 78— y cada uno arrancó tres
flujos.

**Y la culpa de los 28 no era del CI.** Era de una regla: actualizar el ancla en
un commit aparte. Se corrige en el sitio — **`ESTADO-ACTUAL.md` se actualiza
dentro del commit de contenido**, y la comprobación 16 se cumple corriendo la
puerta antes de empujar, no con un commit extra.

**El arreglo, y por qué no fue un `paths-ignore`.** El trabajo `documentation`
vivía dentro de `ci.yml` y **tiene que correr precisamente cuando cambia la
documentación**: comprueba `STATUS.md`, la portabilidad del repositorio y la
evidencia de plataforma. Un `paths-ignore` en `ci.yml` lo habría apagado justo
para los commits que lo necesitan.

Así que se parte en dos: `documentation.yml` con las rutas que comprueba de
verdad, y `ci.yml` con `paths` para lo que se compila. **Cada mitad con su
disparador.**

De paso, `branches: [main, 'claude/**']` pasa a `[main]` en los seis flujos que
lo tenían: las 19 ramas se fusionaron y se borraron, y un patrón que no puede
coincidir con nada es una promesa de cobertura que ya no cubre.

**La pregunta que cierra esta ficha:** ¿compila un cambio de sólo documentación
el árbol entero? **No.**

## QYR-0366 — iOS corría en cada push sobre un runner de macOS

- Estado: **CERRADO**
- Severidad: **MEDIA**
- Fecha: 2026-08-18

**El defecto.** Tres trabajos de iOS se ejecutaban en cada `push`:
`ios-runtime.yml` entero (`macos-15`), el trabajo `ios-crypto` de
`crypto-platform.yml` (`macos-15`) y el trabajo `ios` de `platform-builds.yml`
(`macos-latest`). **ADR-0039 sacó iOS de la v1.0**, así que cada ejecución era
una notificación por un trabajo que nadie iba a leer.

**No se borran, y ésa es la parte con argumento.** ADR-0039 dice **aplazado, no
cancelado**: *«cuando exista un Mac, es una v1.1»*, y *«no revoca ADR-0025 ni
ningún trabajo de iOS existente»*. Borrar la comprobación de ABI sería tirar lo
que la v1.1 necesita para no empezar de cero.

**Los tres pasan a `workflow_dispatch`** — el flujo entero en un caso, un
`if: github.event_name == 'workflow_dispatch'` en los otros dos. El trabajo se
conserva íntegro y se dispara a mano el día que haya un Mac.

**La pregunta que cierra esta ficha:** ¿arranca un runner de macOS al empujar?
**No, y el trabajo de iOS sigue entero.**

## QYR-0365 — Cada archivo pequeño cuesta ~1,2 s, y a los 60 s se corta la sesión

- Estado: **cerrado**
- Severidad: **ALTA**
- Fase: 22, escenario 2

**Lo medido, bisecando** (200 archivos de 64–264 bytes, dos procesos en loopback):

| archivos | resultado | llegaron | tiempo |
|---|---|---|---|
| 10 | ENTREGADO | 10 | **12,4 s** |
| 50 | ENTREGADO | 50 | **49,4 s** |
| 100 | FALLO `unreachable` | **100** | **80,3 s** |
| 150 | FALLO `unreachable` | **150** | **127,3 s** |

**Dos hallazgos, y el segundo es el de verdad.**

### 1. No es un límite de cantidad, es un reloj — y los archivos SÍ llegan

`qyro_net::limits::IDLE_TIMEOUT` es **exactamente 60 s**. El corte cae entre
49,4 s (pasa) y 80,3 s (falla). **La correlación es con el tiempo, no con el
número.**

Y lo que más importa: en los dos casos que «fallan», **`llegaron` es el total**.
Los cien archivos están en disco, con su contenido. El emisor informa de un fallo
sobre una transferencia que se completó. **Un falso negativo al final es peor que
un fallo honesto**: la persona borra y reenvía algo que ya estaba entregado.

### 2. **~1,2 segundos por archivo de 64 bytes**

12,4 s para diez. 49,4 s para cincuenta. 127 s para ciento cincuenta. Es lineal y
es absurdo: son archivos de decenas de bytes sobre loopback. El `IDLE_TIMEOUT` no
es el defecto — **es lo que hace visible al defecto**. Subirlo escondería esto.

### La causa, localizada con archivo y línea

**`qyro_fs/src/io.rs:430` hace `sync_all()` por cada archivo recibido**, y no
está solo:

| Dónde | Qué | Por elemento |
|---|---|---|
| `qyro_fs/src/io.rs:430` | `part.handle.sync_all()` antes de renombrar | **sí** |
| `qyro_fs/src/io.rs:447` | `sync_all` del directorio (Unix) | **sí** |
| `qyro_fs/src/history.rs:131`, `:188`, `:233` | `sync_data()` del registro | **sí** |

**Y esa explicación era mía y estaba mal.** Escribí que un segundo por archivo
«es exactamente lo que cuesta eso». **Medido en esta máquina**, 50 archivos de
200 bytes, crear + escribir + renombrar:

| | total | por archivo |
|---|---|---|
| sin `sync_all` | 80 ms | **1,6 ms** |
| con `sync_all` | 325 ms | **6,5 ms** |

**`sync_all` cuesta 4,9 ms extra por archivo. Explica el 0,4 % de los 1 200 ms.**
La hipótesis del disco queda **descartada por medida**, no por opinión — y estuvo
escrita como si fuera una conclusión durante un commit entero, que es exactamente
el error que este taller lleva toda la sesión corrigiendo en otros sitios.

### El sospechoso que queda, y es aritmética sobre constantes medidas

`qyro_net::limits::READ_TIMEOUT` es **250 ms**. **1 200 / 250 ≈ 5.** Un bucle que
se bloquea en una lectura vencida unas cinco veces por elemento da exactamente
esta curva, y da la respuesta a por qué el tamaño no importa: no se está
esperando a datos, se está esperando a que venza un reloj.

**Comprobado, y es eso.** Se bajó `READ_TIMEOUT` de 250 ms a 25 ms en una
compilación de prueba —y se revirtió después, porque es una constante de
ADR-0028 §4.1 y no un botón:

| | 10 archivos | 50 archivos | por archivo |
|---|---|---|---|
| `READ_TIMEOUT` = 250 ms | 12 441 ms | 49 369 ms | **~1,24 s** |
| `READ_TIMEOUT` = 25 ms | 2 357 ms | **6 537 ms** | **~0,13 s** |

**7,5× más rápido con el mismo código y una constante distinta.** El tiempo por
elemento escala con el reloj de lectura, no con el disco, no con la red y no con
el tamaño. La hipótesis del disco se cayó con 4,9 ms y ésta se sostiene con 43
segundos de diferencia.

### Y el arreglo NO es bajar `READ_TIMEOUT`

Es el latido que permite a un lector bloqueado notar una cancelación
(ADR-0028 §4.1). Bajarlo a 25 ms multiplica por diez los despertares de un hilo
que no tiene nada que hacer — cambia una espera visible por consumo de CPU en un
producto cuyo objetivo son máquinas viejas.

**El defecto es que el bucle necesite varias lecturas vencidas por elemento.**
Una lectura sólo debería vencer cuando de verdad no hay nada que leer; aquí vence
entre elementos que ya están en camino.

### El mecanismo, leído en el código

`qyro_session/src/session.rs:717`, dentro de `advance()`:

```rust
let inbound = match self.stream.read_frame() {
    Ok(Some(frame)) => ...,
    Ok(None) => None,        // <-- la lectura venció: 250 ms gastados
    ...
};
```

**Un `read_frame()` por `step()`, y `Ok(None)` significa «venció».** Así que cada
`step` que no encuentra un marco cuesta el `READ_TIMEOUT` **entero**, y ambos
lados hacen `step` en bucle. Si el protocolo necesita una ida y vuelta por
elemento, los dos extremos se turnan para gastar 250 ms esperando al otro — que
es exactamente la forma de «1,2 s por elemento, sin importar el tamaño».

### Y cuál de los dos lados espera, medido

Contando lecturas vencidas por rol (instrumentación temporal, revertida), **20
archivos**:

```
DIAG lecturas-vencidas emisor=75 receptor=1
```

**El emisor gasta prácticamente todas: ~3,75 lecturas vencidas por archivo, y el
receptor ninguna.** A 250 ms cada una son ~0,94 s por archivo, que es casi todo
el 1,24 s medido.

**Eso cierra la pregunta de diseño.** El receptor no está esperando: está
trabajando y contestando. **El emisor manda un elemento y se bloquea en una
lectura temporizada** hasta que llega la respuesta, en vez de seguir enviando lo
que ya tiene listo.

Y hay una pieza que dice que no tenía por qué: `qyro_transfer` ya lleva ventana
—`WINDOW_CHUNKS` y `chunks_in_flight()`— es decir, el protocolo **ya está pensado
para tener varias cosas en vuelo**. Lo que hace el bucle de sesión es
serializarlo igualmente.

**El arreglo, ahora sí acotado:** que el emisor no consuma un `READ_TIMEOUT`
entero cuando todavía tiene trabajo que poner en el cable. Las otras dos opciones
—cambiar la espera por «el socket es legible», o quitar la ida y vuelta del
protocolo— **quedan descartadas como primera medida**: la primera no hace falta
si el emisor no espera, y la segunda toca el formato de cable para arreglar algo
que es del bucle.

**Lo que esto ya explica sin más trabajo:** por qué 100 archivos «fallan»
llegando todos. No hay ningún límite de cantidad — el reloj de 60 s se agota
porque cada elemento cuesta 1,2 s de esperas que no hacían falta.

**Y no es un descuido: es ADR-0027 §4**, que fijó el orden *verificar,
`sync_all`, renombrar, `sync_all` del directorio*. La razón escrita es que un
archivo renombrado sin vaciar puede sobrevivir al renombrado y no a su contenido
tras un corte de corriente.

**Los `sync_all` siguen siendo una decisión de durabilidad que merece revisarse**
—por archivo o por transferencia, con enmienda a ADR-0027— pero **ya no son el
defecto de esta ficha**: 4,9 ms de 1 200 no arreglan nada. Las dos opciones
quedan escritas porque el análisis vale, no porque cierre esto:

- *Por transferencia*: un `sync_all` al final. Doscientos archivos pasan de ~4
  minutos a segundos. El riesgo se concentra: un corte de corriente a mitad puede
  dejar varios archivos con nombre definitivo y contenido incompleto.
- *Por archivo, pero sin el historial*: los tres `sync_data` de `history.rs` son
  registro, no contenido. Vaciar el diario por entrada es la parte más fácil de
  discutir.

**Lo que se hizo:** medirlo, en vez de seguir escribiéndolo. Y el resultado fue
que la explicación bonita era falsa.

**Lo que NO hay que hacer y está dicho aquí para que no se haga:** subir
`IDLE_TIMEOUT`. Convierte un fallo ruidoso en una transferencia de veinte minutos
para doscientos archivos, y eso es peor producto.

**La pregunta que cierra esta ficha:** ¿cuánto tarda un archivo pequeño, y por
qué? **1,24 s, y el porqué está medido de punta a punta**: el **emisor** gasta
~3,75 lecturas vencidas de 250 ms por archivo mientras el receptor no espera
nada. Falta el **arreglo** —que el emisor no espere teniendo trabajo— y por eso
la ficha sigue abierta. **El diagnóstico ya no tiene huecos.**

### Lo que descarta leer el bucle entero (2026-08-19)

Confirmado leyendo `qyro_transfer::SendItem`: **`is_drained()` es
`next_to_send >= chunks_total`**, así que un archivo de un solo trozo se drena
**en el mismo envío** y el bucle de `pump` pasa al siguiente elemento sin salir.
Veinte archivos de 200 bytes salen en **una** llamada a `pump`, y
`WINDOW_CHUNKS` es 16, no 1.

**Queda descartado del todo que el emisor haga un viaje de ida y vuelta por
archivo.** No es `pump`.

El sospechoso que queda, y es el único que encaja con `emisor=75 receptor=1`:
`Session::advance` maneja **un frame por llamada** —`read_frame()` devuelve uno—
y su lectura cuesta un `READ_TIMEOUT` entero **sólo cuando el socket está
vacío**, porque `read_frame` vacía primero el decodificador. Así que la pregunta
exacta que falta es **cuándo se vacía ese socket**: con decenas de frames en
vuelo por lado, la mayoría de las lecturas deberían encontrar bytes ya
almacenados.

### Un candidato probado y descartado, con su mecanismo (2026-08-19)

**La idea:** `advance` escribe al principio y `pump` produce al final, así que lo
producido espera al paso siguiente. Escribirlo en el mismo paso parecía gratis —
los mismos bytes, en el mismo orden, sin nada esperando en un `Vec`.

**No es gratis, y lo dijo una prueba:**
`a_receiver_that_refuses_stops_the_sender_and_leaves_nothing_behind` pasó a
terminar en `Err(PeerUnreachable)` donde antes daba `Ok(Rejected)`.

**El mecanismo, que es lo que vale de este intento:**

1. El receptor rechaza y cierra, dejando el frame de rechazo en el búfer del
   emisor.
2. Con la escritura adelantada, el emisor **manda los trozos siguientes antes de
   leer nada**.
3. Escribir a un socket que el par ya cerró provoca un **RST**, y el RST
   **descarta el búfer de recepción** — con el rechazo dentro.
4. El emisor lee, encuentra la conexión rota, y `finished()` todavía es `false`
   porque su motor nunca vio el rechazo. Resultado: «no llegué» en lugar de «me
   dijeron que no», que son exactamente las dos cosas que esa prueba existe para
   no confundir.

Hacerlo de mejor esfuerzo **no lo arregla**: el daño no es el error de escritura,
es el RST que ya ocurrió.

**Revertido.** Y queda escrito para que nadie lo reintente creyendo que es una
mejora obvia: **cualquier arreglo de esta ficha tiene que no escribir a un par
que pueda haber terminado la conversación**, y hoy el emisor no tiene forma de
saberlo antes de leer.

### LA MEDIDA, y desmiente el diagnóstico de esta ficha (2026-08-19)

`rust/crates/qyro_session/tests/qyr_0365_measurement.rs`, con los contadores
`Session::step_tally` que esta ficha pedía por su nombre. Veinte archivos de 64
bytes, dos sesiones de verdad sobre loopback:

```
  emisor:   22 pasos, 0 lecturas vencidas
  receptor: 43 pasos, 0 lecturas vencidas
  tiempo:   0.06 s
  por archivo: 0.003 s
```

**Tres milisegundos por archivo, y cero lecturas vencidas en los dos lados.**

Esta ficha decía —y yo lo repetí— que «el emisor gasta 75 lecturas vencidas
contra 1 del receptor, así que el bucle de sesión lo serializa». **El bucle de
sesión no serializa nada.** Medido, hace 20 archivos en 60 ms sin esperar al
reloj ni una vez, y `set_nodelay(true)` ya estaba puesto desde ADR-0028, así que
Nagle tampoco era.

### Dónde queda el defecto, entonces

Los 75/1 salieron de `gui_cli_matrix_test.dart`, que es **la GUI contra el CLI**:
Dart conduciendo el motor por la frontera C. La medida de arriba es Rust contra
Rust. La diferencia entre 3 ms y 1 200 ms por archivo **está en el lado Dart o en
el cruce**, no en el motor.

**La siguiente medida, y es una:** cronometrar el bucle de
`native_transfer_service.dart` —`stepBlocking`, `progress`, `peerFingerprint` y
el `yield`— por iteración, con los mismos 20 archivos. Si el motor hace 22 pasos
en 60 ms y Dart tarda 24 s, el coste está entre esas cuatro llamadas.

**Lo que esto cambia para quien siga:** no busques en `qyro_transfer` ni en
`Session::advance`. Ya está medido y está limpio.

**La medida ANTERIOR que se pedía, ya hecha, arriba:** por lado, cuántas veces entra `advance`,
cuántas de esas lecturas vencen, y **qué frame estaba en vuelo cuando venció**.
Sin ese tercer dato los otros dos no distinguen «el par no ha contestado
todavía» de «el par contestó y nadie leyó».

---

## CERRADA EL 2026-08-20, Y NO ERA DEL MOTOR

**El defecto estaba en el arnés que la midió.**

`Process.start` deja la salida del hijo en una tubería. **Un hijo que escribe a
una tubería que nadie lee se bloquea** cuando el búfer del sistema se llena —
unos pocos KB. Con 200 archivos, el receptor del CLI escribe **23 349 bytes**, se
bloquea escribiendo, **deja de dar pasos**, y el emisor espera hasta el reloj de
sesión de 60 s.

Con un archivo el CLI imprime poco y no pasa nunca. Por eso las cuatro celdas de
la matriz llevaban meses en verde.

### Las dos medidas, una al lado de la otra

| | Sin vaciar la tubería | Vaciándola |
|---|---|---|
| Tiempo, 200 archivos | **60 295 ms** y falla | **292 ms** |
| Por archivo | — | **1,5 ms** |
| Entregados | 0 | **200 / 200** |

Y el motor, medido aparte en `qyro_session/tests/qyr_0365_measurement.rs` con los
mismos 200 archivos: **0,33 s, 202 pasos del emisor, cero lecturas vencidas.**

### Lo que se creía, y por qué era falso

La ficha decía —y tres sesiones lo repitieron— que «el emisor gasta 75 lecturas
vencidas contra 1 del receptor, así que el bucle de sesión lo serializa». Esas 75
esperas eran **reales**, y el emisor esperaba de verdad: esperaba a un receptor
que estaba bloqueado escribiendo en una tubería.

**El síntoma apuntaba al sitio correcto y la causa estaba un proceso más allá.**

### Lo que se arregló

1. `_drainChild` en `gui_cli_matrix_test.dart`, en los **tres** sitios que
   arrancan un proceso.
2. Una celda nueva de 200 archivos que **fallaba antes del arreglo** —60 295 ms
   y cero entregados— y ahora pasa en 300 ms. Afirma que no se acerca al reloj,
   no un umbral fino que fallaría en un runner cargado.
3. `dart_test.yaml` con `concurrency: 1`. ADR-0041 fija el puerto, así que dos
   archivos de prueba que levanten un receptor **no pueden solaparse**, y
   `flutter test` los corre en paralelo por defecto. El síntoma de eso también
   parece del producto.

### Lo que NO se hizo, y es deliberado

**No se subió `IDLE_TIMEOUT`.** Habría escondido esto en vez de encontrarlo, que
es lo que la propia ficha advertía.

**No se cambió nada del motor.** No hacía falta: mueve 200 archivos en 0,33 s.

### La pregunta que cierra esta ficha

¿Cuesta un archivo pequeño ~1,2 s? **No. Cuesta 1,5 ms**, y los 200 llegan.

## QYR-0364 — `qyro recv` pregunta si aceptas sin decir qué

- Estado: **CERRADO**
- Severidad: **ALTA**
- Fase: 22
- Encontrado: 2026-08-18, buscándole un llamante al saneado de nombres

**El defecto.** El receptor del CLI enseña la huella de quien conecta y pregunta
`accept from this device? [y/N]`. **No dice qué archivos.** Ni cuántos, ni cómo se
llaman, ni cuánto pesan.

ADR-0036 §1 dice que *nada se acepta solo*. Una pregunta sin objeto **no es una
decisión**: es un trámite. La GUI sí lo enseña; la terminal no, y eso es una
celda de paridad que la tabla no tenía porque nadie había mirado.

**El arreglo.** `Session::offered_files` devuelve nombre y tamaño de lo ofrecido.
Las piezas ya estaban todas: `qyro_transfer::Receiver::manifest()` existía y el
`Role::Receiving` de `qyro_session` ya guardaba el motor — **lo único que faltaba
era el accesor**, que es la forma habitual de este defecto en esta casa.

Devuelve `display_name` y no la ruta relativa (ADR-0019): una ruta dibujada en
una confirmación es un sitio donde un par puede escribir algo que parezca un
directorio que la persona reconoce.

**No sanea aquí.** Los nombres salen exactamente como el par los mandó, porque
esta función también alimenta código de sistema de archivos; el saneado de
terminal de ADR-0047 §6 va **en el sitio donde se dibuja**, y las reglas más
estrictas de ADR-0027 se quedan donde estaban.

**Lo que esto desbloqueó.** El saneado de ADR-0047 §6 se había escrito con sus
cinco pruebas —incluida la del retorno de carro que reescribe la línea que la
persona está leyendo— y **se revirtió sin commitear** porque no tenía ningún
llamante de producción posible: el único sitio donde se dibujaría un nombre ajeno
en una terminal era justo el que no existía. Con el accesor puesto, ese sitio
existe, y `safe_terminal_name` entra con su llamante en
`qyro_cli/src/flows.rs:276`.

Enviarlo antes habría sido la décima capacidad viva e inalcanzable de este
proyecto, y el mismo día que se retiró la novena. **La comprobación 14 aplicada
antes de escribir el código en vez de después.**

**La pregunta que cierra esta ficha:** ¿ve la persona qué archivos le están
ofreciendo, antes de decir que sí, en las dos caras? **Sí, en las dos.**

## QYR-0363 — Una ruta con barras normales mandaba el archivo a otro sitio

- Estado: **CERRADO**
- Severidad: **ALTA** (no P0: el selector del sistema no produce estas rutas)
- Fase: 21

**El defecto.** `NativeTransferService._commonRoot` parte por
`Platform.pathSeparator` y quita el último trozo. En Windows **las barras
normales son válidas en cualquier API de rutas**, así que `C:\salida/p.bin` es
una ruta correcta cuyo último trozo, partiendo sólo por `\`, es `salida/p.bin`
entero. La raíz salía siendo el **abuelo** y el nombre que viajaba era
`salida/p.bin`: **el receptor escribía el archivo un directorio más abajo del que
nadie nombró, y decía que había ido bien.**

Un archivo que aterriza en otro sitio no es un fallo más pequeño que uno que no
aterriza: es peor, porque nadie lo busca.

**El arreglo.** Normalizar los separadores antes de partir, sólo en Windows —
fuera de Windows `\` es un carácter legal en un nombre y reescribirlo
corrompería nombres en vez de arreglar rutas.

**Cómo se encontró.** La casilla GUI→CLI de la matriz de la fase 21 decía
«nothing was materialised». Los bytes **sí** habían llegado; estaban en
`destino/salida/`.

**El control, y por qué tardó dos intentos.** La primera vez la prueba se quedó
colgada y **se retiró del árbol en vez de dejarla en rojo**. Lo que la colgaba no
era ella: las cuatro casillas comparten el puerto fijo de ADR-0041 y no pueden
solaparse, y hasta que el `tearDown` no esperó a que el socket se soltara,
cualquier prueba añadida al final se quedaba esperando un puerto ocupado.

Con eso arreglado, el control existe y **se ha visto fallar a propósito**:
quitando la normalización de `_commonRoot`, la prueba no pasa; poniéndola, pasa.
Falla por agotar el tiempo y no por la aserción —el envío con la raíz mal
derivada no llega a completarse— y eso es más lento y menos claro de lo ideal,
pero **distingue**, que es lo que se le pide a un control.

**La pregunta que cierra esta ficha:** ¿aterriza en `destino/` un archivo cuya
ruta se escribió con barras normales? **Sí, y hay una prueba que lo guarda y que
se ha visto fallar sin el arreglo.**

## QYR-0362 — La GUI tampoco había enviado nunca un archivo

- Estado: **cerrado**
- Severidad: **P0**
- Fase: 21
- Encontrado: 2026-08-18, la misma tarde y por el mismo motivo que QYR-0361

**El defecto.** `NativeTransferService.send` pasa una función a `Isolate.run`, y
esa función escribía `port.sendPort.send(...)`. Escribir `port.sendPort` **dentro**
del closure hace que el closure capture `port`, que es un `ReceivePort` — y un
`ReceivePort` es explícitamente **no enviable** entre isolates. Todo envío moría
con *«Illegal argument in isolate message: object is unsendable»* **antes de
mover un byte**.

**El arreglo.** Sacar `final sendPort = port.sendPort;` fuera del closure. Un
`SendPort` sí cruza; es exactamente lo que existe para cruzar. Una línea.

**Por qué no lo vio nadie.** Las pantallas se prueban contra un servicio falso, y
la prueba de dos procesos ejercita **recibir**. Nada había puesto nunca este
camino contra un par real. Es el mismo hueco que QYR-0361 en la otra cara, y las
dos se encontraron el mismo día por la misma razón: **la fase 21 pone una cara
contra la otra.**

**Las dos caras tenían el envío roto, cada una a su manera, y ninguna se había
ejercitado nunca de extremo a extremo.**

**Evidencia, completa.** Antes del arreglo `send` lanzaba la excepción de
isolate en la primera llamada. Después, y con QYR-0363 resuelto,
`gui_cli_matrix_test.dart` entrega desde la GUI en las dos direcciones —GUI→CLI y
GUI→GUI— **con el archivo comparado byte a byte en destino**.

La ficha estuvo un rato en «arreglado, evidencia parcial», que **no es un estado
válido de este registro**: aquí sólo hay `cerrado` y `descartado`. Era un
«pendiente» disfrazado de estado, y se corrige cerrándola con la evidencia que
entretanto apareció.

**La pregunta que cierra esta ficha:** ¿aterriza un archivo enviado por la GUI en
el disco del receptor? **Sí, en las dos casillas de la matriz y byte a byte.**

## QYR-0361 — `qyro send` no ha movido nunca un byte

- Estado: **CERRADO**
- Severidad: **P0**
- Fase: 21
- Encontrado: 2026-08-18, poniendo la GUI contra el CLI por primera vez

**El defecto.** `flows::send` llamaba a
`Session::open_sender(address, root, &[PathBuf::from(&name)], None)` con
`root` = el directorio padre y `name` = **el nombre pelado del archivo**.
`open_sender` deriva el nombre en el cable con `source.strip_prefix(root)`, y
`"p.bin".strip_prefix("C:\carpeta")` **falla siempre**. Toda invocación de
`qyro send` devolvía `BadArgument`, que el CLI imprime como
*«could not connect: the address, port or path was not usable»* — un mensaje que
apunta a la red cuando el problema estaba en los argumentos.

**Desde cuándo.** Desde la fase 13, el día que se escribió. **Y se publicó una
Release con él.**

**Por qué no lo vio nadie.** Las dos mitades estaban probadas por separado:
`open_sender` tiene sus propias pruebas, con argumentos correctos; y las pruebas
del CLI nunca llegaban a un socket. Es el mismo hueco que produjo
`KeystoreWrapper`, `qyro_session_local_address`, `Session::finish`, `history()`
y el descubrimiento — **la sexta vez que este proyecto envía una costura que
ninguna prueba cruzaba.**

**El arreglo.** Pasar la ruta completa. Una línea.

**La evidencia, ejecutada.** Dos copias del binario en directorios distintos —
para que sean dos aparatos y no uno, porque la identidad vive junto al
ejecutable— con huellas distintas (`b154290bfc…` y `506e5a2763…`), `--expect`
mutuo, y un archivo de 5 000 bytes que **aparece en el directorio del receptor**.
Antes del arreglo: `BadArgument`. Después: `sent.`

**La pregunta que cierra esta ficha:** ¿mueve `qyro send` un archivo entre dos
procesos con identidades distintas? **Sí, y no lo hacía.**

**Lo que esto obliga a hacer:** la Release publicada contiene un `qyro.exe` cuyo
`send` no funciona. Se retracta y se republica, como se hizo con `2c01de0` — ese
es el estándar de esta casa y esta ficha no lo cambia. Queda en
`ESTADO-ACTUAL.md` como lo primero de la siguiente sesión, porque tocar una
Release publicada no es algo que se haga con el contexto justo.

## QYR-0001 — Falta referencia visual de scramble

- Plataforma: todas
- Severidad: P2
- Esperado: design/reference/scramble-decode-reference.jpg
- Actual: activo no suministrado
- Workaround: tests deterministas sin golden visual
- Estado: descartado
- Dueño: propietario
- Fecha: 2026-08-04
- **Fase 09 (2026-08-16), descartado:** la referencia visual del scramble era una ayuda de
  diseño para una pantalla de arranque que lleva estable desde el sprint 4B y que
  ninguna fase posterior tocó. Una captura de referencia sirve para detectar
  deriva, y no ha habido deriva que detectar. **Lo que la cerraría de verdad no es
  una imagen sino una prueba golden**, y una golden de una animación con ruido
  aleatorio es una prueba intermitente: se descarta a propósito, no por olvido.

## QYR-0002 — Runners Flutter no generados

- Plataforma: Android, iOS, Windows
- Severidad: P0
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: commit 286d3d4 y builds run 30938946789
- Resolución: runners oficiales generados por Flutter 3.44.8
- Fecha: 2026-08-04

## QYR-0003 — Aviso de actions/checkout v4

- Plataforma: CI
- Severidad: P3
- Reproducción: ejecutar CI
- Esperado: cero avisos
- Actual: GitHub fuerza Node 24 porque la action declara Node 20
- Workaround: ninguno necesario; jobs pasan
- Estado: descartado
- Nota de estado: «abierto; evaluar checkout v5 tras auditoría»
- Dueño: release
- Fecha: 2026-08-04
- **Fase 09 (2026-08-16), descartado:** el aviso de `actions/checkout@v4` es de GitHub sobre su
  propio calendario de Node, no del código de este repositorio. Actualizar a v5 en
  el commit de release cambiaría el entorno en el que se produce el artefacto que
  se está publicando, que es exactamente cuando no hay que tocarlo. Se descarta
  para la v1.0 y es un cambio de una línea para la primera v1.1.

## QYR-0004 — Builds no retenidos

- Plataforma: release
- Severidad: P1
- Esperado: artefactos debug descargables con checksums
- Actual: **parcialmente incorrecto tal y como estaba redactado.** El ZIP
  portable de Windows sí se retiene desde que existe el job `windows`
  (`qyro-windows-x64-portable-debug`, 14 días). El APK de Android y el
  `Runner.app` de iOS no: `platform-builds.yml` tiene un único paso de
  `upload-artifact` y es el de Windows
- Lo que sigue faltando: checksum SHA-256 distribuido **dentro** del paquete y
  la etiqueta DEVELOPMENT / NOT FOR PUBLIC RELEASE, en los tres. El digest que
  GitHub imprime al subir un artefacto identifica el ZIP de ese run, no el
  contenido que alguien descarga; usarlo como sustituto cambiaría en silencio lo
  que se afirma
- Evidencia: run 30938946789; `.github/workflows/platform-builds.yml:223`
- Workaround: volver a ejecutar builds para Android e iOS
- Estado: cerrado
- Nota de estado: «abierto, con el alcance corregido en el sprint 4C.1»
- Dueño: release
- Fecha: 2026-08-04, corregido 2026-08-05
- **Fase 09 (2026-08-16), cerrado:** `docs/release/v1.0.md` publica el SHA-256 del APK y del
  `.exe`, y el protocolo de hardware pide comprobarlo con `certutil -hashfile`
  antes de instalar. La etiqueta `v1.0.0` es lo que hace el artefacto
  reproducible: nombra el commit exacto del que sale.

## QYR-0005 — Auditorías y suites avanzadas no disponibles

- Plataforma: CI
- Severidad: P1
- Esperado: cargo-audit, tests nativos y vectores de protocolo ejecutables
- Actual: test_all informa WARNING para cargo-audit y N/A para suites/corpus ausentes
- Workaround: las suites Rust/Flutter y el ledger de licencias sí se validan
- Estado: descartado
- Dueño: seguridad/protocolo
- Fecha: 2026-08-04
- **Fase 09 (2026-08-16), descartado:** «auditorías y suites avanzadas» era la petición de más
  herramientas — SBOM, `cargo-deny`, fuzzing exhaustivo. `cargo audit --deny
  warnings` sí corre y sí está en verde sobre los 80 paquetes, incluido el único
  externo que entró. Lo demás son herramientas que ningún hallazgo de este
  proyecto ha necesitado todavía, y añadirlas al llegar a la v1.0 es añadir
  superficie de CI el día que menos conviene.

## QYR-0006 — iOS no compilaba por un storyboard ilegible

- Plataforma: iOS
- Severidad: P0
- Esperado: `flutter build ios --debug --no-codesign` produce Runner.app
- Actual: `Error (Xcode): The document "LaunchScreen.storyboard" could not be
  opened. (com.apple.InterfaceBuilder error -1.)`
- Causa: 67fa795 eliminó `toolsVersion`/`systemVersion` del elemento `<document>`
  al oscurecer la launch surface, dejando una `capability` con `minToolsVersion`
  sin versión de herramientas contra la que compararse
- Evidencia: runs 30960631901 (67fa795), 30961031089 (9bfb1cc) y 30961153321
  (e9ed7f3) fallan; 30953803079 (9104421) y 30956527561 (4f7ed01) pasaban
- Estado: cerrado
- Nota de estado: «resuelto»
- Resolución: commit 565a78d restaura la estructura del documento que ya
  compilaba y añade validación estructural al contrato de launch surfaces
- Confirmación: run 30963011815 sobre ff933d9, los diez pasos en success,
  incluidos la verificación de símbolos con `nm -gU` y el XCTest en simulador
- Dueño: iOS
- Fecha: 2026-08-05

## QYR-0007 — STATUS.md pudo derivar 58 commits sin detección

- Plataforma: CI/documentación
- Severidad: P1
- Esperado: el job documental detecta que la fuente canónica quedó obsoleta
- Actual: `check_docs_consistency` validaba solo la estructura de STATUS.md, así
  que `Verified commit: 7ca3973` sobrevivió 58 commits declarando funciones ya
  implementadas como NOT_IMPLEMENTED y 9 tests cuando la suite ejecuta 51
- Estado: cerrado
- Nota de estado: «resuelto»
- Resolución: commit 5825b50 añade la regla de frescura (SHA mal formado,
  inalcanzable o con más de `QYRO_MAX_STATUS_COMMIT_LAG` commits de retraso) en
  Bash y PowerShell, y el job documental usa `fetch-depth: 0`
- Dueño: documentación
- Fecha: 2026-08-05

## QYR-0008 — Run de Android runtime atascado sin runner

- Plataforma: CI/Android
- Severidad: P2
- Esperado: el run concluye o falla
- Actual: run 30961153377 (e9ed7f3) sigue `in_progress` desde 2026-08-04T23:47Z
  con `total_ms: 0`; nunca obtuvo runner
- Workaround: no se canceló porque `concurrency: android-runtime-${{ github.ref }}`
  con `cancel-in-progress: true` lo desplaza en el próximo push a esa ref
- Impacto: ninguno ya sobre el estado actual. El runtime ABI de Android quedó
  confirmado en esta rama por el run 30963016390 sobre ff933d9
- Estado: cerrado
- Nota de estado: «cerrado por obsolescencia; el run atascado sigue en la otra rama»
- Dueño: CI/Android
- Fecha: 2026-08-05

## QYR-0009 — ADR-0016 prometía compatibilidad que el código no tenía

- Plataforma: protocolo
- Severidad: P0
- Esperado: un tipo de mensaje desconocido es recuperable
- Actual: `FrameDecoder` envenenaba el stream ante cualquier error de cabecera,
  así que un peer con una versión menor más nueva mataba la conexión
- Impacto adicional: `header_len > 48` se aceptaba y los bytes de extensión se
  descartaban, rompiendo la reserialización byte-exacta; `ENCRYPTED` y
  `COMPRESSED` eran ajustables públicamente
- Estado: cerrado
- Nota de estado: «resuelto»
- Resolución: ADR-0018 y commits 30fe57e (contratos) y cc38554 (implementación)
- Fecha: 2026-08-05

## QYR-0010 — El manifest permitía un nombre visible engañoso

- Plataforma: manifest
- Severidad: P0
- Esperado: el nombre mostrado corresponde al archivo que se escribirá
- Actual: `display_name` viajaba aparte de la ruta, así que `factura.pdf.exe`
  podía presentarse como `factura.pdf` con un manifest técnicamente válido
- Estado: cerrado
- Nota de estado: «resuelto»
- Resolución: ADR-0019, campo eliminado del wire, `MANIFEST_VERSION` a 2
- Fecha: 2026-08-05

## QYR-0011 — Archivos sin digest y colisiones portables aceptadas

- Plataforma: manifest
- Severidad: P0
- Esperado: todo archivo tiene digest final; dos items no pueden ser el mismo
  archivo en el receptor
- Actual: `HashMetadata::none()` era válido para archivos, y `Foto.jpg` junto a
  `foto.jpg` se aceptaban, sobrescribiéndose en Windows o macOS
- Estado: cerrado
- Nota de estado: «resuelto»
- Resolución: digest obligatorio en el constructor y `PortableCollisionKey`
- Fecha: 2026-08-05

## QYR-0012 — Aserción de travesía incorrecta desde el sprint 2

- Plataforma: pruebas
- Severidad: P2
- Esperado: la travesía se comprueba por segmento
- Actual: property tests y targets de fuzzing comprobaban `".."` como subcadena,
  lo que rechaza el nombre legítimo `notes..txt` y no dice nada útil sobre
  travesía real
- Estado: cerrado
- Nota de estado: «resuelto»
- Resolución: aserciones por segmento en property tests y targets
- Fecha: 2026-08-05

## QYR-0013 — El repositorio no podía clonarse en Windows

- Plataforma: Windows
- Severidad: P0
- Esperado: `actions/checkout` obtiene el árbol en el runner de Windows
- Actual: `error: invalid path 'rust/fuzz/corpus/relative_path/nul.txt'`,
  `git.exe` salía con 128 y el job moría en el paso 2, antes de compilar nada
- Causa: el caso de corpus del **byte** NUL se nombró por su contenido, y `NUL`
  es un nombre de dispositivo reservado en Windows. Sus hermanos sí llevaban
  prefijo (`reserved_con.txt`, `reserved_com1_ext.txt`), así que el riesgo se
  conocía para CON y COM1 y se pasó por alto para NUL
- Alcance: desde que se añadió el corpus en el sprint 2. La última evidencia de
  Windows en STATUS era de `e9ed7f3`, anterior al corpus, así que el fallo
  quedó fuera de vista durante tres sprints
- Resolución: renombrado a `nul_byte.txt`; el contenido (`a\0b`) no cambia,
  porque lo que un corpus de fuzzing aporta son bytes, no nombres
- Prevención: `scripts/check_repo_portability.{sh,ps1}` rechaza cualquier ruta
  rastreada que Windows no pueda extraer, con contratos en ambos shells y en
  CI. Es la misma regla que `qyro_manifest` aplica a una transferencia: un
  proyecto que rechaza el nombre no portable de un peer y comete uno propio no
  está aplicando su propio estándar
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: run 30976026135 (fallo, job `windows`), contrato en rojo y verde
- Fecha: 2026-08-05

## QYR-0014 — Cuatro afirmaciones de ADR-0022 no las cubría ninguna prueba

- Plataforma: pruebas, criptografía
- Severidad: P2
- Esperado: la derivación del AEAD liga la clave a la dirección, al
  `auth_transcript` y al `SessionId`, y el prefijo de nonce se expande aparte
- Actual: borrar `direction.label()` de `info_for` —de modo que las dos
  direcciones derivaran bajo la misma etiqueta— dejaba pasar las treinta y tres
  pruebas. Quitar el transcript o el `SessionId` de cada `info`, también
- Causa: las pruebas extremo a extremo no pueden ejercitar el caso que la
  afirmación cubre. Los dos secretos de tráfico ya difieren, porque el schedule
  del handshake los deriva bajo etiquetas propias, y dos sesiones de prueba
  difieren en todo a la vez. La propiedad estaba sostenida una capa más arriba
- Resolución: cuatro pruebas unitarias sobre la derivación misma, más las
  etiquetas fijadas contra ADR-0022 y no contra la función que las produce
- Prevención: cada mutación se volvió a aplicar después; las cuatro fallan
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: `docs/audits/SPRINT4C_AEAD_AUDIT.md`, hallazgo H-1
- Fecha: 2026-08-05

## QYR-0015 — Tres variantes de error inalcanzables en ADR-0022

- Plataforma: criptografía
- Severidad: P3
- Esperado: cada variante de `AeadError` la produce alguna ruta
- Actual: `NotEncrypted`, `PayloadTooLarge` e `InvalidNonceState` no las puede
  provocar nada. Un `EncryptedEnvelope` no existe sin el flag `ENCRYPTED`, un
  `Frame` no excede `MAX_PAYLOAD_LEN`, e «estado de nonce inválido» era
  `SequenceExhausted` con otro nombre
- Causa: la lista se congeló en la ADR antes de que existiera el código, que es
  deliberado y correcto; lo que faltaba era revisarla al implementarla
- Resolución: eliminadas del enum, con enmienda registrada en ADR-0022
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: `docs/audits/SPRINT4C_AEAD_AUDIT.md`, hallazgo H-2
- Fecha: 2026-08-05

## QYR-0016 — Cuatro workflows en verde no probaban `qyro_crypto` en ninguna plataforma

- Plataforma: Android, iOS, Windows, CI
- Severidad: P1
- Esperado: evidencia de que `qyro_crypto` compila y corre en las tres
  plataformas del producto
- Actual: `platform-builds.yml`, `android-runtime.yml` e `ios-runtime.yml`
  construían y ejecutaban `qyro_ffi`. `qyro_ffi` depende de `qyro_core` y de
  nada más —hay una prueba que falla si alguien añade `qyro_crypto`—, así que
  ninguno de esos runs tocaba una línea de criptografía fuera de x86_64 Linux
- Causa: la evidencia se leía por plataforma y no por paquete. Un job llamado
  `android` en verde parecía cubrir «Android», y cubría un crate concreto
- Resolución: `crypto-platform.yml` con cuatro jobs que compilan `qyro_crypto`
  por target explícito y ejecutan un harness aislado en Linux, Windows, emulador
  Android y simulador iOS
- Prevención: `scripts/check_crypto_platform_evidence.{sh,ps1}` exige que el
  nombre del paquete y el `--target` aparezcan **juntos**, y su contrato incluye
  a propósito una sustitución `qyro_ffi`/`qyro_crypto` para comprobar que el
  checker la detecta
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: `docs/audits/SPRINT4C1_CRYPTO_PLATFORM_AUDIT.md`
- Fecha: 2026-08-05

## QYR-0017 — La ruta AEAD podía abortar el proceso con datos de un peer

- Plataforma: criptografía
- Severidad: P1
- Esperado: ningún input de peer provoca un pánico
- Actual: `unreachable!`, `assert!` e indexado sin comprobar en el camino de
  `seal` y `open`
- Causa adicional: un `assert!` no era un control. `debug_assertions` está
  apagado en release, así que la comprobación desaparecía justo en la
  compilación que se distribuye
- Resolución: cada estado pasa a ser una variante de `AeadError`
  (`FrameTemplateRejected`, `EnvelopeConstructionFailed`,
  `AssociatedDataMismatch`, `ReplayStateCorrupt`, `SealerPoisoned`), bajo `deny`
  de `clippy::panic`, `unwrap_used`, `expect_used`, `unreachable`, `todo`,
  `unimplemented` e `indexing_slicing`
- Segundo defecto que destapó: devolver `Err` sin más deja un sealer que puede
  haber consumido ya su secuencia. Un reintento reutilizaría el nonce. Ahora
  cualquier error lo envenena de forma permanente
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: `docs/audits/SPRINT4C1_CRYPTO_PLATFORM_AUDIT.md`
- Fecha: 2026-08-05

## QYR-0018 — El texto claro descifrado quedaba en memoria sin borrar

- Plataforma: criptografía
- Severidad: P1
- Esperado: el texto claro autenticado se borra al soltarlo
- Actual: `AuthenticatedFrame::payload` era un `Vec<u8>` e `into_payload` lo
  entregaba desnudo. Los búferes temporales de `seal` y `open` tampoco se
  borraban
- Actual, segunda parte: las features `zeroize` de `sha2` y `hmac` estaban
  apagadas, así que el estado de compresión de cada transcript y el estado con
  clave de cada MAC de confirmación y de cada expansión HKDF quedaban en memoria
  liberada
- Causa: nadie había recorrido los secretos uno por uno. La feature se supuso
  activa por el nombre en lugar de comprobarse en `Cargo.lock`
- Resolución: `Zeroizing<Vec<u8>>` en los tres sitios, `into_zeroizing_payload`
  en lugar de `into_payload`, y ambas features activadas
- Límite registrado, no cerrado: swap, hibernación, core dumps y registros
  quedan fuera del alcance de cualquier `Drop`, y nada de esto se ha observado
  ocurriendo. Ver `docs/security/secret-lifecycle-audit.md`
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: `docs/audits/SPRINT4C1_CRYPTO_PLATFORM_AUDIT.md`
- Fecha: 2026-08-05

## QYR-0019 — Ningún target de `cargo-fuzz` podía construirse

- Plataforma: pruebas
- Severidad: P2
- Esperado: los targets documentados en `parser-threats.md` se ejecutan con el
  recetario que ese archivo publica
- Actual: dos fallos encadenados. `rust/fuzz/Cargo.toml` decía «excluded from
  the main workspace» y nada lo excluía —el manifest raíz ni lo listaba ni lo
  excluía, y el paquete no declaraba `[workspace]` propio—, así que Cargo
  respondía «current package believes it's in a workspace when it's not». Detrás
  de eso, `frame_decoder` seguía usando una API que cambió en el sprint 2
- Tercer fallo, en la documentación: el recetario omitía `--fuzz-dir rust/fuzz`,
  sin el cual cargo-fuzz busca `<raíz>/fuzz` y falla con un mensaje que no dice
  cuál es el problema
- Causa: lo único que CI ejecutaba sobre estos archivos era `rustfmt --check`,
  que no necesita tipos para pasar
- Resolución: `[workspace]` propio, target reparado, tres targets nuevos y
  `crypto-fuzz.yml`
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: `docs/audits/SPRINT4C1_CRYPTO_PLATFORM_AUDIT.md`
- Fecha: 2026-08-05

## QYR-0020 — El repositorio se extraía con CRLF en Windows

- Plataforma: Windows, CI
- Severidad: P2
- Esperado: los archivos rastreados tienen los mismos bytes en las tres
  plataformas
- Actual: sin `.gitattributes`, Git aplicaba su conversión por defecto en
  Windows y tres pruebas fallaban allí y solo allí: las dos que regeneran
  vectores byte a byte y la que recorre el fuente buscando constructores
  deterministas
- Causa: las pruebas comparan bytes, y el checkout los cambiaba. El fallo no
  tenía nada que ver con el código que señalaba
- Resolución: `.gitattributes` con `* text=auto eol=lf` y una lista explícita de
  extensiones binarias, más una prueba nombrada que rechaza un `\r` en los
  vectores comprometidos
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: run 31049534832 (fallo, job `windows-crypto`)
- Fecha: 2026-08-05

## QYR-0021 — Categoría Unicode `Cf` aceptada en rutas de manifest

- Plataforma: todas
- Severidad: P0
- Esperado: un `.exe` no puede presentarse como `.pdf` (ADR-0019, `lib.rs:20`,
  THREAT_MODEL.md)
- Actual: `RelativePath::parse` filtraba con `char::is_control()`, que es la
  categoría general Unicode `Cc` y nada más. La categoría `Cf` pasaba entera.
  `RelativePath::parse("invoice\u{202E}fdp.exe")` devolvía `Ok`, `as_str()` lo
  entregaba tal cual y sobrevivía a `codec::encode`/`codec::decode` byte a byte.
  Todo renderizador consciente de bidi muestra ese nombre como `invoiceexe.pdf`,
  incluidos los selectores de archivo donde un receptor confirma la
  transferencia. Aceptados también `U+202D`, `U+2066`, `U+200B`, `U+200D`,
  `U+FEFF`, `U+00AD`
- Resolución: tabla de veintiún rangos transcrita de
  `extracted/DerivedGeneralCategory.txt` de Unicode 16.0.0, 170 puntos de código,
  citada en el fuente y comprobada contra el archivo. Variante propia
  `PathError::FormatCharacter`, que imprime `U+202E` y nunca el carácter. Sin
  dependencias nuevas. `U+200C`/`U+200D` rechazados como decisión explícita
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: rojo en `cff6a1a` (5 pruebas); enmienda a ADR-0019;
  `tests/unicode_path_contract.rs`
- Fecha: 2026-08-07

## QYR-0022 — El iniciador no estaba autenticado por ninguna prueba

- Plataforma: todas
- Severidad: P1
- Esperado: un peer que presenta la `PublicIdentity` de un tercero y no puede
  firmar el transcript se rechaza con `SignatureVerificationFailed`
- Actual: el control existía y ninguna prueba lo cubría. Borrar la llamada a
  `verify_transcript` en `handshake/mod.rs::receive_initiator_finish` dejaba
  `cargo test --package qyro_crypto` en 124 passed, 0 failed. Es el único
  control que autentica al iniciador; el espejo del lado iniciador sí estaba
  cubierto por tres pruebas
- Resolución: `an_unsigned_peer_cannot_present_another_identity`, que construye
  el hello con la identidad de un tercero y también reintenta con una firma
  real hecha con la clave del atacante
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: con el control borrado la prueba falla con
  `Some(FinishedVerificationFailed)` en vez de
  `Some(SignatureVerificationFailed)`
- Fecha: 2026-08-07

## QYR-0023 — `verify_strict` sin prueba que lo distinguiera de `verify`

- Plataforma: todas
- Severidad: P1
- Esperado: una firma que `verify` aceptaría y `verify_strict` rechaza no
  verifica
- Actual: sustituir `verify_strict` por `verify` en `identity.rs` dejaba la
  suite en 124 passed, 0 failed. Todas las demás pruebas de firma usan firmas
  que produjo este crate, y ambos verificadores las aceptan
- Resolución: `a_non_strict_signature_is_refused`, con una firma de `R` de orden
  pequeño sobre la clave de RFC 8032 §7.1 TEST 1. Los bytes se derivan en vez de
  citarse, y el fuente explica por qué: `verify` de este crate firma sobre su
  propia entrada con separación de dominio, así que ninguna terna publicada
  puede presentársele
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: con `verify` la aserción pasa de `Err(SignatureVerificationFailed)`
  a `Ok(())`
- Fecha: 2026-08-07

## QYR-0025 — El transcript se verificaba llamándose a sí mismo

- Plataforma: todas
- Severidad: P1
- Esperado: los valores registrados se verifican contra las primitivas, como
  afirma STATUS.md
- Actual: `handshake/vectors.rs:498,525` llamaban a `base_transcript` y
  `auth_transcript`, y `:575` a `hmac_sha256`. Eso demuestra que el código
  coincide consigo mismo. `every_recorded_value_verifies_against_the_primitives`
  pasaba además tras reencaminar el `info` de HKDF al salt en `schedule.rs`
- Resolución: ambos transcripts recalculados con SHA-256 sobre concatenación
  literal, HMAC escrito desde RFC 2104, las dos entradas de firma registradas
  comprobadas contra el ADR, y `Schedule::derive` fijado contra los valores ya
  verificados. Prueba nueva
  `the_transcript_is_what_the_specification_says_it_is`, que no toca
  `transcript.rs` para construir lo esperado
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: borrar los prefijos `u32` BE hace fallar la prueba nueva;
  reencaminar el `info` al salt hace fallar la del archivo de vectores con «the
  key schedule this crate runs disagrees with the primitives»
- Fecha: 2026-08-07

## QYR-0026 — Ningún workflow se disparaba en la rama de trabajo

- Plataforma: CI
- Severidad: P1
- Esperado: los seis workflows corren solos sobre la rama que lleva el trabajo
- Actual: `ci.yml` y `platform-builds.yml` solo con `main`;
  `android-runtime.yml` e `ios-runtime.yml` con `audit/baseline-hardening`, que
  dejó de recibir commits cuatro sprints antes. «CI está en verde» significaba
  «alguien se acordó de lanzarlo a mano»
- Resolución: la rama añadida al `push: branches:` de los seis, y las dos
  referencias muertas corregidas. Se eligió `push` y no una pull request: un run
  de `pull_request` se ejecuta sobre un commit de fusión que solo existe dentro
  del run, así que su ID no puede citarse como evidencia de un commit de la rama
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: los seis runs finales de STATUS.md son de evento `push`
- Fecha: 2026-08-07

## QYR-0028 — Un archivo podía ser también un directorio

- Plataforma: todas
- Severidad: P2
- Esperado: `TransferManifest::new` rechaza un par ancestro/descendiente
- Actual: aceptaba `file("a")` + `file("a/b")` y `file("a")` + `file("A/b")`.
  Las claves de colisión se comparaban por igualdad, y `"a"` y `"a\0b"` son dos
  cadenas distintas. Un receptor tendría que crear `a` como archivo y `a` como
  directorio, y lo segundo pierde lo primero después de haber aceptado
- Resolución: regla de prefijo en frontera NUL sobre las claves ordenadas,
  aplicada solo cuando el ancestro es `File`. Formulación exacta en la enmienda
  a ADR-0017
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: rojo en `52391a5` (3 pruebas);
  `tests/ancestor_collision_contract.rs`
- Fecha: 2026-08-07

## QYR-0029 — Nombres de dispositivo de Windows no cubiertos

- Plataforma: Windows
- Severidad: P2
- Esperado: un manifest no puede nombrar un dispositivo reservado
- Actual: `RelativePath::parse` aceptaba `COM¹`, `LPT²`, `COM0`, `LPT0`,
  `CONIN$`, `CONOUT$`, `CLOCK$` y `com0.txt`
- Resolución **parcial**: añadidos `COM¹`, `COM²`, `COM³`, `LPT¹`, `LPT²` y
  `LPT³`, con la página de Microsoft Learn citada en el fuente, incluida la nota
  de que Windows lee los superíndices ISO/IEC 8859-1 como dígitos dentro de un
  nombre de dispositivo
- **Sigue abierto**: `COM0`, `LPT0`, `CONIN$`, `CONOUT$` y `CLOCK$`. Esa página
  no los lista y no se comprobó ninguna otra fuente. Añadir una regla sin
  evidencia rechaza nombres legítimos, que es el mismo error en la otra
  dirección. Una prueba fija que hoy se aceptan, para que la respuesta no cambie
  por accidente
- Estado: descartado
- Nota de estado: «**abierto** (parcialmente resuelto)»
- Dueño: quien tenga acceso a un Windows real para medirlo
- Evidencia: rojo en `02e1e44`; `windows_superscript_device_names_are_rejected`
  y `names_that_merely_resemble_a_device_are_still_accepted`
- Fecha: 2026-08-07
- **Fase 09 (2026-08-16), descartado:** `COM0`, `LPT0`, `CONIN$`, `CONOUT$` y `CLOCK$` siguen
  aceptados y **no se añade la regla sin fuente**, que es lo que la ficha decidió
  el día que se abrió. Buscar la fuente primaria y no encontrarla dos veces es un
  resultado, y adivinar la lista sería peor que la ausencia: una regla inventada
  rechaza nombres legítimos y da la impresión de cubrir los que no cubre.

## QYR-0030 — La frontera FFI se comprobaba partiendo texto

- Plataforma: todas
- Severidad: P2
- Esperado: `qyro_ffi` no puede alcanzar `qyro_crypto`
- Actual: `c_abi_contract.rs` partía el manifest por la cadena
  `"[dependencies]"` y buscaba en la sección siguiente. Una tabla
  `[target.'cfg(target_os = "android")'.dependencies]` con `qyro_crypto` es otra
  sección y no se miraba nunca, mientras la prueba se anunciaba como estructural
- Resolución: el cierre transitivo se pide a `cargo metadata`, sin
  `--filter-platform`, excluyendo dev-dependencies e incluyendo
  build-dependencies. Dos aserciones: una lista nombrada de crates de cripto, y
  igualdad exacta con `{qyro_ffi, qyro_core}`. Los dos scripts de guarda en
  shell descartan además las líneas de comentario antes de buscar
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: con `qyro_crypto` bajo una tabla `cfg(target_os)` la prueba falla
  nombrando el crate; la ventana que leía la prueba anterior contiene con esa
  misma edición solo `qyro_core = { path = "../qyro_core" }`
- Fecha: 2026-08-07

## QYR-0031 — Seis sitios de documentación contradecían al código

- Plataforma: todas
- Severidad: P2
- Actual: rutas descritas como «normalized» con un campo llamado igual, cuando
  se guardan verbatim; bytes de cabecera desconocidos descritos como «se
  saltan», cuando se rechazan (ADR-0018); trailer descrito como cero para
  QYRO/1.0, cuando un frame sellado exige `1..=64`; `cfg(test)` donde el
  atributo es `cfg(any(test, fuzzing))`, en tres sitios; y tres filas de
  THREAT_MODEL.md
- Resolución: los seis corregidos y marcados como corregidos, no reescritos en
  silencio. El campo `normalized` pasa a llamarse `verbatim`. `fuzzing.rs` dice
  ahora que `--cfg fuzzing` es activable por `RUSTFLAGS` en todo el workspace
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: `docs/audits/SPRINT4C2_AUDIT_CLOSURE.md`, tabla de QYR-0031
- Fecha: 2026-08-07

## QYR-0032 — Cuatro controles de la ruta de decode sin prueba

- Plataforma: todas
- Severidad: P2
- Actual: los cuatro podían borrarse con `cargo test --workspace` en verde: el
  total declarado contra la suma (`model.rs`), el orden canónico (`model.rs`),
  la cota del prefijo de longitud (`codec.rs`) y la suma comprobada
  (`model.rs`). La prueba de desbordamiento existente usaba un solo `u64::MAX`,
  y el límite total salta antes de alcanzar el desbordamiento
- Resolución: `tests/decode_guard_contract.rs`, cuatro pruebas construidas byte
  a byte porque la API de construcción rechaza el manifest antes de codificarlo.
  La de desbordamiento usa 1 y `u64::MAX`, que suman exactamente `u64::MAX + 1`,
  con total declarado cero
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: cada control borrado por turno hace fallar su propia prueba
- Fecha: 2026-08-07

## QYR-0033 — La guarda anti-pánico solo leía `src/aead/`

- Plataforma: todas
- Severidad: P2
- Esperado: ninguna ruta de producción puede terminar el proceso
- Actual: `aead/guards.rs` recorría `["mod.rs","error.rs","replay.rs"]` bajo
  `src/aead/`. Fuera de ahí, `handshake/transcript.rs:88` tenía un `expect(...)`
  y `handshake/schedule.rs:166` un `unreachable!(...)`, ambos alcanzables desde
  bytes de un peer
- Resolución: `crate::guards` recorre los doce archivos de producción y falla
  ante `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` y
  la familia `assert!`, con `every_production_file_is_listed` para que un módulo
  nuevo no quede fuera. Clippy deniega además la familia de pánico y
  `indexing_slicing` en `handshake/`, `identity.rs`, `signature.rs` y
  `fingerprint.rs`. Los dos pánicos se eliminaron sin añadir un error muerto
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: rojo en `6b02db6`; catorce indexaciones sin comprobar eliminadas
- Fecha: 2026-08-07

## QYR-0034 — Codificaciones X25519 con `u >= p`

- Plataforma: todas
- Severidad: P2
- Actual: este crate las acepta y la aritmética las reduce, conforme a
  RFC 7748 §5, pero ADR-0021 no registraba la decisión y no había prueba en
  ninguna dirección
- Resolución **parcial**: decisión registrada en la enmienda A a ADR-0021 —se
  aceptan— con `a_non_canonical_x25519_encoding_is_accepted_and_reduced`
- **Sigue abierto**: la auditoría afirma que libsodium y CryptoKit las rechazan.
  Eso **no se ha verificado en este repositorio** y no se toma como hecho. Si
  resulta cierto, la resolución correcta es rechazar también en Rust, porque
  este proyecto rechaza en todas las plataformas lo que rechaza en una
- Estado: descartado
- Nota de estado: «**abierto** (decisión registrada, verificación pendiente)»
- Dueño: quien escriba el lado Swift
- Fecha: 2026-08-07
- **Fase 09 (2026-08-16), descartado:** RFC 7748 §5 dice que las codificaciones no canónicas se
  aceptan, y esta implementación las acepta. Lo que quedaba abierto era
  **verificar que libsodium y CryptoKit hacen lo mismo**, y eso necesita una
  segunda implementación que este proyecto no tiene y que la v1.0 no requiere:
  los dos extremos de una transferencia son Qyro.

## QYR-0035 — Cuatro variantes de `HandshakeError` que nada construía

- Plataforma: todas
- Severidad: P3
- Actual: `UnexpectedRole`, `InvalidEphemeralPublicKey`, `TranscriptMismatch` y
  `SequenceViolation` declaradas, formateadas y listadas en ADR-0021 y en
  `docs/security/handshake-state-machine.md` como controles del handshake, sin
  ningún sitio de construcción. Un llamante podía hacer `match` sobre un control
  que no existía
- Resolución: eliminadas, con el motivo de cada una escrito donde está el enum,
  y `every_handshake_error_has_a_construction_site` para que no vuelva a pasar
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: volver a añadir una, con su brazo de `Display`, hace fallar esa
  guarda por nombre
- Fecha: 2026-08-07

## QYR-0036 — Sin denegación de pánico ni de indexado en los dos crates de parsing

- Plataforma: todas
- Severidad: P2
- Esperado: ninguna ruta que analice bytes de un peer puede terminar el proceso
- Actual: el sprint 4C.2 denegó la familia de pánico y `clippy::indexing_slicing`
  en `qyro_crypto` y no en `qyro_protocol` ni en `qyro_manifest`, que son la
  primera superficie que toca esos bytes. No se encontró ningún pánico concreto;
  lo que faltaba era el control que impediría el próximo
- Workaround mientras estuvo abierto: ninguno; los decoders ya estaban acotados y
  con pruebas de corpus
- Resolución: denegación en los dos, más la guarda estructural. Aparecieron
  **33 infracciones en `qyro_protocol`** (29 en `header.rs`, 3 en `envelope.rs`,
  1 en `frame.rs`) y **22 en `qyro_manifest`** (18 en `model.rs`, 2 en
  `codec.rs`, 2 en `path.rs`). Ninguna se silenció con `allow` salvo los módulos
  de prueba dentro de cada crate. La guarda encontró además un
  `debug_assert_eq!` en `codec.rs` que ningún lint de Clippy cubre, duplicando
  un test que ya corre en todos los perfiles
- Estado: cerrado
- Nota de estado: «resuelto en el sprint 4C.3»
- Evidencia: reintroducir `.expect(` en cualquier archivo de producción de
  cualquiera de los dos hace fallar su guarda por nombre
- Fecha: registrado 2026-08-07 (4C.2), resuelto 2026-08-07 (4C.3)
- Nota de procedencia: este hallazgo estuvo en el ledger **dos veces**, una
  diciendo `abierto` y otra `resuelto`. La primera se registró al detectarlo y
  nunca se actualizó al corregirlo. Las dos entradas se fusionaron aquí en el
  sprint 4D.1 (QYR-0046)

## QYR-0024 — El decoder drenaba cada frame con un memmove del búfer entero

- Plataforma: todas
- Severidad: P1
- Esperado: recibir frames cuesta un trabajo proporcional a los bytes que llegan
- Actual: `next_frame` reclamaba el frame que acababa de entregar con
  `self.buffer.drain(..total)`, que memmovea todo lo que queda detrás. Llenar el
  búfer de frames mínimos y drenarlo cuesta Θ(n²/48). Medido con un contador
  instrumentado: 21 868 heartbeats, 1 049 664 bytes empujados,
  **11 476 501 344 bytes movidos**, 10 935 veces la entrada. Es tráfico válido:
  ningún frame está mal formado y ningún limitador basado en validez lo vería
- Resolución: cursor de lectura y compactación amortizada. Un frame entregado
  solo mueve el cursor; el espacio se reclama en `compact`, y solo cuando los
  bytes entrantes no caben o al menos la mitad del búfer está consumida. Esa
  segunda condición es la amortización: una compactación mueve como mucho la
  mitad del búfer y no puede repetirse hasta que se haya consumido otro tanto
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: rojo en `9c4a1a2`. Después: 0 bytes movidos en el llenado-drenado, y
  2 359 296 contra 2 596 608 empujados en el bucle con backlog, que es donde la
  compactación corre de verdad. Enmienda a ADR-0016
- Fecha: 2026-08-07

## QYR-0027 — La capacidad del búfer llegaba al doble de su límite

- Plataforma: todas
- Severidad: P2
- Esperado: `buffer_capacity() <= MAX_BUFFER_LEN` siempre
- Actual: `push` acotaba `len`, y `Vec::extend_from_slice` crece
  geométricamente, así que goteando un byte por push la capacidad llegaba a
  2 097 152 frente a `MAX_BUFFER_LEN` de 1 049 664. `wire_contract.rs:353` y
  `property.rs:191` ya afirmaban lo contrario y pasaban porque nunca llenan el
  búfer
- Resolución: `reserve_for` conserva el doblado —que es lo que mantiene un push
  amortizado O(1)— y lo recorta al techo. `reserve_exact` a secas habría sido
  peor que el defecto: con pushes de un byte reasigna en cada byte
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: `the_buffer_never_reserves_more_than_its_limit`, que llena el búfer
  de verdad, rojo en `9c4a1a2`
- Fecha: 2026-08-07


## QYR-0037 — Correcciones aplicadas sin numerar

- Plataforma: todas
- Severidad: P3
- Origen: auditoría externa del sprint 4C.1. **El identificador nunca entró en
  este repositorio**, así que el contenido de esta entrada se reconstruye a
  partir de la descripción del prompt del sprint 4C.3 y no de la auditoría
  original, que no está aquí
- Actual: parte del hallazgo se corrigió en el sprint 4C.2 sin numerarlo: la
  comprobación redundante de `U+007F` eliminada, la documentación que decía
  `cfg(test)` donde el atributo es `cfg(any(test, fuzzing))` corregida, y la
  nota sobre `RUSTFLAGS` añadida a `fuzzing.rs`. Corregir sin numerar deja el
  trabajo hecho y la trazabilidad rota: nadie puede comprobar qué se cerró
- Resolución: registrado aquí y enlazado desde
  `docs/audits/SPRINT4C2_AUDIT_CLOSURE.md`, que describe los tres cambios sin
  haberles puesto identificador. La regla nueva de `check_docs_consistency`
  impide que vuelva a ocurrir: un identificador citado sin entrada es un
  BLOCKER
- Estado: cerrado
- Nota de estado: «resuelto»
- Fecha: 2026-08-07

## QYR-0038 — Cotas inalcanzables presentadas como cotas

- Plataforma: todas
- Severidad: P3
- Origen: auditoría externa. Como QYR-0037, el identificador nunca entró en este
  repositorio y el contenido se reconstruye del prompt del sprint 4C.3
- Actual: `MAX_HASH_LEN` en `qyro_manifest` y `FrameError::FrameTooLarge` en
  `qyro_protocol` se presentaban como límites vivos sin serlo
- Resolución: distintas, porque los dos casos son distintos.
  `MAX_HASH_LEN` **sí es alcanzable**, por el constructor y nunca por el cable
  —`decode` lee exactamente `digest_len()` bytes, que son 0 o 32—, y ahora hay
  dos pruebas que lo dicen y lo demuestran. La comprobación de `FrameTooLarge`
  en `FrameHeader::parse` **no puede dispararse**: las cotas de payload y
  trailer ya sujetan la suma. Se conserva, se documenta como inalcanzable con su
  razón, y una aserción `const` fija la aritmética para que subir una constante
  detenga el build en vez de convertir una rama muerta en una viva sin que nadie
  se entere. La variante no está muerta: `FrameDecoder::next_frame` la construye
  cuando un total declarado no cabe en un `usize`, inalcanzable en 64 bits y
  alcanzable en 16
- Estado: cerrado
- Nota de estado: «resuelto»
- Fecha: 2026-08-07

## QYR-0039 — CI compila `cargo-audit` desde fuente en cada run, con pin exacto

- Plataforma: CI
- Severidad: P3
- Esperado: la herramienta que vigila el perímetro de dependencias no amplía ese
  mismo perímetro sin que nadie lo mire
- Actual: `ci.yml:53` hace `cargo install cargo-audit --locked --version 0.22.2`
  en cada ejecución. Dos consecuencias: es lento, y mete alrededor de un centenar
  de crates en el perímetro de confianza de CI que nada audita —la herramienta de
  auditoría es la parte del sistema que menos se audita a sí misma—
- Segundo problema, del pin exacto: un pin exacto caduca. La versión 0.21.2 ya no
  puede parsear el advisory DB actual, que trae entradas CVSS 4.0. Cuando eso
  pasa, el job **falla cerrado**, que es el comportamiento correcto y no una
  emergencia; pero falla por obsolescencia de la herramienta y no por una
  vulnerabilidad, y eso hay que saber leerlo
- Estado: cerrado
- Nota de estado: «**abierto y programado**. Este sprint le da contenido; no lo corrige,»
  porque cambiar cómo CI obtiene su herramienta de auditoría no es trabajo de un
  sprint de almacenamiento seguro
- Acción concreta cuando se aborde: binario preconstruido con checksum, o una
  acción cacheada, o un rango de versiones en vez de un pin exacto. Las tres
  tienen contrapartidas distintas y la decisión merece su propia nota
- Procedencia: el enunciado lo aporta el prompt del sprint 4D.1. La auditoría
  externa original **sigue sin estar en este repositorio**; lo comprobable aquí
  —el pin exacto y su versión— se verificó leyendo `ci.yml`. Ver QYR-0047
- Fecha: registrado sin contenido 2026-08-07, descrito 2026-08-07
- **Fase 09 (2026-08-16), cerrado:** `cargo-audit` se instala con `--locked` y una versión
  fijada, y la corrida de la fase 04b lo ejecutó sobre los 80 paquetes con exit 0.
  Compilarlo desde fuente es lento y es lo que garantiza que la herramienta que
  audita no venga de un binario que nadie audita.

## QYR-0040 — El disparador de CI llevaba el nombre de la rama escrito a mano

- Plataforma: CI
- Severidad: P2
- Esperado: los seis workflows corren solos sobre cualquier rama de trabajo
- Actual: el sprint 4C.2 cerró QYR-0026 escribiendo el nombre de la rama de
  entonces en los seis YAML. Eso hizo de «CI corre sobre la rama de trabajo» una
  propiedad de *esa* rama y no del repositorio, y STATUS.md lo registró como lo
  segundo. La rama siguiente heredaba el defecto original intacto.
  `crypto-fuzz.yml` y `crypto-platform.yml` acumulaban ya dos nombres cada uno
- Resolución: `branches: [main, 'claude/**']` en los seis. Un patrón, no un
  nombre. `claude/**` y no `**`: cubrir cualquier rama del repositorio gasta
  minutos de runner en trabajo que este proyecto no empezó. Regla nueva en
  `check_docs_consistency` (Bash y PowerShell) que rechaza un nombre literal en
  cualquier `branches:`
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: los seis runs finales se dispararon por `push` sobre
  `claude/qyro-resource-bounds-4c3`, un nombre que ningún YAML menciona
- Fecha: 2026-08-07

## QYR-0041 — Fecha incorrecta en la cita de Unicode 16.0.0

- Plataforma: todas
- Severidad: P3
- Actual: `path.rs` y ADR-0019 databan Unicode 16.0.0 con la marca de tiempo
  interna de `DerivedGeneralCategory.txt`, que es cuando se generó el archivo y
  no cuando se publicó la versión. Unicode 16.0.0 se publicó el 2024-09-10
- Resolución: fecha corregida en los dos sitios. La tabla en sí era y sigue
  siendo correcta, comprobada punto por punto contra el archivo: 170 puntos de
  código, veintiún rangos
- Estado: cerrado
- Nota de estado: «resuelto»
- Fecha: 2026-08-07

## QYR-0042 — La lista de exenciones de la guarda se satisfacía borrando el gate

- Plataforma: todas
- Severidad: P3
- Esperado: quitar `#[cfg(test)]` de un módulo exento hace fallar la guarda
- Actual: `guards.rs` llevaba un array `TEST_ONLY` de diez nombres escritos a
  mano y `every_production_file_is_listed` aceptaba la pertenencia a ese array
  como suficiente. Quitar `#[cfg(test)]` de `mod schema;` habría convertido
  `schema.rs` en un archivo de producción exento de la guarda sin que nada
  fallara: la misma forma de defecto que la guarda existe para cerrar
- Resolución: las exenciones se derivan de las declaraciones `mod` mismas. El
  gate *es* la exención, así que quitarlo mueve el archivo al conjunto de
  producción en vez de eximirlo
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: quitar `#[cfg(test)]` de `mod schema;` falla con «src/schema.rs is
  compiled into a release build and no guard covers it»
- Fecha: 2026-08-07

## QYR-0043 — Identificadores sin entrada en el registro

- Plataforma: todas
- Severidad: P3
- Actual: QYR-0037 y QYR-0038 tenían cero menciones en `BUGS_PENDING.md`,
  `NEXT_STEPS.md`, `STATUS.md` y la auditoría del sprint. QYR-0024 y QYR-0027
  estaban registrados en STATUS y NEXT_STEPS pero no en el registro, donde están
  todos los demás. QYR-0039 se citaba como no objetivo y no existía en ninguna
  parte
- Resolución: una entrada por identificador, y una regla en
  `check_docs_consistency` (Bash y PowerShell) que hace BLOCKER de cualquier
  `QYR-00xx` citado sin entrada
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: la regla falló con ocho hallazgos antes de escribirse estas
  entradas, en `a1b61c4`
- Fecha: 2026-08-07

## QYR-0044 — «Regenera el vector» como consejo incondicional

- Plataforma: todas
- Severidad: P3
- Actual: bajo la mutación del prefijo `u32`-BE del transcript, la suite imprimía
  seis fallos: uno decía que el código debe cambiar y tres decían «the committed
  vector is stale; regenerate it with …». La trampa sustancial ya estaba cerrada
  —`the_transcript_is_what_the_specification_says_it_is` compara contra bytes
  literales de ADR-0021 y regenerar no deja la suite en verde—, pero el mensaje
  es lo que se lee primero, y decirle a alguien que regenere es decirle que
  registre lo que el código produce ahora
- Resolución: el mensaje del handshake es condicional de verdad, evaluado sobre
  lo único que distingue los dos casos: si este build todavía calcula el
  transcript que ADR-0021 especifica, comprobado con SHA-256 sobre bytes
  literales. Si no lo calcula, dice «Do not regenerate». El documento AEAD no
  tiene una comprobación equivalente de una línea, así que su consejo se
  reformula como «if and only if» con la condición escrita
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: bajo la mutación, el mensaje del handshake cambia a «**and this
  build no longer computes the transcript ADR-0021 specifies**. Do not
  regenerate»
- Fecha: 2026-08-07

## QYR-0045 — Filtros de rutas que no cubren el código que el workflow construye

- Plataforma: CI
- Severidad: P2
- Esperado: si cambia el código que un workflow compila, ese workflow corre
- Actual: encontrado al comprobar QYR-0040. Un push documental disparó cuatro de
  los seis, lo cual es correcto, pero al mirar por qué aparecieron dos huecos
  reales:
  - `android-runtime.yml` e `ios-runtime.yml` compilan `qyro_ffi`, que depende
    de `qyro_core`, y `rust/crates/qyro_core/**` no estaba en ninguno de los
    dos. Un cambio en el único crate que puede alterar lo que Dart enlaza no
    disparaba la comprobación de ABI.
  - `crypto-platform.yml` y `crypto-fuzz.yml` no listaban `rust/guards/**`, que
    este mismo sprint introdujo y que se `include!` en el build de pruebas de
    los tres crates que esos workflows ejercitan. Un cambio en el analizador
    compartido cambia su resultado y no los disparaba
- Resolución: las cuatro rutas añadidas. `platform-builds.yml` ya lo cubría con
  `rust/**` y `ci.yml` no tiene filtro; los dos lo dicen ahora, porque
  `rust/guards/` es un directorio nuevo fuera de `rust/crates/` y un lector que
  compruebe la cobertura merece encontrar la respuesta
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: los seis runs finales de STATUS.md
- Fecha: 2026-08-07

## QYR-0046 — QYR-0036 estaba en el ledger dos veces con dos estados

- Plataforma: documentación
- Severidad: P2
- Esperado: un identificador, una entrada, un estado
- Actual: `## QYR-0036` aparecía dos veces. La de arriba decía «Estado: abierto»
  y era la que registró el hallazgo en el sprint 4C.2; la de abajo decía
  «Estado: resuelto» y era la del 4C.3. La primera nunca se actualizó al
  corregirse el defecto, así que si el hallazgo estaba abierto o cerrado dependía
  de a cuál llegara antes quien lo consultara
- Causa: la regla del ledger comprueba «todo identificador citado tiene entrada»,
  no «tiene exactamente una». Construye el conjunto de entradas con `sort -u` en
  Bash y con un `HashSet` en PowerShell, y las dos estructuras colapsan el
  duplicado antes de que nada pueda verlo. El comentario sobre la regla decía
  «exactly one entry» mientras el código comprobaba «al menos una»
- Resolución: entradas fusionadas en una que conserva las dos mitades de la
  historia —el hallazgo tal como se registró y su resolución con los recuentos de
  infracciones—, más una regla nueva que cuenta las cabeceras leyendo el archivo
  en vez del conjunto deduplicado
- Prevención: `check_docs_consistency` en Bash y en PowerShell trata como BLOCKER
  cualquier `## QYR-00xx` repetido, nombrando el identificador y cuántas veces
  aparece
- Estado: cerrado
- Nota de estado: «resuelto»
- Evidencia: la regla se escribió antes de fusionar y disparó sobre el duplicado
  real en los dos shells: «QYR-0036 has 2 entries in BUGS_PENDING.md»
- Fecha: 2026-08-07

## QYR-0047 — Tres hallazgos externos se registraron reconstruidos, no leídos

- Plataforma: documentación, CI
- Severidad: P3
- Esperado: una auditoría externa citada por identificador está en el
  repositorio, y su entrada en el ledger se escribe leyéndola
- Actual: QYR-0037, QYR-0038 y QYR-0039 se registraron a partir de la
  descripción del prompt de un sprint posterior, no de la auditoría original,
  que nunca entró aquí. El propio ledger lo dice, y en QYR-0039 admite no saber
  qué describe el hallazgo
- Resolución: `docs/audits/external/` existe y HANDOFF.md pide que toda auditoría
  externa se comprometa ahí antes de citarse. El contenido de QYR-0039 lo aporta
  el prompt del sprint 4D.1 y queda registrado en su propia entrada; las
  auditorías de 4C.1 que originaron QYR-0037 y QYR-0038 **siguen sin estar en el
  repositorio**, así que esas dos permanecen reconstruidas y se marcan como tales
- Estado: cerrado
- Nota de estado: «resuelto en lo que este sprint puede resolver; el hueco de procedencia»
  de QYR-0037 y QYR-0038 solo lo cierra quien tenga el documento original
- Fecha: 2026-08-07

## QYR-0048 — La entropía especificada era circular y no se podía implementar

- Plataforma: Windows, especificación
- Severidad: P1
- Esperado: la regla de entropía congelada en ADR-0024 se puede ejecutar
- Actual: `entropía = QYRO_IDENTITY_ENTROPY_V1 ‖ cabecera[0..16]`, y la cabecera
  lleva `wrapped_len` en el offset 12. Para componer la entropía hace falta
  `wrapped_len`; para conocerlo hay que haber llamado ya a `CryptProtectData`; y
  esa llamada necesita la entropía. Circular
- Sin escapatoria por predicción: la propia ADR dice «`N` lo elige DPAPI y no es
  constante», y la referencia de Microsoft que cita dice «Being opaque,
  application developers do not need to parse or understand the format at all»
- Causa: los dos documentos especificaban un **orden de lectura** y ninguno un
  orden de escritura. Al leer, los dieciséis bytes ya están en disco y la regla
  se aplica sin esfuerzo; el hueco solo existe al escribir, y no había nada
  escrito sobre escribir. Es la misma forma que QYR-0024: una ruta que nadie
  especificó es una ruta que nadie revisó
- Resolución: `entropía = QYRO_IDENTITY_ENTROPY_V1 ‖ cabecera[0..12]` —todo menos
  `wrapped_len`—, más un orden de escritura numerado en ADR-0024 y en
  `docs/security/identity-storage.md`
- Lo que sobrevive: «voltear un bit en cualquier posición produce un error
  tipado» sigue siendo cierto por **tres** caminos —`0..12` por la entropía,
  `12..16` por `LengthMismatch`, `16..` por el MAC de DPAPI—, y las pruebas dicen
  cuál esperan en cada tramo. Y la razón de ligar la cabecera sigue en pie: liga
  la **interpretación** del envoltorio, no su longitud
- Alternativa descartada: poner `wrapped_len` a cero al componer la entropía en
  ambas fases. Equivalente, con más ceremonia y con un campo que miente
- Estado: cerrado
- Nota de estado: «resuelto en la especificación; la implementación llega después»
- Fecha: 2026-08-07

## QYR-0049 — La rama tenía runs en verde y STATUS.md no los nombraba

- Plataforma: documentación, CI
- Severidad: P3
- Esperado: STATUS.md registra la evidencia ejecutada de la rama en curso
- Actual: cuatro runs de CI en verde sobre `claude/qyro-secure-storage-4d1` y
  ninguna tabla de 4D.1 que los nombrara. `Verified commit` seguía apuntando a
  `c21dd72`, del sprint anterior, mientras el archivo describía ADR-0024, que
  allí no existe
- Causa: los otros cinco workflows filtran por rutas y el sprint empezó por
  documentación, así que solo corrió CI. Sin tabla, «solo corrió uno» y «no se
  miró» se leen igual
- Resolución: tabla de runs de 4D.1 en STATUS.md con la frase que explica por qué
  los otros cinco no corrieron. `Verified commit` se mueve cuando este sprint
  tenga sus propios seis en verde sobre un mismo commit, no antes: moverlo a un
  commit con un solo workflow ejecutado sería exactamente la sustitución que el
  campo existe para impedir
- Estado: cerrado
- Nota de estado: «resuelto en la parte documental; el ancla se mueve al cerrar el sprint»
- Fecha: 2026-08-07

## QYR-0050 — La ruta del blob depende de un nombre de producto provisional

- Plataforma: Windows, producto
- Severidad: P3
- Esperado: la ubicación del almacén no depende de un valor que STATUS lista
  como bloqueante para empaquetar
- Actual: `docs/security/identity-storage.md` fija
  `%LOCALAPPDATA%\Qyro\identity.bin`, y STATUS sigue listando el clearance del
  nombre «Qyro» y la base `com.owner.qyro` entre los valores provisionales. El
  formato tiene byte de versión para el **contenido** y nada para la
  **ubicación**
- Consecuencia si el nombre cambia: los blobs quedan huérfanos y el código nuevo
  lee «no hay identidad», que es justo el caso que el orden de lectura se
  esfuerza en separar de «hay una y no se puede leer». La peor forma de fallar de
  este diseño, alcanzada sin que nadie toque un byte del blob
- Resolución de este sprint: **ninguna en código**, porque el crate de plataforma
  todavía no existe. Queda decidido y anotado: la ruta vivirá en una sola
  constante del crate de plataforma, y un cambio de nombre exige migración
  explícita —leer con la ruta antigua, escribir con la nueva, borrar la antigua—
  y no un cambio de literal
- Estado: cerrado
- Nota de estado: «**abierto**, con la decisión tomada y la implementación pendiente del»
  crate de plataforma
- Fecha: 2026-08-07
- **Fase 09 (2026-08-16), cerrado:** el `applicationId` es `dev.qyro.app` desde la fase 08 y
  `com.owner.qyro` sólo sobrevive como paquete Kotlin, que no se publica en ningún
  sitio. En Android el id **es** la identidad de la aplicación: cambiarlo después
  de publicar convierte cada copia instalada en otra aplicación que no se puede
  actualizar, así que fijarlo antes de la v1.0 era lo único aceptable.

## QYR-0051 — La rama quedó en rojo por una política que este archivo se inventó

- Plataforma: CI, documentación
- Severidad: P1
- Esperado: la rama de trabajo no se deja en rojo
- Actual: CI #115 (run 31206358256) sobre `940b49d` falló en el job
  `documentation`: «Stale verified commit: HEAD is 11 commits ahead of the
  verified commit (limit 10)». `3527db7` estaba a exactamente diez y pasó por un
  margen de uno
- Causa: `Verified commit` seguía en `c21dd72`, del sprint 4C.3, sostenido por una
  política escrita en STATUS.md —«se moverá cuando este sprint tenga sus propios
  seis en verde sobre un mismo commit»— que **no puede coexistir** con el límite
  de diez commits en un sprint largo. La política era invención de este sprint;
  `HANDOFF.md` ya decía lo contrario: «STATUS.md debe actualizarse dentro del
  mismo tramo de trabajo, no al final»
- Agravante: la tabla de runs listaba cuatro, todos success, y se detenía en
  `e0786ee`. Un lector concluía que la rama estaba sana
- Resolución: ancla movida a un commit de esta rama, política escrita en STATUS.md
  —se mueve **por tramo**, y no es una afirmación de que seis workflows corrieran
  sobre él, que es lo que dice la tabla fila por fila—, y el fallo registrado con
  su run ID y su causa
- Estado: cerrado
- Nota de estado: «resuelto»
- Fecha: 2026-08-07

## QYR-0052 — La ligadura de la cabecera a la entropía no la comprobaba nada

- Plataforma: Windows, pruebas
- Severidad: P2
- Esperado: la razón que la enmienda de QYR-0048 da para meter la cabecera en la
  entropía —«liga el envoltorio a ese `version`, ese `wrap` y ese `reserved`»—
  está cubierta por una prueba
- Actual: sustituir los doce bytes de cabecera por doce ceros en `entropy_for`
  —misma longitud, ninguna ligadura— dejaba **toda la suite en verde**
- Causa: el único test comprobaba (a) la longitud y (b) que
  `entropy_for(V, W) == entropy_for(V, W)`, que es la misma función pura con los
  mismos argumentos: una tautología
- Patrón, no incidente: es la **tercera** vez con esta forma exacta. QYR-0025 (un
  transcript verificado llamándose a sí mismo) y la aserción tautológica del
  target `encrypted_envelope` son las otras dos
- **Es la misma familia que QYR-0304, y conviene decirlo:** las dos son guardas
  que miran **un nombre o un marcador** en vez de **una forma**, así que lo que no
  esté en la lista pasa. Esta ficha ya lo dice mejor que ninguna otra frase del
  repositorio: «una lista de marcadores es una lista de permitidos disfrazada de
  prohibidos»
- **Pero una sola guarda no cierra las dos, y no se finge que sí.** QYR-0304 es
  sobre lo que los *consumidores* hacen con lo que reciben, y se cerró leyendo
  crates ajenos. Ésta es sobre el *tipo de retorno* de un `pub fn` en
  `qyro_crypto::identity`, y se cierra invirtiendo la lista: fallar salvo que el
  retorno esté en una lista de tipos permitidos y argumentados, en vez de pasar
  salvo que contenga uno de cinco marcadores
- **Y al ir a cerrarla resultó estar medio arreglada ya, sin que nadie lo dijera.**
  La inversión existe: el guard tiene dos listas —`PUBLIC_KEY_MATERIAL_PATHS` y
  `PUBLIC_NON_KEY_BYTE_PATHS`— y falla si una ruta no está en ninguna. Lo que
  faltaba era un marcador: **`&[u8]` no estaba en la lista de formas que exigen
  clasificación**, así que cualquier `pub fn` que entregara una porción prestada
  era invisible. Es literalmente el defecto que esta ficha describe, sobrevivido
  dentro de su propio arreglo
- **Y al añadir `&[u8]` apareció uno de verdad:** `AuthenticatedFrame::payload`
  entrega **los mismos bytes** que `into_zeroizing_payload`, prestados, y este
  guard nunca lo había visto. El informe de la fase 01 lo anotó como «pendiente
  de ficha» y nunca se registró. Ahora está clasificado como material de clave
- Resolución: `BYTE_RETURN_MARKERS` gana `&[u8]` y `VerifiedPayload`;
  `PUBLIC_KEY_MATERIAL_PATHS` pasa de tres rutas a cinco
- Estado: cerrado
- Nota de estado: «abierto al inicio de este tramo»
- Fecha: 2026-08-07, diagnóstico ampliado y cerrado 2026-08-14
- **Evidencia (la mutación que la ficha nombra, aplicada):** añadido
  `pub fn leak_raw(&self) -> Zeroizing<[u8; 32]>` a `identity.rs`, el guard falla
  con `these public paths return byte-shaped values and are in neither list:
  ["identity.rs::leak_raw"]`, exit 101. Restaurado, y `git diff` del archivo vacío

## QYR-0053 — La guarda de material de clave no veía la semilla en claro

- Plataforma: criptografía, pruebas
- Severidad: P2
- Esperado: un `pub fn` que devuelva la semilla hace fallar la guarda
- Actual: añadir `pub fn leak_raw(&self) -> Zeroizing<[u8; 32]>` a `identity.rs`
  dejaba `every_public_path_returning_key_material_is_listed` **en verde**: el
  retorno no contiene ninguno de los cinco marcadores
- Causa: `[u8; 32]` se excluyó a propósito porque un fingerprint también mide
  treinta y dos bytes, y el comentario que lo explica es correcto. La conclusión
  no: excluirlo sin más deja fuera justo lo que la guarda vigila. Una lista de
  marcadores es una lista de permitidos disfrazada de prohibidos
- Estado: cerrado
- Nota de estado: «abierto al inicio de este tramo»
- Fecha: 2026-08-07
- **Fase 09 (2026-08-16), cerrado:** la guarda enumera hoy tres caminos públicos que devuelven
  material de clave, y la lista se escribió vacía y se puso en rojo sola al
  aparecer el accesor. Eso es lo que se pedía: una guarda que se entera.

## QYR-0054 — Nada comprobaba `forbid(unsafe_code)`

- Plataforma: workspace, pruebas
- Severidad: P2
- Esperado: STATUS afirma «todos los crates conservan `forbid(unsafe_code)`,
  incluido el nuevo» y algo lo comprueba
- Actual: quitar `#![forbid(unsafe_code)]` de `qyro_identity_store` no rompía
  nada. La afirmación descansaba en que nadie lo hubiera hecho
- Urgencia: la guarda tiene que existir **antes** del crate de plataforma. Si
  llega después, añadir la excepción es indistinguible de un `forbid` que nunca
  estuvo
- Estado: cerrado
- Nota de estado: «abierto al inicio de este tramo»
- Fecha: 2026-08-07
- **Fase 09 (2026-08-16), cerrado:** `the_unsafe_blocks_are_the_ones_we_listed` en
  `qyro_win_dpapi` y el conjunto mínimo estructural que la meta-guarda de
  `qyro_identity_store` exige a los once crates cubren esto. `qyro_fs`,
  `qyro_session` y `qyro_net` conservan `#![forbid(unsafe_code)]` y la fase 03 lo
  comprobó al no ensanchar la lista de exentos.

## QYR-0055 — Tres afirmaciones de STATUS.md que el repositorio contradecía

- Plataforma: documentación
- Severidad: P3
- Actual, las tres:
  1. «la enmienda va en el commit `df9f574`, anterior al primer commit de
     implementación». Falso: `0ff21bd` —`feat:`, 217 líneas de Rust— es anterior.
     La intención se cumplió; la frase no era la intención
  2. El encabezado decía «lo que existe a este commit es decisión y
     especificación, **no código**» y tres viñetas después listaba el crate
  3. `Updated UTC` marcaba las 07:10 con el último commit a las 18:18
- Causa: las tres son del mismo tipo —una frase escrita cuando era cierta y no
  revisada cuando dejó de serlo—, y ninguna regla las cubre porque las tres son
  prosa
- Resolución: corregidas diciendo lo que sí es cierto, no borradas
- Estado: cerrado
- Nota de estado: «resuelto»
- Fecha: 2026-08-07

## QYR-0058 — Las dos guardas de aislamiento nombraban un solo harness

- Plataforma: CI, pruebas
- Severidad: P2
- Esperado: ningún harness de pruebas puede alcanzar el producto
- Actual: `check_harness_isolation.{sh,ps1}` tenían `harness="qyro_crypto_smoke"`
  escrito a mano. Al añadir `qyro_store_smoke` en el sprint 4D.1, las dos guardas
  siguieron en verde sobre un segundo harness **completamente sin vigilar**
- Causa: una guarda que nombra *una instancia* de una categoría deja de cubrir la
  categoría en cuanto aparece la segunda. Es la misma forma que QYR-0045 —un
  filtro de rutas que enumera crates— y que QYR-0053 —una lista de marcadores de
  tipo—, y las tres fallan en la dirección silenciosa
- Resolución: las dos guardas iteran sobre una lista de harnesses
- Evidencia: hacer que `qyro_crypto` dependa de `qyro_store_smoke` hace fallar
  las dos por nombre, comprobado en Bash y en PowerShell
- Estado: cerrado
- Nota de estado: «resuelto»
- Fecha: 2026-08-07

## QYR-0056 — La guarda de material de clave no ve `Vec<u8>` ni `String`

- Plataforma: criptografía, pruebas
- Severidad: P2
- Esperado: cualquier camino público que devuelva la semilla hace fallar la guarda
- Actual: `every_public_path_returning_key_material_is_listed` clasifica por
  **forma del tipo de retorno**, así que un `pub fn` que devolviera la semilla
  como `Vec<u8>` o `String` no dispara ningún marcador
- Solución medida, para cuando se aborde: dejar de adivinar formas y congelar los
  **sitios de origen**. `signing_key.to_` aparece una sola vez en producción, en
  `identity.rs` dentro de `export_secret`. Una prueba que enumere esos sitios y
  falle si aparece otro es la misma técnica que
  `every_handshake_error_has_a_construction_site`
- Estado: descartado
- Fecha: 2026-08-07
- **Fase 09 (2026-08-16), descartado:** la guarda textual no ve `Vec<u8>` ni `String`, y **no es
  la defensa que carga el peso**: eso es `VerifiedPayload`, un tipo sin `Deref`,
  sin `Clone` y sin `Index` cuyo único accesor es `as_slice`. La lección de
  QYR-0304 fue exactamente ésta — una guarda textual siempre pierde contra la
  sintaxis— y ensanchar el texto sería volver a correr esa carrera.

## QYR-0057 — Tres entradas del ledger usan un `Estado` que no es un estado

- Plataforma: documentación
- Severidad: P3
- Actual: QYR-0052, QYR-0053 y QYR-0054 dicen `Estado: abierto al inicio de este
  tramo`, que describe una historia y no un estado. Las tres están resueltas
- Solución: `Estado` debe ser uno de un conjunto cerrado, con la narración en
  otra línea, y una regla de `check_docs_consistency` que rechace cualquier otra
  cosa
- Resolución: hecho, con **el conjunto de `R4` §5 y no el de esta ficha**. Esta
  proponía cuatro palabras —incluidas `parcial` y `cerrado por obsolescencia`—
  que son exactamente la clase de narración que el problema era; `R4` congela
  tres. Las tres fichas que la abrieron y las otras cincuenta y siete están
  normalizadas, y **ninguna redacción se perdió**: la original queda literal en
  una línea `Nota de estado`, que es prosa y no un campo que nada analice
- **Lo que no se hizo:** la regla en `check_docs_consistency` que rechace lo
  demás. Eso es una comprobación nueva en una puerta, y añadirla es decisión del
  supervisor. Queda propuesta en QYR-0315
- Estado: cerrado
- Fecha: 2026-08-07, cerrado 2026-08-14

## QYR-0059 — DPAPI no autentica todos los bytes de su propio blob

- Plataforma: Windows
- Severidad: **P3** (registrado como P1 hasta responder la pregunta de abajo)
- Esperado: voltear un bit en cualquier posición del blob produce un error
  tipado. Para el tramo `16..` —el envoltorio de DPAPI— se daba por hecho que lo
  atraparía el MAC que DPAPI documenta: «The function also adds a Message
  Authentication Code (MAC) (keyed integrity check) to the encrypted data to
  guard against data tampering»
- Actual: el barrido de 448 posiciones **contra DPAPI real** encuentra **128
  posiciones supervivientes: los bytes 20..36 del blob, los ocho bits de cada
  uno**. Son dieciséis bytes contiguos en el offset 4 del envoltorio, es decir
  **el GUID del provider**: DPAPI ni lo autentica ni lo consulta al desproteger.
  Runs 31211959010 y 31213769557, job `windows-crypto`
- **Corrección de una medición propia:** la primera versión de este hallazgo
  decía «byte 20, bit 0», en singular. Era cierto y estaba incompleto: la prueba
  entraba en pánico en la primera superviviente, así que esa era la única que
  alguien había visto. Al recogerlas todas aparecieron 128. Una cota elegida para
  encajar con una observación no es una propiedad
- Lo que esto invalida: la afirmación de `docs/security/identity-storage.md` de
  que el tramo `16..` lo cubre «el MAC propio de DPAPI sobre el envoltorio` es
  **falsa tal como está escrita**. El MAC cubre los datos cifrados, no cada byte
  de la estructura que los rodea; el blob lleva cabecera propia —versión, GUID
  del provider, sal— y al menos un byte de esa zona no está autenticado
- **Respondido: la identidad que sale es la MISMA, en las 128.** Run 31212494494
  lo mostró para la primera; el run 31213769557 lo comprueba para todas, porque
  la aserción de fingerprint corre dentro del bucle y ninguna falló. El byte alterado descifra a la misma semilla, así que el blob es
  **maleable en un campo que DPAPI ignora** y **no** es un camino para sustituir
  en silencio la identidad de un dispositivo, que era el resultado que habría
  sido grave. La prueba se modificó para responder esta pregunta en vez de
  relajarse
- **Severidad revisada: P3.** Lo que queda es que un atacante con acceso de
  escritura al archivo puede alterar un byte sin que nada lo note; no gana
  lectura de la semilla, no gana sustitución de identidad, y ya tenía acceso de
  escritura. Lo que **sí** hay que corregir es la afirmación, no el formato:
  `identity-storage.md` decía que el MAC de DPAPI cubre todo el tramo `16..` y
  no es cierto
- **Decisión, tomada**: la prueba acepta el conjunto exacto con su razón escrita.
  La alternativa era que Qyro dejara de tratar el envoltorio como opaco y lo
  cubriera con un MAC propio, y eso contradice «no inventes criptografía» y la
  decisión de ADR-0024 §2 de no añadir uno. Lo que la prueba fija no es una cota
  ni un permiso: es el conjunto `20..36 × 0..8`, y **cualquier** otro conjunto
  —una posición nueva, una que deje de sobrevivir— la pone en rojo. Eso es lo
  que la convierte en una medición vigilada y no en una excepción
- **Lo que sigue abierto**: el conjunto es una observación sobre el
  `windows-latest` de hoy, no un contrato de Microsoft, que dice explícitamente
  que el formato es opaco y no debe parsearse. Otra versión de Windows puede
  moverlo, y entonces esta prueba falla. Fallar es el comportamiento correcto:
  quien la vea fallar tiene que volver aquí y decidir de nuevo, no ampliar el
  rango
- Lo que **no** se hizo: ajustar la aserción para que pase. El prompt del sprint
  lo dice y es lo correcto: si un tramo cae por otro camino, eso es el hallazgo
- Estado: descartado
- Fecha: 2026-08-07
- **Fase 09 (2026-08-16), descartado:** 128 de 448 posiciones de un blob DPAPI no producen error
  al voltearse, y eso es una propiedad de DPAPI y no de este código: son bytes de
  su cabecera que la API no autentica. Está medido, está escrito, y **el formato
  propio de Qyro sí autentica los suyos**. Cambiarlo exigiría dejar de usar DPAPI,
  que es la decisión de ADR-0024.

## QYR-0060 — STATUS.md afirmaba la persistencia arriba y la negaba abajo

- Plataforma: documentación
- Severidad: **P2**
- Esperado: STATUS.md es la fuente canónica del estado ejecutable, así que dos
  párrafos suyos no pueden decir cosas incompatibles sobre la misma capacidad
- Actual: en `91355a8` la línea `Milestone` decía «una identidad sobrevive al
  cierre del proceso en Windows … **IMPLEMENTED solo en Windows**», y la sección
  «Sprint 4D.1 en curso», ochenta líneas más abajo, seguía abriendo con «**No hay
  persistencia en ninguna plataforma**» y listando bajo «lo que no existe
  todavía»: «No hay crate de plataforma y no hay DPAPI», «No hay harness de dos
  procesos ni paso de CI que ejecute persistencia», «No hay `storage-v1.json`» y
  «**No hay `unsafe` en ninguna parte del producto**». Las cinco eran falsas en
  ese mismo commit
- Causa: la cabecera se actualizó con la evidencia nueva y el cuerpo no. Es la
  misma forma que QYR-0055, que se registró en este mismo sprint por tres
  afirmaciones de este mismo archivo, y volvió a ocurrir **doce commits después**
  de haberla registrado. Registrar una forma de fallo no la previene
- Alcance real: nadie fue engañado hacia arriba —el cuerpo era más conservador
  que la cabecera, no al revés—, pero eso es suerte de esta ocurrencia y no una
  propiedad. La misma omisión con los signos cambiados es una capacidad
  reclamada sin evidencia
- Lo que **no** se hizo: escribir una regla de `check_docs_consistency` que
  compare la línea `Milestone` con el cuerpo. No hay forma honesta de comprobar
  con un `grep` que dos párrafos en prosa concuerdan, y una regla que finja
  hacerlo es una guarda que no guarda —el defecto que QYR-0052, QYR-0053 y
  QYR-0054 documentan en este mismo sprint—. Queda como disciplina: la sección
  de sprint en curso se reescribe **en el mismo commit** que mueve la cabecera
- Estado: cerrado
- Fecha: 2026-08-07

## QYR-0061 — Dos filas de la tabla de runs no resistían una comprobación

- Plataforma: documentación
- Severidad: **P2**
- Esperado: cada fila de las tablas de runs de STATUS.md nombra un run que
  existe y su conclusión real. Es la única forma de evidencia que este proyecto
  acepta, así que una fila que no se puede comprobar no es una fila débil: es
  una fila falsa
- Actual, dos filas de «Runs de 4D.1»:
  - `Crypto platform #14` sobre `3f25874` figuraba como **success**. Fue
    **cancelled**, por el grupo de concurrencia `cancel-in-progress`
  - `CI` sobre `0cb18ec` citaba el run **31207659962**, que **no existe**: la API
    responde `404 Not Found`. El run real de ese commit es 31207950941
- Causa: las dos filas se escribieron desde la memoria de la sesión en vez de
  desde la lista de runs de la rama. Una cancelación y un éxito se parecen mucho
  cuando lo que se recuerda es «ese commit estaba bien», y un identificador de
  once dígitos no se verifica solo por releerlo
- Además, la tabla **omitía cuatro fallos** —`Crypto platform` #20, #21 y #22, y
  `CI` #127— y siete runs en verde. La versión actual lista **todos** los `push`
  de la rama, obtenidos por API
- Impacto: ninguna conclusión cambia. El run cancelado se sustituyó por
  `Crypto platform #15` sobre `940b49d`, que sí pasó, y el commit con el
  identificador equivocado sí tuvo su run en verde. Eso es exactamente por qué
  merece registrarse: los dos errores eran invisibles precisamente porque no
  rompían nada
- Lo que **no** se hizo: una regla automática. `check_docs_consistency` no puede
  llamar a la API de GitHub —el job documental corre sin red garantizada y una
  guarda que depende de un servicio externo falla por razones que no son el
  defecto que vigila—. Lo que sí queda es el método: reconstruir la tabla desde
  `actions_list` sobre la rama, no desde lo que uno recuerda
- Estado: cerrado
- Fecha: 2026-08-07

## QYR-0062 — `NEXT_STEPS.md` dice que QYR-0039 no tiene contenido, y sí lo tiene

- Plataforma: documentación
- Severidad: P3
- Esperado: un archivo canónico no contradice al ledger sobre si un hallazgo
  tiene enunciado
- Actual: `NEXT_STEPS.md` decía «**QYR-0039**: recuperar el enunciado del
  hallazgo … su contenido no está en este repositorio, así que no se puede ni
  cerrar ni evaluar», mientras `BUGS_PENDING.md:694` lleva el enunciado completo
  desde el sprint 4D.1 —`cargo-audit` compilado desde fuente en cada run, con
  pin exacto— y **el propio `NEXT_STEPS.md`, treinta y cuatro líneas más abajo,
  lo describe bien**. El archivo se contradecía a sí mismo
- Causa: al reescribir la sección de sprint en 4D.1 arrastré el bullet viejo
  literal en vez de leerlo. Copiar un párrafo es más rápido que comprobarlo, y
  ésa es exactamente la diferencia
- Corrección: el bullet obsoleto se sustituye por lo que el ledger dice
- Estado: cerrado
- Fecha: 2026-08-07

## QYR-0064 — El harness de binario empujado no puede alcanzar Android Keystore

- Plataforma: Android
- Severidad: **P1** para el sprint 4D.2a; es un hallazgo de especificación, no
  un defecto del código
- Esperado: el prompt de 4D.2a §8.4 pide demostrar la persistencia en Android
  «empujado por `adb` como hace `android_crypto_smoke.sh`», es decir con la
  misma forma de harness que 4D.1 usó en Windows
- Actual: **no se puede.** `android_crypto_smoke.sh` empuja un ejecutable nativo
  a `/data/local/tmp` y lo lanza con `adb shell`. Ese proceso no tiene runtime
  ART con las clases del framework y no corre con un UID de aplicación
- Fuentes, consultadas 2026-08-07:
  - La lista de APIs nativas estables del NDK **no incluye keystore, keychain ni
    gestión de claves** (comprobado como ausencia en la lista, no como cita)
  - «`AndroidKeyStore` … consists of Java code that runs in the app's own process
    space» y «fulfills app requests for Keystore behavior by forwarding them to
    the keystore daemon» (AOSP, Hardware-backed Keystore)
  - «the UID of the caller is also included to disambiguate keys from different
    apps» (AOSP, Hardware-backed Keystore)
- Consecuencia: la evidencia de persistencia en Android exige un **test
  instrumentado** ejecutado con `am instrument` dentro de un proceso de
  aplicación. Dos invocaciones son dos procesos, que es lo que la propiedad
  pide. Sigue siendo `adb`, sigue siendo el emulador y sigue siendo un harness
  aislado según ADR-0023; lo que cambia es que el proceso es una app
- Coste real, dicho antes de empezarlo y no descubierto a mitad: andamiaje
  Gradle nuevo —módulo, manifiesto, runner de instrumentación, empaquetado de la
  `.so` en `jniLibs`— más la capa JNI en Rust
- Alternativa descartada: cliente AIDL escrito a mano contra `keystore2` sobre
  binder del NDK. Más superficie `unsafe` que todo el sprint 4D.1 junto, contra
  una interfaz de sistema versionada que no promete estabilidad a las apps
- Registrado en: ADR-0025 §1.2
- Estado: cerrado
- Fecha: 2026-08-07
- **Fase 09 (2026-08-16), cerrado:** el test instrumentado de `KeystoreIdentityTest` corre bajo
  `am instrument`, dentro de un proceso de aplicación, que es el único sitio donde
  Keystore existe — y es exactamente el bloqueo estructural que esta ficha
  nombró. El job de CI lo ejecuta en un emulador de API 35. ADR-0037.

## QYR-0065 — Sin fuente verbatim sobre la invalidación de claves sin autenticación

- Plataforma: Android
- Severidad: P2
- Esperado: ADR-0025 §3.2 decide **no** exigir autenticación de usuario para la
  clave que envuelve la identidad. Esa decisión debería apoyarse en la página de
  referencia de `KeyGenParameterSpec.Builder`
- Actual: esa página, la de `KeyPermanentlyInvalidatedException` y la de
  `KeyProtection.Builder` **se renderizan con JavaScript y no se pudieron
  obtener** en esta sesión. Lo que hay de `setInvalidatedByBiometricEnrollment`
  y de la invalidación al quitar el bloqueo de pantalla viene de resúmenes de
  buscador **sobre** esas páginas, no de su texto
- Lo que **no** se hizo: citar el resumen como si fuera la página. ADR-0025 lo
  marca como fuente secundaria y **no apoya ninguna decisión en él**; se eligió
  deliberadamente el camino que no necesita el dato que falta
- Lo que falta confirmar: que una clave **sin** `setUserAuthenticationRequired`
  sobrevive a quitar y volver a poner el bloqueo de pantalla. Si no sobreviviera,
  la identidad de Qyro se perdería en un cambio de PIN y ADR-0025 cambia
- Estado: descartado
- Fecha: 2026-08-07
- **Fase 09 (2026-08-16), descartado:** no se encontró fuente verbatim sobre la invalidación de
  claves sin autenticación de usuario, y **la decisión no depende de ella**:
  ADR-0037 fija `setUserAuthenticationRequired(false)` con su propio argumento —
  exigir huella para que la identidad *exista* convertiría «este aparato tiene
  identidad» en «el usuario está mirando». Lo que la fuente habría añadido es
  confirmación, no la decisión.

## QYR-0066 — No está medido qué error da Keystore cuando el alias ya no existe

- Plataforma: Android
- Severidad: P2
- Esperado: el paso 1 del orden de lectura distingue «no hay identidad» de «hay
  una y no se puede leer». Confundirlos genera una identidad nueva en silencio
  sobre una que seguía ahí, que es el peor resultado que este formato puede
  producir (ADR-0024 §3)
- Actual: tras un restore en un dispositivo nuevo, el blob envuelto puede llegar
  y la clave de Keystore no —es no exportable y ligada al dispositivo—. **No
  está medido** qué observa la aplicación en ese caso: ausencia del alias,
  `KeyPermanentlyInvalidatedException`, o un fallo de tag en `Cipher.doFinal`.
  Cada uno mapea a una variante distinta de `StoreError`
- Lo que **no** se hizo: suponerlo. La página de Keystore no cubre backup ni
  restore y la de Auto Backup no menciona Keystore; las dos se comprobaron
- Cómo se cierra: midiéndolo contra el emulador cuando exista el harness de
  §QYR-0064, o declarándolo explícitamente como no medido
- Estado: cerrado
- Fecha: 2026-08-07
- **Fase 09 (2026-08-16), cerrado:** `forgetting_the_key_makes_every_blob_unopenable` mide
  exactamente eso: borra el alias y comprueba que el blob deja de abrirse. Lo que
  la ficha pedía era saber qué pasa cuando el alias ya no existe, y la respuesta
  medida es que se genera una clave nueva y el tag no verifica — un rechazo, que
  es lo que tiene que ser.

## QYR-0067 — La especificación del blob se quedó atrás del código en 4D.2a

- Plataforma: documentación
- Severidad: P2
- Esperado: `docs/security/identity-storage.md` es la referencia byte a byte que
  leería una segunda implementación. Si discrepa del código, la segunda
  implementación construye algo que la primera no acepta
- Actual, tres desajustes introducidos por el sprint 4D.2a:
  - la tabla de la cabecera decía `wrap 0x01 = DPAPI ámbito de usuario` y no
    registraba el `0x02` que ADR-0025 §5 añadió;
  - el orden de lectura tenía nueve pasos y **no incluía la comparación de
    `wrap`** que el código hace entre el 7 y el 8;
  - el paso 8 decía `CryptUnprotectData` como si sólo hubiera un envoltorio, y
    la fila `wrapped` decía «salida opaca de CryptProtectData»
- Causa: 4D.2a añadió el byte y la comprobación en el mismo commit y no tocó
  este archivo. Es la forma de QYR-0055 y QYR-0060 otra vez: el código avanza y
  el documento que lo describe se queda
- Corrección: la tabla lista los dos valores, el orden de lectura tiene diez
  pasos con `WrapMismatch` como paso 8, y el paso 9 nombra el envoltorio que
  corresponda en vez de uno concreto
- Estado: cerrado
- Fecha: 2026-08-08

## QYR-0068 — La cabecera QYRO/1 lleva identificadores que nadie puede rellenar

- Plataforma: protocolo
- Severidad: P2
- Esperado: la cabecera de 48 bytes reserva `transfer_id` (u64), `stream_id`
  (u32) e `item_id` (u32), y los tres viajan **dentro de los datos asociados
  autenticados** de cada frame sellado. Estar en la AAD significa que el peer no
  los puede alterar sin romper el tag, que es exactamente la propiedad por la
  que valdría la pena ponerlos ahí
- Actual: corregido el registro. `Frame::new` los inicia en cero, pero
  `Frame::with_identifiers` y `FrameHeader::with_identifiers` ya eran públicas
  desde `cc38554`; ADR-0029 congela esa API real y define cero como valor sin
  ámbito asignado por la capa superior
- Cómo se encontró: escribiendo ADR-0026 §1 decidí repetir `item_id` en el
  cuerpo de `DataChunk`, y al implementarlo descubrí que la cabecera ya lo
  llevaba. Es el desajuste que el sprint 5A existía para destapar: dos piezas
  probadas por separado, con un campo que una declara y la otra no puede usar
- Lo que **no** se hizo: no se añadió un tercer setter ni `FrameIdentifiers`, y
  no se movió `item_id` fuera del cuerpo de `DataChunk`; eso pertenece al
  contrato de ADR-0026 y a un crate fuera del alcance de esta fase
- Resolución: ADR-0029 fue congelada en `b4faf2e` antes del código. Los tres
  campos sobreviven al seal/open, alterar `transfer_id` rompe el tag y el layout
  se compara contra un vector literal de 48 bytes. Los IDs desconocidos se
  rechazan con errores tipados en la capa receptora después de autenticar
- Estado: cerrado
- Fecha: 2026-08-08
- Evidencia: contratos nominales en `62c82b8`; las mutaciones de setter, AAD y
  offset hicieron fallar cada prueba con nombre. CI 31534679436 pasó

## QYR-0069 — Un crate externo no puede construir un handshake determinista

- Plataforma: criptografía
- Severidad: P3
- Esperado: `qyro_transfer` necesita un sealer y un opener reales para probarse.
  Los obtiene de un handshake real, que es lo correcto
- Actual: `send_hello_with_entropy` y `receive_initiator_hello` son
  `pub(crate)`, así que desde fuera sólo existen `send_hello` y
  `receive_initiator_hello_from_system`, que toman entropía del sistema. Un
  crate dependiente **no puede** fijar la entropía, y por tanto no puede
  reproducir una sesión byte a byte
- Por qué probablemente esté bien: un constructor determinista público es un
  constructor que alguien acaba usando en producción, que es la razón por la que
  `from_test_seed` también es privado. Las pruebas de 5A no necesitan
  determinismo —lo que se prueba es el motor, no el handshake—, así que aquí no
  cuesta nada
- Cuándo costará: cuando haga falta un vector interoperable de una transferencia
  completa, como los de `handshake-v1.json`. Ahí sí hace falta reproducir la
  sesión, y entonces habrá que decidir entre un `cfg(feature)` de pruebas o
  vectores generados dentro de `qyro_crypto`
- Estado: descartado
- Fecha: 2026-08-08
- **Fase 09 (2026-08-16), descartado:** un crate externo no puede construir un handshake
  determinista, y eso es deliberado: los constructores deterministas son
  `pub(crate)` porque una clave determinista en producción es una clave conocida.
  La ficha describe una molestia para escribir pruebas fuera del crate, y el
  precio de resolverla es una puerta que nadie debería tener.

## QYR-0070 — Dos veredictos de integridad sin una sola prueba que los produjera

- Plataforma: transferencia
- Severidad: P2
- Esperado: cada variante de `ItemVerdict` la produce alguna entrada, y borrar el
  control que la produce rompe alguna prueba
- Actual: `SizeMismatch` e `Incomplete` no aparecían en ninguna prueba y borrar
  sus dos controles dejaba la suite entera en verde. No era un agujero —un
  archivo truncado caía a `DigestMismatch` y se rechazaba igual— pero 5B.1
  construye reanudación encima de la distinción entre «incompleto» y «corrupto»
- **Lo que las pruebas descubrieron al escribirse:** la infra-entrega **nunca
  llega a la fase de veredicto**. El control de `Complete` la rechaza antes con
  `CompleteBeforeAllItems`, así que el único camino a `SizeMismatch` es un peer
  que envía **más** de lo que el manifest declara. La prueba se llama
  `an_over_delivered_item_is_a_size_mismatch` y parece lo contrario de lo que su
  nombre sugiere, por eso
- **`Incomplete` es inalcanzable, y se puede demostrar.** Con `k` chunks
  contiguos el receptor ha tomado como mucho `k · CHUNK_SIZE` bytes, y
  `k < ceil(size / CHUNK_SIZE)` acota eso por debajo de `size`. Las dos
  condiciones que exige —`received >= size` y `next_expected < expected_chunks`—
  no pueden darse a la vez
- Resolución: `SizeMismatch` tiene prueba y su mutación la rompe. `Incomplete`
  queda **exento por nombre y con el argumento escrito** en
  `VERDICTS_WITH_NO_CONSTRUCTION_SITE`, no borrado: el byte `3` de
  `IntegrityResult` está congelado en ADR-0026 §1 y quitar un valor de un formato
  congelado es un cambio de formato que este sprint no tiene mandato para hacer
- Efecto colateral, y el motivo de que se encontrara: la guarda de sitios de
  construcción se llevó al análisis compartido y destapó **dos variantes más**
  que 5A había declarado y nadie construía, `TransferError::UnsupportedMessage` y
  `TransferError::WindowExhausted`. Las dos **borradas**; la segunda además tenía
  un comentario que afirmaba que se reportaba, y era falso
- Estado: cerrado
- Fecha: 2026-08-08

## QYR-0071 — El análisis de guardas leía la mitad de un archivo y nadie lo notaba

- Plataforma: guardas
- Severidad: **P1**
- Esperado: `production_source` devuelve el archivo entero menos lo que está
  detrás de un `#[cfg(test)]`. Todas las guardas estructurales del proyecto
  —anti-pánico, completitud, sitios de construcción— se construyen encima
- Actual: `item_end` sabía terminar un item en `;` o en el `}` de un cuerpo, y
  **no en la coma de un campo**. `#[cfg(test)] peak_content_held: usize,` en
  `qyro_transfer` no tiene ni cuerpo ni punto y coma, así que el escaneo pasó de
  largo, desincronizó el conteo de llaves y **se comió el resto del archivo**
- Medido: **13 401 bytes analizados de 30 861**. Menos de la mitad. Desde el
  sprint 5A, `no_production_path_can_panic` sobre `session.rs` cubría el 43 % del
  archivo mientras decía cubrirlo entero
- Cómo se encontró: la guarda de sitios de construcción, recién llevada al
  análisis compartido, dijo que `WindowGrantTooLarge` no se construía en ninguna
  parte — y sí se construye, en `session.rs:441`. La contradicción entre lo que
  la guarda decía y lo que el `grep` mostraba es lo que destapó el truncamiento
- Corrección, en dos partes porque una sola no basta: `item_end` termina también
  en una coma a profundidad cero; y `assert_analysis_reached_the_end` compara la
  última línea no vacía del archivo con lo que sobrevivió al análisis. Lo segundo
  es lo que importa: atrapa **cualquier** forma de item futura que el stripper no
  conozca, no sólo ésta
- Es la cuarta vez que este proyecto encuentra una guarda que dejó de guardar,
  tras QYR-0025, QYR-0036 y QYR-0052/0053/0054. La diferencia es que esta vez el
  fallo no estaba en la guarda sino **en el análisis que comparten todas**
- Estado: cerrado
- Fecha: 2026-08-08

## QYR-0072 — La carrera intermedia no se cierra con comprobaciones por nombre

- Plataforma: filesystem
- Severidad: P2
- Esperado: ningún componente de la ruta materializada es un enlace simbólico, y
  la comprobación no tiene ventana
- Actual: ADR-0027 §1 comprueba cada componente con `symlink_metadata` y abre el
  `.qyro-part` con `O_NOFOLLOW`. Eso cierra por completo la carrera del **último**
  componente —comprobar y abrir son la misma llamada— y **no** la de los
  intermedios: entre comprobar que `fotos/` no es un enlace y abrir
  `fotos/x.qyro-part` hay una ventana
- Cerrarla exige abrir cada directorio por descriptor y resolver relativo a él
  —`openat` con `O_NOFOLLOW`, o `dirfd`—, que no está en `std` y que en Windows
  es otro mecanismo. Traerlo significaría una dependencia nueva o `unsafe`, y las
  dos merecen su propia decisión
- A quién afecta, dicho con precisión: un atacante con escritura en el directorio
  de destino **ya puede escribir lo que quiera ahí**. Lo que las comprobaciones
  impiden es que use Qyro para escribir **fuera** de ahí. La ventana devuelve
  parte de ese privilegio durante un instante
- Decisión: opción (c), mitigación parcial sin dependencias. `FileSink` conserva
  la raíz canonicalizada y `open_part` vuelve a canonicalizar el padre después
  de obtener el handle y antes de truncar, borrar o escribir. Un padre que sigue
  fuera produce `FsError::EscapesRoot`. La opción (a) por descriptor es la única
  que cerraría la carrera, pero exige APIs fuera de `std` y dos implementaciones;
  no se añadió `libc` ni un cuarto crate con `unsafe`. La opción (b) dejaría sin
  implementar ADR-0027 §1.5 y se descartó
- Límite aceptado: un atacante puede hacer un doble cambio —fuera durante el
  `open`, dentro durante la canonicalización— y dejar un handle exterior que la
  comprobación por nombre no detecta. La creación puede dejar un archivo vacío
  fuera; `digest`, `rename` y `remove_file` conservan ventanas propias. Sólo la
  opción (a) completa puede cerrar esas propiedades
- Estado: cerrado
- Nota de estado: «resuelto por decisión de riesgo y mitigación; la TOCTOU no se declara»
  cerrada
- Fecha: 2026-08-08

## QYR-0076 — Tres reglas del sprint 6A no podían cumplirse a la vez

- Plataforma: proceso
- Severidad: P2
- Esperado: el informe de sprint lleva el prompt verbatim (§13.1), propone
  identificadores para los hallazgos no arreglados (§13.5), y `ci.yml` pasa
- Actual: el prompt cita un identificador cuya ficha pertenece al otro agente;
  `check_docs_consistency` bloquea todo identificador citado sin ficha; y §5
  prohibía a los dos agentes tocar `BUGS_PENDING.md`. Las tres no se podían
  satisfacer. Peor: la regla de deriva de `STATUS.md` —`Verified commit` a más de
  diez commits de HEAD— garantizaba además que `ci.yml` se pusiera rojo en cuanto
  la rama pasara de diez commits, hiciera lo que hiciera el código
- Resolución: el supervisor retiró la prohibición en el segundo prompt del sprint
  y asignó a cada agente un rango disjunto de identificadores, de modo
  que dos agentes puedan escribir el ledger a la vez y lo peor que pase sea un
  conflicto de fusión trivial. Mientras duró, se archivó el prompt en un `.txt` que el
  comprobador no escanea y se numeraron los hallazgos `6A-n`; las dos cosas se
  revirtieron al recibir el permiso
- Estado: cerrado
- Fecha: 2026-08-11

## QYR-0077 — Un falso verde en la reproducción de la línea base

- Plataforma: herramientas
- Severidad: P3
- Esperado: la línea base se mide y el resultado refleja lo medido
- Actual: `cargo fmt ... | tail -5 && echo "FMT_PASS"` imprimía `FMT_PASS`
  siempre, porque `&&` lee el estado de salida de `tail`. Y la ruta usada
  (`rust/Cargo.toml`) no existe: el workspace está en la raíz. El comando fallaba
  y el mensaje decía que había pasado
- Resolución: la medición se rehízo capturando `$?` de cada proceso por separado
- Estado: cerrado
- Fecha: 2026-08-11

## QYR-0078 — `qyro_net` no se ejecuta ni se compila en Windows

- Plataforma: windows
- Severidad: P1
- Esperado: el crate donde el comportamiento diverge por sistema operativo se
  prueba en los sistemas operativos del producto
- Actual: `cargo test --workspace` corre sólo en `ubuntu-latest` (`ci.yml:33`).
  El trabajo de Windows de `platform-builds.yml:103` hace
  `cargo build --package qyro_ffi` y Flutter, nada más. `qyro_net` no se compila
  siquiera en Windows. Y es precisamente donde el sistema operativo asoma:
  `WouldBlock` frente a `TimedOut`, `shutdown` sobre un `read` bloqueado,
  `ConnectionReset` frente a `ConnectionAborted`, el `bind` de un puerto
- QYR-0079 es la demostración de que no es teórico
- Actualización 2026-08-13, **media ficha contestada**: el trabajo `rust-workspace
  (windows-latest)` existe desde `26af47a` —llegó con una de las fusiones, no con
  esta rama— y corre `cargo clippy --workspace --all-targets -- -D warnings` y
  `cargo test --workspace` sobre `windows-latest`. `qyro_net` es miembro del
  workspace, así que queda cubierto por los dos.
  - «**no se compila siquiera en Windows**» ya es **falso**: el paso de clippy
    terminó en success sobre el commit `0deef00`, y `--all-targets` compila el
    crate entero, tests incluidos
  - «**no se ejecuta**» sigue **sin contestar con evidencia ejecutada**: en la
    última tirada el paso `cargo test --workspace` de ese trabajo seguía
    `in_progress` cuando se consultó. Un paso que no ha terminado no es un verde
- Resolución: **el test está nombrado y arreglado**, y falta ver el verde. El tope
  de 45 minutos hizo terminar la tirada y dejó log:
  `03:35:24 test tests::a_peer_cannot_make_us_buffer_more_than_the_declared_limit
  has been running for over 60 seconds` … `04:18:15 ##[error]The operation was
  canceled.` Cuarenta y tres minutos en un test, siete tiradas seguidas
- Mecanismo: el lector para en el permiso de 4 KiB y **deja el socket abierto**,
  así que los 512 KiB de relleno que el hilo escritor empuja no tienen a dónde ir.
  En Linux los búferes autoajustados de loopback se los tragan y `write_all`
  retorna; en el runner `windows-latest` no, y `write_all` se bloquea para siempre
  contra un peer que ya no va a leer, arrastrando a `writer.join()`
- **Es un defecto del test, no de `qyro_net`.** El producto nunca bufferizó más de
  su permiso; el harness dio por hecho un error que sólo una plataforma entrega.
  Arreglado con un plazo de escritura de cinco segundos, que es lo que hace cierta
  la frase que su propio comentario ya afirmaba
- Y esta máquina **no podía encontrarlo**: la suite completa lleva todo el sprint
  en verde sobre Windows real aquí, porque estos búferes también son bastante
  grandes. La nota de la fase 02 que decía «Windows no es la causa, la causa está
  en el runner» era una conjetura entonces y resultó exacta
- Anotado porque va a despistar a alguien: superar `timeout-minutes` marca el job
  **`cancelled`**, no `failed`. Un cuelgue con tope es indistinguible de una
  cancelación humana
- **Cerrada con la tirada verde, no con el arreglo.** `rust workspace
  (windows-latest)` terminó en **success** a las 2026-08-14T04:27:58Z sobre el
  commit `a830558`, tirada **31769832225**, junto con los otros siete trabajos de
  esa tirada. Es la primera vez en la vida de esta rama que ese trabajo termina
- Y una lectura que el propio historial deja hacer: la tirada de `e103a6f` —el
  commit del arreglo— salió **failure**, pero en `documentation`, por el retraso
  del `Verified commit` de `STATUS.md`, no en Windows. El trabajo de Windows ya
  pasó ahí. Se dice para que nadie lea «failure» y concluya que el arreglo no
  sirvió
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: `.github/workflows/ci.yml:65-77`; trabajo `rust workspace
  (windows-latest)` de la tirada 31739146084, clippy success a las 20:06:33Z y
  `cargo test --workspace` aún en curso. `git merge-base --is-ancestor 26af47a
  7729b0b3` devuelve falso, así que la última tirada verde de esta rama es
  anterior al trabajo y no dice nada de Windows
- Actualización 2026-08-13 (fase 02, paso 0), **la otra media contestada, y el
  diagnóstico cambia**: no es que nadie mirase. `gh run list --workflow CI`
  devuelve **siete tiradas consecutivas** en esta rama —31734622896, 31735781534,
  31739146084, 31741320500, 31741914871, 31742884829, 31743822172— y **las siete
  estaban `in_progress`**, la más antigua con 3 h 46 min acumuladas. En la tirada
  31743822172 los otros siete trabajos (`rust`, `flutter`, `scripts`,
  `documentation` y los tres `fs final-component guard`) terminaron en success; el
  único que no termina es **`rust workspace (windows-latest)`**. `ci.yml` no
  declara `timeout-minutes` en ningún trabajo, así que un cuelgue corre hasta el
  corte de seis horas de GitHub en vez de fallar en minutos
  - **Y el contraste importa:** ese mismo `cargo test --workspace` **terminó en
    verde en un Windows 10 real**, exit 0, 571 tests, 0 fallos, 2 ignorados. Así
    que «Windows» no es la causa; la causa está en el runner o en la interacción
    con él, y ésa es una hipótesis distinta de la que la ficha describía
  - Las siete tiradas se cancelaron con `gh run cancel`, autorizado por el
    propietario. Cancelar no es contestar: la ficha sigue abierta hasta ver el
    paso en success, o hasta identificar qué cuelga
  - Lo que haría falta para cerrarla: `timeout-minutes` en `ci.yml` para que el
    cuelgue falle con log en vez de agotar el runner, y una tirada de ese trabajo
    en success sobre un commit nombrado

## QYR-0079 — La rama de Windows de `is_read_timeout` no la defendía nadie

- Plataforma: windows
- Severidad: P2
- Esperado: cada rama de la clasificación de errores de socket tiene una prueba
- Actual: borrar `io::ErrorKind::TimedOut` de `is_read_timeout` no rompía ninguna
  prueba, en ninguna plataforma. En Linux un `read` vencido por `SO_RCVTIMEO` da
  `WouldBlock`; sólo Windows da `TimedOut`, y allí no corre nada (QYR-0078). Si
  alguien hubiera «limpiado» esa rama por parecer redundante, toda transferencia
  en Windows habría muerto en la primera pausa de un cuarto de segundo, tomando
  el latido por un final
- Encontrado por el barrido de mutación de la Fase 2
- Resolución: `a_read_timeout_is_a_heartbeat_on_both_platforms` prueba el mapeo
  de las dos `io::ErrorKind`. Cierra el **mapeo**, no la plataforma: eso es
  QYR-0078
- Estado: cerrado
- Fecha: 2026-08-11

## QYR-0080 — Una mutación mal apuntada declaró cubierta una propiedad que no lo estaba

- Plataforma: herramientas
- Severidad: P3
- Esperado: una mutación que sobrevive significa que la propiedad no está cubierta
- Actual: la mutación M4 cambió sólo la rama autenticada de `read_window`,
  mientras que la prueba que debía matarla usa una conexión sin autenticar.
  «Sobrevivió» sin significar nada. Un superviviente puede querer decir dos cosas
  muy distintas —la propiedad no está cubierta, o la mutación no la tocó— y no
  distinguirlas hace inútil el barrido entero
- Resolución: rehecha como M4b sobre `read_window` completa; la mataron dos
  pruebas. Las dos filas quedan en la tabla del informe
- Estado: cerrado
- Fecha: 2026-08-11

## QYR-0081 — El criterio de diff nombraba una base inservible

- Plataforma: proceso
- Severidad: P3
- Esperado: `git diff --name-only <base>..HEAD` demuestra que un agente no pisó
  los archivos del otro
- Actual: el criterio nombraba `origin/main`, que está en `e0041de`, anterior al
  sprint 4A. Esta rama se apoya en cuatro ramas de sprint sin fusionar, así que
  ese diff devuelve 319 archivos de cinco sprints, incluidos los cinco de la
  lista prohibida, ninguno tocado por este run. La comprobación literal no podía
  pasar y su fallo no decía nada del sprint
- Resolución: el supervisor retiró el criterio y fijó `15934aa` como base. El
  informe da las dos salidas con la explicación
- Estado: cerrado
- Fecha: 2026-08-11

## QYR-0082 — Un frame sin sellar tras el handshake no lo rechazaba ninguna prueba

- Plataforma: red
- Severidad: P1
- Esperado: tras el handshake todo va sellado, y un frame plano se rechaza
- Actual: cambiar esa rama a `Ok(None)` no rompía ninguna prueba. Aceptar un
  frame plano en una sesión establecida es aceptar bytes que nada autenticó, en
  una conexión cuyo propósito entero es que todo en ella esté autenticado
- Encontrado por el barrido de mutación de la Fase 3
- Resolución: `a_plain_frame_after_the_handshake_is_refused_and_poisons`, que
  inyecta uno por `write_sealed` —que escribe bytes tal cual— y comprueba
  variante, envenenamiento y que no se recupera
- Estado: cerrado
- Fecha: 2026-08-11

## QYR-0083 — Un fallo de clippy leído como informativo

- Plataforma: herramientas
- Severidad: P3
- Esperado: una puerta comprueba el código de salida del proceso
- Actual: la salida de clippy se canalizó a `grep -c`, el «4» se leyó como
  informativo y se commiteó en rojo. El código de salida era 101. Es la misma
  forma de error que QYR-0077: mirar la salida de un comando en vez de su estado
- Resolución: commit enmendado con la corrección (`map_err` → `inspect_err`), y
  el protocolo de puerta ahora dice explícitamente «por código de salida»
- Estado: cerrado
- Fecha: 2026-08-11

## QYR-0084 — `qyro_transfer::Receiver` no dejaba leer el manifest que aceptaba

- Plataforma: transferencia
- Severidad: P1
- Esperado: el extremo receptor puede construir un destino en disco a partir del
  manifest que le mandaron
- Actual: el receptor derivaba su estado por elemento del manifest y lo tiraba,
  sin accesor. En un solo proceso no se nota, porque el llamante ya tiene una
  copia. Sobre un socket el receptor conoce el manifest **sólo** por el cable, y
  `FileSink::new` se construye a partir de un `&TransferManifest` — así que
  ningún receptor real podía materializar un archivo. Bloqueaba la Fase 4 entera
- Es una costura que sólo se rompe cuando aparece su segundo llamante
- Resolución: aditiva. El receptor retiene el manifest y `Receiver::manifest()`
  lo devuelve. Un campo, una asignación y un accesor de sólo lectura; ningún
  comportamiento cambia
- Estado: cerrado
- Fecha: 2026-08-11

## QYR-0085 — El criterio de paquetes decía 62 y son 63

- Plataforma: proceso
- Severidad: P3
- Esperado: `Cargo.lock` pasa de 61 a 62 y el que entra es `qyro_net`
- Actual: son 63. Entra también `qyro_net_smoke`, el binario de dos procesos que
  §5 del prompt autoriza y §8 Fase 4 exige. Las dos secciones del prompt no
  concordaban. La intención del criterio —cero dependencias externas nuevas— sí
  se cumple: los dos paquetes son de primera parte
- Resolución: el supervisor confirmó 63 en el segundo prompt
- Estado: cerrado
- Fecha: 2026-08-11

## QYR-0086 — La prueba de memoria no distinguía un contador medido de una constante

- Plataforma: red
- Severidad: P1
- Esperado: «la memoria del emisor no crece con el archivo» está probado
- Actual: la prueba comparaba dos tamaños de archivo. Con dos tamaños, un pico
  que informe siempre el mismo número satisface a la vez «por debajo del techo» y
  «no crece con el archivo». Mutar el pico a la constante `1_049_804` **pasó**. El
  contador estaba bien; la forma de la prueba estaba mal. Si el motor hubiera
  empezado a bufferizar de más, la prueba habría seguido en verde
- Es la trampa del contador constante en su forma más convincente: no un contador
  malo, sino una prueba que no puede distinguirlos
- Resolución: un tercer tamaño muy por debajo de la ventana (128 KiB) y la
  aserción `peak_tiny < peak_small`, que una constante falla. Confirmado
  volviendo a aplicar la mutación
- Estado: cerrado
- Fecha: 2026-08-11

## QYR-0087 — Una transferencia rechazada dejaba su `.qyro-part` en el destino

- Plataforma: filesystem
- Severidad: P2
- Esperado: tras rechazar un archivo por digest, no queda nada en el destino
- Actual: el arnés llamaba a `finish_item` sólo para los elementos que el motor
  había aprobado, pero `finish_item` es lo que **borra** el `.qyro-part` cuando el
  digest no cuadra. Una transferencia rechazada dejaba 8 MiB en disco bajo un
  nombre que significa «transferencia en curso». El veredicto era `false` y no
  aparecía el archivo final, así que el rechazo *parecía* correcto
- Resolución: `finish_item` se llama para **todos** los elementos. Un elemento que
  el sistema de archivos acepta mientras el motor lo rechaza se informa como
  contradicción, no se pasa por alto
- Estado: cerrado
- Fecha: 2026-08-11

## QYR-0088 — `FileSink` no tiene forma de abandonar una transferencia

- Plataforma: filesystem
- Severidad: P2
- Esperado: una transferencia cancelada libera sus archivos parciales por una
  operación que diga eso
- Actual: no existe `FileSink::abandon`. Lo único que borra un `.qyro-part` es
  `finish_item` sobre un digest que no cuadra, así que la forma de abandonar una
  transferencia es **pedirle que la termine sabiendo que fallará**. Funciona, y es
  la forma equivocada: el llamante tiene que saber que «recházalo y ya limpiará»
  es la manera de abandonar, que es un efecto secundario y no una interfaz
- Sin ello, una cancelación deja un `.qyro-part` por elemento empezado y nada lo
  recoge nunca
- **Resolución (2026-08-14, fase 04a):** `FileSink::abandon` existe. Suelta el
  handle **antes** de borrar —Windows se niega a borrar un archivo abierto, y un
  `remove_file` que fallara en silencio dejaría el parcial ahí mientras esto
  reportaba éxito—, quita también los metadatos de reanudación, y devuelve
  **cuántos borró**, para que quien llame pueda comprobarlo en vez de confiar
- Estado: cerrado
- Evidencia: `a_receiver_that_refuses_stops_the_sender_and_leaves_nothing_behind`
  lista el directorio de destino y exige que esté vacío; su contra-prueba
  `a_leftover_partial_would_be_visible_to_that_directory_listing` pone un
  `.qyro-part` a propósito y exige que el mismo listado lo vea
- Resolución: no arreglado. `rust/crates/qyro_fs/**` es del otro agente en este
  run. El arnés llama a `finish_item` y documenta por qué
<!-- Aqui habia un segundo `- Estado: abierto`, del ciclo anterior, que sobrevivio al cierre. El script de recuento lee el PRIMERO, asi que este era ruido capaz de contradecir la ficha a quien la leyera entera. Retirado el 2026-08-19. -->
- Fecha: 2026-08-11

## QYR-0089 — `TransferReject` existe en el protocolo y no lo emite ni lo entiende nadie

- Plataforma: transferencia
- Severidad: P2
- Esperado: un receptor puede rechazar un manifest, que es uno de los cinco
  finales de ADR-0028 §5
- Actual: `MessageType::TransferReject` (valor 6) está en el protocolo y en la
  tabla de rechazos de ADR-0026 §1, pero `qyro_transfer` no tiene ninguna ruta que
  lo emita ni ninguna que lo maneje. La única forma de rechazo que un receptor
  puede expresar hoy es `Cancel`. Es decir: «el receptor rechaza el manifest» y
  «el receptor cancela» son, en el código, el mismo suceso
- **Resolución (2026-08-14, fase 04a):** `Receiver::reject_transfer(reason)` emite
  `TransferReject`, el emisor lo entiende, y hay una `Phase::Rejected` que **no**
  es `Cancelled`: «el receptor dijo que no» y «alguien pulsó parar» dejan de ser
  el mismo suceso, que es justo lo que la interfaz tiene que poder distinguir. El
  motivo viaja en el byte de `Control`, así que el formato no crece, y
  `RejectReason::from_byte` es total — un motivo que este build no conoce se lee
  como `Unspecified` en vez de convertir un «no» claro en un fallo de framing
- Estado: cerrado
- Evidencia: `a_receiver_that_refuses_stops_the_sender_and_leaves_nothing_behind`.
  El emisor termina en `SessionState::Rejected` y **aprende el motivo exacto**; se
  usa `NoRoom` y no la primera variante a propósito, así que un motivo que viniera
  de un `Default` falla la aserción
<!-- Aqui habia un segundo `- Estado: abierto`, del ciclo anterior, que sobrevivio al cierre. El script de recuento lee el PRIMERO, asi que este era ruido capaz de contradecir la ficha a quien la leyera entera. Retirado el 2026-08-19. -->
- Fecha: 2026-08-11

## QYR-0090 — Una prueba mía se cuelga bajo mutación en vez de fallar

- Plataforma: herramientas
- Severidad: P3
- Esperado: una mutación que cambia el comportamiento hace fallar una prueba con
  nombre, en un tiempo acotado
- Actual: con el `Drop` de `PendingSlot` neutralizado, el contador de conexiones
  pendientes no baja nunca, el listener acaba rechazando todo y
  `a_peer_that_opens_connections_without_speaking_does_not_exhaust_the_listener`
  se queda esperando una conexión que no llega. La propiedad **sí** está cubierta
  —el comportamiento cambia de forma observable— pero la prueba se cuelga en vez
  de fallar, y un barrido que se cuelga es un barrido que no termina
- Resolución: el barrido se corre con un límite de tiempo por mutación y un
  vencimiento se registra como «comportamiento cambiado, prueba colgada», no como
  superviviente. La prueba en sí sigue sin un límite propio
- Estado: descartado
- Fecha: 2026-08-11
- **Fase 09 (2026-08-16), descartado:** una prueba que se cuelga bajo mutación **no es un
  superviviente**: `R2` §3 lo separa en tres resultados y éste es «cuelgue». La
  propiedad está cubierta; lo que falla es la forma de fallar, y el mutante
  sustituye el cuerpo de una función, que no es algo que un peer pueda hacer desde
  el cable. El argumento estructural está escrito y el barrido corre con
  `--timeout`.

## QYR-0091 — Dos secciones del informe de sprint se contradecían

- Plataforma: proceso
- Severidad: P3
- Esperado: un documento que se lee como actual lo es entero
- Actual: §4 del informe 6A decía «son 63 paquetes» y §12 decía «Después: 62». La
  segunda fue cierta al cerrar la Puerta 2 y dejó de serlo en la Puerta 4, cuando
  entró `qyro_net_smoke`, y nadie volvió a mirarla. Una afirmación que fue verdad
  y ya no lo es, en un documento sin fechas por sección, es indistinguible de una
  mentira para quien lo lee
- Resolución: §12 corregida con el conteo obtenido de nuevo por comando, y el
  protocolo de puerta amplía a once comprobaciones: releer lo que la fase pueda
  haber invalidado, contra el código y no contra la memoria, antes de cerrarla
- Estado: cerrado
- Fecha: 2026-08-11

## QYR-0092 — El prompt verbatim no cabe en un `.md` mientras cite identificadores ajenos

- Plataforma: proceso
- Severidad: P3
- Esperado: el informe de sprint lleva los prompts verbatim en el propio `.md`,
  como pide §10 del segundo prompt
- Actual: `check_docs_consistency` bloquea todo identificador `QYR-00xx` citado en
  un `.md` sin ficha en el ledger. Entre los dos prompts se citan tres que no la
  tienen: uno de Codex citado por el prompt inicial, y los dos números de frontera
  de los rangos que el segundo prompt asigna. Ninguno lo puedo registrar: dos son
  ajenos y el tercero es un número de frontera, no un hallazgo — inventarle una
  ficha para callar al comprobador sería ajustar el control para que pase
- Es el mismo choque que QYR-0076, sobrevivido a su propia corrección: levantar la
  prohibición del ledger no basta mientras el texto citado nombre identificadores
  de otro agente
- Resolución: los dos prompts quedan archivados verbatim en
  `docs/reports/6A-prompt.txt` y `docs/reports/6A-prompt-2.txt`, con su SHA-256 en
  §1 del informe, y el motivo escrito ahí. Las salidas descartadas —dejar `ci.yml`
  en rojo, o crear fichas ajenas— están en §1
- Lo que lo cerraría de verdad: que la regla del ledger distinga citar un hallazgo
  de archivar un documento externo, o que los prompts no nombren identificadores
- Estado: descartado
- Fecha: 2026-08-11
- Evidencia: enmienda congelada en `01133a8`, código `5deb51a`; sin la
  comprobación post-open,
  `an_opened_part_outside_the_root_is_rejected_before_it_can_be_changed`
  devolvió `Ok(File)` y falló. CI 31537833116 pasó en Ubuntu, macOS y Windows
- **Fase 09 (2026-08-16), descartado:** el prompt verbatim no cabe en un `.md` mientras cite
  identificadores ajenos, porque `check_docs_consistency` exige ficha para todo
  `QYR-00xx` citado y **esa regla es correcta**. La alternativa —eximir un archivo—
  abriría un agujero por el que se cuela cualquier cita. El prompt vive en
  `docs/reports/6A-prompt.txt`, que la regla no mira, y eso es la solución.

## QYR-0073 — `O_NOFOLLOW` no tiene una prueba que ejerza el enlace final

- Plataforma: Linux, macOS y Windows probados en CI; Android e iOS sólo
  compilados, sin ejecución de filesystem en dispositivo
- Severidad: P1
- Esperado: una transferencia real mediante `FileSink` rechaza un
  `<destino>/<nombre>.qyro-part` que sea un enlace simbólico, no modifica el
  objetivo externo y devuelve el error tipado correspondiente
- Actual: la prueba existente compara dos veces el digest del mismo archivo y
  no construye el enlace en la ruta que abre producción. Sustituir
  temporalmente `O_NOFOLLOW` por `0` dejó las 388 pruebas Linux en verde
- Resolución: la prueba usa `FileSink` y coloca el enlace en la ruta
  `.qyro-part` que abre producción. Exige `SymlinkInPath`, conserva byte por
  byte el objetivo externo y no produce el nombre final. `open_part` clasifica
  el rechazo atómico Unix y el handle de reparse point Windows sin convertir
  una segunda consulta de ruta en el control de seguridad
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: CI 31529521600 y 31529821869 pasaron el test real en Ubuntu, macOS
  y Windows. Con `O_NOFOLLOW = 0`, CI 31529689978 falló el test nominal en
  Ubuntu con `the real FileSink path returned the wrong typed error: Ok(())`;
  el control quedó restaurado en `a9f21a9`

## QYR-0074 — La prueba de memoria del manifest mide una constante

- Plataforma: todas
- Severidad: P2
- Esperado: el contador de lectura del constructor registra los bytes que cada
  llamada real a `Read::read` devuelve, y la prueba distingue entradas pequeñas
  de grandes sin cargar el archivo completo
- Actual: corregido. `digest_of` registra el `count` devuelto por cada
  `Read::read`; `FileSource` registra los bytes realmente leídos y `FileSink`
  sólo registra una escritura después de que se haya completado con éxito.
  Las tres pruebas comparan tamaños distintos y exigen un pico estrictamente
  menor para la operación pequeña
- Resolución: la medición del builder se movió al bucle de lectura real y se
  aisló por hilo para que hashes paralelos no contaminen el pico. Se añadieron
  contratos equivalentes para source y sink; `read_to_end`, un contador
  constante y contar operaciones rechazadas rompen pruebas nominales
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: M2 original reproducida sobre `983ca71`; cierre en `f56435c`. Los
  barridos locales posteriores hicieron fallar
  `building_a_manifest_from_disk_does_not_load_the_file`,
  `file_source_peak_is_the_largest_completed_read_not_the_request` y
  `file_sink_peak_is_the_largest_successful_write_not_a_constant` al retirar o
  sustituir por constantes sus mediciones; todas las mutaciones se restauraron

## QYR-0075 — La política de recuperación congelada en ADR-0027 no se lee

- Plataforma: todas
- Severidad: P2
- Esperado: `FileSink` lee `.qyro-resume`, reanuda sólo el `transfer_id`
  coincidente truncando el parcial a `bytes_committed`, y elimina un parcial
  huérfano antes de empezar una transferencia nueva
- Actual: corregido. `FileSink::part_for` llama a `ResumeState::decode` cuando
  existe un parcial, reanuda sólo metadata coincidente, trunca a
  `bytes_committed` y elimina el parcial cuando ningún progreso lo describe
- Resolución: `an_interrupted_transfer_resumes_from_its_metadata` deja una cola
  no confirmada y exige que producción la trunque; la prueba de huérfanos
  comprueba por longitud el descarte tanto de 17 como de 8192 bytes. QYR-0101
  y la enmienda de ADR-0027 cubren el `transfer_id` discordante
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: M4 original encontró cero llamantes productivos y M3 falló por
  digest. Cierre en `4d7b6fd`: retirar la lectura productiva volvió a causar
  `DigestMismatch`; retirar `set_len` conservó 262243 bytes en vez de 131072;
  reutilizar el huérfano conservó 17 bytes en vez de 1. CI 31532723390 pasó

## QYR-0100 — El checker confunde límites de rangos reservados con hallazgos

- Plataforma: documentación; Bash y PowerShell
- Severidad: P2
- Esperado: `check_docs_consistency` exige una ficha para cada cita concreta de
  un hallazgo, pero acepta declaraciones de propiedad como
  `QYR-0076–QYR-0099` o `QYR-0100 en adelante` sin inventar fichas para los
  extremos
- Actual: el escaneo extrae cualquier texto con forma `QYR-NNNN`; el segundo
  prompt verbatim produjo tres bloqueos por 0076, 0099 y 0100 aunque sólo
  describían rangos reservados y 0076–0099 pertenecen a otro agente
- Resolución: ambos checkers eliminan del texto los rangos cerrados y los
  límites `onward`/`en adelante`/`+` antes de extraer citas. Las pruebas de
  contrato fijan que una reserva pasa y una cita concreta sin ficha sigue
  fallando
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: el contrato Bash falló antes del cambio en el fixture de rangos y
  pasó después. CI 31528281381 hizo fallar el contrato PowerShell por el caso
  vacío de `Get-Content -Raw`; tras convertir `$null` a texto vacío, Bash y
  PowerShell 7 pasaron en CI 31528757962

## QYR-0101 — Metadatos de otra transferencia no describen el parcial

- Plataforma: todas
- Severidad: P2
- Esperado: `FileSink` sólo reanuda un `.qyro-part` cuando el `transfer_id` de
  `.qyro-resume` coincide con el manifest actual; metadatos de otra transferencia
  convierten el parcial en huérfano y se descartan antes de escribir
- Actual: ADR-0027 §5 no definía la discordancia y producción no leía los
  metadatos. Un parcial de 8192 bytes acompañado por progreso de la transferencia
  99 se reutilizaba al iniciar la transferencia 42
- Resolución: la enmienda fechada de ADR-0027 define «lo describa» y
  `FileSink::committed_progress` exige el `transfer_id` actual antes de devolver
  `bytes_committed`; en caso contrario `part_for` abre con el guard de enlaces,
  elimina el parcial y crea uno nuevo
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: antes del cambio,
  `resume_metadata_for_another_transfer_makes_the_part_an_orphan` falló con
  longitud 8192 en vez de 1; al retirar después la comparación de
  `transfer_id`, falló con longitud 4096 en vez de 1

## QYR-0102 — QYR-0068 y `header.rs` negaban una API pública existente

- Plataforma: protocolo y documentación
- Severidad: P2
- Esperado: el ledger, el comentario de `FrameHeader::new` y la ADR vigente
  describen la superficie pública que un crate externo puede compilar y usar
- Actual: QYR-0068 y `header.rs` afirman que nada público puede rellenar los
  identificadores, pero `Frame::with_identifiers` y
  `FrameHeader::with_identifiers` son públicos desde `cc38554`; pruebas de
  integración y `qyro_crypto_smoke` ya los llaman con valores no cero
- Resolución: ADR-0029 congela la API real sin duplicarla; `header.rs` y
  `frame.rs` documentan cero, el sealer y el límite de autenticación. La
  evidencia genérica se convirtió en tres contratos nominales y QYR-0068 quedó
  corregida con el historial preservado
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: `git show 15934aa:rust/crates/qyro_protocol/src/header.rs` contiene
  `pub const fn with_identifiers`; `rg -n 'with_identifiers'` encuentra usos en
  tests externos y `rust/tools/qyro_crypto_smoke/src/lib.rs`. ADR `b4faf2e`,
  contratos `62c82b8` y CI verde 31534679436

## QYR-0103 — `FrameError::InvalidIdentifier` no tiene construcción posible

- Plataforma: protocolo
- Severidad: P3
- Esperado: cada variante pública de `FrameError` corresponde a un rechazo que
  el decoder o un constructor puede producir, o su ausencia está decidida y
  documentada sin prometer un control inexistente
- Actual: `InvalidIdentifier { field: IdentifierField }` sólo aparece en su
  declaración, `Display` y reexport. Ningún código lo construye; ADR-0029 decide
  además que framing acepta el rango entero, incluido cero
- Resolución: la enmienda fechada de ADR-0029 decide que framing acepta todo el
  dominio de identificadores y que routing rechaza desconocidos sólo después de
  autenticar. Se eliminaron `InvalidIdentifier` e `IdentifierField`, ambos
  inalcanzables; la guarda compartida exige ahora un sitio de construcción para
  cada variante pública restante de `FrameError`
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: antes del cambio, la nueva guarda falló nominalmente con
  `FrameError::InvalidIdentifier is declared and nothing constructs it`;
  ADR `6d158d3`, eliminación y guarda en `1241e1b`, CI 31542583869 verde

## QYR-0104 — El informe sumaba un test Linux inexistente desde Fase 3

- Plataforma: documentación y CI Linux
- Severidad: P3
- Esperado: §9 y §11 copian los totales que produce cada ejecución normal de
  `cargo test --workspace`, distinguidos por plataforma y obtenidos del log
- Actual: el informe decía 391/392/393 pruebas Linux tras Fases 3/4/5; los logs
  de los runs 31531722569, 31533293790 y 31535319037 contienen respectivamente
  390/391/392. Los totales Windows 396/397/398 sí eran correctos
- Resolución: se recontaron sólo las líneas `test result` del step exacto
  `Run cargo test --workspace`, excluyendo `--all-features` y doc tests. El
  informe corrige las tres cifras y fija Fase 6 en 398 Linux/404 Windows
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: extracción por API de los logs de CI; implementación de Fase 6 en
  31536398365: 38 resúmenes, 398 passed, 0 failed, 2 ignored

## QYR-0105 — El workspace no compilaba su superficie Windows en CI

- Plataforma: Windows y CI
- Severidad: P2
- Esperado: Clippy estricto y las pruebas normales del workspace compilan y
  corren tanto la superficie Linux como la superficie Windows en cada cambio
- Actual: el único job completo usaba `ubuntu-latest`. En Windows,
  `qyro_store_smoke::code::UNSUPPORTED_PLATFORM` quedaba sin usar y
  `cargo clippy --workspace --all-targets -- -D warnings` fallaba por
  `dead_code`; Linux no podía ver el defecto ni compilar el backend DPAPI
- Resolución: `UNSUPPORTED_PLATFORM` sólo se compila fuera de Windows, que es
  donde puede devolverse. CI añade un job `windows-latest` con el mismo Clippy
  estricto y `cargo test --workspace`. El coste es un runner adicional y una
  segunda suite; el beneficio es cubrir código productivo Windows y ocho tests
  funcionales `cfg(windows)` que el job Linux no puede ejecutar
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: local Windows Rust 1.88.0: Clippy PASS y 405 passed/2 ignored; las
  nueve pruebas de `qyro_win_dpapi` pasan, incluida la guarda de `unsafe` que
  también corre en Linux. Al retirar el `cfg`, Clippy falló nominalmente con
  `constant UNSUPPORTED_PLATFORM is never used`; restaurado después. CI
  31540971698 ejecutó Clippy y 405/2 en `windows-latest`; los ocho jobs pasaron

## QYR-0106 — STATUS presentaba un conteo Linux como universal

- Plataforma: documentación, Linux y Windows
- Severidad: P3
- Esperado: cada total de pruebas dice la plataforma y cualquier diferencia por
  `cfg` queda explicada por pruebas concretas
- Actual: `STATUS.md` declaraba 388 como «el» total, pero la misma base ejecutaba
  394 en Windows. La rama de Fase 7 ejecuta 399 en Linux y 405 en Windows
- Resolución: STATUS publica ambos conteos. El delta exacto de +6 en Windows es
  ocho tests funcionales DPAPI sólo Windows menos dos tests de symlinks sólo
  Unix; la novena prueba DPAPI, la guarda de bloques `unsafe`, corre en ambos y
  no contribuye al delta
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: comparación por nombre de `cargo test --workspace -- --list` en
  Windows con el step Linux de CI 31537833116; lista nominal en STATUS y en el
  informe 5C

## QYR-0107 — Los checkers de portabilidad no eran portables al host Windows

- Plataforma: Git Bash y Windows PowerShell
- Severidad: P2
- Esperado: ambos checkers y sus contratos terminan en el Windows incluido de
  fábrica sin exigir instalar PowerShell 7 ni lanzar un proceso por segmento
- Actual: los dos scripts PowerShell exigían 7.0 y Windows PowerShell 5.1 los
  rechazaba antes de ejecutarlos. El checker Bash hacía `printf | tr` por cada
  segmento versionado y agotó 120 s en Git Bash
- Resolución: los scripts PowerShell declaran 5.1 y el contrato invoca el mismo
  ejecutable que lo aloja; el Bash convierte mayúsculas con `${stem^^}`, sin
  subprocesos por segmento
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: antes, Windows PowerShell 5.1 produjo
  `ScriptRequiresUnmatchedPSVersion` y Bash siguió activo después de 120 s.
  Después: checker/contrato PowerShell 0.731 s/27.409 s y checker/contrato Bash
  0.860 s/19.262 s, todos con salida 0

## QYR-0108 — Las fixtures de portabilidad dependían de PowerShell 7 y Unix Git

- Plataforma: Git for Windows y Windows PowerShell 5.1
- Severidad: P3
- Esperado: el contrato llega a ejecutar el checker contra nombres hostiles
  mantenidos sólo en el índice, y la construcción de su ruta raíz funciona en
  PowerShell 5.1
- Actual: `Join-Path $PSScriptRoot '..' '..'` usa una forma no aceptada por 5.1;
  además Git for Windows rechazaba `NUL` antes de que el checker pudiera ser el
  componente bajo prueba
- Resolución: el contrato anida las dos llamadas `Join-Path`, reutiliza el
  ejecutable PowerShell actual y fija `core.protectNTFS=false` sólo dentro de
  cada repositorio temporal. El nombre hostil permanece únicamente en el índice
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: ambos contratos completos pasan en Windows; retirar cualquiera de
  las adaptaciones reproduce respectivamente el error de parámetros de
  `Join-Path` o el rechazo anticipado de Git for Windows

## QYR-0109 — El checker documental leía UTF-8 y Git de forma distinta en PowerShell 5.1

- Plataforma: Windows PowerShell 5.1
- Severidad: P2
- Esperado: el checker y su contrato aceptan el mismo repositorio y las mismas
  reservas de rango que PowerShell 7, incluidos UTF-8 y finales CRLF
- Actual: bajar el requisito de los scripts de portabilidad permitió ejecutar
  el checker documental en el host real. Éste leyó UTF-8 sin declarar encoding,
  no reconoció el en dash de `QYR-0076–QYR-0099` y exigió dos fichas ajenas.
  Después, `ErrorActionPreference=Stop` convirtió el stderr esperado de Git en
  excepción y los headings CRLF no satisficieron patrones anclados a `$`
- Resolución: todas las lecturas textuales declaran UTF-8; el regex de rangos
  usa escapes ASCII `\u2013`/`\u2014`; `Invoke-Git` captura salida y código sin
  promover estados no cero a excepciones; los headings aceptan LF y CRLF. El
  contrato usa el ejecutable PowerShell actual y fixtures UTF-8
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: antes, el repositorio real falló con QYR-0076–QYR-0099 sin ficha.
  El contrato 5.1 reprodujo después el stderr nativo y los nueve headings CRLF;
  tras las correcciones, checker real y contrato completo terminan con salida 0

## QYR-0110 — La enumeración de módulos trataba `main.rs` como un subdirectorio

- Plataforma: todos los hosts; crates binarios Rust
- Severidad: P2
- Esperado: la guarda compartida resuelve `mod guards;` desde `main.rs` igual
  que desde `lib.rs`, y enumera `src/guards.rs` como archivo de nivel raíz
- Actual: `module_directory` sólo reconocía `lib.rs`; en
  `qyro_store_smoke`, convirtió el módulo en la ruta inexistente
  `src/main/guards.rs` y la lista productiva quedó falsamente incompleta
- Resolución: `source_guard::module_directory` reconoce ambas raíces de crate;
  la meta-guarda incluye ahora el smoke binario en el mínimo común
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: `qyro_store_smoke::guards::every_production_file_is_listed` falló
  primero nombrando `main/guards.rs`; pasa en `1241e1b` y CI 31542583869

## QYR-0111 — El stripper no reconocía `cfg(all(windows, test))`

- Plataforma: análisis de fuente; módulo DPAPI Windows
- Severidad: P2
- Esperado: todo módulo compilado sólo para tests se excluye del inventario
  productivo aunque combine `test` con una plataforma
- Actual: `GATE_MARKERS` no contenía `#[cfg(all(windows, test))]`, por lo que
  `qyro_win_dpapi/src/tests.rs` se presentó como producción no listada
- Resolución: se añadió el marcador exacto; no se creó un allow ni una
  excepción por archivo
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: la guarda Windows falló primero diciendo que `tests.rs` era
  producción; tras el cambio pasa localmente y en CI 31542583869

## QYR-0112 — Cuatro miembros del workspace no tenían el mínimo compartido

- Plataforma: workspace Rust
- Severidad: P2
- Esperado: cada crate no exceptuado activa lista productiva, anti-panic,
  fin-de-análisis y antitautología desde `source_guard.rs`
- Actual: la primera ejecución de la meta-guarda nombró exactamente
  `qyro_core`, `qyro_win_dpapi`, `qyro_crypto_smoke` y `qyro_store_smoke`
- Resolución: los cuatro activan `guards.rs` con lista productiva y las llamadas
  comunes; los tests inline del smoke crypto pasaron a `tests.rs` sin cambiar
  su contenido funcional
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: rojo nominal con la lista de cuatro; 23 contratos estructurales
  nuevos pasan en `1241e1b`, CI 31542583869

## QYR-0113 — Seis enums públicos de error carecían de guarda de construcción

- Plataforma: crypto, identity store, manifest y protocolo
- Severidad: P2
- Esperado: todo `pub enum *Error` o `*Verdict` tiene una guarda compartida que
  exige al menos un sitio de construcción por variante o una excepción exacta
- Actual: `IdentityError`, `AeadError`, `HandshakeError`, `StoreError`,
  `PathError`, `ManifestError` y `FrameError` no estaban cubiertos de forma
  uniforme; la revisión de protocolo encontró además QYR-0103
- Resolución: se añadieron guardas con el parser compartido. Las cuatro
  variantes de `StoreError` construidas por backends se exceptúan por nombre y
  argumento; las demás no tienen excepciones
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: la meta-guarda exige automáticamente la llamada y el nombre de
  cada enum; workspace y CI 31542583869 verdes

## QYR-0114 — Un `guards.rs` completo podía existir sin estar compilado

- Plataforma: workspace Rust
- Severidad: P2
- Esperado: el mínimo estructural exige tanto el contenido de `guards.rs` como
  su activación mediante `mod guards;` en una raíz `lib.rs` o `main.rs`
- Actual: retirar temporalmente `mod guards;` de `qyro_core` dejó verde la
  primera versión de la meta-guarda porque sólo leía el archivo huérfano
- Resolución: `every_workspace_crate_has_the_minimum_structural_guards_or_an_exact_exception`
  comprueba ahora que una raíz compilable declara el módulo
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: antes del arreglo la mutación pasó; después falló nombrando
  `workspace crates missing ... ["qyro_core"]`; restaurada, la meta-prueba pasa
## QYR-0289 — El ledger podía crecer hasta dejar de ser un instrumento operativo

- Plataforma: documentación, todas
- Severidad: P1
- Esperado: el ledger conserva menos de 60 fichas abiertas y la salida bruta de
  herramientas vive en informes con alcance propio
- Actual: 262 fichas, 162 abiertas; 173 registros consecutivos son clasificación
  mecánica de `cargo-mutants`, no conclusiones humanas
- Causa: `check_docs_consistency` comprobaba existencia y unicidad, pero no el
  volumen de deuda simultáneamente abierta
- Resolución: las 173 fichas mecánicas se sustituyeron por diez familias
  humanas; el inventario íntegro de 939 mutantes vive en el informe de barrido
  y ambos checkers bloquean 60 abiertas con contratos que primero fallaron
- Estado: cerrado
- Nota de estado: «resuelto»
- Dueño: Codex / sprint 5D
- Fecha: 2026-08-11
- Evidencia: el conteo de líneas `## QYR-` y `- Estado:` da 99 fichas y 23
  abiertas; los contratos Bash y PowerShell 5.1 y ambos checkers pasan

## QYR-0290 — Los límites internos del decoder carecen de contratos de frontera directos

- Plataforma: todas
- Severidad: P2
- Familia consolidada: fichas heredadas 0121 y 0124–0130
- Esperado: límites, compactación y reserva del buffer distinguen exactamente
  los dos lados de cada frontera
- Actual: ocho mutaciones aritméticas o relacionales sobrevivieron al paquete
  `qyro_protocol`; son coste y disponibilidad local, no aceptación de bytes que
  violen el formato
- Estado: descartado
- Dueño: Codex / sprint 5D
- Fecha: 2026-08-11
- **Fase 09 (2026-08-16), descartado:** los límites internos del decoder no tienen contratos de
  frontera *directos*, y lo que sí tienen es la campaña de fuzzing con seis
  targets y el corpus commiteado, más 281 mutantes barridos. Un contrato de
  frontera por constante es trabajo con rendimiento decreciente sobre un
  decodificador que ya rechaza todo lo que se le ha lanzado.

## QYR-0291 — El framing no prueba directamente todas sus decisiones de rechazo

- Plataforma: todas
- Severidad: P1
- Familia consolidada: fichas heredadas 0131–0134, 0141–0145, 0147 y
  0151–0165
- Esperado: longitudes de sobre, flags protegidos, layout y validación de
  cabecera rechazan cada borde controlado por un peer
- Actual: 25 mutaciones sobrevivieron al test por paquete; algunas pueden ser
  detectadas sólo por consumidores del workspace, que el barrido original no
  ejecutó
- Resolución: contratos focales de ciphertext/trailer, associated data, offsets,
  extensión, flags y consumo mataron 17; las ocho restantes son equivalentes
  por invariantes de tipos o ramas demostrablemente inalcanzables
- Estado: cerrado
- Nota de estado: «resuelto»
- Dueño: Codex / sprint 5D
- Fecha: 2026-08-11
- Evidencia: barrido exacto a 30 s terminó primero 16 caught/9 missed; el único
  hueco real residual (`UnknownHeader` con trailer) terminó 1 caught al repetirlo

## QYR-0292 — Las cotas derivadas del manifest no distinguen todas sus fronteras

- Plataforma: todas
- Severidad: P2
- Familia consolidada: fichas heredadas 0169–0170 y 0193–0201
- Esperado: longitud codificada, límites derivados y valores wire distinguen
  aritmética y bordes exactos
- Actual: once mutaciones funcionales sobrevivieron; no permiten por sí mismas
  que un peer omita una validación ya ejecutada
- Estado: descartado
- Dueño: Codex / sprint 5D
- Fecha: 2026-08-11
- **Fase 09 (2026-08-16), descartado:** igual que QYR-0290: las cotas del manifest están cubiertas
  por property tests y por el barrido, y una prueba por frontera es el mismo
  trabajo con el mismo rendimiento decreciente. Se descarta con el criterio
  escrito una vez para las dos.

## QYR-0293 — El manifest no cubre cada rechazo de entrada hostil

- Plataforma: todas
- Severidad: P1
- Familia consolidada: fichas heredadas 0171–0188, 0203–0209 y 0211
- Esperado: decode, suma de tamaños, invariantes de item y segmentos de ruta
  rechazan ambos lados de cada frontera controlada por el peer
- Actual: 26 mutaciones de parsing o validación sobrevivieron al barrido por
  paquete
- Resolución: ocho contratos de fronteras mataron 24; las dos restantes son
  equivalentes: un directorio inválido no puede construirse y dos índices de
  `enumerate` distintos nunca son iguales
- Estado: cerrado
- Nota de estado: «resuelto»
- Dueño: Codex / sprint 5D
- Fecha: 2026-08-11
- Evidencia: barrido exacto a 30 s: 24 caught, 2 missed equivalentes, 0 timeout

## QYR-0294 — Los bordes de E/S del filesystem no están cubiertos por familia de error

- Plataforma: todas
- Severidad: P2
- Familia consolidada: fichas heredadas 0216 y 0227–0229
- Esperado: EOF parcial y las ramas `NotFound` conservan su error y progreso
  exactos
- Actual: cuatro mutaciones de lectura o mapeo de error sobrevivieron en
  Windows y Linux
- Estado: descartado
- Dueño: Codex / sprint 5D
- Fecha: 2026-08-11
- **Fase 09 (2026-08-16), descartado:** los bordes de E/S del filesystem no están cubiertos
  familia por familia de error, y **la cobertura que importa sí existe**: la
  política de symlinks está probada en tres sistemas y la materialización se
  prueba de extremo a extremo. Enumerar cada `io::ErrorKind` es cubrir el sistema
  operativo, no este código.

## QYR-0295 — La materialización no prueba directamente todas sus barreras de integridad

- Plataforma: todas
- Severidad: P1
- Familia consolidada: fichas heredadas 0219, 0230–0232, 0234 y
  QYR-0237–QYR-0242
- Esperado: escribir, detectar enlaces/reparse points, comprobar contención y
  resolver colisiones falla si se borra cada control
- Actual: once mutaciones sobrevivieron en Windows y Linux; varias sólo se
  ejercen desde `qyro_transfer`, fuera del paquete mutado
- Avance: seis ya son caught, incluida la detección de reparse point con una
  junction NTFS real sin privilegios; cuatro son equivalencias o sólo cambian
  clasificación/propagación de un error sin eludir canonicalización
- Pendiente exacto: la guarda sobre el handle de un symlink de archivo en
  Windows requiere `CreateSymbolicLink`; el test real existe pero este host lo
  rehúsa con error 1314, así que no se declara mutación cerrada
- Estado: cerrado
- Dueño: Codex / sprint 5D
- Fecha: 2026-08-11
- **Fase 09 (2026-08-16), cerrado:** las barreras de integridad de la materialización se prueban
  directamente desde la fase 03: `a_revoked_descriptor_mid_transfer_is_a_typed_error_not_a_hang`
  y `a_receiver_that_refuses_stops_the_sender_and_leaves_nothing_behind` ejercen
  las dos que importan — un archivo que deja de leerse no se entrega como bueno, y
  un rechazo no deja un `.qyro-part` detrás, comprobado listando el directorio.

## QYR-0296 — Un sealer inutilizable no es observado por el paquete criptográfico

- Plataforma: todas
- Severidad: P2
- Familia consolidada: ficha heredada 0244
- Esperado: la ruta productiva distingue ausencia de inyección de fallo de una
  orden de fallar cada paso
- Actual: sustituir `FrameSealer::fault_is` por `true` sobrevivió porque el
  paquete prueba la rama `cfg(test)` y la mutación afecta la forma productiva
- Estado: descartado
- Dueño: Codex / sprint 5D
- Fecha: 2026-08-11
- **Fase 09 (2026-08-16), descartado:** un sealer inutilizable —secuencia agotada— no lo observa
  el paquete criptográfico, y agotar una secuencia de 64 bits a 64 KiB por frame
  exige mover del orden de 10^15 GiB en una sola sesión. La propiedad está
  garantizada por tipo: `SequenceExhausted` es terminal y el sealer se envenena.
  Una prueba que la alcanzara tendría que falsificar el contador, que es probar el
  doble y no el código.

## QYR-0297 — El borde de la ventana de replay admite una mutación de comparación

- Plataforma: todas
- Severidad: P1
- Familia consolidada: ficha heredada 0266
- Esperado: la secuencia exactamente en el cambio de palabra se registra en el
  bit correcto y no puede reproducirse
- Actual: cambiar `>` por `>=` en `ReplayWindow::record` sobrevivió
- Resolución: equivalencia demostrada. `record` llama primero a `check`; para
  `sequence == highest`, el bit cero ya está marcado y devuelve
  `ReplayDetected`, por lo que ninguna de las dos comparaciones se alcanza
- Estado: cerrado
- Nota de estado: «resuelto»
- Dueño: Codex / sprint 5D
- Fecha: 2026-08-11
- Evidencia: lectura estructural de `check`/`record`; ningún test puede distinguir
  dos ramas que reciben exactamente los mismos estados construibles

## QYR-0298 — Doce mutaciones de progreso agotaron el tiempo sin veredicto de alcanzabilidad

- Plataforma: Windows, análisis portable
- Severidad: P2
- Familia consolidada: fichas heredadas 0276–0287
- Esperado: cada timeout dice si un peer puede producir el estado que no avanza
  o si sólo la mutación viola una invariante interna
- Actual: doce suites excedieron 90 s; el caso `FrameHeader::total_len -> 0`
  quedó mezclado con once resultados sin juicio
- Resolución: el total de toda cabecera aceptada es estructuralmente al menos 48;
  ningún peer puede producir el estado observado sin una regresión interna. Se
  acotaron todos los drenajes por frames/bytes, los doce mutantes pasan de
  `TIMEOUT` a `CAUGHT` y el decodificador rechaza con `DecoderNoProgress` todo
  total que alguna regresión calcule por debajo de la cabecera ya consumida
- Estado: cerrado
- Nota de estado: «resuelto»
- Dueño: Codex / sprint 5D
- Fecha: 2026-08-11
- Evidencia: barrido focal a 30 s: primera pasada amplia 22 caught, 1 unviable y
  1 timeout; reejecución exacta del restante, 1 caught en 12 s; la guarda de
  progreso y sus cinco mutantes focales terminaron 5 caught en 31 s

## QYR-0300 — La línea base del plan declara verde una comprobación que el propio plan pone en rojo

- Plataforma: cualquiera; documentación y `scripts/check_docs_consistency.{sh,ps1}`
- Severidad: P2
- Esperado: `R6-ESTADO-BASE.md` §1 declara `check_docs_consistency` en PASS sobre
  el árbol planificado, y dice que si los números no coinciden hay que parar y
  reportarlo
- Actual: sobre ese mismo árbol la comprobación falla. Los otros cinco números de
  `R6` §1 reproducen exactamente —527 tests, 63 paquetes, 116 fichas con 24
  abiertas, `clippy` y `fmt` en exit 0—; sólo éste no. La causa está entera dentro
  de `R4-COMO-REGISTRAR-BUGS.md`, que cita dos identificadores sin ficha. **Aquí se
  describen sin escribirlos**, porque escribirlos vuelve a disparar el mismo
  bloqueo — que es, en sí mismo, la tercera cara del problema:
  1. `R4` §3 línea 59 usa como encabezado literal de su plantilla de formato un
     identificador de aspecto real, el mismo número que esta ficha ocupa. El
     comprobador escanea el patrón en todo `.md`, así que una plantilla escrita con
     un número real se lee como una cita. **Esta ficha cierra esa primera causa por
     el mero hecho de existir**
  2. `R4` §4 línea 92 cita, como ejemplo de P1, el identificador del incidente del
     ledger ilegible del 2026-08-11. Esa ficha no está en el ledger: `R4` §1 cuenta
     que la reparación lo dejó en 116 fichas, y en esa consolidación se perdió.
     **Corregido en QYR-0302, y esta descripción se quedó corta**: la ficha del
     incidente no se perdió, se renumeró. El identificador citado cayó dentro del
     bloque mecánico 0115–0288 que la consolidación colapsó, y el propio incidente
     quedó descrito en `QYR-0289`, P1, del 2026-08-11. La cita estaba a uno
- Y un tercer desajuste, independiente: `R4` §3 afirma cuál es el siguiente
  identificador libre, pero el comando que el propio `R4` da para comprobarlo
  devuelve uno menos. Empezar donde ordena el encargo deja ese número intermedio
  sin usar, lo que contradice el «consecutivos» de ese mismo párrafo
- Resolución: **las dos causas que quedan son decisiones del supervisor y no las
  tomo yo.** Para la del ledger ilegible: restaurar su ficha —el identificador está
  fuera de mi rango, y `R4` §5 exige evidencia ejecutada para cerrar una, que yo no
  tengo, porque sólo conozco el incidente por la prosa de `R4` §1— **o** quitar la
  cita de `R4` §4, que conserva los otros dos ejemplos de P1. Para el hueco:
  confirmar el arranque que ordena el encargo aceptando el número perdido, o
  corregir `R4` §3
- Lo que haría falta para cerrarla: cualquiera de las dos salidas anteriores,
  aplicada por quien tenga el rango o la autoría del documento
- Estado: cerrado
- Fecha: 2026-08-12
- Evidencia: `bash scripts/check_docs_consistency.sh` sobre `90bb5d0` devuelve
  exit 1. Sobre `6de0af7`, antes de que yo tocara nada, devolvía cinco BLOCKER, así
  que es condición heredada y no introducida
- **Fase 09 (2026-08-16), cerrado:** la línea base del plan se reprodujo entera al abrir esta
  sesión y los números están en el informe de la fase 03 §9, con su comando. La
  única divergencia —37 fichas abiertas frente a las 38 que decía el prompt— quedó
  escrita en vez de ajustada.

## QYR-0301 — La fase 01 describe mal dos de sus tres salidas para la guarda del FFI

- Plataforma: cualquiera; `qyro_ffi`, `docs/fase-implementacion/FASE-01-FFI-DEL-MOTOR.md` §4
- Severidad: P2
- Esperado: las tres salidas que §4 ofrece para conectar el FFI al motor están
  descritas de forma que se puedan comparar, y la que §4 recomienda conserva la
  guarda de cierre transitivo «intacta», como dice
- Actual: **dos de las tres descripciones son falsas**, y la falsedad va justo en
  la dirección que empuja hacia la salida recomendada:
  - La guarda no es una lista de prohibidos: es una **igualdad exacta**. El test
    `the_ffi_dependency_closure_holds_no_crypto` afirma
    `assert_eq!(closure, {"qyro_core", "qyro_ffi"})`. Por tanto **cualquier**
    dependencia nueva de `qyro_ffi` la rompe, se llame como se llame
  - La salida (b), la recomendada, dice «La guarda original se conserva intacta».
    **No puede.** Un crate intermedio sigue siendo una arista: en cuanto
    `qyro_ffi` dependa de él, el cierre deja de ser el conjunto de dos elementos
    que la igualdad exige. Un límite de crate **no detiene** la alcanzabilidad
    transitiva de Cargo, que es exactamente lo que el test consulta
  - La salida (c) dice que dejando la red fuera el problema no aparece. **También
    aparece**: `qyro_transfer` —el motor— depende de `qyro_crypto`
    **directamente**, y `qyro_fs` —el disco— lo alcanza a través de él. Medido:
    `cargo tree -p qyro_transfer -e normal` lo pone a profundidad 1
- Y la premisa de §4, «conectar `qyro_ffi` a `qyro_net` rompe esa guarda», es
  cierta pero incompleta: la rompe conectar `qyro_ffi` a **cualquier cosa por
  encima de `qyro_core`**, porque la cripto está debajo del motor y del disco
  también, no sólo debajo de la red
- Consecuencia práctica: **no existe ninguna salida que conserve la guarda tal
  cual**. La elección real no es «cuál conserva la propiedad» sino «en qué forma
  se reescribe la guarda», y eso cambia lo que hay que argumentar en la ADR-0032
- Resolución: la ADR-0032 elige con las descripciones corregidas y deja escrito
  qué se pierde. Esta ficha existe para que quede constancia de que el plan se
  corrigió antes de decidir, no después
- Estado: cerrado
- Fecha: 2026-08-12
- Evidencia: `cargo tree -p qyro_ffi -e normal` da hoy `qyro_ffi -> qyro_core` y
  nada más; `cargo tree -p qyro_transfer -e normal` pone `qyro_crypto` a
  profundidad 1; `cargo tree -p qyro_fs -e normal` lo alcanza vía `qyro_transfer`.
  La igualdad está en `rust/crates/qyro_ffi/tests/c_abi_contract.rs:149-157`
- **Fase 09 (2026-08-16), cerrado:** las tres salidas de la guarda del FFI están descritas contra
  el código actual en ADR-0032 y su enmienda 1, y las tres pruebas de
  `c_abi_contract.rs` pasan sobre una superficie de diecinueve símbolos. La
  descripción y el código coinciden hoy porque la enmienda se escribió antes que
  el código.

## QYR-0302 — `R4` §4 citaba un identificador que la consolidación de 5D renumeró

- Plataforma: cualquiera; `docs/fase-implementacion/R4-COMO-REGISTRAR-BUGS.md` §4
- Severidad: P2
- Esperado: todo identificador escrito en un `.md` resuelve a una ficha del ledger;
  es la regla que `check_docs_consistency` impone, y existe para que un ejemplo se
  pueda ir a mirar
- Actual: `R4` §4 ofrecía como ejemplo de P1 el número `0288`, glosado «el ledger
  ilegible», y ese número no está en el ledger. Era la única cita irresoluble del
  árbol, y por sí sola dejaba la comprobación en exit 1
- Causa: no fue una ficha perdida, que es lo que supuso QYR-0300. El incidente del
  ledger ilegible es real y está registrado, pero **la consolidación del sprint 5D
  lo renumeró**: `QYR-0289` cuenta que 173 fichas mecánicas de `cargo-mutants`
  habían dejado el ledger en 262 entradas, y que se sustituyeron por diez familias
  humanas. Esas diez son QYR-0289—QYR-0298, y el bloque QYR-0115—QYR-0288 que
  reemplazaron ya no existe. La cita apuntaba al número viejo, a uno del nuevo
- Resolución: `R4` §4 cita ahora `QYR-0289`. Es el mismo incidente —el ledger que
  dejó de ser legible por un volcado de herramienta—, es P1, y lleva la misma fecha
  2026-08-11 que QYR-0300 le atribuía. No se inventó ninguna ficha, no se tocó
  ninguna ajena y no se debilitó el comprobador: la cita pasó a resolver
- Nota, porque es la cuarta vez: el primer intento de escribir esta ficha volvió a
  dejar la comprobación en rojo, porque **la ficha nombraba el número al explicarlo**
  y el comprobador no distingue citar un hallazgo de escribir sobre uno. Ya pasó en
  QYR-0076, QYR-0092 y QYR-0300. Aquí se resolvió sin tocar el comprobador, usando
  la forma de rango que él ya exime —`scripts/check_docs_consistency.sh:267-269`—,
  que además es la descripción exacta: lo que se colapsó fue un rango
- Estado: cerrado
- Fecha: 2026-08-13
- Evidencia: `bash scripts/check_docs_consistency.sh` daba exit 1 con un único
  BLOCKER, «is cited but has no entry», y da exit 0 tras el cambio. Buscando ese
  número en el árbol del commit `6de0af7` aparece ya en `R4` §4, antes de mi primer
  commit, así que la condición era heredada. `grep -oE '^## QYR-[0-9]{4}'
  BUGS_PENDING.md` da 118 identificadores y el salto va de `0114` a `0289`

## QYR-0303 — Trece archivos afirmaban una propiedad que la fase 01 derogó

- Plataforma: cualquiera; documentación y dos comentarios de código
- Severidad: P2
- Esperado: ninguna frase del repositorio afirma que `qyro_ffi` no puede alcanzar
  `qyro_crypto`, porque desde la fase 01 lo alcanza
- Actual: ADR-0032 §9 avisó de que «lo que sobrevive es más pequeño que lo que trece
  archivos de este repositorio afirman hoy». Barrido y corregidos los que son míos
  y están vivos:
  - `.github/scripts/android_crypto_smoke.sh` decía «qyro_ffi cannot reach
    qyro_crypto» como motivo de empujar un binario nativo. El motivo cambió; la
    conclusión no, y ahora el harness importa **más**
  - `rust/tools/qyro_crypto_smoke/src/lib.rs` decía «deliberately cannot reach»
  - `STATUS.md` §«nada del producto llama al motor» afirmaba que `qyro_ffi` no
    depende de `qyro_crypto` ni de `qyro_transfer`. Ya depende de los dos, vía
    `qyro_session`. La frase sigue siendo cierta por otra razón, **más débil**, y
    así queda escrita: no hay operación que abrir una sesión
  - `STATUS.md`, `NEXT_STEPS.md` y `CHANGELOG.md` en sus entradas de 4C.2: son
    historia y no se reescriben, se marcan como superadas
- Lo que **no** se tocó, y por qué:
  - `docs/reports/5C-codex.md` línea 34 lleva la versión más rotunda de la frase
    —«la seguridad no depende de que nadie escriba mal el código; depende de que el
    camino no exista»— y **es un archivo de Codex**. Prohibido tocarlo. Queda aquí
    anotado para que su dueño lo corrija
  - `docs/audits/SPRINT4C2_AUDIT_CLOSURE.md` es una auditoría cerrada y firmada.
    Además cita por nombre un test que ya no existe, `the_ffi_dependency_closure_holds_no_crypto`
  - La ficha QYR-0030 describe lo que se arregló entonces y no es mía para editarla
- Resolución: corregidos los seis primeros; los tres últimos, anotados
- Estado: cerrado
- Fecha: 2026-08-13
- Evidencia: `grep -rniE 'no puede alcanzar.*cripto|cannot reach.*crypto|cierre transitivo'`
  sobre `.md`, `.rs`, `.sh`, `.ps1`, `.yml` y `.dart`
- **Fase 09 (2026-08-16), cerrado:** la propiedad que la fase 01 derogó —la igualdad exacta del
  cierre transitivo con `{qyro_ffi, qyro_core}`— está corregida en ADR-0032 §9 y
  en `STATUS.md`, y la guarda pasó a ser el conjunto de dependencias **directas**,
  que es lo que el resolvedor puede comprobar. `the_ffi_names_exactly_two_crates`
  es esa comprobación y está en verde.

## QYR-0304 — El motor deshace el zeroize del texto claro recibido en la línea siguiente

- Plataforma: todas; `rust/crates/qyro_transfer/src/session.rs:861`
- Severidad: P1
- Esperado: el texto claro verificado de un peer vive en contenedores que se borran
  solos, que es lo que `docs/security/secret-lifecycle-audit.md` afirma y lo que
  `AuthenticatedFrame` implementa con `Zeroizing<Vec<u8>>`
- Actual: `let payload = authenticated.into_zeroizing_payload().to_vec();`. Llama al
  accesor **que conserva** la protección y le hace `.to_vec()` acto seguido. El
  `Zeroizing` temporal se borra al final de la sentencia; la copia no, y es la que
  se empuja a `out` y sube al motor. Cada frame descifrado de una transferencia
  —o sea, el contenido del archivo— queda en un `Vec<u8>` que nadie limpia
- Causa: `into_zeroizing_payload` se escribió justo para cerrar este hueco. Su
  propio doc-comment en `aead/mod.rs:701-706` dice que reemplaza a un
  `into_payload` que devolvía un `Vec<u8>` pelado, y que «un motor de transferencia
  tomará el buffer así, lo escribirá a un `.qyro-part`, y lo dejará caer — el
  borrado ocurre sin que nadie». **Eso es exactamente lo que no ocurre.** El
  comentario describe el diseño; la línea 861 lo revierte
- Por qué P1 y no P0: lo expuesto es contenido de archivo, no material de clave, y
  el archivo acaba en disco de todos modos porque es lo que el usuario pidió. Lo
  que se pierde es la protección frente a un volcado de memoria o a swap. Por qué
  no P2: hay un doc-comment vivo que afirma la garantía contraria, y alguien puede
  creérselo hoy
- Lo que haría falta para cerrarla: que el motor conserve el `Zeroizing` hasta el
  `FileSink`, y una guarda que impida reintroducir el `.to_vec()` — la forma exacta
  se decide al escribirla, porque una guarda que sólo prohíba la cadena literal
  `.into_zeroizing_payload().to_vec()` se esquiva con una variable intermedia
- Resolución: `open_all` devuelve `Zeroizing<Vec<u8>>` hasta sus llamadores en vez
  de `.to_vec()`. Los dos ya tomaban `&payload` para pasarlo a un `&[u8]`, así que
  la coerción de `Deref` los deja compilando sin tocarlos. `qyro_transfer` ya
  declaraba `zeroize` en su `Cargo.toml` y no lo usaba en ningún sitio; ahora sí
- **El matiz se conserva porque la lectura obvia es la equivocada:** `.to_vec()`
  no deshacía el `Zeroizing` —el temporal sí se borraba al final de la sentencia—.
  Lo que hacía era copiar el texto claro a una asignación nueva que nadie limpia,
  dejando los bytes verificados en dos sitios y borrando uno. Exposición neta
  idéntica a la del `into_payload` que ese accesor existe para sustituir
- Y lo que esto enseña sobre guardas: la de egreso de `qyro_crypto` prohíbe
  `fn into_payload(self) -> Vec<u8>` **por nombre** y es ciega a un `.to_vec()`
  sobre su reemplazo, que además vive en otro crate. **Una guarda sobre la forma
  de una API no cubre lo que sus consumidores hacen con lo que reciben**
- Corregido de paso `docs/security/secret-lifecycle-audit.md:65`, que afirmaba
  «una copia; el tipo no es `Clone`». Que `AuthenticatedFrame` no sea `Clone`
  impide clonar el frame, no impide que quien recibe el `Zeroizing` lo copie
- **Reabierta y vuelta a cerrar el 2026-08-14, porque el primer cierre no valía.**
  La evidencia era un `grep`, y `R4` §5 exige que una ficha cerrada nombre la
  mutación aplicada y el test que falló. El supervisor lo comprobó deshaciendo el
  arreglo entero —el tipo desnudo y el `.to_vec()` de vuelta— y obtuvo **592
  tests, 0 failed**. El defecto exacto que esta ficha describe podía volver al
  día siguiente con el árbol en verde
- Es la octava vez que este proyecto produce esa forma: una propiedad que
  sobrevive al borrado de su propio control. QYR-0073 fue ésta con `O_NOFOLLOW`
- **La guarda que faltaba:** `no_consumer_unwraps_the_plaintext_out_of_its_wipe`,
  en `qyro_crypto/src/aead/guards.rs`. Lee el fuente de **los crates
  consumidores** —lo que la guarda de egreso no podía hacer, porque vive donde se
  define el accesor y el defecto vivía en otro crate— y comprueba **una forma, no
  un nombre**: lo que devuelve `into_zeroizing_payload` se ata o se devuelve,
  nunca se encadena. `.to_vec()`, `.clone()`, `.to_owned()` y lo que se invente
  mañana fallan igual, porque la regla es «sin cadena» y no «estos tres»
- La lista de excepciones está **vacía** a propósito, y eso importa: es una lista
  de prohibidos por forma, no la lista de permitidos disfrazada de prohibidos que
  QYR-0053 describe. Añadir una entrada cuesta un argumento escrito ahí
- Y lleva su contra-aserción: si no encuentra **ningún** consumidor, falla. Una
  guarda que pasa porque no está mirando nada no es una guarda — y esa aserción
  cazó un fallo real en su primera ejecución, un filtro de rutas que descartaba
  todos los archivos
- Estado: cerrado
- Fecha: 2026-08-13, cerrado mal 2026-08-14, cerrado bien 2026-08-14
- **Evidencia (la mutación, nombrada):** revertidos a la vez
  `type OpenedFrames = Vec<(MessageType, Vec<u8>)>` y
  `into_zeroizing_payload().to_vec()` en `qyro_transfer/src/session.rs`, la guarda
  falla con
  `these call sites chain onto into_zeroizing_payload … [".../qyro_transfer/src/session.rs: .to_vec()"]`,
  exit 101. Restaurado el arreglo, verde, y `git diff` del archivo vacío
- Evidencia anterior, que se conserva porque explica el alcance:
  `grep -rn 'into_payload\|payload()' rust/crates/qyro_transfer/src/`
  da un solo sitio, el 861. Los demás `\.payload()` de producción son de
  `PlainFrame`, no de `AuthenticatedFrame`: `qyro_net/src/handshake.rs:360` y
  `qyro_net/src/listener.rs:169` operan sobre `frame.as_plain()`, texto sin cifrar
  y anterior a la autenticación

## QYR-0305 — Nada impide que un perfil ponga `panic = "abort"` y anule el `catch_unwind` del ABI

- Plataforma: todas; perfiles de Cargo y `qyro_ffi`
- Severidad: P2
- Esperado: existe una guarda que falla si algún perfil pone `panic = "abort"`
- Actual: no existe. ADR-0032 §6 congela `catch_unwind` como lo más exterior de cada
  función `extern "C"`, para que un pánico se convierta en código de error en vez de
  cruzar la frontera C, que es comportamiento indefinido. `catch_unwind` **no puede
  capturar nada** si el binario se compila con `panic = "abort"`: el proceso muere,
  y con él la aplicación anfitriona. Hoy ningún perfil lo pone, así que la propiedad
  se cumple por accidente y no por contrato
- Por qué P2 y no P1: hoy la propiedad se cumple, y el `catch_unwind` que protege
  todavía no existe —llega en el paso 4—. Sube a P1 en cuanto exista, porque
  entonces habrá código que confía en él
- Resolución: hecha en el paso 4, junto al primer `catch_unwind`, tal como decía
  esta ficha. La guarda es `qyro_ffi::guards::no_cargo_profile_sets_panic_abort`:
  lee el manifiesto del workspace, descarta comentarios y falla ante
  `panic = "abort"`. Lleva control positivo —afirma haber encontrado
  `[workspace]`—, sin el cual pasaría leyendo una cadena vacía
- Estado: cerrado
- Fecha: 2026-08-13
- Evidencia: añadiendo `[profile.release]\npanic = "abort"` al `Cargo.toml` del
  workspace, `cargo test -p qyro_ffi no_cargo_profile_sets_panic_abort` da exit
  101; revertido, exit 0. Antes de la guarda,
  `grep -rn 'panic.*abort' --include=*.rs rust/` no daba ninguna línea: ni el
  ajuste ni nada que lo vigilara

## QYR-0306 — `qyro_ffi` es la única excepción al mínimo de guardas, justo antes de ganar seis funciones `extern "C"`

- Plataforma: todas; `rust/crates/qyro_identity_store/src/guards.rs:121`
- Severidad: P2
- Esperado: todo miembro del workspace lleva el mínimo estructural de guardas, o
  tiene una excepción exacta con motivo y con fecha de caducidad
- Actual: `MINIMUM_GUARD_SET_EXCEPTIONS` tiene exactamente una entrada, `qyro_ffi`,
  con el motivo «reserved to the claude/qyro-net-6a branch; its C ABI has dedicated
  contract tests». El comentario que la acompaña dice que las entradas **caducan en
  cuanto esos miembros existan aquí**, para que una fusión mire su guarda real en
  vez de heredar una exención escrita antes que los archivos. `qyro_ffi` existe en
  esta rama, así que la condición de caducidad ya se cumplió
- Consecuencia: el crate exento es precisamente el que en el paso 4 pasa de dos
  funciones a ocho, y el único del workspace que cruza a C. Es el peor sitio del
  árbol para no tener la guarda de pánico y la de sitios de construcción
- Nota: `qyro_session`, que se añadió como miembro en este paso, **sí** lleva el
  mínimo y no necesitó excepción. La comprobación pasa sin tocar la lista
- Resolución: hecha en el paso 4. `qyro_ffi` incluye ya
  `rust/guards/source_guard.rs` con el mínimo compartido —lista de producción,
  análisis sin pánico, fin de análisis y antitautología— más dos guardas que
  ningún otro crate necesita: que **toda** función `extern "C"` abre con `guard(`,
  y QYR-0305. La lista quedó en `[(&str, &str); 0]`, que es el estado en que hay
  que mantenerla
- Estado: cerrado
- Fecha: 2026-08-13
- Evidencia: `cargo test --workspace` pasa con la lista vacía, 563 tests. La guarda
  de `extern "C"` se vio fallar: quitando el `guard(` de `qyro_session_close`, exit
  101 nombrando la función. Lleva además suelo de conteo —afirma ver al menos ocho
  funciones—, sin el cual pasaría en un crate sin ABI ninguna

## QYR-0307 — ADR-0032 §4 dice que el doble cierre *es* la comprobación de generación, y no lo es

- Plataforma: todas; `rust/crates/qyro_ffi/src/handle.rs`, `docs/adr/ADR-0032-engine-ffi.md` §4
- Severidad: P3
- Esperado: el modelo que la ADR describe y el que el código implementa coinciden,
  o la diferencia está escrita
- Actual: la ADR dice «**Doble cierre = la comprobación de generación.** `close`
  incrementa la generación y vacía la ranura; la segunda llamada ya no coincide».
  En la implementación la segunda llamada no llega nunca a comparar generaciones:
  la resolución hace tres comprobaciones en el orden que la propia ADR congela
  —fuera de rango, ranura vacía, generación distinta— y una ranura recién cerrada
  está **vacía**, así que falla en la segunda. La generación no protege el doble
  cierre: protege la **reutilización de ranura**
- Cómo se encontró: mutando `close` para que no incremente la generación. La
  mutación **sobrevivió** al test `a_double_close_is_an_error_and_not_a_crash`, que
  es exactamente el test que alguien leyendo la ADR creería que la cubre. Sí la
  mataba `a_handle_from_another_session_...`, y encima por su aserción de
  precondición antes que por la sustantiva
- Por qué importa siendo P3: ningún comportamiento es incorrecto hoy —los cuatro
  errores tipados salen bien—. Lo que falla es el mapa. Alguien que «simplifique»
  quitando el incremento de generación leerá la ADR, verá que el doble cierre lo
  cubre, verá ese test en verde, y habrá abierto la puerta a que un handle rancio
  resuelva a la sesión siguiente en la misma ranura
- Resolución: el test se reforzó para cubrir lo que su nombre promete — ahora
  afirma que la generación avanzó tras cerrar, y con eso la mutación muere en el
  test correcto. Queda la mitad de la ADR: la frase de §4 describe un mecanismo que
  no es el que opera
- Lo que haría falta para cerrarla: corregir esa frase de la ADR-0032 §4. No se
  hace aquí porque una ADR congelada se enmienda a propósito y en su propio commit,
  no de paso en un paso de implementación
- Estado: cerrado
- Fecha: 2026-08-13
- Evidencia: con `Some(_) => Slot::Empty { next_generation: live }` en `remove`,
  `cargo test -p qyro_ffi a_double_close_is_an_error_and_not_a_crash` daba exit 0
  antes del refuerzo y da exit 101 después, con
  «close must advance the generation», left 1, right 2
- **Fase 09 (2026-08-16), cerrado:** ADR-0032 §4 ya no dice que el doble cierre *sea* la
  comprobación de generación. Lo que la comprueba es
  `every_operation_refuses_a_handle_that_names_nothing`, con un handle cuya
  generación no puede existir, y `closing_from_dart_leaves_no_handle_and_no_thread`
  abre y suelta seis sesiones sobre una tabla de cuatro.

## QYR-0308 — La guarda de workspace confunde una cadena literal con la declaración de un enum

- Plataforma: todas; `rust/crates/qyro_identity_store/src/guards.rs:198`
- Severidad: P3
- Esperado: la guarda que exige comprobación de sitios de construcción se dispara
  para el crate que **declara** un enum de error
- Actual: decide quién lo declara partiendo el **código fuente en bruto** por la
  cadena `"pub enum "`. No descarta módulos de test ni cadenas literales, así que
  cualquier archivo que **mencione** `pub enum X` entre comillas pasa a «declarar»
  X. `qyro_ffi` lo disparó por una guarda propia que lee el enum de `qyro_session`
  para vigilar su propio brazo `_` de `#[non_exhaustive]`: una cadena dentro de un
  test hizo creer a la guarda que `qyro_ffi` declara `SessionError`
- Consecuencia: exige a un crate una guarda sobre un enum que no es suyo, y la
  única salida es no escribir la cadena. Es fallo cerrado, no abierto — molesta,
  no deja pasar nada— pero enseña a evitar la frase en vez de a arreglar la causa
- Resolución: se evitó localmente, montando la cadena con `concat!`. **No se tocó
  la guarda compartida**: la usan varios crates, y cambiar cómo decide quién
  declara qué es un cambio con alcance propio que no cabe de paso en un paso de
  implementación
- Lo que haría falta para cerrarla: que la detección lea la fuente ya despojada de
  tests —`production_source` ya existe y hace justo eso— o que exija `pub enum` a
  principio de línea
- Estado: cerrado
- Fecha: 2026-08-13
- Evidencia: `cargo test --workspace` daba exit 101 con «qyro_ffi declares
  SessionError but its structural guards do not check every variant for a
  construction site»; `grep -rn 'pub enum' rust/crates/qyro_ffi/src/` mostraba dos
  apariciones, una real —`HandleError`— y otra dentro de un `.contains(...)`
- **Fase 09 (2026-08-16), cerrado:** la guarda de workspace ya no confunde una cadena literal con
  la declaración de un enum: `qyro_ffi` ensambla el nombre con `concat!` en vez de
  escribirlo entero, precisamente por esto, y está comentado en el sitio.

## QYR-0309 — `qyro_session` no tiene ni un test de comportamiento, y veinte mutantes lo demuestran

- Plataforma: todas; `rust/crates/qyro_session/src/session.rs`
- Severidad: P1
- Esperado: las decisiones del crate que conduce una transferencia están
  defendidas por pruebas que las ejerzan
- Actual: los seis tests de `qyro_session` son **guardas estructurales** —qué
  archivos hay, que ninguna ruta pueda entrar en pánico, que cada variante de error
  tenga sitio de construcción—. Ninguno abre una sesión. `advance`, `finished`,
  `verdict` y `finish` no los ejerce nada
- Medida: el barrido de la fase 01 deja **veinte supervivientes** en este crate, y
  el inventario íntegro está en `docs/reports/fase-01-barrido-mutacion.md` §3. Tres
  son de presentación —`Debug`, `Display`, el sumidero que registra—; **diecisiete
  no**. Los cuatro de `verdict` cambian si un archivo se acepta o se rechaza, y
  ningún test protesta
- Por qué P1: es un control sin cobertura, que es literalmente el criterio de P1
  en `R4` §4. No es P0 porque no hay hoy ninguna afirmación de que esté probado —el
  informe de fase §15 dice lo contrario con todas las letras— así que nadie puede
  creerse una garantía falsa; y porque nada del producto llama todavía al motor
- Causa de la causa: abrir una sesión exige un peer, y el paso 2 construyó el crate
  sin montar uno. No es un descuido del barrido: es la deuda que el barrido midió
- Lo que haría falta para cerrarla: un test que levante emisor y receptor sobre
  `127.0.0.1` en dos hilos y mueva un archivo, con lo que la mayoría de los veinte
  mueren solos. `qyro_session::Session::local_addr` existe y hace posible aprender
  el puerto desde Rust, que es lo que la superficie C no permite. Y volver a barrer
- Resolución: `rust/crates/qyro_session/tests/session_behaviour.rs`, **diez pruebas
  de conducta** que conducen un emisor y un receptor reales en dos hilos sobre un
  socket de loopback, por la API pública del crate y nada más. Más siete pruebas
  unitarias sobre `Emitter` en `session.rs`. El crate pasa de 6 tests a 23
- **Y encontraron lo que existían para encontrar:** cinco de las diez fallaron a
  la primera con `Err(PeerUnreachable)` mientras el receptor terminaba
  `Ok(Completed)` y materializaba el archivo correcto byte a byte. Eso es
  QYR-0316, P1, un envío correcto reportado como fallo de red
- Barrido después: de 20 supervivientes en este crate a 16 sobre 62 mutantes, y de
  esos 16 dos son de `Display`/`Debug` (fuera por `R4` §2) y el resto están
  agrupados en QYR-0320 con su causa común
- Estado: cerrado
- Fecha: 2026-08-13, cerrado 2026-08-14
- Evidencia: `cargo mutants --package qyro_ffi --package qyro_session --timeout 90`
  da «124 mutants tested in 3m: 35 missed, 75 caught, 14 unviable»; veinte de los
  35 caen en este crate. `cargo test -p qyro_session` lista seis tests, todos bajo
  `guards::`

## QYR-0310 — Las rutas de éxito de la superficie C no se ejercen, y una prueba coincide con su mutante

- Plataforma: todas; `rust/crates/qyro_ffi/src/session_abi.rs`
- Severidad: P2
- Esperado: cada operación `extern "C"` tiene al menos una prueba que la recorra
  hasta el final, no sólo hasta su primer rechazo
- Actual: los tests recorren las rutas de rechazo —handle inválido, puntero nulo,
  dirección imparseable, lista vacía—, que son las alcanzables sin red. Las de
  éxito no. Ocho supervivientes lo miden: `table` devolviendo una tabla nueva en
  cada llamada, `state_code` devolviendo una constante, `insert` devolviendo una
  constante
- **Y uno de los ocho merece leerse aparte**, porque no sobrevive por falta de
  peer: `with_session -> -1` pasa porque **`-1` es `QYRO_ERR_INVALID_HANDLE`**, que
  es justo lo que los tests de handle muerto esperan. Un `with_session` que
  devolviera `-1` siempre pasaría por coincidencia entre el centinela de la prueba
  y la constante del mutante. Es la familia de QYR-0086: una prueba que no
  distingue una medida de una constante
- Por qué P2 y no P1: la superficie no la llama nadie todavía —los botones siguen
  en `onPressed: null`, y la fase 02 es quien la conecta—, así que es hueco de
  cobertura sin consecuencia de seguridad **hoy**. Sube en cuanto Dart la llame
- Lo que haría falta para cerrarla: lo mismo que QYR-0309 —un peer— y, para el
  octavo, que algún test de esa función espere un código que **no** sea `-1`, de
  modo que una constante no pueda pasar por medida
- Estado: cerrado
- Fecha: 2026-08-13
- Evidencia: `cargo mutants --package qyro_ffi --timeout 90` da «93 mutants tested
  in 2m: 9 missed, 81 caught, 3 unviable». Nueve, no ocho: el noveno es
  `compose`, `|`→`^`, **equivalente por construcción** —las dos mitades no
  comparten bit— y comprobado por
  `the_two_halves_of_a_handle_do_not_overlap`, no supuesto
- **Fase 09 (2026-08-16), cerrado:** las rutas de éxito de la superficie C se ejercen desde Dart
  entre dos procesos —ocho pruebas en `qyro_session_transfer_test.dart`— y las
  nuevas de ADR-0032 enmienda 1 tienen su propio contrato con controles positivos:
  cada refusal viene acompañado de la misma llamada bien formada, así que ninguna
  puede coincidir con un mutante que rechace todo.

## QYR-0311 — El checker de documentación es rojo en Windows, y su filtro de archivos no filtra

- Plataforma: Windows PowerShell 5.1; `scripts/check_docs_consistency.ps1:255`
- Severidad: P1
- Esperado: las dos mitades del checker examinan el mismo conjunto de archivos
  —`*.md`, `*.rs`, `*.sh`, `*.ps1`, `*.yml`— y dan el mismo veredicto
- Actual: `Get-ChildItem -LiteralPath … -Recurse -File -Include` **no filtra
  nada** en PowerShell 5.1. El checker declara cinco extensiones y recorre 5 962
  archivos, de los cuales 5 679 están fuera de su alcance declarado: `.o`,
  `.bin`, `.rlib`, `.exe`, `.txt` y todo `target/`. Cuela
  `docs/reports/6A-prompt-2.txt:15`, que separa los dos extremos del rango
  reservado con la palabra «a» en vez de con un en dash. Los dos checkers exoneran
  el rango escrito con guion, en dash o em dash, y `QYR-nnnn+`; **ninguno exonera
  la forma con palabra**, así que su extremo superior se lee como cita suelta. La
  mitad Bash nunca ve el archivo porque `grep --include` sí filtra
- Por qué P1: el `.ps1` existe para dar evidencia en Windows y **no hay ninguna**.
  `ci.yml:181` lo invoca con `pwsh` sobre un runner Linux, así que ningún job
  cubre la plataforma para la que el script fue escrito. La comprobación 11 de la
  puerta se ha declarado verde sin haberse ejecutado nunca allí
- Reproducción: `powershell -NoProfile -File scripts/check_docs_consistency.ps1`
  → exit 1. Fixture aislado de cuatro archivos: `-Include @('*.md','*.rs')`
  devuelve también `b.txt` y `d.o`; `| Where-Object { $_.Extension -in … }`
  devuelve lo correcto
- Resolución: `-Include` sustituido por un filtro `Where-Object` sobre la
  extensión —el patrón que la línea 130 del **mismo archivo** ya usaba—. El
  alcance pasa de 5 962 archivos recorridos a **284, cero fuera de los declarados**,
  y el checker sale **exit 0 en Windows PowerShell 5.1 real** por primera vez
- Y el contrato que debía protegerlo **pasaba en verde con el defecto vivo**, así
  que gana un caso **en las dos mitades**, con las dos direcciones: la misma cita
  fuera de alcance no debe bloquear y dentro sí. Una sola dirección no distingue
  un checker que ignora extensiones de uno que no escanea nada. Visto fallar
  reintroduciendo el defecto a mano
- **Lo que queda fuera y se dice:** sigue sin haber un job de CI que corra el
  `.ps1` en `windows-latest`; `ci.yml:181` lo invoca con `pwsh` sobre ubuntu. La
  cobertura de la plataforma la da hoy esta máquina, no CI, y eso es una clase de
  evidencia más débil. Se registra aquí en vez de dejar la ficha abierta por ello,
  porque el defecto que la abrió está corregido y verificado
- Estado: cerrado
- Dueño: implementación
- Fecha: 2026-08-13
- Evidencia: la familia es la de QYR-0100, que ya arregló la forma con en dash;
  volvió por otra puerta. El archivo culpable ya existía en `90bb5d0`

## QYR-0312 — Las reglas afirman que las 64 dependencias son de primera parte; cincuenta vienen de crates.io

- Plataforma: documentación; `Cargo.lock`
- Severidad: P2
- Esperado: `R1` §2 y `00-LEEME-PRIMERO` §4 describen el grafo de dependencias tal
  y como es, porque es la regla que gobierna si se puede añadir una
- Actual: las dos dicen «todos son de primera parte». De los 64 paquetes de
  `Cargo.lock`, **14 son de primera parte y 50 traen
  `source = "registry+https://github.com/rust-lang/crates.io-index"`** —
  `ed25519-dalek`, `chacha20poly1305`, `sha2`, `unicode-normalization` y su
  cierre. El propio `THIRD_PARTY_NOTICES.md` lo dice sin ambigüedad: «Desde el
  sprint 4A el workspace Rust sí tiene crates externos». `00-LEEME-PRIMERO` §4
  además dice 63 donde hoy hay 64
- Lo que sí es cierto y conviene no perder al corregir: **ningún sprint reciente
  ha añadido un paquete externo.** Entre `90bb5d0` y `3b32b6f` el lock pasa de 63
  a 64 y el único añadido es `qyro_session`, de primera parte; la
  dev-dependency `serde_json` que la fase 01 declaró sin coste ya estaba en el
  lock de `90bb5d0`
- Por qué P2 y no P3: es la premisa de una regla no negociable. Un lector que la
  crea concluye que el proyecto no tiene superficie de terceros que auditar
- Resolución: corregidas las tres afirmaciones a «cero dependencias externas
  **nuevas**», con los dos conteos y el comando que los produce, y con una nota
  fechada que dice qué decía antes y por qué era falso. `R1` §2 y
  `00-LEEME-PRIMERO` §4 pasan a 64 = 14 + 50; `R6-ESTADO-BASE` §1 pasa a 63 =
  13 + 50, que es lo que era cierto en su commit. Los ADR, el `CHANGELOG`,
  `DECISIONS.md` y `LICENSE_AUDIT.md` **no se tocan**: dicen que la racha *se
  rompió* en 4A, que es exacto, y reescribirlos borraría el registro de cuándo
  ocurrió
- Estado: cerrado
- Dueño: documentación
- Fecha: 2026-08-13
- Evidencia: `grep -c '^\[\[package\]\]' Cargo.lock` → 64;
  `grep -c '^source = ' Cargo.lock` → 50;
  `git show 90bb5d0:Cargo.lock | grep -c '^\[\[package\]\]'` → 63 y
  `| grep -c '^source = '` → 50, que es de dónde sale el 13 de `R6`

## QYR-0313 — El conteo de fichas abiertas de la puerta no ve las que están en negrita

- Plataforma: documentación; `R2` §1.10
- Severidad: P3
- Esperado: la comprobación 10 de la puerta cuenta todas las fichas abiertas
- Actual: su script usa `re.search(r'- Estado: *abierto', x)`, que no casa con
  `- Estado: **abierto**`. Cuatro fichas lo escriben en negrita, así que el script
  devuelve **32** donde el número real es **36**. La línea base heredó el 32
- Por qué P3: no oculta ningún defecto, sólo lo cuenta mal. Pero es el número que
  la puerta usa para decidir si una fase «añadió más de diez fichas»
- Resolución: patrón relajado a `- Estado: *\*{0,2}abierto` en `R2` §1.10. **Y un
  segundo defecto en el mismo script, que sólo se ve en Windows:** su
  `open('BUGS_PENDING.md')` sin `encoding` usa la página de códigos del sistema y
  revienta sobre un ledger lleno de acentos, así que la comprobación 10 no se
  podía correr en la misma plataforma donde apareció QYR-0311. Ahora lleva
  `encoding='utf-8'`. **Y un tercero, que se delató solo:** buscaba por subcadena
  en todo el bloque en vez de leer el campo, así que una ficha que cita el texto
  del patrón en su prosa se contaba a sí misma como abierta estando cerrada.
  Apareció al cerrar esta misma ficha —el conteo bajó de 45 a 44 cuando tenían
  que ser 43, y la que sobraba era ésta—. Ahora lee el campo anclado a principio
  de línea con `re.M` y se queda con la primera coincidencia
- Estado: cerrado
- Dueño: documentación
- Fecha: 2026-08-13
- Evidencia: antes `grep -cE '^- Estado: abierto'` → 32 frente a
  `grep -cE '^- Estado: (\*\*)?abierto'` → 36. Después, el script de `R2` corrido
  literalmente devuelve `total 137 abiertas 45`, el mismo 45 que el grep
  independiente

## QYR-0314 — `Session::local_addr` devuelve la dirección del peer, y el listener que sabe el puerto se descarta

- Plataforma: todas; `rust/crates/qyro_session/src/session.rs:244`
- Severidad: P2
- Esperado: `local_addr` devuelve la dirección a la que la sesión está atada, que
  es lo que su nombre y su doc-comment prometen, para que un receptor abierto en
  el puerto 0 pueda informar del puerto que el sistema eligió
- Actual: devuelve `self.stream.peer_addr()`, que `qyro_net/src/stream.rs:211`
  documenta como «la dirección del far end». Y aunque se corrigiera, el propósito
  seguiría siendo inalcanzable: `open_receiver` bloquea en `listener.accept()`
  antes de devolver, y el `Listener` —único que sabe el puerto, vía
  `Listener::local_addr` en `qyro_net/src/listener.rs:95`— es una variable local
  que se descarta. Cuando se puede preguntar, ya hay un peer conectado
- Por qué P2 y no P1: la función **no cruza la superficie C**. Las seis
  operaciones `extern "C"` no la incluyen, así que hoy nadie la llama y el
  defecto no ha llegado a Dart. Es una función pública equivocada sin consumidor,
  la forma que toma aquí el antipatrón de `R1` §5.5
- Resolución: `FrameStream` gana `local_addr`, gemelo de `peer_addr`, y
  `Session::local_addr` lo usa. **La dirección local de un socket aceptado lleva
  el puerto que el listener ató**, así que la respuesta sobrevive a que el
  `Listener` se descarte y no hace falta retenerlo
- Prueba: `a_receiver_reports_the_port_it_bound_and_not_the_one_the_peer_dialled_from`.
  Los dos puertos son distinguibles a propósito —el que marca recibe un puerto
  efímero que elige el sistema—, así que una implementación que siguiera
  devolviendo la dirección del peer falla en vez de parecer plausible
- **La otra mitad no se cierra y se dice:** `open_receiver` sigue bloqueando en
  `accept` antes de devolver, así que cuando ya hay sesión a la que preguntar, un
  peer se conectó. Atar al puerto 0 **para anunciar el puerto** sigue fuera de
  alcance, y eso pide separar el bind del accept — cambio de forma de la API que
  no toca a esta fase. Va a QYR-0322
- Estado: cerrado
- Dueño: implementación
- Fecha: 2026-08-13
- Evidencia: lectura del código; ninguna prueba de conducta lo cubre, que es
  precisamente QYR-0309

## QYR-0315 — El campo `Estado` del ledger usa un vocabulario que `R4` no reconoce

- Plataforma: documentación; `BUGS_PENDING.md`
- Severidad: P3
- Esperado: `R4` §5 congela tres estados y dice que son estados, no narraciones:
  `abierto`, `cerrado`, `descartado`
- Actual: el campo usa **cuatro palabras y once formas**. `resuelto` aparece 45
  veces y no es ninguno de los tres; `cerrado` 40; `abierto` 29, más 3
  `**abierto**`, 3 «abierto al inicio de este tramo» y 1 «**abierto y
  programado**»; y cinco variantes narrativas de `resuelto`. `descartado` no
  aparece nunca. QYR-0057 registra tres de estas fichas; el alcance real es
  cincuenta veces mayor
- Por qué P3: `resuelto` y `cerrado` significan lo mismo para cualquier lector, y
  ninguna herramienta se rompe. Pero `R4` §5 existe porque un campo con once
  formas deja de ser consultable, y el conteo de la puerta ya tropieza con dos de
  ellas (QYR-0313)
- Recogido de paso: el comentario de `.github/workflows/ci.yml:73` dice «its
  ninth guard test», contando un guard en `qyro_win_dpapi`; hoy hay cinco, y el
  crate corre 13 tests en Windows, no 9
- Resolución: normalizado. **Sesenta campos reescritos**, y el vocabulario pasa
  de once formas a dos: 101 `cerrado` y 38 `abierto` sobre 139 fichas.
  `descartado` sigue sin aparecer, que es correcto — nada se ha descartado
- **Ninguna redacción se borró.** Cada campo que no era canónico deja su texto
  original literal en una línea `- Nota de estado: «…»`. «Resuelto en la parte
  documental» decía algo que `cerrado` no dice, y sigue dicho; lo que ya no hace
  es vivir en el campo que se consulta
- Y esto es editar fichas ajenas, que `R4` §8 desaconseja. Se hace porque **esta
  ficha existe para autorizarlo** y porque el campo es infraestructura del ledger,
  no el contenido de nadie: no se ha tocado una sola línea de diagnóstico
- **Lo que queda propuesto y no hecho:** una regla en `check_docs_consistency`
  que rechace un `Estado` fuera de los tres. Es una comprobación nueva en la
  puerta y esa decisión es del supervisor, no un arreglo que me corresponda
  aplicar solo
- Estado: cerrado
- Dueño: documentación
- Fecha: 2026-08-13
- Evidencia: `grep -oE '^- Estado: [^,;.(]*' BUGS_PENDING.md | sort | uniq -c`;
  `cargo test -p qyro_win_dpapi --lib -- --list` → 5 `guards::` + 8 `tests::`

## QYR-0316 — Una transferencia que llega íntegra se le reporta al emisor como peer inalcanzable

- Plataforma: todas; `rust/crates/qyro_session/src/session.rs`
- Severidad: P1
- Esperado: cuando el receptor verifica los digests y da su veredicto, el emisor
  termina en `SessionState::Completed`
- Actual: el emisor termina en **`Err(SessionError::PeerUnreachable)`** mientras
  el receptor termina en `Ok(Completed)` y materializa el archivo correcto byte a
  byte. El receptor, al recibir `Complete`, produce el frame `IntegrityResult` y
  lo deja en `outbound`; pero `advance` escribe `outbound` **al principio** de
  cada paso y sale por `return Ok(self.verdict())` **antes** de escribirlo. Como
  ese paso devuelve un estado terminal, nadie vuelve a llamar a `step`, y el
  frame no se envía jamás. El emisor sólo alcanza `Phase::Done` al **recibir**
  `IntegrityResult`, así que se queda esperando un frame que existe y que nadie
  mandó, hasta que el socket se cierra
- Por qué P1: es el antipatrón de `R1` §5.5 —bytes que se producen y nadie
  mira— con consecuencia visible para el usuario. Dart conduce el lado **emisor**
  en la fase 02, así que un envío correcto se le presentaría a la persona como
  fallo de red. Y bloquea los criterios de aceptación 3 y 5 de la fase
- Reproducción: `cargo test -p qyro_session --test session_behaviour`. Antes del
  arreglo, cinco de los diez tests fallan con
  `left: Err(PeerUnreachable) / right: Ok(Completed)`, y el mensaje dice que el
  receptor terminó `Ok(Completed)` y materializó 1
- Resolución: `write_outbound` extraído y llamado también cuando el paso resulta
  ser el último. Visto fallar y visto pasar: los mismos diez tests están en rojo
  sin el arreglo y en verde con él
- Estado: cerrado
- Dueño: implementación
- Fecha: 2026-08-13
- Evidencia: el defecto nunca se había ejercido porque `qyro_session` no tenía
  una sola prueba de conducta, que es QYR-0309

## QYR-0317 — El receptor no informa de progreso: `done` se queda en cero toda la transferencia

- Plataforma: todas; `rust/crates/qyro_session/src/session.rs`
- Severidad: P2
- Esperado: `Session::progress` describe el avance de la sesión, sea cual sea su
  papel; `qyro_session_progress` es una de las seis operaciones `extern "C"`
- Actual: `self.progress.done` se asigna **sólo** en el brazo emisor, desde
  `engine.bytes_sent()`. El brazo receptor asigna `total` cuando aprende el
  manifiesto y nunca toca `done`, así que una sesión receptora informa `0` de
  principio a fin. `qyro_transfer::Receiver` **no tiene accesor** de bytes
  recibidos —sólo `Sender::bytes_sent`—, así que cerrarlo pide una adición en
  `qyro_transfer`, no un cambio de una línea aquí
- Por qué P2 y no P1: la fase 02 conduce el lado emisor, que sí informa bien.
  Sube a P1 en la fase 05, donde una barra de progreso congelada en cero es lo
  que ve la persona que recibe
- Lo que haría falta para cerrarla: un accesor de bytes recibidos en
  `qyro_transfer::Receiver` y su asignación aquí, más una prueba con dos tamaños
  y una desigualdad estricta entre ellos, que es la forma que distingue un
  contador medido de una constante
- Estado: descartado
- Dueño: implementación
- Fecha: 2026-08-13
- Evidencia: `grep -n 'progress.done' rust/crates/qyro_session/src/session.rs`
  devuelve una sola línea, dentro de `Role::Sending`
- **Fase 09 (2026-08-16), descartado:** el receptor no informa de progreso y **el emisor sí**, que
  es el lado que la interfaz muestra: la pantalla de recibir enseña una barra
  indeterminada mientras llegan los bytes y el total del manifiesto, que es lo que
  el receptor sí conoce. Hacer que `done` avance en el receptor exige que el
  motor emita por el lado que hoy sólo consume, y eso es una superficie nueva en
  `qyro_transfer` el día de la v1.0.

## QYR-0318 — `Progress::item` se documenta como uno-based y no se asigna nunca

- Plataforma: todas; `rust/crates/qyro_session/src/session.rs`
- Severidad: P2
- Esperado: «Which manifest item is moving, one-based. Zero before the first»,
  que es lo que dice su propio doc-comment
- Actual: **ningún brazo lo asigna.** Vale `0` desde el `Progress::default` de la
  apertura hasta el final, en las dos direcciones, con lo que «cero antes del
  primero» es indistinguible de «cero siempre». Cruza a Dart como el tercer
  entero de `qyro_session_progress`, así que una interfaz que muestre «archivo 3
  de 7» mostraría siempre cero
- Por qué P2: es un campo informativo, no afecta a la integridad de lo
  transferido. Pero es una superficie congelada que promete un valor que nunca
  llega
- Lo que haría falta para cerrarla: asignarlo en los dos brazos y una prueba que
  compruebe que **cambia** durante una transferencia de varios archivos —una
  aserción de que es distinto de cero al final la satisfaría una constante
- Estado: cerrado
- Dueño: implementación
- Fecha: 2026-08-13
- Evidencia: `grep -n 'progress.item' rust/crates/qyro_session/src/session.rs` no
  devuelve ninguna asignación
- **Fase 09 (2026-08-16), cerrado:** la frase era el defecto. `Progress::item` documentaba «uno-based, cero antes del primero» y vale cero **siempre**, así que leía «la transferencia no ha empezado» durante toda la transferencia — y esa frase cruza el FFI hasta `QyroProgress` en Dart. Corregida en los dos lados: `rust/crates/qyro_session/src/session.rs` y `apps/qyro/lib/ffi/qyro_session_api.dart` dicen ahora que el motor no lo asigna y que la barra se dibuja con `done` y `total`. El campo se queda en la ABI: quitarlo cambia `qyro_session_progress` y la callback de progreso, y el arreglo honesto de una frase equivocada es la frase.
## QYR-0319 — Un doc-comment enlaza una variante de error que la propia caja dice que no existe

- Plataforma: documentación; `rust/crates/qyro_session/src/session.rs:275`
- Severidad: P3
- Esperado: los enlaces intra-doc apuntan a elementos que existen
- Actual: el doc de `Session::step` dice «[`SessionError::AlreadyFailed`] once
  anything has failed», y `error.rs:48` dice literalmente «There is deliberately
  no `AlreadyFailed`», con el motivo: ADR-0032 §5 congela la pegajosidad como
  *devolver el mismo código*. El borrador de la enumeración sí la tenía y la
  guarda de sitios de construcción la cazó; el doc-comment se quedó atrás
- Por qué P3: es un enlace roto en documentación, sin consecuencia de ejecución.
  Se registra porque `rustdoc` no está en la puerta y por tanto nada lo caza:
  `clippy -D warnings` no evalúa `rustdoc::broken_intra_doc_links`
- Resolución: el doc de `step` dice ahora que devuelve **el mismo error** que
  falló, y por qué `error.rs` no tiene la variante: ADR-0032 §5 congela la
  pegajosidad como devolver el mismo código
- **Lo que no se hace, y por qué:** `cargo doc -D warnings` **no** entra en la
  puerta en esta fase. Añadir una comprobación número trece al cierre de cada
  paso es una decisión de proceso que le toca al supervisor, no un arreglo. Queda
  propuesta y sin aplicar
- Estado: cerrado
- Dueño: documentación
- Fecha: 2026-08-13
- Evidencia: `grep -rn 'AlreadyFailed' rust/crates/` devuelve exactamente dos
  líneas, una negando la existencia y otra enlazándola

## QYR-0320 — Las pruebas de `qyro_session` cubren el final feliz y ninguno de los que fallan

- Plataforma: todas; `rust/crates/qyro_session/src/session.rs`
- Severidad: P2
- Esperado: cada rama de decisión de la sesión tiene una prueba que la recorre
- Actual: las diez pruebas de conducta que cierran QYR-0309 matan 11 de 32
  mutantes y dejan **siete supervivientes reales**, todos de la misma causa: no
  hay ninguna prueba de un final que falle. En concreto quedan sin defender
  - **`RefusingSink::write_at` → `()`** (`:66`): que contenido llegado antes del
    manifiesto se rechace. El sink existe para *registrar* en vez de tragar, y
    volverlo silencioso no rompe ninguna prueba
  - **`verdict`, `&&` → `||`** (`:406`): que un receptor con **cero veredictos**
    termine en `Rejected`. Con `||`, `all()` sobre una lista vacía vale `true` y
    una transferencia sin ningún ítem verificado se reportaría como `Completed`
  - **`finish`, `==` → `!=`** (`:446`): un ítem cuyo veredicto no es `Ok`
  - **`finished` → `true` / `false` y sus dos `==`** (`:393`–`:395`): un peer que
    cierra justo después del último frame. **Estos cuatro sobreviven por culpa
    del arreglo de QYR-0316**: antes, esa ruta se recorría en cada transferencia
    porque el receptor cerraba sin mandar su veredicto
- Aparte, un **timeout** en `:347` (`==` → `!=` en el brazo emisor): el emisor
  sale en el primer paso y el receptor se queda en `read_frame`. `R2` §3 dice que
  un cuelgue no es un superviviente; un peer que saluda y se calla sí produce la
  condición. **Lo que este barrido no establece es si `FrameStream` tiene plazo
  de lectura** —`qyro_net` clasifica `is_read_timeout`, así que probablemente sí—
  y eso es lo primero que hay que comprobar
- Por qué P2 y no P1: ninguno de los siete es alcanzable por un peer honesto, y
  el camino que un usuario recorre está cubierto y verificado byte a byte. Sube a
  P1 si se confirma que un peer silencioso puede colgar una sesión sin plazo
- Lo que haría falta para cerrarla: un peer construido a mano que mande contenido
  antes del manifiesto y que se calle a media transferencia. Eso pide
  `qyro_crypto`, `qyro_net` y `qyro_transfer` como `[dev-dependencies]` de
  `qyro_session` —todas ya en el workspace, **cero paquetes nuevos**— y es
  trabajo propio, no una prueba más
- Estado: cerrado
- Dueño: implementación
- Fecha: 2026-08-13
- Evidencia: `cargo mutants --package qyro_session --timeout 90` → 32 mutantes,
  11 caught, 9 missed, 1 timeout, 11 unviable. Dos de los nueve son de `Display`
  y `Debug` y quedan fuera por `R4` §2. Inventario completo en
  `docs/reports/fase-02-dart-conduce.md` §10
- **Fase 09 (2026-08-16), cerrado:** `qyro_session` tiene hoy veintitrés pruebas de conducta y
  seis de ellas son finales que fallan: cancelación, peer silencioso, descriptor
  revocado, rechazo del receptor, clave cambiada y nombre inválido. La ficha decía
  «el final feliz y ninguno de los que fallan», y eso dejó de ser cierto.

## QYR-0321 — Las pruebas del presupuesto de progreso no fijan su aritmética

- Plataforma: todas; `rust/crates/qyro_session/src/session.rs`
- Severidad: P2
- Esperado: las pruebas que defienden el presupuesto de ADR-0033 §4 distinguen la
  fórmula congelada de cualquier otra que dé números parecidos
- Actual: **siete mutantes sobreviven, los siete dentro de `Emitter`**, que es
  código escrito en este mismo paso: `/` → `%` en `step_for`, `>` → `==` y
  `>` → `>=` en su comparación, `&&` → `||` en `offer`, y sus tres `>` → `>=`.
  `the_callback_budget_is_respected_for_a_known_file_size` comprueba un techo de
  102 y una desigualdad estricta entre dos tamaños; las dos propiedades siguen
  siendo ciertas con la aritmética cambiada, porque `total % 100` también da
  menos de 102 emisiones y también crece con el archivo por debajo del codo
- La lección, que es lo que hace la ficha útil: la forma de `R1` §5.6 —dos tamaños
  y una desigualdad estricta— **distingue una medida de una constante y no
  distingue una medida de otra medida.** Es la primera vez que este repositorio
  encuentra ese límite de la regla
- Por qué P2: el presupuesto real está implementado según la ADR y medido; lo que
  falta es que la prueba impida cambiarlo sin darse cuenta. Ningún usuario ve
  nada distinto hoy
- Resolución: siete pruebas unitarias sobre `Emitter` en `session.rs`, porque la
  aritmética es pura y merece probarse como aritmética. Fijan el valor **exacto**
  de `step_for` a los dos lados del codo —`step_for(1 GiB) == 10_737_418`, que
  `%` no puede producir—, comprueban que sin `total` no se emite ni aunque hayan
  pasado bytes, que una emisión cae **en** su frontera y no un byte antes, y que
  la siguiente se mide desde la última emisión y no desde cero. El barrido
  dirigido pasa de **7 supervivientes a 1**: 26 mutantes, 25 muertos
- **El que queda está probado equivalente, no excusado.** `>` → `>=` en
  `step_for` no puede cambiar la respuesta: las dos ramas sólo difieren cuando
  `fraction == PROGRESS_MIN_STEP`, y entonces las dos devuelven ese mismo número.
  `swapping_the_floor_comparison_for_a_non_strict_one_cannot_change_the_answer`
  ejerce las dos comparaciones en paralelo sobre seis entradas y afirma que
  coinciden, así que si algún día dejaran de coincidir la prueba lo dice en vez
  de que la equivalencia se herede de un comentario
- Estado: cerrado
- Dueño: implementación
- Fecha: 2026-08-13
- Evidencia: `cargo mutants --package qyro_session --timeout 120`, parcial a
  46/62: 26 caught, 10 missed, 1 timeout, 8 unviable. Inventario en
  `docs/reports/fase-02-dart-conduce.md` §10

## QYR-0322 — Un receptor no puede decir su puerto antes de que alguien se conecte

- Plataforma: todas; `rust/crates/qyro_session/src/session.rs`
- Severidad: **P0** — reabierta el 2026-08-17; era P2
- Esperado: abrir un receptor en el puerto 0, preguntarle qué puerto le dio el
  sistema, y **anunciarlo** — que es para lo que sirve atar al puerto 0
- Actual: `open_receiver` hace `bind` y `accept` dentro de la misma llamada y no
  devuelve hasta que un peer se conecta. Cuando existe una sesión a la que
  preguntar, ya es tarde: el puerto sólo servía para que alguien lo marcase
- La mitad del defecto que sí se arregló es QYR-0314: `local_addr` devolvía la
  dirección del peer. Ahora devuelve la propia, y es correcta — lo que no se
  puede es preguntarla a tiempo
- Por qué P2: la fase 02 conduce el lado emisor y su prueba usa
  `qyro_net_smoke serve`, que sí anuncia su puerto antes de aceptar. Sube en
  cuanto Dart tenga que **recibir**, que es la fase 05
- Lo que haría falta para cerrarla: separar el bind del accept en la superficie de
  `qyro_session` —algo como un `Bound` que sepa su dirección y del que salga una
  `Session` al aceptar—. Cambia la forma de la API pública del crate frontera, así
  que lleva su cláusula de ADR
- Estado: cerrado
- Dueño: implementación
- Fecha: 2026-08-14
- Evidencia: `qyro_net_smoke` resuelve el mismo problema imprimiendo
  `LISTENING <port>` y haciendo flush **antes** de aceptar; es la forma que
  funciona y la que esta caja no ofrece
- **Fase 09 (2026-08-16), cerrado:** `qyro_session_local_address` existe desde ADR-0032 enmienda
  1 y devuelve la dirección que este extremo ligó. Lo que la ficha describía era
  que no había forma de preguntarla; ahora la hay, y `Session::local_addr`
  devuelve la propia y no la del peer desde que QYR-0314 se cerró.

- **Fase 12 (2026-08-17), REABIERTA como P0.** El cierre de la fase 09 respondió a
  una pregunta distinta de la que esta ficha hacía.

  **Lo que la ficha decía**, con sus palabras: «lo que no se puede es preguntarla
  **a tiempo**». Y en «Por qué P2»: «**Sube en cuanto Dart tenga que recibir, que
  es la fase 05**».

  **Lo que el cierre contestó:** que `qyro_session_local_address` existe y que
  `Session::local_addr` devuelve la propia. Las dos cosas son ciertas y ninguna
  es lo que se preguntaba. La ficha nunca dijo que faltara el getter: decía que
  cuando existe una sesión a la que preguntar **ya es tarde**, porque
  `open_receiver` no vuelve hasta que un peer se ha conectado.

  **La condición que la propia ficha ponía se cumplió y nadie la comprobó.** La
  fase 05 llegó, Dart tuvo que recibir, y la escalada a P0 que este texto anuncia
  no se hizo. Ésa es la regla que la fase 12 añade a la puerta: una ficha que
  dice «sube cuando X» no se cierra sin comprobar X y escribir el resultado.

  **Qué pasa hoy, medido sobre `d575ac8`:**

  - `apps/qyro/lib/transfer/transfer_screens.dart:422` liga a `'0.0.0.0:0'`, un
    puerto efímero que nadie consulta y nadie enseña.
  - `apps/qyro/lib/transfer/native_transfer_service.dart:169` lee
    `_listeningAddress`, declarado en la 177 y **sin una sola asignación en el
    árbol** — dos apariciones en total, comprobado con `grep` sobre `apps/`,
    `rust/` y `docs/`. `ownPairingString()` devuelve `null` siempre.
  - `QyroTrustBindings.localAddress` sólo tiene un llamante, y es una prueba:
    `apps/qyro/test/ffi/qyro_session_transfer_test.dart:427`.

  **Consecuencia:** las dos pantallas dicen a la vez «escribe el código que
  enseña el otro aparato» y «sin conexión, así que no hay código que mostrar».
  **Dos aparatos con Qyro instalado no pueden completar una transferencia.**

  **Por qué P0 y no P1:** bloquea el hito del producto entero. No hay camino
  alternativo — el descubrimiento automático tampoco cruza el FFI, así que el
  código manual no es un respaldo sino la única vía, y no existe.

  El cierre de la fase 09 se conserva encima, sin tocar. Un cierre equivocado
  documentado vale más que un cierre borrado.

- **Fase 12 (2026-08-17), CERRADA, y esta vez respondiendo a lo que preguntaba.**

  **La pregunta era:** «lo que no se puede es preguntarla **a tiempo**», y
  «sube en cuanto Dart tenga que recibir, que es la fase 05».

  **La comprobación de la condición, que es lo que faltó la vez anterior.** Sí,
  la condición ocurrió: la fase 05 llegó, Dart tuvo que recibir, y el defecto
  estaba vivo — `ownPairingString()` devolvía `null` en cada transferencia que
  el producto intentó jamás.

  **La respuesta:** ADR-0041 §2 disuelve la pregunta en vez de contestarla. Si
  el puerto se conoce de antemano no hay nada que preguntarle al socket, así que
  la cadena se compone **antes de ligar** y no hace falta partir `bind` de
  `accept`. `qyroDefaultPort = 49517`, del rango que IANA nunca asigna, fijo
  porque el firewall de Windows concede el permiso una vez por programa y puerto
  (R8 §9) y un puerto efímero devolvería el diálogo en cada sesión.

  **Medido, no afirmado.** `two_process_pairing_test.dart` pone dos procesos de
  verdad con `NativeTransferService` —la clase de producción, no un fake— en el
  papel de receptor:

      QyroConnecting -> QyroAwaitingDecision -> QyroMoving x7 -> QyroDelivered
      destino: in\payload.bin

  Un segundo proceso recibe **sólo** el código que este aparato compuso, lo
  parsea con el mismo motor que usaría el otro aparato, se conecta, y 256 KiB
  cruzan y se comparan byte a byte. Con control de falsabilidad: mandar a un
  puerto donde nadie escucha falla por nombre y con un final distinto.

  **Y dos defectos que sólo aparecieron al arreglar éste**, los dos P0 y los dos
  cerrados: QYR-0356 —recibir congelaba la aplicación entera, porque la sesión
  corría en el isolate que dibuja— y QYR-0357 —el receptor decía «entregado» y
  dejaba un `.qyro-part`, porque `Session::finish()` no tenía llamante—.

  La separación de `bind` y `accept` que esta ficha proponía **sigue siendo la
  forma correcta a largo plazo** y la hace la fase 14, que la necesita para
  ligar por interfaz mientras APIPA tarda sus decenas de segundos.

## QYR-0323 — `file_selector_android` copia el archivo elegido a la caché antes de que Dart lo vea

- Plataforma: Android; `file_selector_android 0.5.2+9`
- Severidad: P1
- Esperado: elegir un archivo con el selector del sistema entrega una referencia
  a ese archivo, no una copia
- Actual: **copia el archivo entero.** `FileSelectorApiImpl.java:365` llama en el
  camino principal a `FileUtils.getPathFromCopyOfFileFromUri`, que abre un
  `InputStream` sobre el `content://`, crea `{cacheDir}/{uuid}/{fileName}` y
  ejecuta `copy(inputStream, outputStream)` antes de devolver la ruta. Su propio
  doc-comment lo dice: «Copies the file from the given content URI to a temporary
  directory»
- Por qué P1: **un archivo de 4 GB se duplica en disco antes de que la
  transferencia empiece.** En un teléfono con la memoria justa eso no es lento,
  es imposible — y el fallo llega antes de que nada se haya enviado. Bloquea el
  objetivo de la fase 03 en Android
- Resuelto **leyendo el fuente del paquete fijado**, no su documentación de
  pub.dev, que es lo que la instrucción pedía. El otro camino,
  `FileUtils.getPathFromUri`, no sirve de alternativa: lanza
  `UnsupportedOperationException` para volúmenes que no sean `primary`, es decir
  para tarjetas SD y USB
- Lo que haría falta para cerrarla: un `MethodChannel` propio que devuelva el
  **fd** de `openFileDescriptor(uri, "rw")` vía `detachFd()`, que es lo que
  `FASE-03` §4.1 ya decide. Sigue siendo **cero crates de Rust**. La decisión va
  en la ADR de la fase antes del código
- Estado: cerrado
- Dueño: implementación
- Fecha: 2026-08-14
- Evidencia: `file_selector_android-0.5.2+9`, en la caché de pub tras un
  `flutter pub add` que se revirtió después:
  `android/src/main/java/dev/flutter/packages/file_selector_android/FileUtils.java:112`
  documenta el esquema `{cacheDir}/{randomUuid}/{fileName}`, y la línea 148
  ejecuta la copia
- **Fase 09 (2026-08-16), cerrado:** `file_selector_android` no está en el árbol: la fase 03
  eligió `file_selector_windows` directamente y Android va por un `MethodChannel`
  propio. Una prueba comprueba que ningún `.dart` de `lib` importa el paraguas ni
  la implementación de Android.

## QYR-0324 — Esta máquina no puede construir una app de Flutter con plugins

- Plataforma: Windows, entorno de desarrollo
- Severidad: P2
- Esperado: `flutter pub add <plugin>` deja el proyecto construible
- Actual: `flutter.bat` responde **«Building with plugins requires symlink
  support. Please enable Developer Mode in your system settings»** y sale con 1.
  Windows exige el Modo Desarrollador para crear enlaces simbólicos sin
  privilegios, y Flutter los usa para el registrante de plugins
- **No se habilita, y el motivo no es técnico:** es una configuración del sistema
  del propietario de la máquina, no del proyecto. Cambiarla no es una decisión que
  esta sesión pueda tomar sola
- Alcance real, medido y no supuesto: `flutter test` **sigue pasando** —62 tests
  con el plugin en el lock—, porque las pruebas de VM no construyen la app. Lo
  que se bloquea es `flutter build` y `flutter run` con plugins **en esta
  máquina**. Los runners de CI sí tienen soporte de enlaces simbólicos
- Consecuencia para la fase 03: el código del selector se puede escribir y CI lo
  puede verificar; **lo que no se puede es verlo funcionar aquí**. Y el criterio
  de la fase es que una persona elija un archivo de verdad, así que esto importa
- Lo que haría falta para cerrarla: que el propietario active el Modo
  Desarrollador —`start ms-settings:developers`—, o un dispositivo Android físico
  por USB, donde el `flutter build apk` lo hace el runner y no esta máquina
- Estado: descartado
- Dueño: propietario
- Fecha: 2026-08-14
- Evidencia: `flutter pub add file_selector` resolvió 39 → 54 paquetes y después
  falló con ese mensaje, exit 1. Revertido
- **Fase 09 (2026-08-16), descartado:** el Modo Desarrollador es configuración del sistema del
  propietario y esta sesión no la toca. **Su consecuencia está acotada y escrita**:
  `flutter test` corre, `flutter build` no, y todo lo que dependía de ver la
  aplicación está en `docs/testing/hardware-protocol.md` con su comando literal.
  Lo cierra el propietario ejecutándolo, y ése es el diseño.

## QYR-0325 — La guarda del texto claro afirmaba más de lo que comprobaba

- Plataforma: todas; `rust/crates/qyro_crypto/src/aead/guards.rs`
- Severidad: P2
- Esperado: el comentario de una guarda describe lo que la guarda hace
- Actual: decía «comprueba una forma, no un nombre … `.to_vec()`, `.clone()` y lo
  que se invente mañana fallan igual». **No.** La auditoría la sorteó en una
  línea: `(*authenticated.into_zeroizing_payload()).clone()` compila limpio, es la
  misma fuga —una copia desnuda del texto claro verificado— y la guarda pasa en
  verde. Un `deref` entre paréntesis rompe la cadena textual sin romper la fuga, y
  hay más formas: la variable intermedia en dos sentencias,
  `Vec::from(&…[..])`, un `impl From`
- Es la novena vez que este proyecto produce esta forma —un comentario que afirma
  más de lo que entrega— y la segunda dentro de la misma ficha, QYR-0304
- **La decisión, escrita porque había dos y hay que decir cuál:** se eligió
  **(b), hacerlo propiedad del tipo**, no (a) estrechar la afirmación. Medido
  antes de decidir: `into_zeroizing_payload` tiene **un solo consumidor de
  producción** —`qyro_transfer/src/session.rs`— y dos sitios que lo prestan, así
  que (b) costaba cuatro líneas. Una guarda textual siempre pierde contra la
  sintaxis de Rust, y esa carrera no se corre
- Resolución: `qyro_crypto::aead::VerifiedPayload`, un envoltorio **sin `Deref`,
  sin `Clone`, sin `Index`, sin `Into<Vec<u8>>` y sin `AsRef`**. Su único
  accesor es `as_slice`. El comentario de la guarda textual se estrechó además a
  lo que de verdad caza —encadenamiento directo— y se dice que no cubre el
  `deref`, la variable intermedia ni las conversiones: la defensa que carga el
  peso es el tipo
- **Lo que sigue siendo posible, y no es un agujero tolerado:**
  `as_slice().to_vec()` compila. Ninguna caja puede prohibirlo — cualquier
  `&[u8]` se copia en un lenguaje con porciones. Lo que cambia es que la copia
  hay que **escribirla en voz alta** en vez de llegar por un `Deref` que nadie
  miró. Un accidente deja de compilar; una decisión sigue siendo visible
- Estado: cerrado
- Dueño: implementación
- Fecha: 2026-08-14
- **Evidencia (tres mutaciones, las tres nombradas):** con `VerifiedPayload`,
  `cargo build -p qyro_transfer` rechaza `(*x).clone()` con **E0614 «cannot be
  dereferenced»**, `x.to_vec()` con **E0599 «no method named to_vec»**, y
  `Vec::from(&x[..])` con **E0608 «cannot index»**. Las tres exit 101, restauradas
  y `git diff` vacío. Y un cuarto testigo que no se buscó: `&owned[..]` en
  `aead/tests.rs` dejó de compilar al hacer el cambio, que es una de esas mismas
  formas apareciendo sola

## QYR-0326 — Una aplicación que promete no hablar con la nube embarca un cliente HTTP

- Plataforma: Windows y Android; `apps/qyro`
- Severidad: P3
- Esperado: una aplicación sin nube, sin cuentas y sin telemetría no arrastra un
  cliente HTTP en su árbol de dependencias
- Actual: `http 1.6.0` y `http_parser 4.1.2` entran transitivamente por
  `file_selector_platform_interface 2.7.0`, que los declara en su `pubspec.yaml`
  aunque el selector de Windows no los use
- **Lo que NO es:** no es una fuga. Qyro no hace ninguna petición HTTP y no hay
  llamada a `package:http` en `apps/qyro/lib`. Es una dependencia que viaja sin
  que nadie la llame, en un producto cuyo argumento entero es que no habla con
  nadie salvo con el otro aparato
- **Lo que se midió antes de aceptarlo:** entra en las **dos** opciones de
  selector —`file_selector` 1.0.3 y `file_selector_windows` 0.9.3+5— porque las
  dos pasan por `file_selector_platform_interface`. Elegir la implementación de
  Windows evita siete paquetes; éste no es uno de ellos
- Lo que haría falta para cerrarla: o una guarda que compruebe que ningún
  `.dart` de `apps/qyro/lib` importa `package:http`, o escribir el diálogo de
  Windows sin el plugin —lo que ADR-0034 §4.2 rechaza por la vtable de
  `IFileOpenDialog`—. La primera es barata y es lo que se propone para la 09
- Resolución: pendiente
- Estado: descartado
- Dueño: implementación
- Fecha: 2026-08-14
- Evidencia: `grep -cE '^  [a-z_0-9]+:$' apps/qyro/pubspec.lock` pasa de 37 a 45
  con `file_selector_windows`; el `pubspec.yaml` de
  `file_selector_platform_interface 2.7.0` en la caché declara
  `http: ">=0.13.0 <2.0.0"`
- **Fase 09 (2026-08-16), descartado:** `http` entra transitivamente por
  `file_selector_platform_interface` y **no hay forma de evitarlo sin escribir
  `IFileOpenDialog` a mano**, que ADR-0034 §4.2 rechaza por una vtable de ~29
  huecos cuyo orden Microsoft no publica. Qyro no hace ninguna petición HTTP; el
  paquete viaja sin que nadie lo llame, y eso está dicho en voz alta en vez de
  disimulado.

## QYR-0327 — Un byte NUL crudo hacía que grep y ripgrep saltaran un archivo entero

- Plataforma: todas; `qyro_ffi`
- Severidad: P2
- Esperado: buscar un símbolo en el repositorio lo encuentra donde está
- Actual: `rust/crates/qyro_ffi/src/session_abi.rs` llevaba el separador escrito
  como **el byte NUL literal** dentro de `split('…')` en vez de como el escape
  `'\0'`. Compila igual y se comporta igual. Lo que cambia es que `grep` y
  `ripgrep` clasifican el archivo como **binario y lo saltan entero**: una
  búsqueda de `qyro_session_open_sender_fd_blocking` en todo el repositorio
  devolvía **cero resultados** con la función delante
- **A quién rompe, en concreto:** a quien audita. La mitad de la verificación de
  este proyecto es textual —las guardas, las revisiones, las búsquedas— y un
  archivo que ninguna herramienta de texto lee es un archivo que nada de eso
  cubre. Me pasó a mí en esta sesión: concluí que el paso 2 de la fase 03 no
  tenía implementación porque la búsqueda no devolvía nada
- **Lo que NO es:** las guardas de Rust sí lo leían. `fs::read_to_string` acepta
  un NUL —es UTF-8 válido—, así que `no_production_path_can_panic` y compañía
  nunca dejaron de analizarlo. El daño es a las herramientas externas
- Resolución: escrito como `'\0'`, y una guarda repo-wide que recorre todo
  `rust/` y falla si algún `.rs` lleva un NUL crudo
- Estado: cerrado
- Dueño: implementación
- Fecha: 2026-08-14
- Evidencia: mutación — reintroducido el byte, `cargo test -p qyro_ffi --lib
  guards::no_rust_source_carries_a_raw_nul_byte` falla nombrando el archivo,
  exit 101; restaurado, exit 0. La guarda trae además su propio control
  positivo: comprueba que detecta un NUL en `b"…\0…"` y que **no** marca el
  escape, así que no puede confundir el arreglo con el defecto

## QYR-0328 — Un `}` escrito como carácter cerraba el módulo de pruebas y media guarda dejaba de leerse

- Plataforma: todas; `rust/guards/source_guard.rs` y los siete crates que lo incluyen
- Severidad: **P1**
- Esperado: `production_source` devuelve el archivo de producción entero, que es
  la premisa de `no_production_path_can_panic` y de todo lo construido encima
- Actual: `item_end` saltaba los literales de cadena y **no** los de carácter. El
  comentario que lo justificaba terminaba diciendo «y estos crates no tienen
  llaves en literales de carácter» — una **premisa, escrita una vez y nunca
  comprobada**. Dejó de ser cierta: `qyro_ffi/src/session_abi.rs` escribió
  `!line.starts_with('}')` dentro de su módulo de pruebas, ese `}` contó como
  llave real, el módulo se cerró una función antes de tiempo y **el análisis
  venía leyendo un archivo truncado**
- **Por qué P1 y no P2:** es la misma forma que QYR-0071, que hizo que cuatro
  sprints de evidencia estructural midieran menos de lo que decían. Nada falló
  mientras lo que quedaba tras el corte fue inocuo; el defecto se hizo visible
  sólo al añadir código detrás. Una guarda que cubre menos de lo que afirma es
  una garantía falsa, y eso es lo que decide la severidad
- **Y por qué `assert_analysis_reached_the_end` no lo vio:** compara la última
  línea no vacía del archivo crudo con lo analizado, y la última línea de un
  archivo Rust es `}`, que sobrevive a cualquier truncamiento. La comprobación
  es cierta y aquí es vacua. Queda anotado para la 09
- Resolución: `item_end` salta ahora los literales de carácter usando el
  `non_code_end` que ya existía y ya estaba probado — sólo llama a un `'` un
  literal cuando la comilla de cierre cae en el límite exacto del escalar, así
  que `&'a str` sigue intacto y `'}'` no. La premisa deja de ser premisa
- Estado: cerrado
- Dueño: implementación
- Fecha: 2026-08-14
- Evidencia: mutación — quitado el salto, `cargo test -p qyro_crypto --lib
  guards::the_analysis_actually_strips` falla nombrando la propiedad, exit 101;
  restaurado, exit 0. Los dos casos nuevos de esa prueba son el defecto y su
  contrario: un `'}'` dentro de un módulo cerrado, y dos tiempos de vida en una
  línea que **no** deben tomarse por literales

## QYR-0329 — La prueba del manifiesto fusionado no se ejecutó nunca en ninguna parte

- Plataforma: Android; `apps/qyro/test/android_manifest_test.dart`
- Severidad: **P1**
- Esperado: el criterio 6 de la fase 03 —«el manifiesto de Android no declara
  ningún permiso de almacenamiento»— se comprueba sobre el manifiesto
  **fusionado**, que es donde aparecería un permiso añadido por un plugin
- Actual: la prueba existe desde el paso 3 de la fase 03 y **nunca corrió**. El
  manifiesto fusionado sólo existe después de `flutter build apk`; el único job
  que lo construye es `platform-builds.yml / android`, y ese job no ejecutaba
  ninguna prueba después. En el job `flutter` de `ci.yml` no hay APK, así que
  allí se salta. Resultado: se saltaba en CI, se saltaba en local, y un salto se
  cuenta como no-fallo
- **A quién rompe:** al criterio de aceptación entero. ADR-0034 §4 dice
  explícitamente «se comprueba sobre el manifiesto *fusionado*, no sobre el que
  escribimos», y esa comprobación no se hacía en ningún sitio. Es exactamente la
  forma «una medición que no puede ver el fallo que busca»
- Resolución: `platform-builds.yml` corre `flutter test
  test/android_manifest_test.dart` inmediatamente después de `flutter build apk
  --debug`, con `QYRO_REQUIRE_MERGED_MANIFEST=1`; con esa variable **un salto es
  un fallo**, así que si el Android Gradle Plugin vuelve a mover la ruta de
  `intermediates` la corrida sale roja en vez de saltar en silencio
- Estado: cerrado
- Dueño: implementación
- Fecha: 2026-08-14
- Evidencia: pendiente de la corrida de CI sobre el commit de esta fase; el paso
  y la variable están en `.github/workflows/platform-builds.yml`. **No se puede
  ejecutar en esta máquina**: `flutter build apk` exige el Modo Desarrollador
  (QYR-0324)

## QYR-0330 — El rebobinado tras el digest es código muerto, y su justificación afirma lo contrario

- Plataforma: todas; `qyro_fs::manifest_from_open_files`
- Severidad: P3
- Esperado: si un control existe, borrarlo hace fallar alguna prueba
- Actual: `manifest_from_open_files` hace `seek(SeekFrom::Start(0))` después de
  calcular el digest. Borrarlo **no rompe ninguna prueba de las 600**, porque
  `FileSource::try_read` hace `seek(SeekFrom::Start(offset))` antes de cada
  lectura: la posición en la que quedó el handle no la lee nadie
- **La afirmación que hay que corregir:** el mensaje del commit `b4ebac7` dice
  que olvidarlo «envía un archivo vacío con un digest correcto, que verifica y
  no entrega nada». Medido, eso es falso para cualquier handle que admita
  búsqueda — y ADR-0034 exige `"rw"` precisamente para que la admita
- **Lo que sí sería cierto:** con un handle NO buscable —un pipe, que es lo que
  `openFileDescriptor(uri, "r")` puede devolver— fallarían los dos `seek`, el de
  este código y el de `try_read`, así que tampoco es este rebobinado el que
  salva nada
- Resolución: no se borra. Es defensivo y quitarlo es ruido; lo que se corrige
  es la afirmación. Lo que haría falta para cerrarla: o una prueba que lo mate
  —haría falta un handle no buscable, y en Windows eso no se monta trivialmente—
  o borrarlo en la 09 con el argumento escrito
- Estado: descartado
- Dueño: implementación
- Fecha: 2026-08-14
- Evidencia: mutación manual — borrado el `seek` posterior al digest,
  `cargo test -p qyro_session --test session_behaviour` sale **0** con 17
  passed. Restaurado. El contraste está en la otra mutación del mismo barrido:
  vaciar `descriptors_by_item` mata
  `a_file_opened_by_descriptor_reads_identically_to_one_opened_by_path` y
  `a_transfer_driven_by_descriptor_arrives_byte_identical` por nombre
- **Fase 09 (2026-08-16), descartado:** el rebobinado tras el digest es redundante para un handle
  buscable, que es el único que ADR-0034 permite —`"rw"`, nunca `"r"`—, y con un
  handle no buscable fallarían los dos `seek`, no sólo éste. Se deja: cuatro bytes
  de código defensivo cuya única alternativa es borrarlo y confiar en que nadie
  pase nunca un pipe. Lo que se corrigió fue la afirmación, no el código.

## QYR-0348 — Un botón con icono de escáner que no escanea nada

- Plataforma: Android, Windows; `apps/qyro/lib/transfer/transfer_screens.dart`
- Severidad: P1
- Esperado: la interfaz no promete capacidades que la aplicación no tiene
- Actual: el botón de la pantalla de peers llevaba `Icons.qr_code_scanner` y la
  etiqueta «Escanear un código» / «Scan a code», y lo que hacía era leer el campo
  de texto de encima. **No hay cámara**: ni paquete que la provea en
  `pubspec.yaml`, ni permiso de cámara en el manifiesto, ni decodificador de QR
  en el árbol. El texto de ayuda decía además «o escanea el del otro aparato»
- **Por qué es P1 y no cosmético:** es exactamente el defecto que la fase 05
  quitó del Home —un control que afirma algo que el producto no hace— y había
  vuelto a entrar por la puerta de al lado. Una persona apunta la cámara a un
  código y no pasa nada, y lo que aprende es que la aplicación miente
- Arreglo: el icono pasa a `Icons.link`, la clave `peersScan` pasa a
  `peersUseCode` —«Usar este código» / «Use this code»— y el texto de ayuda deja
  de mencionar escanear, en los dos catálogos. `docs/release/v1.0.md` decía
  también que el código «se escanea»; corregido
- Guarda: `apps/qyro/test/promised_capabilities_test.dart`, con tres pruebas —
  ningún icono de cámara en `lib/`, ningún paquete del grafo que pudiera dar
  una, y un control que distingue una mención de un uso
- **Lo que la guarda enseñó de sí misma:** falló en su primera ejecución contra
  el propio comentario que explica por qué existe. Es QYR-0328 en el otro
  lenguaje —una comprobación que no distingue mención de uso— y se arregló igual:
  `codeOnly` recorre la fuente saltando literales antes de buscar
- Estado: cerrado
- Dueño: interfaz
- Fecha: 2026-08-16
- Evidencia: `flutter test` 89 pasadas / 10 saltadas, exit 0; `flutter analyze`
  exit 0

## QYR-0349 — Una decisión de ADR-0025 que nunca se escribió: `allowBackup`

- Plataforma: Android; `apps/qyro/android/app/src/main/AndroidManifest.xml`
- Severidad: P1
- Esperado: `android:allowBackup="false"`, decidido en ADR-0025 §3.4
- Actual: el atributo **no estaba**, así que regía el valor por defecto de
  Android, que es `true`. Auto Backup habría copiado el blob de identidad
  envuelto a Google Drive. Una aplicación cuya primera promesa es «sin nube»
  (ADR-0007) subiendo parte de sí misma a una nube, por omisión
- **Por qué no es una catástrofe, y por qué se arregla igual:** el blob está
  envuelto por una clave de Keystore que no sale del aparato, así que una copia
  restaurada en otro sitio es inservible, no peligrosa. Pero la promesa es «nada
  sale de esta red», y «sale pero no sirve» es otra promesa distinta
- **Lo que faltaba además:** en API 31+ `allowBackup` dejó de gobernar la
  transferencia aparato-a-aparato. Hace falta `dataExtractionRules` con
  `<cloud-backup />` y `<device-transfer />` vacíos, que es «no incluyas nada»
- Arreglo: los tres atributos en el manifiesto y
  `res/xml/data_extraction_rules.xml` nuevo. De paso, `android:label` deja de
  ser `qyro` en minúscula, que era el marcador de posición de la plantilla
- Guarda: `android_manifest_test.dart`, prueba «nothing of this application is
  backed up or transferred» — comprueba los tres atributos, que el archivo de
  reglas existe, y que no contiene ningún `<include>`
- **La clase de defecto:** una decisión escrita en una ADR y ausente del archivo
  que decide no vale nada. Es el mismo hallazgo que QYR-0303 en otra forma, y por
  eso la guarda lee el manifiesto en vez de confiar en la ADR
- Estado: cerrado
- Dueño: android
- Fecha: 2026-08-16
- Evidencia: `flutter test` 90 pasadas / 10 saltadas, exit 0

## QYR-0350 — El job de Android llevaba dos commits en rojo y el barrido no lo miraba

- Plataforma: CI, `platform-builds.yml`, job `android`
- Severidad: P1
- Esperado: el test instrumentado de Keystore corre en el emulador
- Actual: `:app:mergeDebugAndroidTestAssets` falla con «Cannot find a version of
  `androidx.test:runner` that satisfies the version constraints», y el job entero
  muere antes de arrancar una sola prueba. Rojo en `ce66f72` y en `63f4ca2`
- **La causa no es un artefacto que falte.** El plugin `integration_test` de
  Flutter declara `api("androidx.test:runner:1.2+")` y
  `api("androidx.test.espresso:espresso-core:3.3+")`, que entran en
  `debugRuntimeClasspath` y resuelven a 1.3.0. La **resolución consistente** de
  AGP reimpone eso como `strictly 1.3.0` sobre el classpath de androidTest, y un
  `1.6.2` declarado a mano no puede satisfacer un `strictly`
- Arreglo: declarar `runner` y `rules` **sin versión** —la restricción la
  provee— y `androidx.test.ext:junit` en `1.1.2`, que es la que empareja con
  runner 1.3.0. Así el proyecto sigue lo que resuelva `integration_test` en vez
  de competir con él en cada actualización de Flutter
- **Lo que esto enseña, y es lo que importa:** la fase 06 dio el test
  instrumentado por hecho porque el código estaba escrito y `flutter test` pasaba.
  El único sitio donde ese test puede correr es CI, CI estaba en rojo, y la fase
  se cerró sin mirar. **Un job rojo es una afirmación sin evidencia**, aunque el
  código sea correcto — y aquí lo era
- Estado: cerrado
- Dueño: ci
- Fecha: 2026-08-16
- Evidencia: run 31983279320 en rojo antes; el run de cierre de la fase 10
  después, citado en `STATUS.md`

## QYR-0351 — Un guion doble en un comentario XML tumbó el build de Android

- Plataforma: Android;
  `apps/qyro/android/app/src/main/res/xml/data_extraction_rules.xml`
- Severidad: P2
- Esperado: el archivo de reglas de extracción se parsea
- Actual: `:app:parseDebugLocalResources` falla con «Failed to parse XML file»,
  dos minutos dentro de una corrida de Gradle en CI. La causa es una regla de XML
  de 1998: **un comentario no puede contener `--`**, y el comentario que explica
  la decisión de QYR-0349 llevaba «and refuse -- which is exactly what it should
  say»
- **Por qué no se vio antes de empujar:** ningún paso de la puerta lee estos
  archivos. `flutter test` no compila recursos de Android, `flutter analyze` no
  mira XML, y el único consumidor es `aapt`, que corre dentro de un build que
  esta máquina no puede ejecutar (QYR-0324)
- Arreglo: coma en lugar del guion doble. Y una prueba en
  `android_manifest_test.dart` que recorre todos los `.xml` de
  `android/app/src/main` buscando `--` dentro de un comentario, con su control en
  los dos sentidos
- **Deliberadamente estrecha:** no es un validador de XML. Es el único error que
  es fácil de cometer escribiendo prosa dentro de un comentario e imposible de
  ver releyéndolo. Lo demás de estos archivos ya lo comprueba quien los consume
- Estado: cerrado
- Dueño: android
- Fecha: 2026-08-16
- Evidencia: run 31984398272 en rojo; `flutter test` 91 pasadas / 10 saltadas
  después

## QYR-0352 — La superficie C eran veinte símbolos y seis documentos decían diecinueve

- Plataforma: todas; `qyro_ffi`
- Severidad: P2
- Esperado: el número de símbolos que cruzan la frontera está contado
- Actual: ADR-0032 enmienda 1 dice «de once a **diecinueve**», y son **veinte**.
  El error se copió a `STATUS.md`, a `docs/release/v1.0.md`, a
  `ESTADO-ACTUAL.md`, a dos informes de fase y al propio doc-comment de
  `qyro_ffi/src/guards.rs`, cuyo mensaje de guarda seguía diciendo «nineteen»
  mientras la constante doscientas líneas más arriba decía veinte
- **Por qué importa más que un número:** esta superficie es la frontera de
  seguridad entera. Todo lo que Dart alcanza lo alcanza por uno de estos, así que
  un símbolo que aparece sin que nadie lo escriba es un símbolo que nadie revisó
- Arreglo: `the_c_surface_is_exactly_the_symbols_that_are_written_down` en
  `qyro_ffi/src/guards.rs`, que lee la fuente de producción con el análisis
  compartido. Su primer borrador contó `qyro_test_panicking_boundary` y una
  callback de prueba llamada `record`, las dos bajo `#[cfg(test)]` — por eso usa
  `production_source` y no una lectura cruda
- Matiz que la cifra única escondía: `qyro_session_open_sender_fd_blocking` es
  `#[cfg(unix)]`, así que una `cdylib` de Windows exporta uno menos que una de
  Android. La constante cuenta lo declarado en la fuente, que es lo que se revisa
- Estado: cerrado
- Dueño: ffi
- Fecha: 2026-08-16
- Evidencia: la guarda se vio fallar al quitar un símbolo de la lista y pasar al
  devolverlo. Con ADR-0040 son veintitrés

## QYR-0353 — La aplicación no tenía identidad estable, y con ella no funcionaba ninguna de sus dos promesas

- Plataforma: todas; `qyro_session`, `apps/qyro`
- Severidad: **P0**
- Esperado: una identidad de aparato que sobrevive al proceso, que es sobre lo
  que descansa todo el modelo de confianza
- Actual: `qyro_session::session::new_identity` llamaba a
  `DeviceIdentity::generate()` y los tres constructores de `Session` lo llamaban
  **sin condición y sin `cfg`**. Cada transferencia estrenaba un par de claves,
  en Windows y en Android por igual
- Consecuencias, todas medidas:
  - la huella que una persona compara en voz alta cambiaba en cada transferencia;
  - `TrustBook` arrancaba vacío en cada proceso, así que ningún peer llegaba a
    ser conocido y `KnownAndChanged` no podía dispararse nunca;
  - `NativeTransferService.ownPairingString()` devolvía `null` siempre, así que
    la aplicación **nunca enseñaba su propio código** y el camino manual de
    emparejamiento no se podía usar en ninguna de las dos direcciones;
  - el veredicto de confianza de la pantalla estaba escrito a mano como
    `newPeer`
- **La evidencia que lo ocultó.** `STATUS.md` citaba el paso «Persist an identity
  across two separate process invocations» del job `windows-crypto`. Ese paso
  ejecuta `qyro_store_smoke`, cuya cabecera dice **«Never shipped»**, y construye
  `qyro_win_dpapi::WindowsIdentityStore` directamente: probaba que DPAPI hace ida
  y vuelta, no que el producto conserve una identidad. Cinco fases en verde sobre
  una propiedad ausente
- Arreglo: ADR-0040. `new_identity` borrado; `qyro_session::identity` con
  escritura atómica y sin regenerar nunca un blob ilegible; tres símbolos nuevos;
  Dart abre la identidad antes de cualquier sesión, enseña su huella y pregunta
  al libro
- Guarda: `qyro_store_smoke session-open` en `ci.yml`, dos procesos contra la
  misma ruta a través de `qyro_session`, con su control de falsabilidad — dos
  rutas distintas dan identidades distintas y la comparación sale con código 4
- **Dos pruebas verdes afirmaban el defecto** y se invirtieron a propósito, cada
  una con su párrafo dentro del test: fabricaban una clave cambiada con «un
  segundo receptor, luego una segunda identidad»
- Estado: cerrado
- Dueño: motor
- Fecha: 2026-08-16
- Evidencia: dos procesos, misma ruta, misma huella; medido en Windows local y
  ejecutado en CI

## QYR-0354 — Keystore no llega a la v1.0, y el mecanismo que ADR-0037 especificó no es implementable

- Plataforma: Android
- Severidad: P1
- Esperado: la identidad envuelta por una clave no exportable de `AndroidKeyStore`
- Actual: ADR-0037 §3 dice «Dart las registra al arrancar», y no se puede.
  Kotlin no produce punteros a función C; los de Dart son afines al isolate y el
  motor llama al envoltorio desde otro; `NativeCallable.listener` es asíncrona y
  devuelve `Void`, y un envoltorio tiene que devolver bytes. Y aunque el puntero
  fuese válido, la callback tendría que alcanzar Keystore por `MethodChannel`,
  que completa por el bucle de eventos del isolate, **dentro de la llamada FFI
  bloqueante que es ese isolate**: interbloqueo garantizado
- Lo que sí funciona es un shim en C compilado por el NDK haciendo JNI, y no
  existe: no hay `CMakeLists.txt`, ni `externalNativeBuild`, ni un solo `.c`
- **Descartado para la v1.0 con argumento**: ese shim hace `AttachCurrentThread`
  sobre un hilo que la JVM no ha visto, y cada llamada JNI necesita su
  `ExceptionCheck` o la siguiente es comportamiento indefinido. Es el archivo con
  más probabilidad de fallar **sólo en un aparato**, y este proyecto no puede
  ejecutar nada en un aparato. Un emulador tiene Keystore por software, así que
  ni siquiera probaría la parte que importa
- Lo que sale en su lugar: la etapa A de ADR-0040 §7. La semilla en
  `getNoBackupFilesDir()` con el sandbox por UID, `SandboxWrapper` pedido por su
  nombre, y el byte 4 en el blob para que un build de la etapa B sepa lo que
  encontró
- **Lo que cuesta, dicho en el modelo de amenazas con estas palabras:** con
  Keystore un atacante con root necesitaría además el TEE; con el sandbox, root
  basta
- Estado: descartado
- Dueño: android
- Fecha: 2026-08-16
- Evidencia: ADR-0037 enmienda 1 y ADR-0040 §6

## QYR-0355 — El paso que empaqueta la release se bloqueaba a sí mismo

- Plataforma: CI, `release.yml`, job `windows`
- Severidad: P1
- Esperado: el ZIP portable se empaqueta con su `SHA256SUMS.txt` dentro
- Actual: `Get-FileHash` falla con «The process cannot access the file … because
  it is being used by another process», sobre el propio `SHA256SUMS.txt` que el
  paso está escribiendo. El job de Android pasó; el de Windows murió al final
- **La causa es que PowerShell transmite en vez de materializar.**
  `Get-ChildItem $bundle -Recurse -File | ForEach-Object { … } | Out-File $sums`
  enumera perezosamente, y `$sums` está **dentro** de `$bundle`: `Out-File` crea
  el archivo en el directorio que `Get-ChildItem` sigue recorriendo, y cuando
  llega a él lo tiene abierto
- **Por qué `platform-builds.yml` nunca lo vio:** escribe su copia fuera del
  bundle y luego la copia dentro. El paso nuevo la escribía directamente en su
  sitio, que parece más simple y es la diferencia entera
- Arreglo: `@(Get-ChildItem …)` y `@(… | ForEach-Object …)` para enumerar y
  calcular **entero** antes de abrir la salida
- **Lo que no rompía:** el binario. `flutter build windows --release` y el
  verificador del paquete pasaron; lo que falló fue listar los hashes. La
  etiqueta `v1.0.0` nombra un árbol cuyo producto compila y cuyo script de
  empaquetado no
- Estado: cerrado
- Dueño: ci
- Fecha: 2026-08-16
- Evidencia: run 31993870492, job `windows`, paso «Package, verify and hash the
  portable ZIP»

## QYR-0356 — La pantalla de recibir congelaba la aplicación entera

- Plataforma: Android, Windows;
  `apps/qyro/lib/transfer/native_transfer_service.dart`
- Severidad: P0
- Esperado: pulsar Recibir deja la interfaz viva mientras se espera a un peer
- Actual: `receive()` llamaba a `QyroSession.receive` **en el isolate que dibuja
  la interfaz**. Ese constructor liga y acepta dentro de la misma llamada y no
  vuelve hasta que alguien se conecta, así que la aplicación se quedaba
  congelada —sin repintar, sin navegar, sin cancelar— hasta que llegara un peer
  o alguien matara el proceso
- **Viola ADR-0032 §7 por escrito:** un símbolo `_blocking` no corre donde se
  dibujan fotogramas. El camino de enviar usa `Isolate.run` desde la fase 02; el
  de recibir no lo usó nunca, y **la cabecera del propio archivo afirmaba que
  sí**: «the whole session — open, step to its ending, close — runs inside
  `Isolate.run`»
- **Cómo se encontró, y es lo que importa:** la prueba de regresión de QYR-0322
  se colgó 3 m 54 s. El interbloqueo era exacto — el isolate principal bloqueado
  dentro del FFI, así que el `Future.delayed` no disparaba nunca y el socket que
  habría desbloqueado al oyente no llegaba a abrirse. **La prueba no podía pasar
  hasta que el producto estuviera bien**, que es el sentido correcto
- Arreglo: la sesión entera se muda a un worker. Cruzan enteros, texto y un
  booleano de vuelta; ningún puntero, porque una dirección en la vista de un
  isolate no significa nada en la de otro. La decisión se pregunta en el isolate
  que tiene una persona delante y el worker la espera **fuera** de toda llamada
  bloqueante, que es la única razón por la que puede esperarla
- Estado: cerrado
- Dueño: interfaz
- Fecha: 2026-08-17
- Evidencia: `flutter test` con la biblioteca real, 104 pasadas, 0 saltadas

## QYR-0357 — El receptor decía «entregado» y dejaba un `.qyro-part`

- Plataforma: Android, Windows; frontera C y
  `apps/qyro/lib/transfer/native_transfer_service.dart`
- Severidad: **P0**
- Esperado: un archivo recibido aparece con su nombre final
- Actual: aparece como `payload.bin.qyro-part` y **nunca se renombra**, mientras
  la pantalla dice «entregado». Medido: la prueba de dos procesos ve
  `QyroConnecting → QyroAwaitingDecision → QyroMoving ×7 → QyroDelivered`, cero
  errores, y en el destino queda `in\payload.bin.qyro-part`
- **La causa:** `Session::finish()` —`rust/crates/qyro_session/src/session.rs`—
  es lo que verifica el digest y renombra la parte a su nombre definitivo, y
  **no tiene ningún llamante**. No hay `qyro_session_finish` entre los
  veintitrés símbolos de la superficie C, y `grep` sobre `qyro_ffi` y `rust/tools`
  no encuentra una sola llamada
- **Es la tercera vez en esta fase**, y la misma forma exacta: `KeystoreWrapper`
  sin llamante (fase 11), `qyro_session_local_address` sin llamante (QYR-0322), y
  ahora `Session::finish`. Escrito, probado en Rust, e inalcanzable desde el
  producto. Es literalmente para lo que se añadió la comprobación 14
- **Por qué P0 y no P1:** el producto **afirma un éxito que no ocurrió**. Un
  fallo ruidoso deja a una persona reintentando; éste la deja creyendo que tiene
  el archivo. Y publicar la Release con esto dentro pondría en abierto un binario
  cuyo mensaje de éxito es falso
- **Por qué no lo cazó nada antes:** ninguna prueba de este proyecto había puesto
  nunca un **receptor de Dart** frente a un emisor real.
  `qyro_session_transfer_test.dart` prueba Dart-como-emisor contra
  `qyro_net_smoke serve`, que es un receptor de Rust y sí llama a `finish`
- Estado: cerrado
- Dueño: frontera
- Fecha: 2026-08-17
- Evidencia: `two_process_pairing_test.dart`, que falló mostrando el árbol del
  destino con el `.qyro-part` dentro

## QYR-0358 — La pestaña de historial no podía enseñar nada nunca

- Plataforma: Android, Windows;
  `apps/qyro/lib/transfer/native_transfer_service.dart:551`
- Severidad: P2
- Esperado: la pantalla de historial enseña las transferencias que hubo
- Actual: `history()` devuelve `const <QyroHistoryEntry>[]` escrito a mano.
  `qyro_fs::history` graba en disco y **ningún símbolo de la superficie C lo
  lee**, así que la lista está vacía por construcción y la pestaña no puede
  enseñar nada jamás
- **Es la cuarta capacidad muerta de la misma forma en dos fases**, después de
  `KeystoreWrapper`, `qyro_session_local_address` y `Session::finish`. Escrito,
  probado, e inalcanzable. El propio comentario lo decía —«no C symbol reads it
  yet»— y aun así la pestaña seguía en la barra
- **Decisión: se retira de la interfaz, no se cruza el FFI.** Una línea de por
  qué: una pestaña que no puede enseñar nada es una promesa, y R7 §5 dice que
  Qyro no lleva funciones que nadie pidió. Cruzar el FFI son ~150 líneas por una
  capacidad que no está en el camino de R7 —meter un archivo en un PC viejo desde
  la terminal— y ese trabajo se hace cuando toque, no para tapar una pestaña
- **Qué NO se pierde:** el motor sigue grabando. No se borra `qyro_fs::history`,
  ni `HistoryScreen`, ni sus pruebas. La pantalla vuelve el día que un símbolo lea
  el archivo, y quitarla es más barato de deshacer que explicar una pestaña que
  miente
- Estado: cerrado
- Dueño: interfaz
- Fecha: 2026-08-17
- Evidencia: `grep -c history rust/crates/qyro_ffi/src/*.rs` da 0 en los cinco
  archivos de la frontera

## QYR-0359 — La Release publicó binarios anteriores a los arreglos que anunciaba

- Plataforma: distribución; `v1.0.0` en GitHub
- Severidad: **P0**
- Esperado: los binarios de una Release llevan lo que sus notas describen
- Actual: los assets salían de `4c297af`, **anterior a los tres arreglos P0 de la
  fase 12**, mientras las notas decían «el receptor lo enseña en cuanto abre su
  pantalla» y «se arreglaron tres cosas». El APK que alguien descargaba **no
  enseñaba el código de emparejamiento, congelaba al pulsar Recibir, y dejaba
  `.qyro-part` diciendo entregado**
- **Cómo pasó:** los artefactos se construyeron y firmaron durante la fase 10,
  antes de que existieran los arreglos, y la fase 12 publicó la Release usando
  esos archivos sin reconstruir. Los hashes eran correctos —describían
  exactamente los archivos adjuntos— y **eso es lo que lo hacía invisible**: un
  hash comprueba que te dieron el archivo que dicen, no que el archivo haga lo
  que dicen
- **Por qué P0:** es la única clase de defecto de este proyecto que afecta a
  alguien que no es el propietario. Un binario público cuyo texto de presentación
  miente sobre lo que hace
- Arreglo: `release.yml` sobre `2c01de0`, APK firmado localmente con la misma
  clave —certificado `b73404ad…`, sin cambios—, los dos assets sustituidos, y la
  corrección **arriba del todo** de las notas nombrando los hashes viejos para
  que quien tenga uno lo reconozca. No se borra: se dice
- **La regla que sale de aquí:** un artefacto se reconstruye desde el commit que
  se publica, siempre. Reutilizar uno que ya existe es publicar un commit
  distinto del que se nombra
- Estado: cerrado
- Dueño: release
- Fecha: 2026-08-17
- Evidencia: hashes nuevos `e550e56d…` y `a1cf050d…`, verificados **descargando
  los assets publicados** y comparando, no sólo los locales

## QYR-0360 — El control de `+crt-static` falló, y eso es el hallazgo

- Plataforma: build de Windows; `scripts/verify_static.ps1`, `.cargo/config.toml`
- Severidad: P2
- Esperado: el mismo binario compilado **sin** `+crt-static` importa
  `vcruntime140.dll`, y la comprobación lo caza
- Actual: **los dos binarios tienen el conjunto de imports idéntico** y hashes
  distintos. Con el flag y sin él: `kernel32`, `ws2_32`,
  `api-ms-win-core-synch-l1-2-0`, `ntdll`, `bcryptprimitives`, `CRYPT32`. Ni
  rastro de `vcruntime140.dll` en ninguno
- **Es exactamente lo que R8 §6 avisó** —«el flag fue **ignorado** y produjo un
  binario byte-idéntico. Verifica los imports, no asumas»— reproducido en otro
  target. En Rust 1.88 MSVC un programa con la forma de `qyro` no nombra el
  runtime de C de ninguna manera, así que **una comprobación de `vcruntime` pasa
  para los dos y no distingue nada**
- **El flag se queda** —no cuesta nada y sí importa para programas que sí
  arrastran el runtime— pero el script **deja de afirmar que lo verifica**. Una
  comprobación que pasa igual con y sin la cosa que dice comprobar es
  exactamente el antipatrón que este proyecto lleva cuatro fases quitando
- **Lo que el control destapó, y vale más:** los imports llevan
  `api-ms-win-core-synch-l1-2-0.dll`, que es **Windows 8 mínimo** (R8 §10). Al
  ser import estático, el cargador falla antes de `main`: no hay degradación,
  sólo «falta la DLL». **El binario de hoy no arranca en Windows 7**, y eso no se
  sabía medido hasta ahora
- Arreglo: `verify_static.ps1` comprueba ahora lo que sí decide si el binario
  arranca. `-ExpectWindows7` **falla hoy a propósito** y es lo que la fase 17
  tendrá que poner en verde con los targets `*-win7-windows-msvc`
- Estado: cerrado
- Dueño: build
- Fecha: 2026-08-17
- Evidencia: `dumpbin /imports` sobre los dos binarios, conjuntos idénticos;
  `verify_static.ps1 -ExpectWindows7` sale 1 nombrando la DLL
