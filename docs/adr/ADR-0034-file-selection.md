# ADR-0034 — El selector de archivos

- **Estado:** aceptada
- **Fecha:** 2026-08-14
- **Fase:** 03, paso 1
- **iOS:** fuera de la v1.0 por ADR-0039. Lo que aquí se decide para Android y
  Windows vale; la mitad iOS queda aplazada.

---

## 1. Qué cruza el FFI

| Plataforma | Selector | Qué cruza | Crates Rust nuevos |
|---|---|---|---|
| **Android** | Storage Access Framework, `MethodChannel` propio | **`int32` — un fd** por archivo, más su nombre | **0** |
| **Windows** | `file_selector` (`flutter.dev`, BSD-3) | **`String`** — ruta UTF-8 | **0** |

### Por qué un `MethodChannel` propio en Android y no `file_selector_android`

**Porque copia.** `FileSelectorApiImpl.java:365` llama en el camino principal a
`FileUtils.getPathFromCopyOfFileFromUri`, que abre un `InputStream` sobre el
`content://`, crea `{cacheDir}/{uuid}/{fileName}` y ejecuta
`copy(inputStream, outputStream)` antes de devolver una ruta. **Un archivo de
4 GB se duplica en disco antes de que la transferencia empiece.** Leído del Java
del paquete fijado, no de su documentación (QYR-0323).

Y el otro camino del plugin no salva: `getPathFromUri` lanza
`UnsupportedOperationException` para todo volumen que no sea `primary`, es decir
para cualquier SD o USB.

En Windows el plugin **no tiene ese problema** —un selector de escritorio
devuelve una ruta real— y escribir `IFileOpenDialog` a mano es una vtable de ~29
huecos cuyo orden Microsoft no publica en la web, donde un hueco desplazado no da
error de compilación ni de enlace: da UB silencioso. Así que ahí se usa el
plugin.

### Los dos detalles de Android que no se negocian

- **`openFileDescriptor(uri, "rw")`, nunca `"r"`.** El javadoc de AOSP dice que
  con los modos exclusivos `"r"` o `"w"` el `ParcelFileDescriptor` devuelto
  **puede ser un pipe o un par de sockets**; `"rw"` implica un archivo en disco
  que admite búsqueda. Con `"r"`, `seek()` falla con `ESPIPE` y rompe la
  reanudación que ya existe.
- **`detachFd()`, nunca `getFd()`.** `getFd()` deja la propiedad en el
  `ParcelFileDescriptor`, que también cierra al recolectarse: doble cierre. En un
  proceso multihilo eso puede cerrar un descriptor ajeno reasignado entretanto
  —el socket de la transferencia, por ejemplo—. Corrupción silenciosa, no un
  fallo ruidoso.

---

## 2. Quién cierra el descriptor

**Rust, y sólo Rust.**

`detachFd()` ya renunció a la propiedad en el lado Kotlin, así que nadie allí
puede cerrarlo. En Rust, `File::from_raw_fd` toma la propiedad y el `Drop` del
`File` lo cierra. **La sesión posee el descriptor desde que cruza hasta que la
sesión muere**, y una sesión muere en `qyro_session_close`.

**Si la apertura falla después de que el fd cruzó**, Rust lo cierra igual: el
`File` ya existe y se suelta. No hay ruta en la que un fd cruce y no se cierre,
salvo que el proceso muera — y entonces el sistema los cierra todos.

**Dart no vuelve a tocarlo.** Ni para leerlo, ni para cerrarlo, ni para saber su
tamaño.

---

## 3. Si el usuario revoca el acceso a mitad de una transferencia

**No pasa nada, y esto no es optimismo.** SAF concede un permiso sobre un
`content://`; revocarlo impide **abrir** de nuevo. Un descriptor ya abierto sigue
siendo un descriptor abierto: el kernel no lo cierra porque una capa de permisos
de Android cambie de opinión.

Si aun así una lectura fallara, la ruta ya existe y ya está probada:
`FileSource::read_at` no tiene canal de error, devuelve un conteo, así que un
fallo se lee como lectura corta → el digest del manifiesto no cuadra → el
receptor emite `ItemVerdict` distinto de `Ok` → la sesión termina en
`Rejected`. **Un archivo que se volvió ilegible a mitad no se entrega como
bueno.**

---

## 4. Dónde caen los archivos recibidos

| Plataforma | Destino | Permiso |
|---|---|---|
| **Android** | `getExternalFilesDir(null)/Qyro` — el directorio específico de la app | **ninguno** |
| **Windows** | `%USERPROFILE%\Downloads\Qyro` | ninguno |

**Cero permisos de almacenamiento, en ninguna plataforma.** La cita oficial de
SAF: «Because the user is involved in selecting the files or directories that
your app can access, this mechanism doesn't require any system permissions.» Y el
directorio específico de la app nunca los ha necesitado.

**Y se comprueba sobre el manifiesto *fusionado*, no sobre el que escribimos.**
Un plugin de Flutter puede añadir un permiso sin que aparezca en nuestro
`AndroidManifest.xml`, y una prueba sobre el archivo fuente no lo vería. Si
alguna vez hace falta declarar un permiso de almacenamiento, el diseño está mal.

---

## 5. Lo que esta decisión NO promete

- **No promete elegir carpetas.** `ACTION_OPEN_DOCUMENT_TREE` y la recursión
  quedan fuera: la v1.0 manda archivos.
- **No promete reanudar entre ejecuciones de la app en Android.** Un fd no
  sobrevive al proceso, y el `content://` que lo produjo puede haber perdido su
  permiso. La reanudación dentro de una sesión sí, que es la que existe.
- **No promete nada visto en un aparato.** Al congelar esto no hay emulador
  arrancado ni teléfono conectado.

---

## Enmienda 1 (2026-08-14) — el paquete de Windows es `file_selector_windows`, no `file_selector`

**Qué cambia:** el §1 de esta ADR nombraba `file_selector`. La dependencia que
entra es **`file_selector_windows` 0.9.3+5**, la implementación endosada de
Windows del **mismo** plugin federado: mismo publisher verificado
(`flutter.dev`), misma licencia (BSD-3), mismo código de diálogo. Lo que cruza
el FFI **no cambia**: una ruta UTF-8.

**Por qué, medido.** Se resolvieron las dos opciones y se contaron los paquetes
del `pubspec.lock`:

```
# antes:  37 paquetes
# file_selector 1.0.3           -> 52 paquetes  (+15)
# file_selector_windows 0.9.3+5 -> 45 paquetes  (+8)
flutter pub get
grep -cE '^  [a-z_0-9]+:$' apps/qyro/pubspec.lock
```

Los **siete** que añade el paquete paraguas y no la implementación de Windows:
`file_selector`, `file_selector_android`, `file_selector_ios`,
`file_selector_linux`, `file_selector_macos`, `file_selector_web` y
`flutter_web_plugins`.

**El que decide es el segundo.** `file_selector_android` **es** la
implementación que copia (QYR-0323, §1 de esta ADR). Depender del paraguas mete
ese Java en el APK de una aplicación cuyo diseño dice que nunca debe llamarlo.
*Una dependencia que hay que acordarse de no llamar es una trampa para quien
venga después*, y este proyecto ya tiene un `MethodChannel` propio precisamente
para no llamarla.

No es una decisión de permisos: el manifiesto de `file_selector_android`
0.5.2+9 está vacío —`<manifest package="dev.flutter.packages.file_selector_android"></manifest>`,
leído del paquete, no de su documentación—. Es una decisión de **no embarcar el
código que copia**.

**Lo que cuesta.** No hay selector en Linux ni en macOS. Ninguna de las dos es
plataforma de la v1.0. `pickerForPlatform()` las **rechaza por nombre**, igual
que a iOS, en vez de devolver una lista vacía: una lista vacía se lee como «la
persona canceló», y eso es mentir en silencio.

**Lo que esta enmienda NO arregla.** `http` entra igual, transitivamente por
`file_selector_platform_interface` 2.7.0, en las **dos** opciones. Qyro no hace
ninguna petición HTTP; el paquete viaja sin que nadie lo llame. Queda registrado
en QYR-0326, no resuelto aquí.

**Consecuencia local, medida en esta máquina.** Con cualquiera de las dos
opciones, `flutter pub get` termina en **código 1** en un Windows sin Modo
Desarrollador: el paso que crea los symlinks de plugins falla (QYR-0324).
`.dart_tool/package_config.json` sí queda escrito, así que **`flutter test`
sigue funcionando**; `flutter build windows` no. Clase de evidencia de todo lo
que dependa de ver el diálogo: **ninguna en esta máquina**.
