# Cómo probar Qyro en un PC y un teléfono

**Esta página está escrita para alguien que no ha leído nada de este
repositorio.** No hace falta saber qué es Rust, ni Flutter, ni un manifiesto de
Android. Hace falta un PC con Windows, un teléfono Android y un cable USB.

Todo lo que hay que teclear está en bloques como éste, para copiar y pegar tal
cual:

```
así
```

---

## 0. Lo que NO va a funcionar, y se dice antes

**Nada de esto se ha ejecutado nunca en un aparato de verdad.** Está probado
entre dos programas en la misma máquina y en los servidores de pruebas. Ningún
teléfono ha ejecutado nunca esta aplicación. Es la primera vez.

Cosas que **no** van a funcionar todavía, para que no se pierda tiempo en ellas:

| Qué | Por qué |
|---|---|
| **Windows 7 y Windows 8** | El `.exe` usa una función del sistema que Windows 8 introdujo, así que en Windows 7 ni siquiera arranca: sale un error de «falta un DLL». Hace falta Windows 10 u 11. |
| **iPhone** | No hay versión para iOS. Hace falta un Mac para construirla y este proyecto no tiene ninguno. |
| **El canal por cable serie (COM)** | Sólo existe en el `.exe`. El teléfono **no** lo tiene, así que teléfono↔PC por serie no es una prueba posible. |
| **Mandar un archivo por QR desde el teléfono** | La dirección está fijada al revés: **el PC dibuja los códigos y el teléfono los lee**. Al revés no existe: el PC no tiene cámara. |
| **La pestaña de historial** | No está. La aplicación tiene tres pestañas, no cuatro. |
| **Que los dos aparatos se encuentren solos** | Puede que sí y puede que no, y **depende del router, no de Qyro**. Casi todos los routers domésticos lo permiten; casi ninguna Wi-Fi pública. Cuando no funciona, el **código tecleado sí**, siempre. Por eso el código tecleado es el camino principal de esta guía. |

**Si algo de la lista de arriba falla, no es un fallo:** es lo que dice esta
tabla.

---

## 1. Lo que hace falta

- Un PC con **Windows 10 u 11**.
- Un teléfono **Android 11 o más nuevo** (API 30+).
- Un **cable USB** que una los dos.
- Los dos **en la misma red Wi-Fi**. Si el PC va por cable de red y el teléfono
  por Wi-Fi, sirve igual **siempre que sea el mismo router**.
- Un archivo de prueba grande: **100 MB o más**. Un archivo pequeño cruza tan
  rápido que no se ve nada.

En el teléfono, activa antes la **depuración por USB**:

> Ajustes → Acerca del teléfono → pulsa siete veces sobre «Número de compilación»
> → vuelve atrás → Opciones de desarrollador → **Depuración por USB**.

---

## 2. Construir los dos archivos

**Salta esta sección si ya te han dado el `qyro.exe` y el `.apk`**, y ve a §3.

Todo lo de aquí se hace **una vez**, en el PC, con una ventana de **PowerShell**
abierta en la carpeta del repositorio.

> **Aviso, y es el que más tiempo puede hacer perder: hay DOS `qyro.exe`.**
>
> | Cuál | Dónde queda | Qué es |
> |---|---|---|
> | `target\x86_64-pc-windows-msvc\release\qyro.exe` | §2.1 | **El de terminal.** Un solo archivo, sin instalador, se copia y funciona. **Es el de esta guía.** |
> | `apps\qyro\build\windows\x64\runner\Release\qyro.exe` | `flutter build windows` | La **aplicación con ventanas**, la misma que el teléfono. Necesita **toda la carpeta** que la rodea: si copias sólo el `.exe`, no arranca. |
>
> Se llaman igual y **no son intercambiables**. Esta guía usa el primero, porque
> un solo archivo es más fácil de mover y de comprobar. El segundo es opcional y
> está en §8.

### 2.1 El binario de terminal (`qyro.exe`)

```
cd D:\Qyro\repo
cargo build --release -p qyro_cli --target x86_64-pc-windows-msvc
```

Queda en `target\x86_64-pc-windows-msvc\release\qyro.exe`.

Y se comprueba que no necesita nada instalado para arrancar:

```
pwsh -File scripts\verify_static.ps1 -Binary target\x86_64-pc-windows-msvc\release\qyro.exe
```

Tiene que decir `[PASS]`. Si dice `[BLOCKER]`, ese `.exe` **no arranca en otra
máquina** y no sirve para la prueba.

> Puede además imprimir una línea `[NOTE] imports api-ms-win-core-synch-l1-2-0.dll`.
> Eso **no** es un fallo: es la nota de que este binario necesita Windows 8 o
> superior, que es lo que ya dice §0.

### 2.2 La aplicación del teléfono (`app-release.apk`)

**Primero, decirle a Rust con qué enlazar.** Éste es el paso que no está en
ningún sitio del repositorio y sin el cual el siguiente falla con
`linker 'cc' not found`: Rust sabe compilar para Android y **no sabe con qué
enlazador**, y eso se le dice con dos variables de entorno.

```
cd D:\Qyro\repo
$ndk = "$env:LOCALAPPDATA\Android\Sdk\ndk"
$ndk = (Get-ChildItem $ndk -Directory | Sort-Object Name -Descending | Select-Object -First 1).FullName
$bin = "$ndk\toolchains\llvm\prebuilt\windows-x86_64\bin"
Test-Path "$bin\aarch64-linux-android21-clang.cmd"
```

Ese `Test-Path` tiene que decir **True**. Si dice `False`, el NDK no está
instalado: ábrelo desde Android Studio → SDK Manager → SDK Tools → marca
**NDK (Side by side)**.

> **Los 16 KB ya no dependen de tu NDK.** Android 15 corre con páginas de 16 KB
> en los aparatos nuevos, y una biblioteca alineada a 4 KB **no carga**: la
> aplicación muere al abrirse. El NDK 28 y posteriores alinean así por omisión;
> los anteriores, no. Desde hoy el propio repositorio se lo pide al enlazador
> —está en `.cargo/config.toml`— así que **no tienes que hacer nada** y el paso
> §2.3 lo mide de todas formas (QYR-0394).
>
> **Lo que sí tienes que no hacer: no pongas `RUSTFLAGS` a mano** en esta
> ventana. Cargo usa la primera fuente de flags que encuentra y no las suma, así
> que un `RUSTFLAGS` puesto **borra** lo que el repositorio pide: los 16 KB aquí,
> y el enlazado estático del `.exe` de Windows más abajo. Está medido.

```
$env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = "$bin\aarch64-linux-android21-clang.cmd"
$env:CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER = "$bin\armv7a-linux-androideabi21-clang.cmd"
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo build --release --package qyro_ffi --target aarch64-linux-android
cargo build --release --package qyro_ffi --target armv7-linux-androideabi
mkdir apps\qyro\android\app\src\main\jniLibs\arm64-v8a
mkdir apps\qyro\android\app\src\main\jniLibs\armeabi-v7a
copy target\aarch64-linux-android\release\libqyro_ffi.so apps\qyro\android\app\src\main\jniLibs\arm64-v8a\
copy target\armv7-linux-androideabi\release\libqyro_ffi.so apps\qyro\android\app\src\main\jniLibs\armeabi-v7a\
cd apps\qyro
flutter build apk --release
```

> **`armv7a-…-androideabi21-clang`, con la `a` y con `eabi`.** El nombre del
> enlazador de 32 bits no se deriva del nombre del objetivo de Rust
> (`armv7-linux-androideabi`), y escribirlo como el de 64 es el error que da
> «no such file or directory» sin decir cuál.

Queda en `apps\qyro\build\app\outputs\flutter-apk\app-release.apk`.

**Son dos arquitecturas y no una a propósito:** casi todos los teléfonos de hoy
son `arm64-v8a`, pero los más baratos y los de hace unos años son de 32 bits
(`armeabi-v7a`). Un APK con sólo la primera **se instala igual** en el segundo y
se cierra al abrirlo.

> **Si falla con «Building with plugins requires symlink support»**, activa el
> Modo Desarrollador de Windows y repite:
> ```
> start ms-settings:developers
> ```
> Es una opción de Windows, no del proyecto.

### 2.3 Comprobar lo que de verdad quedó dentro del APK

```
cd D:\Qyro\repo\apps\qyro
python3 ..\..\tools\apk_inspector\inspect_apk.py build\app\outputs\flutter-apk\app-release.apk --require-abi arm64-v8a --require-abi armeabi-v7a
```

Tiene que terminar en `[OK]`. Comprueba tres cosas que sólo se ven abriendo el
paquete: que están las dos arquitecturas, que la biblioteca interna está
alineada como Android 15 exige, y que se puede cargar desde dentro del APK.

Y el permiso sin el cual **nada de la sección 5 funciona**:

```
cd D:\Qyro\repo\apps\qyro
flutter test test\android_manifest_test.dart
```

Todas en verde. Una de esas pruebas comprueba que el APK pide
`android.permission.INTERNET`; sin él, la aplicación se instala, abre, y **no
puede abrir una conexión** — y el error que sale no menciona ni a Qyro ni a un
permiso.

### 2.4 Dejarlos donde la guía los busca, con sus hashes

```
cd D:\Qyro\repo
mkdir release\prueba-en-hardware
copy target\x86_64-pc-windows-msvc\release\qyro.exe release\prueba-en-hardware\
copy apps\qyro\build\app\outputs\flutter-apk\app-release.apk release\prueba-en-hardware\
cd release\prueba-en-hardware
certutil -hashfile qyro.exe SHA256 > SHA256SUMS.txt
certutil -hashfile app-release.apk SHA256 >> SHA256SUMS.txt
git -C D:\Qyro\repo rev-parse HEAD > BUILD-INFO.txt
type SHA256SUMS.txt
type BUILD-INFO.txt
```

`BUILD-INFO.txt` guarda **de qué commit salieron**. Es lo único que permite
saber, dentro de un mes, si el archivo que hay en el USB es el que se cree.

---

## 3. Comprobar que los archivos son los que crees

Cada vez que un archivo cambie de sitio —del PC al USB, del USB a otro PC— se
comprueba así, en PowerShell:

```
certutil -hashfile qyro.exe SHA256
```

Sale una línea larga de letras y números. **Tiene que ser idéntica** a la que
guarda `SHA256SUMS.txt`. Si no lo es, el archivo se copió mal o no es el mismo:
vuelve a copiarlo.

En el teléfono, después de instalar:

```
adb shell pm path dev.qyro.app
```

Sale la ruta del paquete instalado. Que la aplicación abra y no se cierre sola ya
dice que el APK llegó entero.

---

## 4. Llevar los archivos a los dos aparatos

### 4.1 El `.exe` al otro PC

1. Formatea el USB en **FAT32** o **exFAT**. NTFS también vale entre dos Windows,
   pero FAT32 y exFAT los lee todo.
2. **Crea una carpeta propia en el USB**, por ejemplo `Qyro\`, y pon el `.exe`
   dentro. No lo dejes suelto en la raíz.
3. En el otro PC, **copia esa carpeta al disco duro** —por ejemplo al Escritorio—
   y ejecútalo desde ahí.

> **Por qué una carpeta propia, y por qué copiarlo al disco.** La primera vez que
> arranca, Qyro crea **junto al ejecutable** un archivo llamado
> `qyro-identity.bin`. Ésa es la identidad de ese aparato: la huella que la otra
> máquina compara. Si el `.exe` está suelto en la raíz del USB, la identidad
> también, y si el USB se usa en tres máquinas las tres comparten identidad y
> ninguna puede distinguirse de las otras. **Un ejecutable, una carpeta, una
> identidad.**
>
> No hay instalador y no lo va a haber: Qyro no escribe nada fuera de su carpeta.

### 4.2 El APK al teléfono

Con el teléfono conectado por USB y la depuración activada:

```
adb install -r D:\Qyro\repo\release\prueba-en-hardware\app-release.apk
```

Si `adb` no existe, viene con las herramientas de Android; también sirve copiar
el `.apk` al teléfono y abrirlo desde el explorador de archivos, aceptando
«instalar de orígenes desconocidos» cuando lo pida.

> **El APK que construyes va firmado con la clave de depuración**, y eso está
> bien para probar. Lo que hay que saber es la consecuencia: **Android no deja
> actualizar** una aplicación con otra firmada por una clave distinta. Así que el
> día que instales una versión firmada de verdad, tendrás que **desinstalar
> primero** — y desinstalar **borra la identidad del teléfono**, con lo que su
> huella cambia y hay que volver a leer los códigos.
>
> Mientras uses los APK que tú construyes, se actualizan entre ellos sin
> problema.
>
> Y si al instalar sale «la aplicación no se ha instalado» sin más explicación,
> casi siempre es esto: ya hay una Qyro instalada con otra firma. `adb uninstall
> dev.qyro.app` y repite.

**Qué permisos pide, y cuándo:**

| Permiso | Cuándo se pide | Qué pasa si dices que no |
|---|---|---|
| **Conexión de red** | **Nunca aparece un diálogo.** Se concede al instalar. | — |
| **Wi-Fi multicast** | **Nunca aparece un diálogo.** Se concede al instalar. | — |
| **Cámara** | La **primera vez** que pulsas «Leer códigos con la cámara», y sólo entonces. | El escaneo de QR no funciona. **Todo lo demás sí.** No lo necesitas para la prueba de la sección 5. |

**No pide ningún permiso de almacenamiento, y no debe pedirlo.** Los archivos se
eligen con el selector del sistema, que no necesita permiso. Si alguna vez ves
que pide acceso a fotos o archivos, eso **sí** es un fallo y hay que anotarlo.

---

## 5. Las tres pruebas

Haz la de PC→PC primero. Es la más simple y, si falla, dice si el problema está
en la red antes de meter el teléfono en la ecuación.

### 5.1 PC → PC

**Necesita dos PCs con el `.exe`.** Si sólo tienes uno, salta a §5.2.

**En el PC que RECIBE**, abre PowerShell en la carpeta del `.exe`:

```
.\qyro.exe recv --out .
```

En pantalla sale esto:

```
  THIS DEVICE

  fingerprint  6a0dfe4d-9eae93fa-a94da337-af01313c

  pairing code -- read this to the other device:

    [this machine]
    "QYRO1|192.168.1.42:49517|6a0dfe4d9eae93faa94da337af01313c"

  waiting for the other device. Ctrl-C to stop.
```

**La primera vez, Windows abrirá una ventana de cortafuegos** («¿Permitir que
Qyro se comunique en estas redes?»). Marca **Redes privadas** y pulsa
**Permitir el acceso**. Si tu red aparece como pública, marca también
**Redes públicas** o no entrará nada.

> Se pregunta **una sola vez**, por programa y por puerto. Por eso Qyro usa
> siempre el mismo puerto en lugar de uno distinto cada vez: para que este
> diálogo no vuelva.
>
> Si te has equivocado y has dicho que no, se arregla desde:
> ```
> wf.msc
> ```
> Reglas de entrada → busca `qyro` → bórrala → vuelve a lanzar `qyro recv`.

**Copia la línea del código entera, con las comillas incluidas.**

**En el PC que ENVÍA**, en la carpeta del `.exe`:

```
.\qyro.exe send C:\ruta\a\tu\archivo.zip --to "QYRO1|192.168.1.42:49517|6a0dfe4d9eae93faa94da337af01313c"
```

> **Las comillas no son decoración.** El carácter `|` que lleva el código
> significa «tubería» en PowerShell y en `cmd`: sin comillas, la consola parte la
> línea por la mitad, intenta ejecutar `192.168.1.42:49517` como si fuera un
> programa, y el error que sale **no menciona a Qyro**. Por eso Qyro lo imprime
> ya entrecomillado: se copia entero.

El emisor muestra:

```
  connecting to 192.168.1.42:49517 ...
  the other device says it is:
    6a0dfe4d-9eae93fa-a94da337-af01313c
```

**Compara esa huella con la que el receptor tiene en pantalla.** Tienen que ser
la misma, grupo a grupo. Es la única comprobación que hace un humano y es la que
impide que haya alguien en medio.

Y el receptor pregunta, **diciendo qué**:

```
  someone connected. They say they are:
    b76c0bb3-034672e9-4c9ab47b-632ddcc0

  they want to send 1 file(s), 104857600 bytes:
    archivo.zip (104857600 bytes)
  accept from this device? [y/N]
```

Escribe `y` y Enter. Sale una barra de progreso en los dos lados y al final:

```
  1 file(s) saved in .
```

**Comprueba que el archivo es el mismo**, en los dos PCs:

```
certutil -hashfile archivo.zip SHA256
```

Los dos hashes tienen que ser idénticos.

### 5.2 Teléfono → PC

**En el PC**, en la carpeta del `.exe`:

```
.\qyro.exe recv --out .
```

Apunta el código que enseña. **En el teléfono**, abre Qyro:

1. Pulsa **Recibir** o **Enviar** para entrar (las dos llevan a las mismas
   pestañas).
2. Pestaña **Aparatos**. En el campo **Código de emparejamiento**, escribe el
   código del PC. Puedes teclearlo **sin** las comillas: eso es sólo para la
   consola. Con comillas también lo acepta.
3. Pulsa **Usar este código**. Debajo aparece la dirección.
4. Pestaña **Enviar** → elige la foto con el selector del sistema → **Enviar**.

En el PC, el receptor dice quién es, **qué manda**, y pregunta. Escribe `y`.

Al terminar, el archivo está en la carpeta desde la que lanzaste `qyro recv`.
Compara su SHA-256 con el del teléfono si puedes; si no, que abra y se vea es
suficiente para esta prueba.

### 5.3 PC → Teléfono

Ahora al revés.

**En el teléfono:** Qyro → **Recibir**. La pantalla muestra **su propio código**
y se queda esperando. Apúntalo, o mejor, léelo en voz alta mientras lo tecleas.

**En el PC**, en la carpeta del `.exe`:

```
.\qyro.exe send C:\ruta\a\tu\archivo.zip --to "QYRO1|192.168.1.99:49517|<la huella del teléfono>"
```

Con las comillas, otra vez.

En el teléfono aparece una tarjeta con **la huella del PC**, **cuántos archivos**
y **cuántos bytes**, y dos botones: **Aceptar** y **Rechazar**. Compara la huella
con la que el PC muestra, y acepta.

El archivo queda en la carpeta propia de Qyro dentro del almacenamiento del
teléfono:

```
/sdcard/Android/data/dev.qyro.app/files/Qyro/
```

Esa carpeta **no necesita ningún permiso** para que Qyro escriba en ella, se ve
por USB desde el PC, y **desaparece si desinstalas la aplicación**. Para mirarla
desde el PC:

```
adb shell ls -l /sdcard/Android/data/dev.qyro.app/files/Qyro/
adb shell sha256sum /sdcard/Android/data/dev.qyro.app/files/Qyro/archivo.zip
```

> **Si al pulsar Recibir en el teléfono no pasa nada** —ni código, ni «esperando»—
> el teléfono no ha podido crear esa carpeta. Anótalo: es el escenario **D2** y
> es exactamente lo que hay que saber.

---

## 6. Qué se ve cuando falla, y qué significa cada caso

### «no se reconoce como un comando interno o externo»

O, en PowerShell, **«Expected expression after '|'»**.

**Le faltan las comillas al código.** Vuelve a escribir el comando con el código
entre comillas dobles:

```
.\qyro.exe send archivo.zip --to "QYRO1|192.168.1.42:49517|6a0dfe..."
```

### «'...' is not a pairing code and not an ip:port»

Lo mismo, visto desde dentro de Qyro: le llegó medio código. El propio mensaje lo
explica y da el ejemplo correcto.

### «port 49517 is not free on this machine»

```
qyro: port 49517 is not free on this machine.
  Another program holds it, or Windows has reserved it.
```

**El puerto está ocupado.** Casi siempre es Hyper-V, WSL o Docker: Windows les
reserva rangos enteros de puertos y no avisa. Para verlos:

```
netsh interface ipv4 show excludedportrange protocol=tcp
```

Qyro **no se cambia de puerto solo** —perdería el permiso del cortafuegos y el
código dejaría de ser predecible—, pero te ofrece elegir otro ahí mismo. Escribe
por ejemplo `49518` y Enter. **El código nuevo lleva el puerto nuevo dentro**, así
que el emisor no tiene que hacer nada distinto: copiar el código que salga.

También se puede pedir de entrada:

```
.\qyro.exe recv --out . --port 49518
```

### «could not connect: the peer could not be reached, or the wire ended»

**El emisor no encuentra a nadie en esa dirección.** Por orden de probabilidad:

1. **El receptor no está escuchando.** Comprueba que la otra máquina tiene `qyro
   recv` en marcha *ahora*, no que lo tuvo hace un rato.
2. **El cortafuegos dijo que no.** Mira §5.1: `wf.msc` → Reglas de entrada.
3. **No están en la misma red.** Comprueba que los dos ven la misma Wi-Fi. Un
   teléfono con datos móviles activos puede estar saliendo por ahí; apágalos.
4. **La dirección que copiaste no es la buena.** Si el receptor enseñó varias
   líneas de código, cada una es una tarjeta de red distinta. Prueba con otra.
5. **Aislamiento de cliente en el router.** Frecuente en Wi-Fi de hotel, de
   cafetería y de oficina: el router deja salir a internet y prohíbe que dos
   aparatos se hablen. **Con eso Qyro no puede hacer nada**, y ninguna aplicación
   puede. Usa otra red, o el punto de acceso del teléfono.

### «REFUSED. You expected ... and it is ...»

**La huella no coincide con la que dijiste esperar.** Es una negativa, no un
error: alguien al otro lado no es quien creías, o reinstalaste la aplicación —lo
que crea una identidad nueva— y la otra máquina todavía recuerda la vieja.

**No hay «continuar de todos modos» y no lo va a haber.**

### En el teléfono: «se ha detenido la aplicación» nada más abrirla

El APK no lleva la arquitectura de ese teléfono. Vuelve a §2.2 y comprueba que
construiste **las dos** (`arm64-v8a` y `armeabi-v7a`), y que §2.3 terminó en
`[OK]`.

### Se queda parado y no pasa nada

Espera **un minuto entero**. Qyro tiene un reloj de 60 segundos: si el otro lado
se calla, lo dice al vencer. Si a los dos minutos sigue igual, **Ctrl-C** en el
PC, y empieza otra vez por el receptor.

**Tómate el tiempo que quieras para aceptar.** Hasta hoy no podías: el reloj de
60 segundos corría también mientras leías la pregunta, así que tardar más de un
minuto en contestar mataba la transferencia y decía «el otro aparato no
responde» — culpando a la red justo cuando acababas de contestar. Ahora, cuando
un lado no tiene nada que mandar y sólo espera al otro, el plazo son **diez
minutos** (QYR-0393). Medido: 65 segundos pensando pasaban de `PeerUnreachable`
a los 60,11 s a entregado a los 65,76 s.

### «los archivos llegaron y no se guardó ninguno»

O, en el teléfono: «El destino no lo aceptó».

**Ya hay un archivo con ese nombre en la carpeta de destino.** Qyro **nunca**
sobrescribe: prefiere no entregar a pisar algo que ya estaba. Es lo que pasa al
mandar dos veces el mismo archivo, que es justo lo que se hace al repetir una
prueba.

Mueve o renombra el que ya está, o recibe en otra carpeta:

```
.\qyro.exe recv --out C:\Users\tu-usuario\Desktop\qyro-2
```

**No es un fallo.** Anótalo como el resultado que es.

### La aplicación pide permiso de fotos o de archivos

**Eso sí es un fallo.** No debería pedir ningún permiso de almacenamiento.
Anótalo.

---

## 7. Dónde se anota el resultado

Los resultados van en
[`docs/testing/hardware-protocol.md`](testing/hardware-protocol.md), que ya tiene
**veintiséis escenarios numerados**, cada uno con un hueco en blanco:

```
Resultado: `[ ]` ______________________
```

Se rellena poniendo el resultado dentro y a continuación:

```
Resultado: `[x]` OK, 2026-08-31, hash origen y destino iguales
```

**Qué prueba de esta guía es qué escenario:**

| Lo que acabas de hacer | Escenarios que rellena |
|---|---|
| Instalar y abrir la aplicación (§4.2) | **A1**, **A2** |
| `adb shell dumpsys package dev.qyro.app` para ver los permisos | **A3** — tienen que salir **tres**: `INTERNET`, `CHANGE_WIFI_MULTICAST_STATE` y `CAMERA`, y **nada de almacenamiento** |
| Abrir el `.exe` (§5.1) | **A4** |
| Cerrar y reabrir la aplicación, misma huella | **B1** |
| Reiniciar el teléfono, misma huella | **B2** |
| Leer las dos huellas en voz alta (§5.1) | **B4** |
| El código tecleado (§5.1, §5.2, §5.3) | **C1** |
| Si los aparatos se ven solos en la lista | **C2**, **C3** |
| Reinstalar la aplicación y ver que el PC la rechaza | **C4** — ver el aviso de abajo |
| Probar en una Wi-Fi con aislamiento de cliente | **C5** |
| Teléfono → PC (§5.2) | **D1** |
| PC → teléfono con 100 MB o más (§5.3) | **D2** |
| Los casos de la sección 6 | **E1**, **E2**, **E3** |

> **Sobre C4, y hay que saberlo antes de intentarlo.** Ese escenario espera que
> el PC muestre el aparato **en rojo** con «la clave de este aparato ha
> cambiado». **Eso no puede ocurrir todavía**: Qyro no lleva una libreta de
> aparatos conocidos —nada la escribe— así que para Qyro todos los aparatos son
> nuevos cada vez. Anota C4 como **no ejecutable**, no como fallido.
>
> Lo que **sí** puedes comprobar, y es la mitad que protege de verdad: teclea un
> código con **una letra cambiada en la huella** y manda. Tiene que salir
> `REFUSED`, diciendo qué esperabas y qué hay. Eso es **E3**.

**Los escenarios que no ejecutes se quedan en blanco.** No los marques «no
probado» ni los borres: un hueco vacío ya significa exactamente eso, y es la
única forma en que esto sigue siendo útil dentro de seis meses.

**Un escenario sin marcar no es un aprobado.** Y escribir un resultado que no
ocurrió arruina todos los demás, porque a partir de ahí ninguno se puede creer.

---

## 8. Si quieres ir más allá

### La aplicación con ventanas en Windows

El PC también puede usar la **misma aplicación** que el teléfono, en vez del
binario de terminal. Se construye así:

```
cd D:\Qyro\repo
cargo build --release -p qyro_ffi --target x86_64-pc-windows-msvc
cd apps\qyro
flutter build windows --release
copy ..\..\target\x86_64-pc-windows-msvc\release\qyro_ffi.dll build\windows\x64\runner\Release\
```

Y se abre `apps\qyro\build\windows\x64\runner\Release\qyro.exe`.

**Si lo mueves, mueve la carpeta entera.** Esa carpeta lleva DLLs y datos que la
aplicación necesita; el `.exe` solo no arranca. Es la diferencia con el binario
de terminal, que sí es un archivo único.

Con esto, PC→PC se puede hacer también entre dos ventanas en lugar de dos
terminales. Los pasos son los mismos: uno pulsa **Recibir** y enseña su código,
el otro lo teclea en **Aparatos** y manda desde **Enviar**.

### Los otros comandos del binario de terminal

Cosas que el `.exe` sabe hacer y que esta guía no necesita:

```
.\qyro.exe                      abre un menú, sin argumentos
.\qyro.exe help                 la lista entera
.\qyro.exe whoami               el código de este aparato, sin ponerse a recibir
.\qyro.exe find                 busca otros aparatos en la red, 3 segundos
.\qyro.exe qr                   dibuja el código como QR, para leerlo con el teléfono
.\qyro.exe how <archivo>        qué camino conviene para ese archivo
.\qyro.exe beam <archivo>       manda el archivo entero por QR, sin red
.\qyro.exe serial               los puertos COM que hay
.\qyro.exe send --self --to "<código>"    manda el propio qyro.exe a otra máquina
```

`qyro send --self` es la respuesta al huevo y la gallina: una vez hay un Qyro
funcionando en una máquina, puede llevarse a sí mismo a la siguiente sin USB.
