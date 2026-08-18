# FASE 20 — Distribución

> La última, y la que tiene el riesgo que **ninguna cantidad de ingeniería en Rust
> arregla**.

---

## 1. El riesgo, dicho primero

De `R8` §12, con fuente de Microsoft:

- **Smart App Control (Windows 11)** bloquea *«unknown, unsigned code … by default»*
  y **«There is currently no way to bypass Smart App Control protection for
  individual apps.»** No hay «Run anyway». En una máquina con SAC activo, un binario
  sin firmar **no corre, punto**.
- **SmartScreen** sí tiene «Run anyway», pero la reputación tarda *«several weeks and
  hundreds of clean installs from a wide audience»* y **se pierde en cada release**
  salvo que firmes con la misma identidad.
- **EV ya no sirve para esto.** Microsoft documenta que *«EV certificates no longer
  bypass SmartScreen»* y que pagar el premium por eso *«is no longer justified»*.
- **AppLocker con reglas por defecto** permite por *path* (`%WINDIR%`,
  `%PROGRAMFILES%`) ⇒ `%USERPROFILE%\Downloads\qyro.exe` queda fuera. En esa máquina
  el binario no arranca y no hay nada que el binario pueda hacer.

**Firmar cuesta dinero, y el dinero es una de las cuatro excepciones de autonomía.**
No lo decidas tú. **Prepara todo, mide el coste, y deja la decisión escrita con sus
dos salidas y lo que cuesta cada una.**

---

## 2. La vía que sí funciona hoy, y hay que construirla

**Mark of the Web vive en un ADS de NTFS.** Copiar el `.exe` a un USB con **FAT32 o
exFAT lo elimina**, y con él la fricción de SmartScreen. Para máquinas viejas y
bloqueadas, **la ruta USB es la más fiable que existe.**

Y hay una ironía útil que hay que aprovechar: **Qyro es una herramienta para meter
archivos en máquinas difíciles.** Una vez que hay un Qyro corriendo en una máquina,
**Qyro puede llevarse a sí mismo a la siguiente** — son 800 KB, que por serie son
ochenta segundos y por QR son un minuto y medio.

**Entregable: `qyro send --self`.** Manda el propio binario. Es la respuesta correcta
al problema del bootstrap y sale casi gratis.

---

## 3. Entregables

1. **Un artefacto por target**, con SHA-256 publicado, y un `BUILD-INFO.txt` dentro
   que diga con qué se firmó — o que **no** se firmó, en mayúsculas.

| Target | Cuándo |
|---|---|
| `x86_64-pc-windows-msvc` | fase 13 |
| `i686-pc-windows-msvc` | fase 13 |
| `x86_64-unknown-linux-musl` | fase 13 |
| `i686-unknown-linux-musl` | fase 13 |
| `x86_64-win7-windows-msvc` | fase 17 |
| `i686-win7-windows-msvc` | fase 17 |
| APK Android | ya existe |

2. **`qyro send --self`** (§2).
3. **Un `README` de instalación de cinco líneas**, no de cincuenta. Descarga,
   desbloquea, ejecuta. Con la pantalla de SmartScreen **fotografiada o descrita
   exactamente**, porque un usuario que ve «Windows protegió tu PC» sin aviso previo
   cierra la ventana.
4. **La página de la Release**, con la advertencia de no-aprobado arriba del todo
   mientras el protocolo de hardware siga en blanco.
5. **La decisión de firma, escrita para el propietario**, con:
   - qué cuesta un certificado OV al año,
   - qué resuelve exactamente (SAC sí, SmartScreen parcialmente y con el tiempo),
   - qué **no** resuelve (AppLocker corporativo),
   - y la alternativa de no firmar nunca y distribuir por USB.
   **No la tomes tú.**

---

## 4. Lo que NO hay que hacer

- **No compres nada.** Dinero es excepción.
- **No metas actualizador automático.** `R7` §5: Qyro no es un producto de tienda.
- **No subas a ninguna tienda.** Ni Play Store, ni Store, ni winget, ni Chocolatey.
  Cero terceros no es sólo sobre el transporte de los archivos.
- **No firmes con una clave nueva** si eso rompe la actualización del APK: Android se
  niega a instalar una actualización firmada por otra clave, y ése es el
  comportamiento correcto.
