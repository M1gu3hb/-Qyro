# R8 — Lo que la investigación dice. Números duros, con fuente.

> **No vuelvas a investigar nada de este documento.** Está medido, tiene fuente
> primaria y fecha, y algunas cifras se verificaron compilando código. Si un número
> de aquí te parece pesimista, **es que está medido y tu intuición no**.
>
> Investigación del 2026-08-17. Cada afirmación lleva URL y fecha.

---

## 1. QR: capacidad real por frame

**ISO/IEC 18004:2024** — https://www.iso.org/standard/83389.html. Versión 40 =
177×177 módulos; máximo modo byte **2 953 B** en nivel L. Derivado y verificado
contra las tablas RS de ZXing (`Version.java` líneas 565-573).

| Versión | Módulos | L | M | Q | H |
|---|---|---|---|---|---|
| 10 | 57×57 | 271 | 213 | 151 | 119 |
| 20 | 97×97 | **858** | 666 | 482 | 382 |
| **27** | **125×125** | **1 465** | 1 125 | 805 | 625 |
| 33 | 149×149 | 2 068 | 1 628 | 1 168 | 898 |
| 40 | 177×177 | 2 953 | 2 331 | 1 663 | 1 273 |

### La decisión, ya tomada: **versión 20–27, nivel L, modo byte crudo**

**No versión 40.** Tres proyectos reales en producción convergen:

- **BBQr (Coinkite/Coldcard)** — https://github.com/coinkite/BBQr/blob/master/BBQr.md
  — recomienda *siempre* nivel L «since we are not printing these codes, and only
  showing them on a perfect LCD screen», advierte contra v40 («scanning those QR's
  can be more difficult»), y señala **v27 como «a good sweet spot»**.
- **qr-backup** — https://github.com/za3k/qr-backup/blob/master/docs/FAQ.md — default
  **v10**, motivo raíz: *«a webcam's resolution is lower than a printer's
  resolution»*.
- **Sparrow Wallet** (producto real) — fragmentos de **400 B**, animación a **5 FPS**.

**Por qué v40 falla:** 177 módulos + quiet zone de 4 = 185 de ancho. Un decodificador
fiable necesita 3–4 px de sensor por módulo ⇒ ~648 px sólo para el código. A 1080p
con el QR ocupando el 60 % del alto, estás en el límite sin margen para desenfoque,
glare, moiré ni pulso. A v27 el mismo presupuesto da 4,9 px/módulo.

**Nivel L y no más:** una pantalla no es papel. La corrección de errores del QR
protege contra suciedad y arrugas que aquí no existen. La pérdida real es de
**frames enteros**, y de eso protege el fountain code, no el nivel EC.

**Modo byte crudo, nunca Base64.** Base64 cuesta +33 %. Bytewords/hex de BC-UR cuesta
+37,5 % en bits. Si controlas los dos extremos —y los controlas— el modo byte crudo
cuesta 0 %.

---

## 2. QR animado: frame rate

**El limitante dominante no es el ancho de banda: son los frames perdidos.** La
pantalla y la cámara no están sincronizadas; cualquier frame de cámara que caiga a
caballo de una transición es basura. Con cámaras de ~30 FPS, la pantalla debe ir
**muy por debajo de fps_cámara/2**.

Tres fuentes independientes convergen en el mismo rango:

| Fuente | FPS |
|---|---|
| divan/txqr, barrido automatizado 3–12 FPS | **óptimo 6–7** |
| BBQr / Coldcard Q (recomendación del vendor) | **4** (`250 ms`) |
| Sparrow Wallet (código fuente, `ANIMATION_PERIOD_MILLIS = 200d`) | **5** |

**Decisión: 5 FPS por defecto, ajustable 3–10.** Empezar lento y acelerar si el
receptor confirma que no pierde.

---

## 3. Fountain codes

`TXQR` (MIT, https://github.com/divan/txqr) y **BC-UR de Blockchain Commons**
(https://developer.blockchaincommons.com/animated-qrs/, usado por Sparrow, Keystone,
Passport) usan ambos **Luby Transform**. BBQr **no** usa fountain y por eso exige
escanear las N piezas: *«All 'N' QR codes must be scanned, there is no way to
'skip' one»*. Ése es el error que no hay que copiar.

| | Luby Transform | RaptorQ (RFC 6330) |
|---|---|---|
| Complejidad de implementar | **200–400 líneas** | miles |
| Overhead de recepción | 5–15 % | 0,02 % |
| Patentes | **ninguna viva** | **sí, Qualcomm** |

**RaptorQ está gravado.** IETF IPR Disclosure #2554, QUALCOMM, 2015-03-19 —
https://datatracker.ietf.org/ipr/2554/. Para dispositivos no-wireless-WAN Qualcomm
promete no ejercer, **pero condicionado a implementar el RFC completo y con cláusula
de reciprocidad**. Es un compromiso unilateral condicionado, no una licencia limpia.

**Decisión: Luby Transform, implementado en el árbol.** 300 líneas, cero patentes,
cero dependencias nuevas, y encaja con la cultura del proyecto. Si prefieres
interoperar con el ecosistema airgap existente, `ur` 0.5.2 (**MIT**,
https://github.com/dspicher/ur-rs, 4 deps ligeras) implementa BC-UR completo —
decide tú y escribe por qué en una línea.

---

## 4. QR animado: cuánto tarda de verdad. **Esto va en la interfaz.**

Techo físico absoluto (v40-L a 15 FPS, sin pérdidas): **43 KB/s**. Nadie lo sostiene.

| Escenario | Config | Throughput útil |
|---|---|---|
| Conservador (a pulso, lo que ya se envía) | v10–v20, 4–5 FPS | **1–2 KB/s** |
| **Realista optimizado** (soporte fijo, LT) | **v27, 1 465 B, 8 FPS** | **6–10 KB/s** |
| Límite de laboratorio | v33–v40, 12–15 FPS | 15–25 KB/s |

Cita literal de divan tras un barrido automatizado de medio día: *«While maximum
data transfer rate was around 9KB/s, in the vast majority of cases you can expect
more modest rates – **1-2KB/s**»* — https://divan.dev/posts/animatedqr/ (2018-11-18).

### Tiempos, con 8 KB/s

| Payload | Tiempo |
|---|---|
| Clave, seed, certificado (≤4 KB) | < 1 s |
| Config, script, `.env` (≤50 KB) | < 7 s |
| Documento de texto o código comprimido (1 MB) | **2 min** |
| PDF (5 MB) | **11 min** |
| **Una foto JPEG (3–8 MB)** | **6–17 min** |
| 50 fotos (200 MB) | **7 h** |
| **Un minuto de vídeo 1080p (60–150 MB)** | **2–5 h** |

**Correcciones obligatorias antes de enseñar una estimación:**

- **JPEG, PNG, MP4, ZIP: ganancia por compresión = 0.** Ya están comprimidos.
- **Texto, código, logs, dumps: gzip/zstd da 3–5×.** Ahí el canal brilla.
- **Sesión no atendida de 3 h: probabilidad de fallo cercana a 1** (salvapantallas,
  notificación, batería, throttling térmico del móvil). **Hace falta checkpoint y
  reanudación**, no es opcional.

**Regla de producto:** por encima de **20 MB** el canal óptico **se niega por
defecto** y explica por qué, con la estimación. `--force` existe pero avisa.

---

## 5. Meter datos en una máquina que no puede leer un QR

Ordenado por lo que de verdad funciona.

### 5.1 — RS-232 null-modem: **la respuesta**

- **115 200 bps**, framing 8N1 = 10 bits/byte ⇒ **11 520 B/s teóricos, 9–11 KB/s
  reales**. **1 MB en 1,6 min. 10 MB en 16 min.** Un orden de magnitud mejor que el QR.
- `CBR_115200` es universal en UARTs 16550. API de Windows desde XP —
  https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-dcb
- **El receptor no necesita instalar nada:**
  - **Windows XP:** **HyperTerminal viene de serie**, con XMODEM/YMODEM/ZMODEM/Kermit.
  - **Windows 7:** Microsoft quitó HyperTerminal, **pero Win 7 trae PowerShell 2.0**,
    que expone `System.IO.Ports.SerialPort` directamente. Un receptor completo son
    ~15 líneas que se teclean en tres minutos. Después `certutil -decode` reconstruye
    el binario — https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/certutil
- **Qyro debe generar ese script** y enseñarlo en pantalla. Ésa es la función.

### 5.2 — Emulación de teclado PS/2: **sólo como bootstrap, nunca como canal**

Reloj PS/2 10–16,7 kHz, trama de 11 bits ⇒ techo **900–1 500 B/s** de scan codes
(Chapweske, 2001, https://www.tayloredge.com/reference/Interface/atkeyboard.pdf;
medición en http://www.os2museum.com/wp/how-fast-is-a-ps-2-keyboard/, 2018-07-23).
Real, con make+break y el buffer del BIOS: **37–375 B/s**. **1 MB = entre 47 min y
8 h.** Como canal, no. Para teclear 2–10 KB de un receptor, **sí, y es la pieza que
desbloquea todo lo demás.**

### 5.3 — Audio: **no**

`ggwave` (MIT) — el README dice 8–16 B/s; la tabla medida de `gg-transfer` v0.2.13
(2024-12-13, https://pypi.org/project/gg-transfer/) da **11,17 / 16,76 / 33,52 B/s**
según protocolo, y con Base64 encima el binario real cae a **~12,6 B/s**. **1 MB =
17,4 horas. 10 MB = 7,2 días.**
`minimodem` (GPLv3, https://www.whence.com/minimodem/) por cable de audio: **120–480
B/s**. Su man page marca los 12 000 bps como válidos **sólo sobre ficheros WAV**, no
sobre un enlace real — si alguien te cita «minimodem hace 12 kbps», está mal.
**Descartado como transporte. Anotado por si algún día se quiere como bootstrap
alternativo de unos pocos KB.**

### 5.4 — Lo aburrido que gana por goleada, y hay que decírselo al usuario

Si la máquina vieja tiene **lector de CD, disquetera, ranura PCMCIA o tarjeta de
red**, cualquiera de las cuatro es entre **10 y 10 000 veces más rápida** que todo lo
anterior. **Qyro debe preguntarlo antes de proponer el canal lento.** Un CD-R mueve
700 MB en cinco minutos.

---

## 6. El binario portátil: lo medido

**Cifras reales, de compilar un crate con QR encode+decode, X25519, ChaCha20-Poly1305,
BLAKE3, TCP/UDP/broadcast.** No estimaciones.

| Build | Tamaño |
|---|---|
| `--release` por defecto, sin strip | 1 386 KiB |
| `opt-level="z"` **solo**, sin LTO | **1 151 KiB ← creció** |
| `opt="s"` + `lto` + `cgu=1` + `panic=abort` + `strip` | **847 KiB ← el ganador** |
| `opt="z"` + los mismos | 865 KiB |
| `x86_64-unknown-linux-musl`, estático | 913 KiB |
| `x86_64-pc-windows-gnu` | 733 KiB |
| `x86_64-win7-windows-gnu` (nightly, build-std) | 723 KiB |

**Dos hallazgos contraintuitivos, ambos reproducidos:** `opt-level="z"` por su cuenta
**agranda** el binario; y **`"s"` gana a `"z"`** cuando se combinan las palancas.
Mide las dos, no asumas.

**Perfil, ya decidido:**
```toml
[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### La cifra que hay que citar: **~750–950 KB por target.** No 200 KB, no 3 MB.

### Linking estático

- **Windows:** un binario Rust MSVC por defecto necesita `vcruntime140.dll`. Hace
  falta `-C target-feature=+crt-static`, y va en
  `[target.<triple>] rustflags` de `.cargo/config.toml` — **nunca en `[build]`**,
  que `RUSTFLAGS` del entorno pisa en silencio en CI
  (https://doc.rust-lang.org/cargo/reference/config.html). Y **siempre pasar
  `--target` explícito**, o los build scripts reciben los flags y se rompen.
  **Aviso medido:** en `x86_64-pc-windows-gnu` el flag fue **ignorado** y produjo un
  binario byte-idéntico. Verifica los imports, no asumas.
- **Linux:** `x86_64-unknown-linux-musl` es **Tier 2** y estático por defecto
  (verificado: `static-pie linked`). Rompe NSS y el resolver de nombres — **da
  igual, porque Qyro no debe resolver nombres nunca**: el descubrimiento devuelve
  IPs literales y se conecta a IPs literales.

### La regla dura de dependencias

> **Cero crates que compilen C en el árbol del binario portátil.**

Medido: `blake3` por defecto **rompió el build** de win7 con
`undefined reference to blake3_compress_in_place_sse41`; se arregla con
`features = ["pure"]`. Esto descarta `ring` y `openssl` para siempre. RustCrypto
puro, que es lo que el proyecto ya usa.

---

## 7. Dependencias externas nuevas: **pre-autorizadas, con licencia**

| Crate | Versión | Licencia | Para qué |
|---|---|---|---|
| `qrcode` | 0.14.1 | MIT OR Apache-2.0 | generar QR. **`default-features = false`** para no arrastrar `image` |
| `rqrr` | 0.10.1 | (MIT OR Apache-2.0) AND ISC | decodificar QR |
| `socket2` | 0.5.x | MIT OR Apache-2.0 | `IPV6_MULTICAST_IF` y join por índice de interfaz, que `std` no expone |
| `ur` | 0.5.2 | MIT | **opcional.** Sólo si eliges BC-UR en vez de LT propio |

**Ninguna otra sin ADR.** Declara el delta de `Cargo.lock` y `cargo audit` en verde.
El LT fountain, el módem serie y el render de QR en terminal **se escriben en el
árbol**: son 300, 200 y 150 líneas y no justifican una dependencia.

---

## 8. Red sin router

- **IPv6 link-local (`fe80::/10`, RFC 4291)** — *«All interfaces are required to have
  at least one Link-Local unicast address»*. **Siempre presente, cero espera.** Es el
  transporte más limpio. https://www.rfc-editor.org/rfc/rfc4291.html
- **Trampa de Rust, verificada:** `SocketAddrV6` sólo acepta el scope-id **como
  entero decimal**. **`"[fe80::1%eth0]:9000".parse()` FALLA.** Hace falta
  `if_nametoindex`. https://doc.rust-lang.org/std/net/struct.SocketAddrV6.html
- **El zone-id es local al nodo** (RFC 4007): la zona del emisor **no viaja**. Nunca
  se la enseñes al usuario ni la metas en un código de emparejamiento.
- **IPv4 link-local / APIPA (`169.254/16`, RFC 3927)** — habilitado por defecto en
  Windows. **Pero el cliente DHCP intenta primero y falla**: en la práctica son
  **decenas de segundos** antes de que ambos lados tengan IP. **La interfaz debe
  tolerar ~60 s de «sin dirección», mostrarlo y reintentar.**
  https://learn.microsoft.com/en-us/windows-server/troubleshoot/how-to-use-automatic-tcpip-addressing-without-a-dh
- **mDNS sí funciona sin router** (RFC 6762, 224.0.0.251 / FF02::FB, UDP 5353). **Pero
  Windows no ofrece un responder usable:** la especificación oficial de resolución de
  nombres de Windows ([MS-WPO]) lista DNS, NetBIOS/WINS, LLMNR, PNRP — **mDNS no
  está**. https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wpo/f00add7f-a321-4a5f-a5d8-1748e748cd44
  ⇒ **implementa mDNS/DNS-SD dentro del binario.** Es además lo que resuelve el
  musl-sin-NSS del §6.
- **Respaldo cuando mDNS falla:** broadcast a **255.255.255.255** (RFC 1122: *«will be
  received by every host on the connected physical network»*), broadcast de subred, y
  **ff02::1**. Dispara los tres, por **cada interfaz enumerada**, cada 1–2 s, y
  desduplica **por huella criptográfica, nunca por IP**.
- **Cable directo:** Auto-MDI-X está en IEEE 802.3 cláusula 40.4.4 — pero **es la
  cláusula de 1000BASE-T**. Una NIC vieja de sólo 10/100 —exactamente la del PC de la
  escena— **puede no tenerlo**. Documenta «si no enlaza, prueba un cable cruzado».

### Wi-Fi sin router: **no lo prometas**

- `netsh wlan set hostednetwork` es **de Windows 7 / Server 2008 R2**, la
  documentación de drivers de Wi-Fi Direct lo llama **deprecated**, y **depende del
  driver** — la mayoría de drivers WDI de Windows 10/11 reportan «Hosted network
  supported: No». Trátalo como no disponible.
- Wi-Fi Direct es WinRT y **Windows 10+** — adiós Windows 7.
- Mobile Hotspot está construido alrededor de **compartir una conexión existente** y
  **pide admin**.
- https://learn.microsoft.com/en-us/windows-hardware/drivers/partnerapps/wi-fi-direct (2024-09-27)

**Promete «cable directo o red compartida». Nunca «Wi-Fi sin router».**

---

## 9. El firewall, y la decisión de diseño que impone

- Default de Windows Firewall: **bloquear todo el inbound**, permitir todo el
  outbound. Un enlace sin gateway = red no identificada = **perfil Public**, el más
  restrictivo. Es exactamente el caso del cable directo.
  https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/
- El diálogo «permitir acceso» es un **MAY**, no un MUST (Firewall CSP,
  `DisableInboundNotifications`), crear reglas es operación de admin, y con
  `AllowLocalPolicyMerge=false` por GPO **ni siquiera una regla creada con admin
  sobrevive**.

> **Diseña asumiendo que NO podrás escuchar en un puerto inbound.** El protocolo
> debe permitir que **un solo lado** escuche y el otro sólo conecte hacia afuera. El
> código de emparejamiento lleva `IP:puerto` del que escucha, así que el permiso se
> concede **una vez, en la máquina donde el usuario sí manda**.

---

## 10. Windows 7 y Windows XP

- **El piso oficial de Rust es Windows 10.** Cambió en **Rust 1.78** (2024-05-02).
  https://blog.rust-lang.org/2024/02/26/Windows-7/
- **Un binario Rust stock literalmente no arranca en Windows 7.** Verificado
  compilando e inspeccionando imports: importa **estáticamente**
  `WaitOnAddress`/`WakeByAddress*` de `api-ms-win-core-synch-l1-2-0.dll`, que es
  **Windows 8 mínimo**. Al ser import estático el loader falla antes de `main`; no
  hay degradación, sólo «falta la DLL».
- **La solución existe y está verificada:** targets `x86_64-win7-windows-msvc` /
  `i686-win7-windows-*`, **Tier 3**, que sustituyen esas APIs por **SRW locks
  (Vista+)** y producen un import set limpio (ADVAPI32, KERNEL32, WS2_32, bcrypt,
  msvcrt, ntdll — todas existen en Win7). Requiere **nightly + `-Z build-std`**;
  `rustup target add` **no funciona** para Tier 3.
- **Windows XP: no hay target. Descartado.** Y ahí es donde entra el §5: **a una XP
  no se le lleva Qyro, se le lleva un archivo por serie desde HyperTerminal**, que ya
  está en la máquina. Eso es la fase 16 y es la respuesta correcta.

---

## 11. Terminal: qué se puede dibujar

- **Windows 7 conhost NO soporta VT/ANSI.** El mecanismo oficial de detección:
  `SetConsoleMode` devuelve 0 y `GetLastError` da `ERROR_INVALID_PARAMETER` ⇒
  *«gracefully degrade behavior and try again»*.
  https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences
  `ENABLE_VIRTUAL_TERMINAL_PROCESSING` = 0x0004, desde Windows 10 v1607.
- **Regla de diseño: todo lo que dibujes debe funcionar con sólo `\r` y `\n`.** VT es
  un upgrade opcional, nunca la base. Barra de progreso con `\r`: funciona en todo.
- **QR en terminal: sí, y en cp437 también.** Verificado carácter a carácter:

| Carácter | cp437 |
|---|---|
| `█` U+2588 | **0xDB** ✓ |
| `▀` U+2580 | **0xDF** ✓ |
| `▄` U+2584 | **0xDC** ✓ |
| `⠀` Braille U+2800 | **✗ no existe** |
| `▖` Quadrant U+2596 | **✗ no existe** |

  La técnica de **half-block** (`▀`/`▄`/espacio/`█`) **divide la altura del QR por
  dos** — crítico, porque un QR de v27 con módulos cuadrados no cabe en una consola
  de 25 líneas. **Funciona en un conhost legacy si emites los bytes OEM 0xDB/0xDF/
  0xDC**, no UTF-8. **Nunca uses Braille ni quadrant blocks.**
- **Code page:** `GetConsoleOutputCP()` primero. 437/850 ⇒ bytes OEM crudos. 65001 ⇒
  UTF-8. Si nada es fiable ⇒ ASCII `##`/`  ` o `--qr-png`. **Nunca llames a
  `chcp 65001` por el usuario**: con fuente raster —la de cmd.exe en Win7— no arregla
  nada y rompe la I/O.
  https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/chcp

---

## 12. Distribución: el riesgo más grande, y no es técnico

- **SmartScreen** para un binario sin firmar: diálogo «Windows protected your PC» y
  «Run anyway». La reputación tarda *«several weeks and hundreds of clean installs»*
  y **se pierde en cada release** salvo que firmes con la misma identidad.
  **EV ya no sirve:** Microsoft documenta que *«EV certificates no longer bypass
  SmartScreen»* y que pagar el premium por eso *«is no longer justified»*.
  https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation (2026-05-06)
- **Smart App Control (Windows 11) es cualitativamente peor:** bloquea *«unknown,
  unsigned code … by default»* y **«There is currently no way to bypass Smart App
  Control protection for individual apps.»**
  https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/overview
- **Mark of the Web** vive en un ADS de NTFS ⇒ **copiar el .exe a un USB FAT32/exFAT
  lo elimina**. La ruta USB es la más fiable en máquinas viejas y bloqueadas.
- **AppLocker con reglas por defecto** permite por *path* (`%WINDIR%`,
  `%PROGRAMFILES%`) ⇒ `%USERPROFILE%\Downloads\qyro.exe` **queda fuera**. En esa
  máquina no hay nada que el binario pueda hacer. **Detéctalo y di la verdad.**

**Consecuencia para el plan: firmar cuesta dinero, y el dinero es una de las cuatro
excepciones. No lo decidas tú.** Prepara todo lo demás y deja la decisión escrita.

---

## 13. Nota sobre la versión de Rust

El repositorio fija **1.88.0** en `rust-toolchain.toml`. El stable vigente al
2026-08-17 es **1.97.1** (publicado 2026-07-16,
https://blog.rust-lang.org/2026/07/16/Rust-1.97.1/). No es un defecto —fijar la
toolchain es correcto— pero la **fase 17 necesita `nightly` de todos modos** para
`-Z build-std`, así que ahí toca decidir si se sube el pin. **Decide tú y escribe por
qué.**
