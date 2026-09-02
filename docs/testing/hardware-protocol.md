# El protocolo de hardware físico — fase 07

**Esto es lo único que esta sesión no puede hacer.** Necesita dos aparatos y una
persona, y **no se inventa evidencia de hardware**: cada escenario de abajo tiene
un hueco de resultado en blanco, y un hueco en blanco es la verdad hasta que
alguien lo llene.

Lo demás está terminado. Esto está listo para ejecutarse.

---

## 0. Qué hace falta, y por qué cada cosa

| Cosa | Por qué |
|---|---|
| Un teléfono Android, API 30+ | Nada se ha ejecutado nunca en hardware. Un emulador no tiene radio Wi-Fi |
| Un PC con Windows 10/11 | El otro extremo |
| **Los dos en la misma Wi-Fi** | Sin eso no hay nada que probar |
| Un cable USB | Para `adb`, y para instalar sin tienda |
| Un archivo de ≥ 100 MB | Los archivos pequeños caben en una ventana y no ejercitan nada |

**Activa antes:** Opciones de desarrollador → Depuración por USB, en el teléfono.

---

## 1. Preparar, una vez

> **Los pasos de abajo son el resumen.** Si es la primera vez, la página
> completa —con lo que se ve en pantalla en cada paso y qué significa cada
> error— es [`docs/GUIA-DE-PRUEBA.md`](../GUIA-DE-PRUEBA.md).

```bash
# En el PC, desde la raíz del repositorio.
cd D:\Qyro\repo

# 1. La biblioteca nativa para el telefono. DOS arquitecturas de ARM, no una:
#    arm64-v8a es casi todo telefono de hoy y armeabi-v7a es el de 32 bits, que
#    instala el APK igual y muere con UnsatisfiedLinkError si no esta su .so.
# Rust sabe compilar para Android y NO sabe con que enlazar. Sin estas dos
# variables el cargo build de abajo falla con "linker `cc` not found".
# La guia de prueba tiene la version de PowerShell que resuelve la ruta sola.
#
# La alineacion de 16 KB que Android 15 exige NO hace falta ponerla aqui: sale
# de .cargo/config.toml (QYR-0394). Lo que no hay que hacer es poner RUSTFLAGS
# a mano, porque eso la borra --y borra tambien el enlazado estatico del .exe.
#   $bin = "$env:LOCALAPPDATA\Android\Sdk\ndk\<version>\toolchains\llvm\prebuilt\windows-x86_64\bin"
#   $env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER  = "$bin\aarch64-linux-android21-clang.cmd"
#   $env:CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER = "$bin\armv7a-linux-androideabi21-clang.cmd"
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo build --release --package qyro_ffi --target aarch64-linux-android
cargo build --release --package qyro_ffi --target armv7-linux-androideabi
mkdir apps\qyro\android\app\src\main\jniLibs\arm64-v8a
mkdir apps\qyro\android\app\src\main\jniLibs\armeabi-v7a
copy target\aarch64-linux-android\release\libqyro_ffi.so apps\qyro\android\app\src\main\jniLibs\arm64-v8a\
copy target\armv7-linux-androideabi\release\libqyro_ffi.so apps\qyro\android\app\src\main\jniLibs\armeabi-v7a\

# 2. El APK firmado.
cd apps\qyro
flutter build apk --release

# 2b. Y lo que de verdad quedo dentro: las ABIs, y la alineacion de 16 KB que
#     Android 15 exige. Se mide sobre el APK y no sobre lo que salio de cargo.
python3 ..\..\tools\apk_inspector\inspect_apk.py ^
  build\app\outputs\flutter-apk\app-release.apk ^
  --require-abi arm64-v8a --require-abi armeabi-v7a

# 3. El .exe de Windows. La DLL se CONSTRUYE primero: ningun paso de este
#    protocolo la construia, y copiar target\release\qyro_ffi.dll copiaba lo
#    que hubiera dejado ahi otra compilacion, o nada.
cd ..\..
cargo build --release -p qyro_ffi --target x86_64-pc-windows-msvc
cd apps\qyro
flutter build windows --release
copy ..\..\target\x86_64-pc-windows-msvc\release\qyro_ffi.dll build\windows\x64\runner\Release\
```

**Si el paso 2 o el 3 falla con «Building with plugins requires symlink
support»**, activa el Modo Desarrollador de Windows —`start ms-settings:developers`—
y repite. Es QYR-0324 y es configuración del sistema, no del proyecto.

```bash
# 4. Instalar en el teléfono.
adb install -r build\app\outputs\flutter-apk\app-release.apk

# 5. Comprobar que el APK es el que crees.
certutil -hashfile build\app\outputs\flutter-apk\app-release.apk SHA256
```

Compara ese hash con el que quede anotado en `docs/release/v1.0.md`.

---

## 2. Los escenarios

Cada uno: **comando literal**, qué tiene que pasar, y un hueco.
`[ ]` sin marcar es «no ejecutado». **No marques nada que no hayas visto.**

### A — Arranque y presencia

**A1. La aplicación arranca en el teléfono.**
```bash
adb shell am start -n dev.qyro.app/com.owner.qyro.MainActivity
adb logcat -d -s flutter:* AndroidRuntime:E | tail -40
```
Esperado: la pantalla de arranque, después la de inicio con **Enviar** y
**Recibir** pulsables. Ni un `AndroidRuntime: E`.
Resultado: `[ ]` ______________________

**A2. La biblioteca nativa está en el APK y se carga.**
```bash
adb logcat -c && adb shell am start -n dev.qyro.app/com.owner.qyro.MainActivity
adb logcat -d | findstr /C:"qyro_ffi" /C:"UnsatisfiedLink"
```
Esperado: **ningún** `UnsatisfiedLinkError`.
Resultado: `[ ]` ______________________

**A3. El manifiesto instalado pide exactamente tres permissions.**
```bash
adb shell dumpsys package dev.qyro.app | findstr /C:"permission"
```
Esperado: `INTERNET`, `CHANGE_WIFI_MULTICAST_STATE` y `CAMERA`. Y **nada de
almacenamiento**, y **nada de `ACCESS_LOCAL_NETWORK`**.

**`INTERNET` es el que faltaba, y su ausencia era un P0 (QYR-0368):** estaba
declarado sólo en los sourceSets de debug y de profile, que **no llegan a una
build de release**, así que el APK que se instala no podía abrir un socket. Si
aquí no aparece, **nada de la sección D va a funcionar**, y el error que sale en
el teléfono no menciona ni a Qyro ni a un permiso.
Resultado: `[ ]` ______________________

**A4. El `.exe` arranca en Windows.**
```bash
build\windows\x64\runner\Release\qyro.exe
```
Esperado: la ventana, y los dos botones pulsables.
Resultado: `[ ]` ______________________

### B — Identidad y confianza

**B1. La identidad sobrevive a cerrar la aplicación en el teléfono.**
```bash
adb shell am force-stop dev.qyro.app
adb shell am start -n dev.qyro.app/com.owner.qyro.MainActivity
```
Esperado: la huella que muestra la pantalla de peers es **la misma** que antes.
Anota las dos.
Resultado: `[ ]` antes ____________ después ____________

**B2. La identidad sobrevive a un reinicio del teléfono.**
```bash
adb reboot && adb wait-for-device && adb shell am start -n dev.qyro.app/com.owner.qyro.MainActivity
```
Esperado: la misma huella. **Esto es lo que ningún emulador prueba.**
Resultado: `[ ]` ______________________

**B3. Keystore, desde dentro de una aplicación real.**
```bash
cd apps\qyro\android
gradlew connectedDebugAndroidTest
```
Esperado: los seis tests de `KeystoreIdentityTest` en verde, **en el teléfono**.
Resultado: `[ ]` ______________________

**B4. Las dos huellas coinciden leídas en voz alta.**
Esperado: el formato agrupado es idéntico en los dos aparatos, grupo a grupo.
Resultado: `[ ]` ______________________

### C — Emparejamiento

**C1. El código manual, tecleado.**
Esperado: la dirección aparece; un código mal tecleado da «Eso no es un código de
emparejamiento de Qyro» y **no** conecta.
Resultado: `[ ]` ______________________

**C2. Descubrimiento automático: Windows anuncia, Android encuentra.**
Esperado: el aparato aparece con su huella. **Si no aparece, no es un fallo del
código**: comprueba el aislamiento de cliente del router (C5).
Resultado: `[ ]` ______________________

**C3. Descubrimiento: Android anuncia, Windows encuentra.**
```bash
dns-sd -B _qyro._tcp
```
Esperado: una entrada. `dns-sd` viene con Bonjour; si no está, sáltalo y usa C2.
Resultado: `[ ]` ______________________

**C4. Un peer conocido con la clave cambiada se rechaza.**
Reinstala la aplicación en el teléfono **sin** desinstalarla en Windows:
```bash
adb uninstall dev.qyro.app && adb install -r build\app\outputs\flutter-apk\app-release.apk
```
Esperado: en Windows el peer aparece en **rojo**, con «la clave de este aparato ha
cambiado», y **sin botón de enviar**. No hay «continuar de todos modos».
Resultado: `[ ]` ______________________

**C5. Aislamiento de cliente: el camino manual sigue funcionando.**
Activa el aislamiento en el router, o usa una Wi-Fi pública.
Esperado: el descubrimiento **no** encuentra nada y el código manual **sí**
funciona. Ésta es la razón de que el camino manual se construyera primero.
Resultado: `[ ]` ______________________

### D — La transferencia

**D1. Una foto, del teléfono al PC.**
Esperado: el selector de Android se abre, la foto llega, y el SHA-256 coincide.
```bash
certutil -hashfile "%USERPROFILE%\Downloads\Qyro\<nombre>" SHA256
```
Resultado: `[ ]` hash origen ____________ destino ____________

**D2. Un archivo de 100 MB o más, del PC al teléfono.**
```bash
adb shell sha256sum /sdcard/Android/data/dev.qyro.app/files/Qyro/<nombre>
```
Esperado: los dos hashes coinciden. Y **no se duplicó en disco** al elegirlo.
Resultado: `[ ]` ______________________

**D3. Varios archivos a la vez.**
Esperado: llegan todos, con sus nombres, sin colisiones.
Resultado: `[ ]` ______________________

**D4. El receptor rechaza.**
Esperado: el emisor dice **por qué**, y el destino queda sin archivos nuevos:
```bash
adb shell ls -la /sdcard/Android/data/dev.qyro.app/files/Qyro/
```
No debe quedar ningún `.qyro-part`.
Resultado: `[ ]` ______________________

**D5. Un desconocido no entra solo.**
Esperado: la oferta **espera**. Deja el teléfono en la pantalla un minuto sin
tocarlo: no debe aceptarse por sí sola.
Resultado: `[ ]` ______________________

### E — Lo que sale mal

**E1. El Wi-Fi se cae a mitad de una transferencia.**
```bash
adb shell svc wifi disable
```
Esperado: un error con texto, no un cuelgue. Nada se entrega como bueno.
Resultado: `[ ]` ______________________

**E2. La aplicación se cierra a mitad.**
```bash
adb shell am force-stop dev.qyro.app
```
Esperado: en el receptor no queda un archivo completo a medias; a lo sumo un
`.qyro-part`, y **nunca** un archivo con el nombre final.
Resultado: `[ ]` ______________________

**E3. Sin espacio en el receptor.**
Llena el disco del destino y manda algo grande.
Esperado: un error con texto, y el archivo parcial recogido.
Resultado: `[ ]` ______________________

---

### F — Los otros tres canales

**Los veintiún escenarios de arriba prueban un canal: la red.** `R7` promete
cuatro, y los otros tres no tienen ni una casilla. Éstas son: **nueve**, contando
las variantes, que son escenarios y no notas al pie.

> **Esta sección decía «los cinco de F» y son nueve** (QYR-0396). De ahí salía el
> «veintiséis» que repetían seis documentos más: veintiuno y cinco. Y el error se
> sostenía porque cuatro de estos nueve anotaban su resultado bajo otra etiqueta
> —`Respuesta:`, `¿Cambió algo?`, `¿Lo dice?`, `¿Arrancó?`— así que **nada podía
> contarlos**. Ahora los treinta tienen su línea `Resultado:`, y una guarda del
> gate cuenta los encabezados, cuenta los huecos, y exige que los seis documentos
> digan el mismo número.

**Nada de esto está ejecutado.** Todos los huecos están en blanco a propósito:
**un escenario sin marcar no es un aprobado**, y escribir un resultado que nadie
vio es la única cosa que arruinaría este proyecto.

#### F1 — El cable directo, sin router (fase 14)

Un cable de red entre las dos máquinas. **Nada más**: sin switch, sin router, sin
Wi-Fi encendido.

```
qyro find
```

- **Esperado, lo primero:** una cuenta atrás que habla — *«waiting for a network
  address ... 12s -- this is normal on a direct cable»*. `R8` §8 midió que APIPA
  tarda decenas de segundos porque el cliente DHCP tiene que fracasar antes.
- **Esperado a los 60 s sin dirección:** un consejo, **no un error** — probar un
  cable cruzado, y que el código tecleado funciona igual.
- **Lo que hay que anotar aunque salga bien:** ¿cuántos segundos tardó de verdad?
  Es la primera vez que ese número se mide fuera de `R8`.

Resultado: `[ ]` ______________________
Segundos hasta la dirección: `[ ]` ______  · ¿Se vieron el uno al otro? `[ ]` ___

**F1b. Con una NIC de sólo 10/100.** Auto-MDI-X está en la cláusula 40.4.4 de
IEEE 802.3, que es la de **1000BASE-T**: una tarjeta de 10/100 puede no tenerlo y
hacer falta un cable cruzado. **Es justo la máquina para la que se hizo esto.**

Resultado: `[ ]` ______________________  · ¿Hizo falta cruzado? `[ ]` ______

#### F2 — El canal óptico: la pantalla y el teléfono (fases 15 y 24B)

En la máquina que envía:

```
qyro beam clave.pem
```

En el teléfono: **Aparatos → Leer códigos con la cámara**.

- **Antes de nada, la medida que falta y que ADR-0048 §4 dejó en blanco:**
  **¿cuántos frames por segundo sostiene el aparato?** La pantalla del escáner
  enseña «N mirados · M leídos». 921 600 bytes por frame a 720p cruzan un
  MethodChannel y una copia por FFI. **Si sostiene 5 o más, el puente está hecho
  para siempre. Si no, el cruce de copia cero por JNI tiene su argumento medido.**

  fps sostenidos: `[ ]` ______   · mirados/leídos al terminar: `[ ]` ____ / ____

- **Esperado:** el archivo llega **byte a byte**. Compruébalo con
  `Get-FileHash`, no de vista.
- **Lo que hay que probar aunque funcione:** tapa la cámara medio segundo a
  mitad. **El fountain existe para que perder frames cueste frames y no la
  transferencia.**

Resultado: `[ ]` ______________________
¿Llegó con frames perdidos? `[ ]` ______

**F2b. Sin visor.** La pantalla **no tiene vista previa** — `camera-view` es una
vista de Android y esta aplicación dibuja con Flutter. Quien sostiene el teléfono
se guía por las cifras. **¿Es suficiente, o hace falta un visor?** Es una
pregunta de producto y la respuesta sale de sostener el teléfono, no de discutir.

Resultado: `[ ]` ______________________
¿Hace falta un visor? `[ ]` ______________________

**F2c. El brillo.** `R10` §8 T4: la pantalla es una fuente de luz y el
autoexposímetro la sobreexpone. Prueba al 100 % y al 60 %.

Resultado: `[ ]` ______________________
¿Cambió algo entre el 100 % y el 60 %? `[ ]` ______________________

#### F3 — El canal serie (fase 16)

Dos máquinas y un cable serie, o un adaptador USB-serie en cada una.

```
qyro serial
```

- **Esperado, lo primero:** que **pregunte si esa máquina tiene CD, disquetera,
  PCMCIA o red**, porque cualquiera es entre 10 y 10 000 veces más rápida. Un
  producto que ofrece el canal lento sin preguntar le cuesta horas a alguien.
- Después imprime el receptor de PowerShell para pegar en la máquina vieja.

```
qyro send informe.pdf --serial COM1
```

Resultado: `[ ]` ______________________
Velocidad real observada: `[ ]` ______ · ¿Llegó el hash igual? `[ ]` ______

**F3b. El modo degradado no autentica nada**, y eso está en el modelo de
amenazas. **Comprueba que la pantalla lo dice** antes de que alguien lo use para
algo que le importe.

Resultado: `[ ]` ______________________
¿Lo dice la pantalla? `[ ]` ______________________

#### F4 — La máquina que no puede instalar nada (`R7` §2)

**El escenario que da sentido a todo el producto**, y el único que no se puede
simular: una máquina real, sin permisos de administrador, quizá sin Windows 10.

1. Copia `qyro.exe` a un USB con **FAT32 o exFAT**.
2. Ejecútalo desde el USB en esa máquina.

- **Esperado:** que arranque **sin advertencia de SmartScreen** — copiar a FAT32
  borra el Mark of the Web, porque ese flujo vive en NTFS.
- **Y si la máquina es Windows 7:** que arranque, punto. Ése es el bloqueo que
  `verify_static.ps1` lleva señalando desde la fase 13 y que ADR-0049 dice que
  **no está confirmado en `msvc`**.

Resultado: `[ ]` ______________________
¿Arrancó? `[ ]` ______ · ¿Salió SmartScreen? `[ ]` ______
Versión de Windows: `[ ]` ______________________

**F4b. `qyro send --self`.** Desde una máquina que ya tenga Qyro, mándalo a la
siguiente. Son unos 800 KB.

Resultado: `[ ]` ______________________ · ¿Cuánto tardó? `[ ]` ______

---

## 3. Cómo se registra

Cuando termines, pega la tabla rellena en un archivo nuevo:
`docs/reports/fase-07-hardware-fisico.md`, con:

- **el modelo del teléfono y su versión de Android**, y la versión de Windows;
- **el hash del APK y del `.exe`** que instalaste;
- **todos los huecos** —los veintiuno de A–E y los **nueve** de F, **treinta** en
  total, los cuatro canales—,
  incluidos los que fallaron y **los que no ejecutaste**;
- y para cada fallo: qué esperabas, qué pasó, y el `adb logcat` si lo hay.

**Un escenario sin marcar no es un aprobado.** La única cosa que arruinaría este
proyecto es escribir un resultado que nadie vio.
