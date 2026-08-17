# Estado actual — dónde se corta

**2026-08-18** · rama `claude/qyro-cerrar-cadena-12` · último commit de esta
sesión abajo.

> Este archivo dice **dónde se corta y qué es lo siguiente**, para que quien siga
> no tenga que reconstruirlo leyendo commits. Se actualiza al cerrar cada paso.

---

## 1. Lo que se cerró en esta sesión

**El gate rojo, primero.** `check_docs_consistency` estaba en rojo en `5459a64`
—*«Stale verified commit: HEAD is 11 commits ahead»*— y se arregló actualizando
el ancla de `STATUS.md` **y volviendo a correr la puerta sobre el commit
resultante**, que es la comprobación 16 aplicada a sí misma.

**Fase 15 — HECHA.** Informe en `docs/reports/fase-15-canal-optico.md`, puerta
corrida en `dc993d3`.

| Commit | Qué |
|---|---|
| `3633ec0` | `qyro_fountain`: Luby Transform, cero dependencias, generador congelado porque es formato de cable |
| `0125f2e` | `qyro qr` y `qyro beam`: medios bloques, invertido a propósito, 5 FPS |
| `dc993d3` | La vuelta completa: un decodificador real lee lo que dibuja la terminal |
| `ab947ab` | El informe |

**Lo que corrigió el uso y no el diseño:** el consejo de tamaño mentía (decía 37
columnas para un código de 41 — un consejo corto es peor que ninguno); un archivo
de 51 bytes dibujaba una v27 entera, el código más difícil de escanear para el
payload más pequeño; y `DrawError` no se ganaba el sueldo, porque su único
consumidor lo imprimía con `{:?}` y «TooLong» le llegaba a una persona como la
palabra TooLong.

**El receptor de CI se hizo, y no como estaba planteado.** No un directorio de
imágenes: rasterizando en memoria, con `rqrr` de dev-dependency. Un fixture
caduca y falla como «se rompió el renderizador»; esto dibuja lo que dibuja
`qyro beam`, en el momento, y lo vuelve a leer. `zune-jpeg` no hizo falta y la
trampa del MJPEG sin DHT no llega a existir.

**Coste medido:** +67 KB en el binario (1 306 624 → 1 373 696). `rqrr` no pone
ninguno: no viaja.

---

**Fase 14 — HECHA.** Informe en `docs/reports/fase-14-sin-router.md`, puerta
corrida en `07278ff`, el commit que el informe nombra.

| Commit | Qué |
|---|---|
| `f81c15a` | La cuenta atrás de APIPA (`qyro_session/src/link.rs`) y la trampa de `SocketAddrV6` |
| `b89a89a` | **ADR-0043 enmienda 1**, en commit propio antes del código |
| `dab9fa3` | El beacon por interfaz con `socket2`, y el puerto colapsado a una definición |
| `07278ff` | El lado Dart de `dev.qyro/discovery` y su llamante de producción |
| `f50ab2c` | El informe de la fase 14 |

**Dos hallazgos que no buscaba, los dos con cifra:**

- **D9** — `mdns-sd` casi dobla el binario: **666 624 → 1 295 872 bytes** al
  llegar `qyro find`. **+614 KB**, diez veces los 63 KB que este taller discutió
  para conservar el desenrollado de pila. El beacon propio hace lo mismo por
  **8 KB**. La ADR-0043 §7 citaba un presupuesto de 750–950 KB que el binario ya
  no cumple; la enmienda 1 lo corrige con la medida. **No se toca hoy** — lo
  decide la fase 19 con red de verdad.
- **D10** — el puerto que ADR-0041 congeló estaba escrito **dos veces y en
  ningún sitio del motor**, bajo un comentario que decía «no re-derivado: dos
  copias son dos puertos» siendo la segunda copia. Cerrado: `qyro_net::QYRO_PORT`
  es el original y una guarda lee el `.dart` y falla si se separan — **vista
  fallar a propósito** antes de darla por buena.

---

## 2. Lo siguiente, en orden

```
16 → 21 → 22 → 17 → 18 → 19 → 20 → 23
```

- **16 — canal serie.** Sin abrir. **Es lo siguiente.**
- **21 — las dos caras se hablan.** GUI ↔ CLI. Hoy nadie las ha puesto una
  contra la otra ni una vez.
- **22 — lo que la gente hace de verdad.** Carpetas, tamaño, interrupción.

---

## 3. Lo que sigue en blanco, y sigue en blanco a propósito

- **Cero pruebas en hardware físico.** Dos procesos en `127.0.0.1` no son dos
  máquinas. Que dos aparatos se encuentren por un cable **no está verificado**.
- **`NsdManager` no está ejercitado.** Las pruebas Dart usan un `MethodChannel`
  falso: prueban el lado Dart, no Android.
- **Ninguna cámara ha leído un QR de Qyro.** La vuelta completa la hace un
  decodificador sobre píxeles perfectos. Desenfoque, obturador rodante, moiré,
  brillo y pantalla en ángulo son fase 19.
- **El teléfono no acumula frames todavía.** El motor los produce y son legibles;
  el lado Android que los junta no existe.
- **La reanudación del canal óptico no existe** (D11). ADR-0044 §5 la exige para
  sesiones largas; el límite de 20 MB es lo que hoy impide llegar a una.
- **La GUI de escritorio no tiene descubrimiento.** No hay símbolo en la
  frontera C. Lo dice con una frase, no con una lista vacía.
- El binario **no arranca en Windows 7** (`api-ms-win-core-synch-l1-2-0.dll`,
  fase 17).

---

## 4. Cuatro trampas de este entorno, para no repetirlas

1. **Heredocs de bash** destrozan `\n` y `\t` antes de que Python los vea. Usa
   `chr(92)`, escribe el script con la herramienta Write, o usa Edit.
2. **`git commit -m @'...'@` en PowerShell** se rompe si el mensaje lleva
   comillas: escribe el mensaje a un archivo y usa `git commit -F`.
3. **Flutter no está en el PATH.** Está en `D:\flutter\bin`.
4. **`verify_static.ps1` exige `-Binary`**, y el binario de la tubería es
   `target/x86_64-pc-windows-msvc/release/qyro.exe` — no `target/release`, que
   se compila con otro perfil y pesa distinto.

---

## 5. La regla que más valor dio, otra vez

**Cuando una guarda te dice que estás equivocado, tiene razón más veces de las
que crees.** En esta sesión pararon tres y acertaron las tres: el registro de
`beacon.rs`, `clippy` sobre un `assert!` entre constantes que se optimiza y no
prueba nada, y —la mejor— `qyro_session_re_exports_nothing_it_does_not_own`
rechazando `pub use qyro_net::QYRO_PORT`, porque todo lo que la fachada republica
se vuelve nombrable desde `qyro_ffi` y una excepción juzgada inofensiva de una en
una es cómo llega la primera peligrosa.

Y una cuarta cosa lo dijo sin ser una guarda: **el enlazador**. Con `beacon.rs`
escrito y sin llamante el binario no cambió ni un byte. Una capacidad sin
llamante no se envía, se compila.
