# FASE 03 — El selector de archivos

## 1. Objetivo

**Que el usuario elija qué mandar y dónde recibir, con el selector de su propio
sistema, sin que Qyro pida ni un permiso de almacenamiento.**

## 2. Por qué esta fase va aquí

**Depende de:** fase 02 (una superficie Dart que funcione).

Va antes que el descubrimiento porque **el selector es lo que convierte «Qyro
mueve un archivo que le pasas» en «Qyro mueve el archivo que el usuario quiere»**,
y porque en Android obliga a un cambio en la superficie del FFI —un **descriptor
de archivo** en vez de una ruta— que es mejor hacer antes de que haya UI encima.

## 3. Estado de partida

Reproduce lo de la fase 02, y lee además:

- `apps/qyro/pubspec.yaml` — qué paquetes hay hoy.
- `apps/qyro/android/app/src/main/AndroidManifest.xml` — qué permisos se declaran.
- La superficie C de la fase 01: **hoy recibe rutas.**

## 4. La decisión, ya investigada

**El selector se hace en Dart. A Rust cruza un `fd` en Android e iOS, y una ruta
en Windows. Cero crates nuevos de Rust.**

| Plataforma | Selector | Qué cruza la FFI | Crates Rust |
|---|---|---|---|
| Android | Storage Access Framework | **`int`** — fd de `detachFd()` | **0** |
| iOS | `UIDocumentPickerViewController` | `int` (fd) o `String` | **0** |
| Windows | `IFileOpenDialog`, desde Dart | `String` (ruta UTF-8) | **0** |

### 4.1 — Android: un `content://` no es una ruta, y Rust no puede abrirlo nunca

La lista oficial de APIs estables del NDK **no incluye ninguna API de
`ContentResolver` ni de SAF**
(`developer.android.com/ndk/guides/stable_apis`, act. 2026-03-06). El mecanismo
real es:

```
ContentResolver.openFileDescriptor(uri, "rw")   // Kotlin
   → ParcelFileDescriptor.detachFd() : int      // cede la propiedad
   → JNI: jint a extern "C" fn(..., fd: c_int)
   → Rust: unsafe { File::from_raw_fd(fd) }     // toma la propiedad, cierra en Drop
```

El fd es un entero en la tabla del **mismo proceso**; JNI no cruza fronteras de
proceso. El Binder ya transfirió el fd desde el proveedor antes de que
`openFileDescriptor` retorne.

**Dos cosas que son trampas de verdad, las dos con fuente primaria:**

**(a) El modo importa, y afecta a la reanudación que ya está construida.** Javadoc
de AOSP, verbatim:

> «If opening with the exclusive `"r"` or `"w"` modes, the returned
> ParcelFileDescriptor **could be a pipe or socket pair** to enable streaming of
> data. Opening with the **`"rw"` mode implies a file on disk that supports
> seeking**.»

Si abres en `"r"` y luego haces `File::seek()` para reanudar, **falla con
`ESPIPE`** cuando el proveedor devolvió un pipe —Drive, proveedores de red—.
**Abre siempre en `"rw"` si quieres seek.**

**(b) `detachFd()` y no `getFd()`.** Javadoc, verbatim:

> `getFd()`: «Return the native fd int for this ParcelFileDescriptor. **The
> ParcelFileDescriptor still owns the fd.**»
> `detachFd()`: «Return the native fd int … and detach it from the object here.
> **You are now responsible for closing the fd in native code.**»

`getFd()` + `File::from_raw_fd()` es un **doble cierre**: el
`ParcelFileDescriptor` también cierra al recolectarse, y en un proceso multihilo
eso puede cerrar un fd **ajeno** reasignado entretanto —el socket de la
transferencia, por ejemplo—. **Corrupción silenciosa, no un crash.** Si por algún
motivo necesitas `getFd()`, duplica primero.

**(c) SAF no necesita ningún permiso.** Cita oficial:

> «Because the user is involved in selecting the files or directories that your
> app can access, this mechanism **doesn't require any system permissions**.»

Ni `READ_EXTERNAL_STORAGE` ni ningún `READ_MEDIA_*`. **Si acabas declarando un
permiso de almacenamiento, algo va mal en tu diseño.**

**(d) `ACTION_CREATE_DOCUMENT` no sobrescribe** — «the system appends a number in
parentheses». Para recibir en una carpeta elegida, usa
`ACTION_OPEN_DOCUMENT_TREE` y gestiona los nombres con `DocumentsContract`.

**(e) Persistir el acceso** requiere `takePersistableUriPermission`, y aun así:
«your app doesn't retain access to the URI if the associated document is moved or
deleted». **Hay un tope de grants persistidos por paquete que no pude verificar en
documentación oficial** — si vas a persistir un URI por entrada de historial,
investígalo antes de diseñarlo.

### 4.2 — Windows: no escribas COM a mano

El argumento «ya escribimos DPAPI a mano» **no se transfiere**. DPAPI son tres
símbolos planos exportados por nombre. `IFileOpenDialog` son **~29 slots de
vtable** —`IUnknown` 3 + `IModalWindow` 1 + `IFileDialog` 23 + 2—, más GUIDs
transcritos, más `IShellItem`, más el modelo de apartments.

**Y la página de referencia de Microsoft lista los métodos en orden alfabético, no
de vtable.** El orden real sólo sale del IDL del Windows SDK. Un slot desplazado
no da error de compilación ni de link: da **UB silencioso**.

Además `Show()` es modal y tiene que correr en el hilo de UI de Flutter, así que
la llamada se orquesta desde Dart de todos modos.

*(`GetOpenFileNameW` está oficialmente superseded desde Vista: «the Open and Save
As common dialog boxes have been superseded by the Common Item Dialog».
learn.microsoft.com, act. 2024-11-20.)*

### 4.3 — El paquete

**`file_selector`**, publisher **`flutter.dev` verificado**, BSD-3, federado por
plataforma. `file_picker` es más completo pero es un mantenedor individual, y para
el puente de una app de transferencia privada la superficie de confianza importa.

**Bloqueador que hay que resolver ANTES de escribir código:** no se pudo confirmar
con fuente primaria si `file_selector_android` devuelve el `content://` o **una
copia en la caché de la app**. Si copia, **un archivo de 4 GB se duplica en disco
antes de empezar la transferencia**.

**Compruébalo empíricamente** —leyendo el Kotlin del plugin o en un dispositivo—
y si copia, escribe un `MethodChannel` propio de ~60 líneas Kotlin que devuelva el
fd. **Sigue siendo cero crates de Rust.**

## 5. Lo que hay que construir, paso a paso

### Paso 1 — Resolver el bloqueador de §4.3 y congelar ADR-0034

`docs/adr/ADR-0034-file-selection.md`, con:

- el resultado de la comprobación de §4.3, **medido, no supuesto**;
- la superficie FFI nueva: `qyro_send_open_fd(...)` junto a
  `qyro_send_open_path(...)`, y por qué dos y no una;
- **quién cierra el fd** en cada plataforma, y qué pasa si Rust falla antes;
- la política de destino en cada plataforma;
- qué pasa si el usuario **revoca el acceso a mitad** de una transferencia larga;
- lo que la decisión no promete.

**Puerta.**

### Paso 2 — La superficie FFI por descriptor

- `from_raw_fd` con `SAFETY:` escrito, y la lista de crates exentos de
  `forbid(unsafe_code)` actualizada **con justificación** — ese número es una
  guarda.
- Pruebas en Linux, que también tiene fds: abrir un fd con `open(2)`, pasarlo, y
  comprobar que se lee igual que por ruta. **El fd es portable; el SAF no.**
- **Y la prueba del doble cierre**: comprueba que Rust cierra exactamente una vez.

**Puerta.**

### Paso 3 — Android

- El selector desde Dart, con `"rw"`.
- El puente Kotlin si §4.3 lo exige.
- **Prueba instrumentada o en emulador**: elegir un archivo, transferirlo, y
  comprobarlo. El workflow `android-runtime.yml` ya existe.
- **Y comprueba el manifiesto: si aparece un permiso de almacenamiento, el diseño
  está mal.**

**Puerta.**

### Paso 4 — Windows y iOS

- Windows: `file_selector` desde Dart, ruta a Rust. Job de Windows ya existe.
- iOS: `UIDocumentPicker`. **Y el detalle que importa:**
  `startAccessingSecurityScopedResource()` y
  `stopAccessingSecurityScopedResource()` **están contados** —«the last balanced
  call»— y Apple avisa: «If you fail to relinquish your access … **your app leaks
  kernel resources**».
  **La forma limpia:** Swift abre el fd con `open(url.path, ...)` **dentro** del
  scope y lo cierra el scope inmediatamente; el fd sobrevive. Eso unifica la API
  de Rust a «recibe un fd» en Android **e** iOS.
- Los archivos **recibidos** en iOS van a `Documents/` del contenedor, sin picker
  y sin permisos. Con `UIFileSharingEnabled = YES` la carpeta queda expuesta.
  *(`LSSupportsOpeningDocumentsInPlace` no está verificado; si lo usas,
  compruébalo primero.)*

**Puerta de fase.**

## 6. Pruebas obligatorias

- `a_file_opened_by_descriptor_reads_identically_to_one_opened_by_path`
- `the_descriptor_is_closed_exactly_once`
- `a_transfer_driven_by_descriptor_arrives_byte_identical`
- `a_revoked_descriptor_mid_transfer_is_a_typed_error_not_a_hang`
- Android, en emulador: `a_file_chosen_through_saf_transfers_and_verifies`
- Android: `the_manifest_declares_no_storage_permission`
- Windows: `a_file_chosen_through_the_system_dialog_transfers_and_verifies`
- iOS, en simulador: el equivalente, o **registrado por qué no**

## 7. Criterios de aceptación

1. **El bloqueador de §4.3 resuelto con evidencia medida**, no supuesta.
2. ADR-0034 congelada antes del código.
3. La superficie por fd existe, con `SAFETY:` escrito y la lista de exentos
   actualizada y justificada.
4. **El fd se cierra exactamente una vez**, con prueba.
5. **Android abre en `"rw"`**, y hay una prueba o un argumento escrito de por qué
   el seek de reanudación funciona.
6. **El manifiesto de Android no declara ningún permiso de almacenamiento.**
7. Un archivo elegido por el usuario se transfiere y se verifica **en emulador
   Android y en Windows**. iOS: probado en simulador o registrado.
8. **Cero crates de Rust nuevos.** Y en Dart, sólo `file_selector` de `flutter.dev`
   —o ninguno, si escribes el `MethodChannel`—. Di los dos conteos.
9. Barrido con `cargo-mutants`, alcance declarado.
10. `R2` en todas las puertas. Informe según `R5`.
11. **Los botones siguen `onPressed: null`.**

## 8. Cómo tiene que quedar el resultado

El usuario pulsa un botón de prueba —todavía no el de Enviar—, sale el selector de
**su** sistema, elige un archivo de 4 GB, **y no se duplica en disco**. Rust lo lee
por descriptor y lo transfiere.

## 9. No objetivos

- Descubrimiento, UI, emparejamiento, Keystore, empaquetado.
- **Permisos de red.** Eso es la fase 04.
- Persistir URIs para el historial — se registra, se hace en la 09 si hace falta.

## 10. Qué desbloquea

La fase 05: sin selector no hay pantalla de Enviar que valga nada.
