# FASE 27 — Las tres superficies de Android

> El tile del centro de control, salir en el menú de compartir, y la ventanita de
> recepción. **`R13` ya decidió las seis cosas difíciles** (§7). Esta fase las
> construye. **Ninguna se reabre.**
>
> Y las dos primeras son huecos que `R11` §6 encontró **pedidos y no hechos por
> nadie del sector**.

---

## 1. Orden, y por qué éste

```
0. la trampa de 16 KB   →   1. share sheet   →   2. servicio   →   3. ventanita   →   4. tile
```

- **La de 16 KB va primera** porque si el `.so` está mal alineado, **todo lo demás
  se prueba sobre un APK que no arranca** en la mitad de los teléfonos modernos.
  Descubrirlo al final es rehacer la fase.
- **El share sheet va antes que el tile** porque trae consigo el problema del
  descriptor (`R13` §2.1), que toca la frontera C. Lo caro primero.
- **La ventanita va antes que el tile** porque el tile sólo abre una pantalla que
  ya tiene que existir.

---

## 2. Paso 0 — 16 KB, y se verifica sobre el APK

`R13` §6. NDK **r27+**, `RUSTFLAGS` con **`-C link-arg=-Wl,-z,max-page-size=16384`**
para cada target Android, y después:

> `readelf -l` sobre el `.so` **extraído del APK**, y **la alineación tiene que ser
> `0x4000`**. No sobre el que salió de `cargo`: sobre el que se instala.

**Comprobación 20 de la puerta.** Este proyecto ya publicó una vez binarios que no
eran los que decía; la forma de no repetirlo es comprobar el artefacto final.

---

## 3. Paso 1 — El menú de compartir

**Nivel 1:** `ACTION_SEND` y `ACTION_SEND_MULTIPLE` con `mimeType="*/*"`.

**Nivel 2 — la fila de arriba:** `res/xml/shortcuts.xml` con `<share-target>`,
`<meta-data android:name="android.app.shortcuts">`, y
`ShortcutManagerCompat.pushDynamicShortcut` con **`setLongLived(true)`** y las
mismas `categories`. **Un atajo por peer emparejado**: «Compartir con el portátil»
sale en el menú del sistema. **Eso no lo tiene nadie** (`R11` §6).

**El descriptor** (`R13` §2.1), y es el punto delicado de toda la fase:

1. **Antes de escribir una línea**, comprobar en el repo si la crate de la frontera
   C **ya tiene excepción a `forbid(unsafe_code)`**. Si la tiene, `File::from_raw_fd`
   **no añade una excepción nueva** — que es la condición que la FASE 24B fijó.
   **Si no la tiene, se para y se decide en una ADR antes de codificar.**
2. `contentResolver.openFileDescriptor(uri,"r")` → `detachFd()` → `int` por el FFI.
   **Nunca copiar a caché:** un vídeo de 8 GB necesitaría 8 GB libres antes de
   empezar.
3. El nombre y el tamaño por `ContentResolver.query` sobre `OpenableColumns`.
   **Ese nombre es una sugerencia** y pasa por la canonicalización de la
   FASE 25 §2 igual que cualquier otro.
4. `takePersistableUriPermission` cuando el Uri venga de un árbol de documentos.

**Prueba:** compartir desde otra app un archivo con `content://` **sin ruta real**
—el caso que rompe `getPath()`— y exigir que cruce y verifique byte a byte.
**Control:** cerrar el descriptor a propósito y exigir que falle **por nombre**.

---

## 4. Paso 2 — Recibir con la app cerrada

`ForegroundService` con `foregroundServiceType="dataSync"` y
`FOREGROUND_SERVICE_DATA_SYNC`. Las tres que muerden (`R13` §4):

1. **`Service.onTimeout()` implementado.** Android 15 corta `dataSync` a las 6 h por
   cada 24 y llama ahí. Sin implementarlo, es un ANR.
2. **Nada de arrancar desde `BOOT_COMPLETED`.** Arranca cuando la persona enciende
   «recibir».
3. **`POST_NOTIFICATIONS` denegado** → el servicio sigue pero es invisible. Se avisa
   y se ofrece el ajuste. **Nunca se calla.**

**La prueba que decide si el modo existe:** *matar la Activity y mandar un
archivo.* Si no llega, no existe. Es el fallo exacto que `R11` §2.9 documenta en el
sector.

---

## 5. Paso 3 — La ventanita

`R13` §3, ya decidido: **`setFullScreenIntent` descartado** —desde Android 14
`USE_FULL_SCREEN_INTENT` llega denegado a quien no sea app de llamadas o alarmas—
y **Bubbles descartada**.

1. **Notificación en canal `IMPORTANCE_HIGH`** → sale como *heads-up* encima de lo
   que sea, sin permisos especiales.
2. **Tres acciones en la propia notificación: Aceptar · Rechazar · Aceptar y
   recordar.** Es el `Y/N/P` de `R11` §3 y significa que **el caso normal se
   resuelve sin abrir nada**.
3. **Tocarla** abre la **Activity translúcida flotante**: `windowIsTranslucent`,
   `windowIsFloating`, fondo transparente, `windowCloseOnTouchOutside`. Una tarjeta
   de vidrio centrada con **el peer, su huella en 16 iconos, el archivo y el
   tamaño**. Nada más.
4. **Aquí manda `R12` §4.2: α = 0.84 y sólo `text.primary`**, porque detrás está la
   pantalla de otra app y su luminancia es desconocida. A α 0.72 el texto
   secundario da **3.13** y suspende.

> **La notificación nunca acepta sola.** Enseña quién, qué y la huella. El dedo
> decide. Es la garantía entera del producto y no se relaja por comodidad.

---

## 6. Paso 4 — El tile

`R13` §1:

- `BIND_QUICK_SETTINGS_TILE` + `<intent-filter>` con `QS_TILE` + `exported="true"`.
- **`startActivityAndCollapse(PendingIntent)`** con **`FLAG_IMMUTABLE`**. La
  sobrecarga con `Intent` **lanza `UnsupportedOperationException` en API 34+**:
  funciona en un emulador viejo y revienta en un teléfono real. **Rama por versión,
  probada en las dos.**
- `isSecure = true`, para que con la pantalla bloqueada el sistema pida desbloquear
  en vez de no hacer nada.
- **`requestAddTileService()`** ofrecido **una vez** desde ajustes, y se recuerda la
  respuesta. Sin él, nadie añade la baldosa a mano.
- **Qué hace:** abre Qyro **en recibir, con el código ya generado y visible**. Ése
  es el gesto real.

---

## 7. Permisos: nadie ve una pantalla en blanco

`R13` §5. **Cada denegación tiene una frase y un camino alternativo**, y cada una
tiene **un test que exige que la pantalla contenga ese texto**:

| Denegado | Qué pasa |
|---|---|
| `NEARBY_WIFI_DEVICES` | se pierde el **descubrimiento**, no el envío: *«teclea el código»* |
| `ACCESS_LOCAL_NETWORK` (API 36+) | igual, y se declara antes de que sea obligatorio |
| `POST_NOTIFICATIONS` | recepción en segundo plano invisible → se avisa |

---

## 8. Paridad

Las cuatro superficies son **de una sola cara** y eso **se escribe en la tabla**:
`NO -- superficie del sistema operativo Android; el CLI tiene su equivalente en
qyro recv`. Una celda con argumento es una respuesta completa; una celda vacía es
un olvido.

---

## 9. Lo que NO hay que hacer

- **No uses `startActivityAndCollapse(Intent)`.**
- **No copies el archivo compartido a caché.**
- **No dependas de `setFullScreenIntent`.**
- **No aceptes automáticamente desde la notificación**, ni siquiera de un peer
  emparejado. «Aceptar y recordar» promueve al peer; **no salta la próxima
  decisión**.
- **No verifiques la alineación sobre el `.so` de `cargo`.** Sobre el del APK.
