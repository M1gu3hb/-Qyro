# FASE 13 — `qyro` en la terminal: el binario único

> **Es la fase que resuelve el problema del que nació el proyecto.** Lee `R7` §2
> antes de escribir una línea.

---

## 1. Por qué existe esta fase

La GUI de Flutter no puede correr en la máquina de la escena. Esa máquina no puede
instalar nada, puede que no tenga una GPU que Flutter acepte, y puede que ni
siquiera tenga Windows 10. **Lo que sí tiene, con total seguridad, es una terminal.**

El motor de Rust ya hace todo lo difícil. Lo que falta es **una segunda cara**:

```
qyro
┌────────────────────────────────────────┐
│   ██████  QYRO                          │
│   transferencia directa, sin nube       │
│                                         │
│   1) Enviar un archivo                  │
│   2) Recibir un archivo                 │
│   3) Este aparato                       │
│   q) Salir                              │
└────────────────────────────────────────┘
>
```

Un binario. Sin instalación. Sin administrador. Sin dependencias. **~800 KB.**

---

## 2. La decisión que hay que congelar antes del código

`docs/adr/ADR-00XX-cli.md`. Decide y escribe:

1. **Dónde vive.** Un crate nuevo `rust/crates/qyro_cli` en el workspace, que
   depende de `qyro_session` **directamente y no del FFI**. El FFI existe para que
   Dart cruce una frontera de lenguaje; el CLI es Rust hablando con Rust y no tiene
   que pagar ese peaje. **Consecuencia que hay que aceptar y escribir: habrá dos
   consumidores del motor, y una capacidad no está hecha hasta que los dos la
   alcanzan o hasta que se declara de uno solo.**
2. **Interactivo por defecto, con banderas para scripts.** `qyro` sin argumentos abre
   el menú. `qyro send <archivo> --to <código>` y `qyro recv --out <dir>` hacen lo
   mismo sin preguntar. Las dos rutas ejecutan **el mismo código**, no dos.
3. **Qué pasa cuando no hay terminal interactiva** (pipe, redirección, servicio):
   detectar y **negarse a abrir un menú que nadie puede contestar**, con un mensaje
   que diga qué bandera usar.
4. **El nombre del ejecutable y el del paquete.** Decide tú.
5. **Idiomas.** El motor ya tiene español e inglés en la GUI. Decide si el CLI los
   comparte —y cómo, porque las cadenas de Flutter viven en `.arb`— o si tiene su
   propia tabla. **Escribe por qué.** Una tabla duplicada que se desincroniza es peor
   que una sola en el idioma equivocado.

---

## 3. Entregables

### 3.1 — El crate y el perfil de build

`rust/crates/qyro_cli`, `#![forbid(unsafe_code)]`, y el perfil **ya medido** de `R8`
§6:

```toml
[profile.release]
opt-level = "s"      # medido: gana a "z" por ~18 KB
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

**Verifica el tamaño y escríbelo en el informe.** La cifra esperada es **750–950 KB**
por target. Si te sale 3 MB, algo arrastró una dependencia que no debía.

### 3.2 — Linking estático, para que no pida nada

- Windows: `-C target-feature=+crt-static` en
  `[target.<triple>] rustflags` de `.cargo/config.toml`. **Nunca en `[build]`** —
  `RUSTFLAGS` del entorno lo pisa en silencio en CI. **Siempre `--target` explícito**,
  o los build scripts reciben los flags y se rompen.
- Linux: `x86_64-unknown-linux-musl`, estático por defecto.
- **Comprobación por código de salida, no por vista:** un paso que inspeccione los
  imports del `.exe` (`dumpbin /imports` o equivalente) y **falle** si aparece
  `vcruntime140.dll`. `R8` §6 documenta un caso medido en el que el flag fue
  **ignorado en silencio** y produjo un binario byte-idéntico. **No asumas.**

### 3.3 — La pantalla de arranque

Logo, versión, y el menú. Con una regla dura de `R8` §11:

> **Todo lo que dibujes debe funcionar con sólo `\r` y `\n`.**

- Intenta habilitar VT con `SetConsoleMode(ENABLE_VIRTUAL_TERMINAL_PROCESSING)`.
  Si devuelve 0 con `ERROR_INVALID_PARAMETER` —que es lo que hará el conhost de
  Windows 7— **degrada y sigue**. Ése es el mecanismo oficial de detección de
  Microsoft, no un truco.
- Colores: con VT, secuencias ANSI. Sin VT, `SetConsoleTextAttribute` (16 colores) o
  ninguno. **Ninguno es una respuesta aceptable; un binario que no arranca no lo es.**
- Barra de progreso con `\r`. Funciona en todas partes, incluida una XP.
- **Prueba mecanizada:** que el render del menú y de la barra **no emita ni un byte
  de escape** cuando VT está deshabilitado. Con su control: que **sí** los emita
  cuando está habilitado. Sin las dos direcciones, la prueba no distingue nada.

### 3.4 — Los tres flujos

1. **Enviar.** Elegir archivo (ruta como argumento, o un explorador de texto simple —
   **no** un selector gráfico, esa máquina no lo tiene), elegir destino, ver la
   huella del otro extremo **antes** de mandar nada, confirmar, progreso, resultado.
2. **Recibir.** Elegir dónde guardar, ligar, **enseñar el código de emparejamiento
   completo en pantalla**, esperar, **enseñar quién es y qué trae antes de aceptar**,
   aceptar o rechazar, progreso, y **decir dónde quedaron los archivos**.
3. **Este aparato.** La huella propia en el formato agrupado que se lee en voz alta,
   las direcciones de todas las interfaces útiles, y el estado del canal.

**ADR-0036 §1 sigue mandando: nada se acepta solo, nunca.** No hay `--yes` que salte
la confirmación de una transferencia entrante de un desconocido. Si añades una
bandera para scripts, que **exija la huella esperada** en la línea de comandos: eso
no es aceptar a ciegas, es haber decidido antes.

### 3.5 — La huella, y el error que no hay que repetir

**La huella la formatea el core, nunca la interfaz** (ADR-0035 §4). Si el CLI la
formateara por su cuenta, dos aparatos podrían renderizar la misma huella distinta y
compararla en voz alta dejaría de significar algo. Usa
`HumanFingerprint::to_grouped_hex`. **Exponlo, no lo reescribas.**

### 3.6 — El pipeline de build

Un workflow que produzca, con hash:

| Target | Tier | Notas |
|---|---|---|
| `x86_64-pc-windows-msvc` | 1 | `+crt-static` |
| `i686-pc-windows-msvc` | 1 | `+crt-static`. 32 bits, Pentium 4 |
| `x86_64-unknown-linux-musl` | 2 | estático |
| `i686-unknown-linux-musl` | 2 | estático, 32 bits |

Windows 7 es **fase 17** y necesita nightly. No lo intentes aquí.

---

## 4. Las trampas, con su fuente

- **Cero crates que compilen C.** Medido en `R8` §6: `blake3` por defecto rompe el
  build con `undefined reference to blake3_compress_in_place_sse41`. Esto descarta
  `ring` y `openssl` para siempre.
- **musl no tiene NSS ni resolver decente.** **Da igual, y es una regla, no una
  excusa: el CLI no resuelve nombres nunca.** El descubrimiento devuelve IPs
  literales y se conecta a IPs literales. Si dejas que el usuario escriba
  `pc-de-juan.local`, no funcionará ni en musl ni en Windows (`R8` §8).
- **`opt-level="z"` por su cuenta agranda el binario.** Medido. Las palancas sólo
  pagan combinadas.
- **La consola de Windows con fuente raster ignora `SetConsoleOutputCP`.** No llames
  a `chcp 65001` por el usuario.

---

## 5. La prueba que cierra la fase

> **El binario, copiado a una máquina limpia sin toolchain de Rust ni runtime de
> nada, se ejecuta y transfiere un archivo a otra instancia de sí mismo, verificado
> por SHA-256 en destino.**

En esta máquina eso se aproxima con un **contenedor mínimo sin `glibc` ni
`vcruntime`** para el binario musl, y con la comprobación de imports del §3.2 para
el de Windows. **Escribe la clase de evidencia con precisión** — «ejecutado en un
contenedor sin libc, no en una máquina física» — y no la subas de categoría.

**Control:** el mismo binario, compilado **sin** `+crt-static`, debe **fallar** esa
comprobación de imports. Una comprobación que nunca se ha visto fallar no prueba
nada.

---

## 6. La puerta

Quince comprobaciones. Y en la comprobación 14, la tabla de llamantes debe cubrir
**cada símbolo de `qyro_session` que el CLI usa**, porque a partir de esta fase el
motor tiene dos consumidores y una capacidad puede estar viva en uno y muerta en el
otro — que es exactamente cómo se rompió la v1.0.

---

## 7. Lo que NO hay que hacer

- **No un TUI a pantalla completa con `ratatui`.** Es tentador y es la trampa: la
  ruta de degradación a Console API para Windows 7 no está garantizada, el binario
  crece, y el requisito es «sale el logo y eliges 1 o 2», no un panel.
- **No toques la GUI de Flutter.** Sigue siendo la cara principal en Android.
- **No dupliques lógica del motor.** Si algo hace falta y no está expuesto, **expónlo
  en `qyro_session`**, no lo reimplementes en el CLI.
- **No metas el QR todavía.** Fase 15.
