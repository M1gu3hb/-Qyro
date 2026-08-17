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

```bash
# En el PC, desde la raíz del repositorio.
cd D:\Qyro\repo

# 1. La biblioteca nativa para el teléfono.
rustup target add aarch64-linux-android
cargo build --release --package qyro_ffi --target aarch64-linux-android
mkdir apps\qyro\android\app\src\main\jniLibs\arm64-v8a
copy target\aarch64-linux-android\release\libqyro_ffi.so apps\qyro\android\app\src\main\jniLibs\arm64-v8a\

# 2. El APK firmado.
cd apps\qyro
flutter build apk --release

# 3. El .exe de Windows.
flutter build windows --release
copy ..\..\target\release\qyro_ffi.dll build\windows\x64\runner\Release\
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

## 2. Los veinte escenarios

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

**A3. El manifiesto instalado pide exactamente una permission.**
```bash
adb shell dumpsys package dev.qyro.app | findstr /C:"permission"
```
Esperado: `CHANGE_WIFI_MULTICAST_STATE` y **nada de almacenamiento**, y **nada de
`ACCESS_LOCAL_NETWORK`**.
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

## 3. Cómo se registra

Cuando termines, pega la tabla rellena en un archivo nuevo:
`docs/reports/fase-07-hardware-fisico.md`, con:

- **el modelo del teléfono y su versión de Android**, y la versión de Windows;
- **el hash del APK y del `.exe`** que instalaste;
- **los veinte huecos**, incluidos los que fallaron y los que no ejecutaste;
- y para cada fallo: qué esperabas, qué pasó, y el `adb logcat` si lo hay.

**Un escenario sin marcar no es un aprobado.** La única cosa que arruinaría este
proyecto es escribir un resultado que nadie vio.
