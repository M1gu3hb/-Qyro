# ADR-0042 — `qyro` en la terminal: la segunda cara del motor

- **Estado:** **congelada** antes de una sola línea de código.
- **Fecha:** 2026-08-17
- **Fase:** 13
- **Depende de:** ADR-0028 (transporte), ADR-0035 (emparejamiento), ADR-0036
  (política de producto), ADR-0040 (identidad), ADR-0041 (primer contacto).
- **Gobernada por:** `R7` §2 — *«en su terminal pongo el comando, listo, sale el
  logo, le das en recibir o enviar»*. Esa frase es el requisito.

---

## 1. Qué resuelve, y por qué no lo resuelve la GUI

La máquina de la escena **no puede instalar nada, puede no tener GPU que Flutter
acepte, y puede no tener Windows 10**. Lo único seguro es que tiene una terminal.

El motor ya hace lo difícil. Falta una **segunda cara**, y esta ADR la fija.

---

## 2. Decisión 1 — `qyro_cli` depende de `qyro_session`, **no del FFI**

Crate nuevo `rust/crates/qyro_cli`, `#![forbid(unsafe_code)]`, que llama a
`qyro_session` directamente.

**Por qué no por el FFI.** La frontera C existe para que Dart cruce un límite de
lenguaje: enteros y texto por búfer prestado, nada que cruce un tipo. El CLI es
Rust hablando con Rust y **pagar ese peaje no compra nada** — perdería los tipos,
ganaría `unsafe`, y ataría el CLI a una superficie diseñada para otra cosa.

### La consecuencia, aceptada y escrita, que es la parte importante

> **El motor pasa a tener dos consumidores, y una capacidad no está hecha hasta
> que los dos la alcanzan o hasta que se declara de uno solo.**

Esto es exactamente cómo se rompió la v1.0: `Session::finish` estaba viva para
`qyro_net_smoke` y muerta para Dart, y nadie lo miró porque «el motor lo hace».
**A partir de aquí, la comprobación 14 se aplica por consumidor**, y la tabla del
informe de fase lleva una columna más: GUI, CLI, o ambas.

---

## 3. Decisión 2 — interactivo por defecto, banderas para scripts, **un solo camino**

```
qyro                              menú
qyro send <archivo> --to <código> sin preguntar
qyro recv --out <dir>             sin preguntar
qyro whoami                       huella y direcciones
```

Las dos rutas **ejecutan el mismo código**. El menú no es un programa distinto:
recoge argumentos y llama a la misma función que la bandera. Dos caminos que
hacen lo mismo son dos caminos que divergen, y este proyecto ya sabe qué pasa
entonces.

### Sin terminal interactiva: se niega, y dice qué bandera usar

Si la entrada no es un TTY —pipe, redirección, servicio— **abrir un menú que
nadie puede contestar es colgarse**. Se detecta, se sale con código distinto de
cero, y el mensaje nombra la bandera que hacía falta.

---

## 4. Decisión 3 — **nada se acepta solo, tampoco aquí**

ADR-0036 §1 no tiene una excepción para la terminal. **No habrá `--yes`** que
salte la confirmación de una transferencia entrante de un desconocido.

Lo que sí existe es `--expect <huella>`: si la huella del otro extremo coincide
con la que se escribió en la línea de comandos, la transferencia sigue sin
preguntar. **Eso no es aceptar a ciegas: es haber decidido antes**, y una huella
equivocada es un rechazo, no una pregunta.

---

## 5. Decisión 4 — el nombre

Ejecutable **`qyro`**. Crate **`qyro_cli`**, siguiendo la convención del
workspace. En Windows, `qyro.exe`.

No `qyro-cli` ni `qyroctl`: la frase del propietario es *«en su terminal pongo el
comando»*, y el comando más corto que se puede teclear mal es el que se teclea
bien.

---

## 6. Decisión 5 — **el CLI es sólo en inglés en la v1.x**, y por qué

Las cadenas de la GUI viven en `.arb` de Flutter y las consume código generado
por `flutter gen-l10n`. **Un crate de Rust no puede leer eso** sin un generador
propio que convierta `.arb` a Rust en tiempo de build.

Tres opciones y el argumento de cada una:

| Opción | Coste | Riesgo |
|---|---|---|
| Tabla propia en el CLI | Baja | **Dos tablas que se desincronizan.** La fase 12 encontró una frase que decía lo contrario en dos sitios; ésta es la misma trampa multiplicada por dos idiomas |
| Generador `.arb` → Rust | ~120 líneas y un `build.rs` | Un `build.rs` es código que corre en cada compilación, en cuatro targets, incluido musl |
| **Sólo inglés** | Cero | Una persona hispanohablante lee un menú en inglés |

**Se elige sólo inglés.** Una línea de por qué: *una tabla duplicada que se
desincroniza es peor que una sola en el idioma equivocado*, y el propio documento
de la fase lo dice.

**Y se dice en la interfaz, no se disimula:** la pantalla de arranque lleva una
línea que reconoce que la GUI está en los dos idiomas y el CLI todavía no. Cuando
haga falta, el camino es el generador, no la segunda tabla.

---

## 7. Decisión 6 — todo se dibuja con `\r` y `\n`

`R8` §11, y es una regla dura: **el conhost de Windows 7 no soporta VT.**

- Se intenta `SetConsoleMode(ENABLE_VIRTUAL_TERMINAL_PROCESSING)`. Si devuelve 0
  con `ERROR_INVALID_PARAMETER`, **se degrada y se sigue** — es el mecanismo de
  detección que Microsoft documenta, no un truco.
- Con VT: secuencias ANSI. Sin VT: **sin color**. Ninguno es una respuesta
  aceptable; un binario que no arranca no lo es.
- La barra de progreso es `\r`. Funciona hasta en una XP.
- **Nunca `chcp 65001`** por el usuario: con fuente raster no arregla nada y
  rompe la I/O.

**Prueba mecanizada en las dos direcciones**: sin VT el render **no emite ni un
byte de escape**; con VT **sí**. Una prueba en una sola dirección no distingue un
render degradado de un render vacío.

---

## 8. Decisión 7 — la huella la formatea el core

`HumanFingerprint::to_grouped_hex`, expuesto, **nunca reimplementado**
(ADR-0035 §4). Si el CLI formateara por su cuenta, dos aparatos podrían
renderizar la misma huella distinta y compararla en voz alta dejaría de
significar algo.

Si algo hace falta y no está expuesto, **se expone en `qyro_session`**. No se
duplica.

---

## 9. Decisión 8 — el CLI **no resuelve nombres, nunca**

`R8` §8. musl no trae NSS, y da igual: se conecta a IP literales y el
descubrimiento devuelve IP literales. Si alguien teclea `pc-de-juan.local` no va
a funcionar ni en musl ni en Windows, así que **no se acepta**: se rechaza con un
mensaje que dice que hace falta una IP o un código de emparejamiento.

Es una regla, no una limitación que haya que disimular.

---

## 10. El perfil de build, ya medido

`R8` §6, y no se vuelve a investigar:

```toml
[profile.release]
opt-level = "s"      # medido: gana a "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

**Cifra esperada: 750–950 KB por target.** Si sale 3 MB, algo arrastró una
dependencia que no debía, y eso es el hallazgo, no el tamaño.

**Cero crates que compilen C.** Medido: `blake3` por defecto rompe el build de
win7. Descarta `ring` y `openssl` para siempre.

### Estático, y comprobado por código de salida

- Windows: `-C target-feature=+crt-static` en `[target.<triple>] rustflags` de
  `.cargo/config.toml`. **Nunca en `[build]`** — `RUSTFLAGS` del entorno lo pisa
  en silencio. Siempre `--target` explícito.
- Linux: `x86_64-unknown-linux-musl`, estático por defecto.
- **Un paso que inspeccione los imports y falle si aparece `vcruntime140.dll`**,
  con su control: el mismo binario sin el flag **debe** fallar esa comprobación.
  `R8` §6 documenta un caso medido en el que el flag fue **ignorado en silencio**
  y produjo un binario byte-idéntico. No se asume.

---

## 11. Lo que esta ADR NO decide

- **El QR.** Fase 15.
- **El puerto serie.** Fase 16.
- **Windows 7.** Fase 17, y necesita nightly con `-Z build-std`.
- **Un TUI a pantalla completa.** Descartado, ver §12.

---

## 12. Alternativas descartadas

**`ratatui` y un panel a pantalla completa.** Es lo bonito y es la trampa: la
degradación a Console API para Windows 7 no está garantizada, el binario crece, y
el requisito es *«sale el logo y eliges 1 o 2»*, no un panel. R7 §1: alcanzar más
máquinas antes que mejor aspecto.

**Que el CLI hable por el FFI.** Perdería los tipos y ganaría `unsafe` para no
comprar nada. La frontera C existe por Dart.

**Reimplementar en el CLI lo que no está expuesto.** Es cómo nace una segunda
verdad. Se expone en `qyro_session` o no se hace.
