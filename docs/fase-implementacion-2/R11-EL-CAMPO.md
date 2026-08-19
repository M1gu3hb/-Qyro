# R11 — El campo: qué tienen los demás y Qyro no

> Investigación del 2026-08-19 sobre trece proyectos reales, sus READMEs, su
> código y **sus issue trackers**. Cada afirmación tiene URL y fecha en el informe
> original. **No lo reinvestigues.**

---

## 1. Las cinco carencias, ordenadas

| # | Qué falta | Aparece en | Por qué duele |
|---|---|---|---|
| 1 | **Cola: varios archivos con progreso agregado y cancelar** | **13 de 13** | Sin ella Qyro es un transferidor de **un** archivo, no una app de transferencia |
| 2 | **Carpetas con rutas relativas** | 10 de 13 | Y trae consigo el bug más grave del sector — §4 |
| 3 | **Recepción en segundo plano (Android)** | 6 de 13 | Hoy el receptor tiene que tener la app abierta **en el momento exacto** |
| 4 | **Share sheet, tile, notificación** | 6 de 13 | Sin share sheet, «estoy en Galería y quiero mandar esto» obliga a salir de la app: la fricción que devuelve a la gente a WhatsApp |
| 5 | **Enviar texto / portapapeles** | 8 de 13 | Un frame más en `QYRO1` con `kind=text`. Barato y muy usado |

**Ninguna choca con las reglas de Qyro.** LocalSend, Warpinator y KDE Connect
descubren automáticamente **y aun así piden confirmación**: descubrir no compra
nada contra «el receptor siempre decide».

---

## 2. Los diez modos de fallo que Qyro va a sufrir

Sacados de issues abiertos, con años de antigüedad. **Adelantarse a éstos es más
valioso que cualquier función nueva.**

1. **Cortafuegos bloqueando la entrada — el problema nº 1 del sector.** LocalSend
   lleva la issue #26 abierta **desde 2023-01-25**. → Qyro con puerto **fijo** es
   *más* frágil: si en 60 s no entra nada habiendo peers visibles, **banner** y un
   botón «diagnosticar red».
2. **`errno 10013` en Windows: el puerto fijo no se puede vincular.** Hyper-V, WSL
   y Docker reservan rangos dinámicos. LocalSend tiene cinco issues de esto, una
   con 22 comentarios abierta desde 2025-10-06. → **Qyro necesita puerto de
   respaldo**, y su formato `QYRO1|ip:port|fingerprint` **ya lo soporta**.
3. **Descubrimiento roto por topología**: VLANs, subredes >/24, VPN, aislamiento de
   AP, Ethernet↔Wi-Fi. → TTL configurable y **selector de interfaz desde el día
   uno**; LocalSend tardó dos años en añadirlo.
4. **Conexión unidireccional**: «me ve pero no lo veo». → enseñar el estado **por
   dirección**, no un booleano.
5. **Archivos grandes: cuelgue al 99 %, OOM, degradación progresiva.** LocalSend
   tuvo que arreglar explícitamente «recibir archivos que exceden la RAM
   disponible». → invariante de prueba: **memoria O(1) por frame**, y un archivo
   mayor que la RAM en el protocolo de hardware.
6. **Nombres de archivo**: encoding, `&`, `:`, chinos, longitud, colisiones. Diez
   issues, una abierta desde 2023-02-01. → **el nombre viaja en UTF-8 NFC, con
   longitud máxima en bytes, y el receptor sanea y desambigua. Es una sugerencia,
   nunca una ruta.**
7. **Path traversal y symlinks al escribir.** **CVE-2022-42725 en Warpinator, CVSS
   7.5**, por enlaces simbólicos. LocalSend arregló un path traversal en 1.17.0.
   → §4.
8. **Permisos de Android 13/14/15/16/17.** `NEARBY_WIFI_DEVICES`,
   `ACCESS_LOCAL_NETWORK`. LocalSend 1.18.0 se volvió **indetectable** en Android
   17 por esto (issue del 2026-08-11). → **denegado debe degradar a «sólo por
   código tecleado»**, nunca a pantalla en blanco.
9. **Recibir sin abrir la app** — la petición más repetida durante tres años. Y el
   fallo que introduce: arrancar en bandeja y **no poder recibir hasta abrir la
   ventana una vez**.
10. **Accesibilidad.** LocalSend tiene abierta y **sin respuesta** una issue del
    2026-08-12: los ajustes son inutilizables con NVDA y TalkBack. → **el líder de
    categoría no es accesible. Es un diferenciador barato y real.**

---

## 3. Los patrones de interfaz que hay que robar

**Una fila, no dos listas.** LocalSend pinta igual un dispositivo descubierto y
uno tecleado; lo que cambia es un **badge**. Y durante la transferencia **la barra
de progreso ocupa el lugar del badge** — eso elimina la pantalla de progreso
separada.

**El SAS de dieciséis iconos.** LocalSend 1.18.0 combina las huellas de los **dos**
extremos, hace SHA-256, coge los primeros 128 bits y los mapea a **16 iconos** de un
alfabeto de 256 con siluetas distinguibles, en rejilla de 4×4, junto al hash en
texto. La pregunta literal: **«¿Se ve igual en el otro aparato?»**
→ Qyro ya tiene huella comparable en voz alta. **Con iconos es más rápido, menos
propenso a error, no necesita idioma común y funciona por videollamada.**

**`Y / N / P` = Aceptar / Rechazar / Aceptar-y-recordar.** Del CLI de LocalSend.
Es **la única forma de tener favoritos sin romper «nada se acepta solo»**: acepta
esta transferencia **y** promueve al peer. Róbalo, incluida la letra.
**Y no copies lo otro:** LocalSend activa auto-accept de favoritos **por defecto**
desde 1.18.0. Eso sí viola la regla.

**Reflejar la propia identidad.** PairDrop pone, permanente en pantalla: **«Te
conocen como: …»** y **«Se te puede descubrir: en esta red»**. Para una app cuyo
argumento entero es saber con quién hablas, decir primero **quién eres tú y quién
te ve** es traducir la tesis a la interfaz.

**Alias determinista.** PairDrop deriva «Green Turtle» de un hash del id. Qyro
puede derivar el suyo del **fingerprint Ed25519**: estable, no editable sin cambiar
identidad, sin registro.

**Errores que se distinguen.** LocalSend separa «el destinatario ha rechazado» de
«el destinatario está ocupado» de un timeout. El handshake de cuatro mensajes de
Qyro **ya puede distinguir los tres**. Cuesta casi nada y elimina el 80 % de los
«no funciona».

---

## 4. Lo más valioso de toda la investigación: el aislamiento del destino

**Warpinator es el único que lo hace**, y lo hace porque se llevó un **CVE 7.5 por
symlinks**: durante la transferencia el directorio de destino *«essentially exists
in a vacuum»* — Landlock en Linux ≥5.13, bubblewrap si no.

**Es el bug que Qyro va a tener el día que acepte la primera carpeta.** Qyro cifra
y verifica el *contenido* de forma impecable, y eso **no protege contra un
nombre**. En Windows el equivalente barato es `canonicalize()` sobre el destino y
rechazar todo lo que salga de él; en Android el SAF ya lo da casi gratis.

**Va a `THREAT_MODEL.md` antes de aceptar la primera carpeta.**

---

## 5. Lo que valida la tesis de Qyro, y pertenece al README

**Snapdrop y ShareDrop —los dos referentes históricos— fueron comprados por
LimeWire** y dejaron de ser locales y privados (issue del propio repo, 2025-04-06).
**PairDrop**, el sucesor, sigue dependiendo de un servidor de signaling y un TURN.
**LocalSend** es local pero activa auto-accept por defecto.

Hasta donde alcanza esta investigación, **Qyro es el único proyecto activo cuyo
diseño hace imposibles los dos fallos.** Eso pertenece al README, con las citas.

---

## 6. Y dos huecos que nadie del campo ha llenado

- **Widget de pantalla de inicio para mandar a un aparato concreto**: pedido en
  LocalSend y en Warpinator-Android, hecho por nadie.
- **UI flotante de recepción**: pedida en LocalSend, hecha por nadie.

Son exactamente las dos cosas que el propietario pidió.
