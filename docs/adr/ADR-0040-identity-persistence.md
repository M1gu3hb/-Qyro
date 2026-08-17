# ADR-0040 — La identidad persiste, y el Keystore se aplaza

- **Estado:** **congelada** antes de una sola línea de código.
- **Fecha:** 2026-08-16
- **Fase:** 11
- **Sustituye el mecanismo de ADR-0037 §3** y lo dice en su propia enmienda.
- **Depende de:** ADR-0020 (identidad), ADR-0024 (DPAPI), ADR-0025 (Keystore),
  ADR-0031 (confianza), ADR-0032 (FFI), ADR-0035 (emparejamiento).

---

## 1. Qué se descubrió, y por qué esta ADR existe

**La aplicación no tiene identidad estable.** `qyro_session::session::new_identity`
llama a `DeviceIdentity::generate()` y los tres constructores de `Session` lo
llaman sin condición. **Cada transferencia estrena un par de claves.**

Consecuencias, todas medidas y ninguna supuesta:

| Lo que el producto promete | Lo que hace |
|---|---|
| «Compara la huella en voz alta una vez» | La huella cambia en cada transferencia |
| «Si la clave cambia, Qyro se niega» | `TrustBook` arranca vacío en cada proceso; ningún peer llega a ser conocido |
| «Teclea el código de emparejamiento» | `NativeTransferService.ownPairingString()` devuelve `null` **siempre**: la aplicación nunca enseña su propio código |
| «La identidad sobrevive al reinicio» (fase 06) | Cierto del mecanismo, falso de la aplicación |

Y la evidencia que `STATUS.md` citaba para Windows —«Persist an identity across
two separate process invocations»— ejecuta `qyro_store_smoke.exe`, un arnés cuya
propia cabecera dice **«Never shipped»** y que construye
`qyro_win_dpapi::WindowsIdentityStore` directamente, **sin pasar por
`qyro_session` ni por `qyro_ffi`**. Probaba DPAPI. No probaba el producto.

**Esto no es deuda de calidad. Es la propiedad de seguridad principal, ausente.**

---

## 2. La decisión, en una frase

**El proceso abre una identidad antes de la primera sesión, o no hay sesión.**

`Session` deja de generar. Un módulo nuevo, `qyro_session::identity`, carga la
identidad de un archivo que el llamante nombra, o crea una si no hay ninguna, y
la guarda en un `OnceLock` de proceso. Los tres constructores piden
`identity::current()?`.

**Si nadie abrió una identidad, los tres constructores se niegan.** No generan
una de repuesto. Un fallback que genera es este mismo defecto con más código
alrededor.

---

## 3. Tres símbolos nuevos, y ninguno cruza un tipo

ADR-0032 §5 sigue rigiendo: `i32` de retorno, valores por búfer prestado,
longitudes explícitas.

```c
int32_t qyro_identity_open_blocking(const uint8_t *path, size_t path_len);

int32_t qyro_identity_set_wrapper(
    int32_t (*wrap)(uintptr_t ctx, const uint8_t *in, size_t in_len,
                    uint8_t *out, size_t out_cap, size_t *out_len),
    int32_t (*unwrap)(uintptr_t ctx, const uint8_t *in, size_t in_len,
                      uint8_t *out, size_t out_cap, size_t *out_len),
    uintptr_t context);

int32_t qyro_identity_fingerprint(uint8_t *out, size_t capacity, size_t *out_len);
```

La firma de `wrap`/`unwrap` **ya existe**: es `qyro_session::WrapFn`
(`bridged_wrapper.rs`). No se inventa nada. Se toma como `Option<QyroWrapFn>` en
Rust y un puntero nulo es `QYRO_ERR_BAD_ARGUMENT`, porque llamar a un puntero a
función nulo es comportamiento indefinido y esa deuda es de la frontera.

`qyro_identity_set_wrapper` **no** va tras `cfg(target_os = "android")`. Un
envoltorio falso instalable en Windows y en Linux es lo que permite ejercitar el
camino del puente en CI — y es exactamente lo que habría cazado el defecto de §5.

**La superficie pasa de veinte a veintitrés.** Veinte, no diecinueve: ADR-0032
enmienda 1 dijo diecinueve, todos los informes lo repitieron, y nadie contó
(QYR-0352). Ahora lo cuenta
`the_c_surface_is_exactly_the_symbols_that_are_written_down`.

**Ningún secreto cruza a Dart.** Dart maneja una ruta y un código de retorno. El
blob envuelto y la semilla en claro se quedan debajo.

---

## 4. Dónde vive el archivo, y por qué lo nombra el llamante

| Plataforma | Ruta |
|---|---|
| Windows | `%LOCALAPPDATA%\Qyro\identity.bin` — la que ADR-0024 §2 decidió |
| Android | `context.getNoBackupFilesDir()/identity.qyro` — la que ADR-0025 §3.4 decidió, precisamente para que la propiedad no dependa de `allowBackup` |

**Rust no adivina rutas.** Un solo camino de código en las dos plataformas, y la
diferencia vive del lado de Dart, que ya sienta ese precedente con
`defaultDestination()`. Es además lo que hace posible la prueba entre procesos:
un test apunta a un directorio temporal.

*(El test instrumentado actual usa `context.filesDir` y afirma sobre `/files/`,
que contradice a su propia ADR. Cambia.)*

---

## 5. El defecto que bloqueaba el puente, y que nadie podía ver

`qyro_identity_store::blob::KNOWN_WRAPS` es `[1, 2]` —DPAPI y Keystore— y
`blob::parse` rechaza cualquier otro byte. `BridgedWrapper::wrap_id()` devuelve
**3**, como fijó ADR-0037 §2.

**Un blob sellado por el puente no se puede volver a abrir.** `seal_identity`
escribe una cabecera que `open_identity` rechaza con `UnsupportedWrap { found: 3 }`.

Nadie lo cazó porque `bridged_wrapper_contract.rs` ejercita el envoltorio contra
`entropy_for` y **nunca a través de `seal_identity`/`open_identity`**. Es la forma
exacta de todos los defectos de este proyecto: dos piezas correctas por separado
y una costura que ninguna prueba cruzaba.

Se añade `WRAP_BRIDGED = 3` y su prueba de ida y vuelta.

---

## 6. Android: el mecanismo de ADR-0037 no puede funcionar

ADR-0037 §3 dice «Dart las registra al arrancar». **No es implementable**, y basta
cualquiera de estas cuatro:

1. **Kotlin no puede producir un puntero a función C.** No existe en la JVM.
2. Dart sí puede —`Pointer.fromFunction`, `NativeCallable.isolateLocal`— pero
   ambos son **afines al isolate**: sólo se pueden invocar desde el hilo del
   isolate que los creó. Rust llama a `wrap` dentro de
   `qyro_session_open_*_blocking`, y el emisor corre eso dentro de
   `Isolate.run` — otro isolate, otro hilo. Invocar ahí aborta el proceso; no
   devuelve un código de error.
3. `NativeCallable.listener` es la forma segura entre hilos y es **asíncrona y
   devuelve `Void`**. Un envoltorio tiene que devolver bytes, ahora. Excluido por
   estructura.
4. Aun con un puntero válido, la callback tendría que alcanzar Keystore, y la
   única vía desde Dart es `MethodChannel.invokeMethod`, que completa por el
   bucle de eventos. Dentro de una callback síncrona anidada en una llamada FFI
   bloqueante, el isolate no está bombeando su bucle. Ese `Future` **no puede
   completarse nunca**. Es un interbloqueo garantizado, no una carrera.

Lo que sí funciona es un **shim en C compilado por el NDK** que haga JNI. No
existe en este repositorio: no hay `CMakeLists.txt`, no hay `externalNativeBuild`,
no hay un solo `.c`. `libqyro_ffi.so` lo copia CI a `jniLibs/`, no lo construye
Gradle.

---

## 7. La decisión que duele: **la v1.0 sale sin Keystore**

**Etapa A, que es lo que se implementa ahora.** La identidad persiste en las dos
plataformas:

- **Windows:** envuelta con DPAPI de ámbito de usuario (ADR-0024, sin cambios).
- **Android:** en `getNoBackupFilesDir()`, protegida por **el sandbox por UID de
  Linux** y por nada más.

**Etapa B, aplazada:** el shim JNI instala el envoltorio de Keystore, y lo único
que cambia es qué envoltorio elige `identity::open`.

### Por qué se aplaza, y qué se pierde

Escribir hoy `qyro_keystore_bridge.c` significa: `AttachCurrentThread` sobre un
hilo de un isolate que la JVM no ha visto nunca; desatar un hilo que la JVM posee
rompe Flutter; cada llamada JNI necesita su `ExceptionCheck` o la siguiente es
comportamiento indefinido. Es el archivo con más probabilidad de producir un
fallo **que sólo aparece en un aparato**.

Y este proyecto **no puede ejecutar nada en un aparato**. Un emulador tiene un
Keystore por software, así que ni siquiera prueba la parte que importa.

**Enviar un shim JNI que nadie puede validar es peor que enviar el sandbox y
decirlo.** El sandbox por UID es una defensa real —otra aplicación no lee los
archivos privados de ésta sin root— y es *más débil* que Keystore de una forma
concreta y escribible:

> Con Keystore, un atacante con root necesita además el TEE. Con el sandbox, root
> es suficiente.

Eso va en `THREAT_MODEL.md` con esas palabras, no como nota al pie.

### Lo que esto NO cambia

`allowBackup=false`, `fullBackupContent=false` y `dataExtractionRules` con
secciones vacías siguen puestos (QYR-0349), y `getNoBackupFilesDir()` está
excluido del backup **por definición del directorio**, no por una promesa. El
blob no sale del aparato en ninguna de las dos etapas.

---

## 8. Alternativas descartadas

**Generar y no persistir, y quitar la confianza de la interfaz.** Sería coherente
—dejaría de mentir— y convertiría a Qyro en «manda un archivo a quien sea», que
es otro producto. La comparación de huella es la razón de que este exista.

**Persistir en `SharedPreferences`.** Prohibido desde la primera sesión de este
proyecto, y con razón: es texto plano legible por cualquier proceso con root, y
entra en las copias de seguridad salvo que se excluya a mano.

**Escribir el shim JNI igualmente y marcarlo «no probado».** Es la opción que
parece más completa y es la que rompe la regla: código que no se puede ejecutar
ni una vez no es una función, es una afirmación. La etapa B se hace con un
teléfono delante.

**Hacer que Dart genere y guarde la identidad.** La clave privada cruzaría a
Dart. No, nunca, y es la única regla de este proyecto que no ha tenido enmiendas.

---

## 9. Qué prueba CI, y qué no

**Prueba** —y hoy no lo prueba nada— que una huella escrita por un proceso es la
huella que un segundo proceso reporta **a través de `qyro_session`**, en Windows;
y que **un blob ilegible se rechaza en vez de sustituirse**, que es la propiedad
sobre la que descansa todo el modelo y no necesita hardware.

**No prueba** un TEE de verdad, ni un reinicio, ni StrongBox, ni DPAPI contra un
perfil con dominio o móvil, ni que el servicio de copias de Google respete
`getNoBackupFilesDir()`. Nada de eso se afirma en ningún sitio.

---

## 10. El orden, y por qué importa

**La persistencia aterriza antes que el cableado de confianza en Dart, o con él,
nunca después.** Hoy Dart fija `trust: QyroPeerTrust.known` a mano. Cablear el
veredicto real **antes** de que la identidad persista fabricaría un `Changed`
falso en cada reinicio de un peer legítimo — y la pantalla quita el botón de
aceptar en ese estado, sin salida.

Commits, cada uno el suyo: esta ADR y las enmiendas de ADR-0032 y ADR-0037 →
el byte 3 y su ida y vuelta → `identity.rs` y el borrado de `new_identity` →
los tres símbolos → el cableado de Dart → las pruebas entre procesos → la
retractación de lo que la documentación afirmaba.

---

## 11. Enmienda 1 (2026-08-16) — la protección se pide por su nombre

**§2 dice «no hay fallback en claro, nunca» y §7 dice que Android guarda el blob
bajo el sandbox por UID. Escritas así, se contradicen**, y la contradicción se
resuelve en la dirección estricta: **no hay fallback, porque no hay nada
automático que elegir.**

`qyro_identity_open_blocking` toma un argumento más, un entero:

```c
int32_t qyro_identity_open_blocking(const uint8_t *path, size_t path_len,
                                    uint32_t protection);
```

| Valor | Significa | Dónde |
|---|---|---|
| `0` **PLATFORM** | El envoltorio de la plataforma: DPAPI bajo `cfg(windows)`, o el que se instaló con `qyro_identity_set_wrapper` | Windows hoy; Android en la etapa B |
| `1` **SANDBOX** | **Sólo el sandbox del sistema de archivos.** La semilla se guarda sin envolver | Android en la etapa A |

`PLATFORM` sin envoltorio disponible **se niega**. No cae a `SANDBOX`; nadie
recibe menos protección de la que pidió por no haber mirado.

### Por qué un byte de wrap nuevo, y no «ninguno»

`WRAP_NONE_SANDBOX = 4`, y va en el blob como cualquier otro. **El archivo dice
qué lo protegía.** Eso importa el día de la etapa B: un build con Keystore que
encuentra un blob con el byte 4 sabe que esa identidad vivió sin envolver y puede
negarse, migrarla, o avisar — y las tres son decisiones posibles sólo si el dato
está escrito. Un formato que no distingue «protegido» de «no protegido» obliga a
adivinar, y adivinar sobre material de clave es lo que este proyecto no hace.

El envoltorio se llama `SandboxWrapper` y su `wrap` es la identidad. **No es
criptografía y su nombre no finge que lo sea.** Vive en `qyro_identity_store`
junto a los demás, no escondido, porque un envoltorio que hay que buscar es un
envoltorio que alguien reimplementa peor.

### Lo que esto le cuesta al modelo de amenazas

Una fila, con estas palabras:

> **Android, etapa A.** La semilla está en `getNoBackupFilesDir()`, legible por
> cualquier cosa que sea este UID o sea root. Con Keystore, un atacante con root
> necesitaría además el TEE. **Con el sandbox, root basta.**

Y una consecuencia que hay que decir aunque incomode: en Android la etapa A es
**más débil que lo que ADR-0025 decidió**, y la aplicación no lo sabrá decir en
pantalla. Lo dice el documento de release y lo dice el modelo de amenazas.
