# Auditoría del sprint 4C.1 — endurecimiento del AEAD y evidencia multiplataforma

Rama: `claude/qyro-crypto-platform-hardening`, desde `f7ae943`.
Commit funcional verificado de partida: `bcca339` (sprint 4C).

Este sprint **no añade funcionalidad**. Comprueba que lo que el sprint 4C
implementó corre donde el producto dice correr, y cierra los defectos que esa
comprobación destapó.

## El problema que lo motivó

Al cerrar el sprint 4C había cuatro workflows en verde sobre el mismo commit:
CI, Platform builds, Android runtime ABI e iOS runtime ABI. De ahí se podía leer
—y se leyó— que la criptografía estaba probada en las tres plataformas.

No lo estaba. Los tres workflows de plataforma construyen y ejecutan `qyro_ffi`,
y `qyro_ffi` depende de `qyro_core` y de nada más. Hay una prueba en este
repositorio que **falla** si alguien añade `qyro_crypto` a su grafo, porque
mantener la criptografía fuera de la frontera FFI es deliberado. Así que ningún
run tocaba una línea de `qyro_crypto` fuera de x86_64 Linux.

La evidencia se estaba leyendo por plataforma cuando lo que un job demuestra es
**un paquete en un target**. Un job llamado `android` en verde parece cubrir
«Android»; cubre lo que compiló.

De ahí la regla que este sprint deja escrita: **el nombre del paquete y el
target se comprueban juntos.**

## Hallazgos

### H-1 — Ninguna evidencia de `qyro_crypto` en Android, iOS ni Windows

Descrito arriba. Registrado como QYR-0016.

**Resolución.** `.github/workflows/crypto-platform.yml`, cuatro jobs:

| Job | Compila | Ejecuta |
|---|---|---|
| `linux-crypto` | x86_64 Linux | sí, el harness nativo |
| `windows-crypto` | x86_64 Windows MSVC | sí, el harness nativo |
| `android-crypto` | x86_64 y arm64 Android | sí en x86_64, vía `adb` en un emulador API 35 |
| `ios-crypto` | arm64 device y arm64 simulator | sí en el simulador, vía `xcodebuild test` |

Seis targets compilados, cuatro entornos de ejecución. Android arm64 e iOS
device se compilan y **no** se ejecutan, porque no hay hardware; la matriz de
`docs/testing/crypto-platform-matrix.md` lo dice fila por fila en lugar de
resumirlo en un tick.

**Prevención.** `scripts/check_crypto_platform_evidence.{sh,ps1}` exige que
`--package qyro_crypto` y `--target <triple>` aparezcan **en la misma orden**.
Su contrato construye a propósito un workflow falso que compila `qyro_ffi` para
un target de Android y comprueba que el checker lo rechaza: un verificador que
acepta la sustitución que este sprint existe para impedir no sirve de nada.

### H-2 — La ruta AEAD podía abortar el proceso con datos de un peer

`unreachable!`, `assert!` e indexado sin comprobar en `seal`, `open` y el bitmap
de replay. Registrado como QYR-0017.

Cada byte que llega a `open` lo eligió alguien que no tiene la clave. Un pánico
ante un input de peer es una denegación de servicio que no cuesta nada montar.

Y el `assert!` no era un control de seguridad aunque lo pareciera:
`debug_assertions` está apagado en release, así que la comprobación desaparecía
exactamente en la compilación que se distribuye.

**Resolución.** Cinco variantes nuevas de `AeadError` —`FrameTemplateRejected`,
`EnvelopeConstructionFailed`, `AssociatedDataMismatch`, `ReplayStateCorrupt`,
`SealerPoisoned`— y un `deny` sobre `clippy::panic`, `unwrap_used`,
`expect_used`, `unreachable`, `todo`, `unimplemented` e `indexing_slicing`.

**El segundo defecto, que solo apareció al arreglar el primero.** Devolver `Err`
sin más deja un sealer que puede haber consumido ya su secuencia. Si el llamante
reintenta, dos frames salen con el mismo nonce, y repetir un nonce en un cifrador
de flujo revela el XOR de los dos textos claros. Por eso el estado del sealer
pasa de `Option<u64>` a un enum de tres variantes y **cualquier** error lo deja
en `Poisoned` de forma permanente.

Quitar una macro que aborta no es gratis. Lo que la macro hacía —parar— seguía
siendo lo correcto; lo que había que cambiar era *cómo* para.

### H-3 — El texto claro descifrado quedaba en memoria sin borrar

`AuthenticatedFrame::payload` era un `Vec<u8>` e `into_payload` lo entregaba
desnudo. Registrado como QYR-0018.

Al inventariar los secretos apareció el defecto mayor: **las features `zeroize`
de `sha2` y `hmac` estaban apagadas.** El estado de compresión de SHA-256 detrás
de cada transcript, y el estado con clave de HMAC detrás de cada MAC de
confirmación y de cada expansión HKDF, quedaban en memoria liberada. La feature
se había supuesto activa por el nombre en lugar de comprobarse en `Cargo.lock`.

**Resolución.** `Zeroizing<Vec<u8>>` en el payload y en los dos búferes
temporales, `into_zeroizing_payload` en lugar de `into_payload`, y ambas features
encendidas. `hkdf` no tiene feature equivalente y no la necesita: `Hkdf<Sha256>`
guarda un `Hmac<Sha256>`, comprobado leyendo `GenericHkdf` en hkdf 0.13 y no
deducido del nombre.

**Lo que esto no garantiza** está en `docs/security/secret-lifecycle-audit.md`, y
se enumera allí porque un documento que solo lista lo que sí hace engaña por
omisión: swap, hibernación, `fork`, core dumps, registros de CPU y las
reasignaciones previas de un `Vec` quedan fuera del alcance de cualquier `Drop`.
Nada aquí bloquea páginas en memoria.

Ninguna de estas garantías se ha **observado**. Leer memoria liberada es
comportamiento indefinido y el asignador puede haber reutilizado la página, así
que una prueba que afirmara verlo estaría mintiendo. Lo que las pruebas
comprueban es el tipo, que es donde vive la garantía.

### H-4 — Ningún target de `cargo-fuzz` podía construirse

Registrado como QYR-0019. Dos fallos encadenados, más uno documental:

1. `rust/fuzz/Cargo.toml` decía «excluded from the main workspace» y nada lo
   excluía. El manifest raíz ni lo listaba ni lo excluía, y el paquete no
   declaraba `[workspace]` propio, así que Cargo respondía «current package
   believes it's in a workspace when it's not» y no llegaba a compilar nada.
2. Detrás de eso, `frame_decoder` usaba campos de `FrameHeader` que pasaron a ser
   privados y una API que el sprint 2 sustituyó por `DecodedFrame`.
3. El recetario de `parser-threats.md` omitía `--fuzz-dir rust/fuzz`. Sin él
   cargo-fuzz busca `<raíz>/fuzz`, encuentra el manifest del workspace y responde
   «could not read the manifest file», que no dice nada sobre el problema real.

Nada lo detectó porque lo único que CI ejecutaba sobre esos archivos era
`rustfmt --check`, que no necesita tipos para pasar. La frase «el corpus smoke
reproduce las mismas aserciones que hacen los targets» era cierta solo porque los
smoke tests las reimplementaban.

**Resolución.** `[workspace]` propio, `frame_decoder` reparado, tres targets
nuevos (`encrypted_envelope`, `frame_opener`, `replay_window`) y
`.github/workflows/crypto-fuzz.yml`, con un job por target, sin `fail-fast` y con
`-print_final_stats=1` para que «se fuzzeó» sea un número de ejecuciones.

La sesión determinista que `frame_opener` necesita vive en `qyro_crypto::fuzzing`
bajo **`--cfg fuzzing`, que no es una feature de Cargo**. Las features son
aditivas: cualquier crate del grafo puede encender una para todos, así que una
feature pública `test-vectors` estaría a una línea de meter un constructor
determinista en un build de release. `--cfg fuzzing` lo pone cargo-fuzz en la
línea de órdenes para una compilación y ninguna dependencia puede pedirlo.

**Esto no es fuzzing exhaustivo.** Dos minutos por target encuentra defectos
superficiales. Es un suelo que CI puede sostener, no una revisión de seguridad, y
`docs/testing/crypto-fuzzing.md` empieza por lo que no demuestra.

### H-5 — El repositorio se extraía con CRLF en Windows

Registrado como QYR-0020. Tres pruebas fallaban en Windows y solo allí: las dos
que regeneran vectores byte a byte y la que recorre el fuente buscando
constructores deterministas.

Ninguna tenía nada que ver con el código que señalaba. Sin `.gitattributes`, Git
aplicaba su conversión por defecto al extraer, y las pruebas comparan bytes.

**Resolución.** `* text=auto eol=lf` con una lista explícita de extensiones
binarias, más una prueba nombrada que rechaza un `\r` en los vectores
comprometidos, para que el siguiente que ocurra falle por su nombre.

### H-6 — Documentación que contradecía la implementación

- ADR-0016 describía `header_len > 48` como aceptado y a los pares antiguos
  saltándose bytes que no entienden, y exigía `trailer_len == 0` en QYRO/1.0.
  ADR-0018 revirtió lo primero y ADR-0022 lo segundo, hace cuatro sprints.
- `parser-threats.md` terminaba diciendo «el AEAD sigue sin existir».
- `TESTING.md` decía que el fuzzing real no se había ejecutado, lo cual era
  cierto y omitía lo peor: que no podía ejecutarse.
- `nonce-lifecycle.md` describía el contador del sealer como un `Option<u64>`.
- QYR-0004 afirmaba que no se retiene ningún build. El ZIP portable de Windows sí
  se retiene; el APK y el `Runner.app` no.

Todas corregidas. Las de las ADR como **enmiendas fechadas**, no como ediciones
en su sitio: un registro de decisión que se reescribe para parecer que siempre
dijo lo correcto deja de ser un registro.

## Aislamiento del harness

`rust/tools/qyro_crypto_smoke` responde a una sola pregunta —¿corre `qyro_crypto`
en esta plataforma?— y no forma parte de la aplicación. Es `publish = false` y
ningún crate del producto lo depende.

Dos guardas lo mantienen así, porque «nadie lo va a incluir» no es un control:

1. Una prueba busca sus símbolos en los bundles que CI construye.
2. `scripts/check_harness_isolation.{sh,ps1}` rechaza que cualquier workflow que
   no sea `crypto-platform.yml` lo compile y suba como artefacto.

La segunda regla tuvo que estrecharse: en su primera versión bastaba con nombrar
el harness para disparar el guard, y `platform-builds.yml` lo nombra
legítimamente —busca su símbolo dentro de los bundles para comprobar que **no**
está—. Un guard que rechaza el caso inofensivo se acaba relajando, y un guard
relajado deja de atrapar el dañino. La regla exige ahora `--package`/`-p` junto a
`upload-artifact`.

### El guard de símbolos no pasa por vacío

Una prueba que busca una cadena en un artefacto pasa igual si esa cadena no
existe en ningún sitio. Comprobado que sí podría fallar:

    $ grep -a -c qyro_crypto_smoke_run target/debug/libqyro_crypto_smoke.so
    3
    $ grep -a -c qyro_crypto_smoke_run target/debug/libqyro_ffi.so
    0

El patrón aparece tres veces en la biblioteca del harness y ninguna en la que el
producto distribuye. Es la diferencia entre «el guard dice que no está» y «el
guard sabría decir que está».

Decisión completa en ADR-0023.

## Método

El mismo de los sprints anteriores: **borrar la corrección y comprobar que alguna
prueba falla.**

Aplicado aquí a las propias guardas, que son pruebas que leen el código fuente y
por tanto son fáciles de escribir mal:

- `the_stripper_actually_strips` comprueba que el descarte de bloques
  `#[cfg(test)]` hace lo que dice, antes de que ninguna otra guarda confíe en él.
  Sin eso, una guarda que no encuentra `assert!` en producción podría estar
  simplemente no encontrando nada.
- `the_production_aead_path_contains_no_assertions`,
  `verified_plaintext_lives_in_a_zeroizing_container` y
  `the_temporary_buffers_are_zeroizing_too` se verificaron restaurando lo que
  prohíben.
- `every_committed_vector_arrives_with_the_bytes_that_were_committed` fue
  primero roja: es la que reproduce el fallo de Windows en cualquier plataforma.

Una guarda de este sprint hubo que reescribirla por rechazar el caso inofensivo:
prohibía la cadena `Clone` en el módulo del AEAD y saltaba con el
`derive(Clone, Copy, Debug…)` de un enum interno sin secretos. Se sustituyó por
una comprobación sobre los cinco tipos que sí importan —`FrameSealer`,
`FrameOpener`, `SealedFrame`, `AuthenticatedFrame`, `DirectionalKeys`—.

## Lo que este sprint NO demuestra

Se enumera antes de las tablas de evidencia, y no después.

- **No hay hardware.** Un emulador y un simulador no son un teléfono. Android
  arm64 e iOS device se compilan y nadie los ha ejecutado.
- **No hay transporte.** Nada de esto mueve un byte. No hay sockets, ni
  descubrimiento, ni escritura en disco, y Qyro sigue sin transferir archivos.
- **El fuzzing es acotado.** Dos minutos por target, semanal. La cobertura más
  allá de ese presupuesto es desconocida.
- **La zeroización no se ha observado.** Se comprueba el tipo, no la memoria.
- **Nadie ha verificado los vectores desde otra implementación.** Hasta que
  exista un lado Swift o Kotlin, «formato sin ambigüedad» es una intención.
- **No hay medición de canales laterales.** ChaCha20-Poly1305 en software es de
  tiempo constante por construcción y el tag lo compara `subtle`, pero nada aquí
  lo mide.
- **No hay auditoría criptográfica externa.**
- **El harness no es la aplicación.** Que `qyro_crypto` corra en un simulador no
  dice nada sobre la app Flutter, que sigue sin depender de él a propósito.

## Evidencia

Baseline local, host Linux, Rust 1.88.0. Este contenedor no trae Flutter ni Dart,
así que lo que los necesita corre en CI y no aquí.

| Comprobación | Resultado |
|---|---|
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS, sin avisos |
| `cargo test --workspace` | PASS, **278 tests** |
| `cargo audit --deny warnings` | PASS, 0 vulnerabilidades, 56 crates |
| `cargo run --package qyro_crypto_smoke -- --json` | `{"target":"linux-x86_64-unix","outcome":"success","code":0}` |
| `bash scripts/check_crypto_platform_evidence.sh` | PASS |
| `bash scripts/check_harness_isolation.sh` | PASS |
| `bash scripts/check_repo_portability.sh` | PASS |

Los runs de CI que cierran el sprint están en STATUS.md, que es la fuente
canónica y apunta al commit sobre el que se ejecutaron.

## Siguiente tarea

Persistencia segura y versionada de `DeviceIdentity` mediante Android Keystore,
iOS Keychain y Windows DPAPI/CNG: creación, carga, rotación, borrado, corrupción
detectada y pruebas en runtime. **Sin** conectar sockets ni transferencia
todavía.
