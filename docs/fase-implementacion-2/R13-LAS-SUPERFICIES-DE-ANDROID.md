# R13 — Las tres superficies de Android, y las trampas que tienen

> Investigación del 2026-08-19 sobre las tres cosas que pidió el propietario:
> **el tile del centro de control**, **salir en el menú de compartir**, y **la
> ventanita de recepción**. Con las restricciones de plataforma que las tres han
> ido acumulando entre Android 13 y Android 15.
> **Cada una tiene una trampa que no se ve hasta que el APK está en un teléfono.**

---

## 1. El tile de Ajustes Rápidos

**Qué es:** `TileService`, la baldosa que sale junto a linterna y Bluetooth.

**Lo que hace falta en el manifiesto**, y las tres partes son obligatorias:

- `android:permission="android.permission.BIND_QUICK_SETTINGS_TILE"`
- `<intent-filter>` con `android.service.quicksettings.action.QS_TILE`
- `android:exported="true"`

**La trampa, y es la que rompe la mitad de las implementaciones:**

> `startActivityAndCollapse(Intent)` **está deprecado y lanza
> `UnsupportedOperationException` en Android 14 (API 34) y superiores.**
> La sobrecarga viva es **`startActivityAndCollapse(PendingIntent)`**, y el
> `PendingIntent` **tiene que llevar `FLAG_IMMUTABLE`**.

Un tile que funciona en el emulador de API 33 y revienta en un teléfono de 2025 es
exactamente el tipo de defecto que este proyecto ya ha cometido dos veces
(compilar en Windows y romper Linux). **Se escribe la rama por versión y se
prueba en las dos.**

**Segunda trampa:** desde Android 14, si la pantalla está bloqueada el sistema pide
desbloquear antes de lanzar la Activity. Con `isSecure = true` en el tile eso es
explícito; sin él, el usuario ve que no pasa nada.

**El regalo:** `StatusBarManager.requestAddTileService()` (API 33+) **abre un
diálogo del sistema pidiéndole al usuario que añada la baldosa.** Sin él, el
usuario tiene que editar el panel a mano y **nadie lo hace**. Se ofrece una vez,
desde ajustes, y se recuerda la respuesta.

**Qué hace el tile de Qyro:** no envía nada por sí solo. Abre Qyro **directamente
en la pantalla de recibir, con el código ya generado y visible**. Ése es el gesto
real: «alguien me va a mandar algo, dame mi código ya».

---

## 2. Salir en el menú de compartir

**Nivel 1 — aparecer, y cuesta seis líneas.** Un `<intent-filter>` en la Activity:

- `ACTION_SEND` con `mimeType="*/*"`
- `ACTION_SEND_MULTIPLE` con `mimeType="*/*"`

**Nivel 2 — salir en la fila de arriba.** La fila superior del selector la ocupan
los *sharing shortcuts*. Hacen falta: `res/xml/shortcuts.xml` con un
`<share-target>`, el `<meta-data android:name="android.app.shortcuts">`, y
publicar el atajo con `ShortcutManagerCompat.pushDynamicShortcut` usando
**`setLongLived(true)`** y las mismas `categories` que declara el `<share-target>`.
Qyro publica **un atajo por peer emparejado**: «Compartir con el portátil» sale en
el menú del sistema, y eso es el hueco de `R11` §6 que **nadie del sector ha
llenado**.

### 2.1 — La trampa de verdad: llega un `Uri`, y el motor quiere una ruta

`EXTRA_STREAM` da un `content://`, **no un archivo**. Y muchos no tienen ruta real:
un `content://media/...` de otra app, un documento en la nube, un archivo dentro de
un `.zip` servido por un `DocumentsProvider`. **`getPath()` sobre eso es basura.**

Dos salidas, y hay que elegir con el número delante:

| | Copiar a caché | Pasar el descriptor |
|---|---|---|
| Coste | **Duplica el archivo en disco.** Un vídeo de 8 GB necesita 8 GB libres antes de empezar | cero |
| Riesgo | `ENOSPC` en un teléfono lleno, y el usuario no entiende por qué | Rust abre con `File::from_raw_fd`, que es `unsafe` |
| Veredicto | inaceptable para el caso que `R7` nombra | **el correcto** |

> **Se pasa el descriptor.** `contentResolver.openFileDescriptor(uri, "r")` →
> `detachFd()` → cruza el FFI como `int` → `File::from_raw_fd`.
> **Antes de escribir una línea: comprobar si la crate de la frontera C ya tiene
> excepción a `forbid(unsafe_code)`.** Si la tiene —y es casi seguro, porque es la
> frontera C— **esto no añade una excepción nueva**, que es la condición que la
> FASE 24B fijó. Si no la tiene, se para y se decide en una ADR antes de codificar.

Y con el descriptor viene lo que **no** viene: el nombre. Se obtiene aparte, del
`ContentResolver.query` sobre `OpenableColumns.DISPLAY_NAME` y `SIZE`. **Ese nombre
es una sugerencia del emisor, no una ruta** — la regla de `R11` §2.6 y §4 aplica
igual aquí.

**Y no se olvida `takePersistableUriPermission` si el Uri viene de un árbol de
documentos**, o al reintentar dentro de cinco minutos ya no hay permiso.

---

## 3. La ventanita de recepción

Lo que pidió el propietario: *«como si fuera una notificación, pero no notificación,
sino una aplicación no completa»*. Hay tres formas y **dos son trampas**.

| Forma | Veredicto |
|---|---|
| `setFullScreenIntent` | ❌ **Desde Android 14, `USE_FULL_SCREEN_INTENT` sólo se concede automáticamente a apps de llamadas y alarmas.** Para las demás llega **denegado** y hay que mandar al usuario a `Settings.ACTION_MANAGE_APP_USE_FULL_SCREEN_INTENT`. Una app de transferencia **no puede depender de esto** |
| Bubbles | ❌ Exige un atajo de conversación y que el usuario acepte la burbuja. Es API de mensajería; forzarla aquí es pelear con el sistema |
| **Activity translúcida flotante** | ✅ **La correcta** |

**La arquitectura que sale de ahí, y es mejor que lo que se pidió:**

1. Llega una petición → **notificación en canal `IMPORTANCE_HIGH`**, que sale como
   *heads-up* encima de lo que sea. Sin permisos especiales.
2. **Las acciones van en la propia notificación:** **Aceptar · Rechazar ·
   Aceptar y recordar.** Es el `Y / N / P` de `R11` §3, y significa que **el caso
   normal se resuelve sin abrir nada**.
3. **Tocar la notificación** abre la Activity flotante: tema translúcido
   (`windowIsTranslucent`, `windowIsFloating`, `windowBackground=transparent`,
   `windowCloseOnTouchOutside`), tarjeta de vidrio centrada con el nombre del peer,
   su huella, el archivo y su tamaño. **Nada más.**
4. **Ahí y sólo ahí manda `R12` §4.2: α = 0.84 y sólo `text.primary`**, porque
   detrás hay la pantalla de otra app y su luminancia es desconocida.

**Regla que no se negocia:** la notificación **nunca** acepta sola. Enseña quién,
qué y la huella; el dedo decide. Es la garantía entera del producto.

---

## 4. Recibir con la app cerrada

Un `ForegroundService` con `android:foregroundServiceType="dataSync"` y el permiso
`FOREGROUND_SERVICE_DATA_SYNC`. Y tres cosas que muerden:

1. **Android 15 (API 35) limita `dataSync` a 6 horas por cada 24.** Al agotarse, el
   sistema llama a **`Service.onTimeout()`** y **la app tiene que pararlo ahí
   mismo** o se lleva un ANR. Hay que implementar `onTimeout` — no es opcional.
2. **Android 15 prohíbe arrancar un `dataSync` desde `BOOT_COMPLETED`.** El servicio
   arranca cuando el usuario enciende «recibir», no al arrancar el teléfono.
3. **`POST_NOTIFICATIONS` es permiso en tiempo de ejecución desde API 33.** Denegado
   → **el servicio sigue funcionando pero es invisible**. Hay que enseñarlo en la
   app y degradar con un texto claro, no callar.

Y el fallo que `R11` §2.9 documenta del sector: **arrancar en segundo plano y no
poder recibir hasta abrir la ventana una vez.** La prueba que lo caza:
**matar la Activity y mandar un archivo.** Si no llega, el modo no existe.

---

## 5. Los permisos, y qué pasa cuando dicen que no

`R11` §2.8: **LocalSend 1.18.0 se volvió indetectable en Android 17** por un cambio
de permisos. La lección no es «pedir más», es **degradar visiblemente**.

| Permiso | Si lo deniegan |
|---|---|
| `NEARBY_WIFI_DEVICES` (API 33+) | Se pierde **el descubrimiento**, no el envío. La pantalla dice *«no puedo buscar aparatos cerca; teclea el código»* y **el camino por código sigue entero** |
| `ACCESS_LOCAL_NETWORK` (API 36+) | Igual, y hay que declararlo antes de que sea obligatorio |
| `POST_NOTIFICATIONS` | Recepción en segundo plano invisible → se avisa y se ofrece el ajuste |

> **Ninguna denegación produce una pantalla en blanco.** Cada una tiene una frase y
> un camino alternativo. Eso se prueba: un test por permiso denegado que exige que
> la pantalla **contenga el texto alternativo**.

---

## 6. La trampa de empaquetado que hunde el APK entero

> **Google Play exige, para apps que apuntan a Android 15+, que las librerías
> nativas soporten páginas de 16 KB.** Un `.so` alineado a 4 KB **no carga** en un
> dispositivo de 16 KB: la app se cae al abrir, y sólo en esos aparatos.

Qyro empaqueta un `.so` de Rust. Por tanto:

- NDK **r27 o superior**.
- `RUSTFLAGS` con **`-C link-arg=-Wl,-z,max-page-size=16384`** para cada target
  Android.
- Comprobar el resultado: `readelf -l libqyro.so | grep LOAD` y **verificar que la
  alineación es `0x4000`**, no `0x1000`.
- **La puerta lo comprueba sobre el `.so` que va dentro del APK**, no sobre el que
  salió de `cargo`. Son cosas distintas y sólo la primera es la que se instala.

Es exactamente el tipo de defecto que este proyecto tiene historial de cometer:
verificar el artefacto intermedio y publicar otro.

---

## 7. Resumen de decisiones ya tomadas aquí

Para que la FASE 27 **no reabra nada**:

1. Tile → abre **recibir con el código ya visible**; `PendingIntent` con
   `FLAG_IMMUTABLE`; `requestAddTileService` ofrecido una vez.
2. Share sheet → `ACTION_SEND` + `ACTION_SEND_MULTIPLE` + `<share-target>` con
   **un atajo por peer emparejado**.
3. Uri → **descriptor de archivo**, nunca copia a caché; nombre por
   `OpenableColumns`; **comprobar la excepción de `unsafe` existente antes**.
4. Ventanita → **notificación `IMPORTANCE_HIGH` con tres acciones** + Activity
   translúcida flotante. **`setFullScreenIntent` descartado con motivo.**
5. Segundo plano → `dataSync` con **`onTimeout()` implementado**.
6. Empaquetado → **16 KB verificado sobre el `.so` del APK**.
