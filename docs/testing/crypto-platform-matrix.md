# Matriz de evidencia criptográfica por plataforma

Qué se compila y qué se **ejecuta** de `qyro_crypto` en cada plataforma del
producto, y con qué mecanismo. ADR-0023 explica por qué existe el harness.

La distinción entre compilar y ejecutar es el motivo de este documento. Compilar
demuestra que el toolchain acepta el código; no demuestra que X25519, Ed25519,
HKDF-SHA256, ChaCha20-Poly1305 y la ventana de replay se comporten con el tamaño
de palabra, el endianness y las instrucciones de esa plataforma.

## Qué había antes

Cuatro workflows en verde, y ninguno decía nada sobre `qyro_crypto` fuera de
x86_64 Linux. Todos compilan y ejecutan `qyro_ffi`, que deliberadamente no puede
alcanzar `qyro_crypto`, así que un job de Android en verde era evidencia sobre
una ABI de dos símbolos.

`scripts/check_crypto_platform_evidence.{sh,ps1}` rechaza exactamente esa
sustitución: exige el nombre del paquete **junto al** target, de modo que
`--package qyro_ffi --target aarch64-linux-android` no puede satisfacer una regla
sobre `qyro_crypto`. Su contrato hace esa sustitución y comprueba que el checker
la rechaza.

## La matriz

| Plataforma | Target | Compila | Ejecuta | Mecanismo |
|---|---|---|---|---|
| Linux x86_64 | host | sí | **sí** | `cargo test -p qyro_crypto` y el binario del harness en el runner |
| Windows x64 | host MSVC | sí | **sí** | `cargo test -p qyro_crypto` y el binario del harness en el runner |
| Android x86_64 | `x86_64-linux-android` | sí | **sí** | `adb push` a un emulador API 35 y `adb shell` |
| Android arm64 | `aarch64-linux-android` | sí | **no** | el runner es x86_64 |
| iOS simulator | `aarch64-apple-ios-sim` | sí | **sí** | XCTest sobre un simulador arrancado |
| iOS device | `aarch64-apple-ios` | sí | **no** | no hay hardware en CI |

Las dos filas con «no» son afirmaciones de compilación y se declaran así también
en STATUS.md. Llamarlas ejecución sería el error que este trabajo corrige.

**Nada de esto es hardware físico.** Un emulador y un simulador no son un
teléfono. La diferencia importa para rendimiento, para la entropía del sistema y
para el comportamiento térmico, y ninguna de las tres cosas se ha medido.

## Qué ejecuta el harness

Una sesión completa, solo con API pública: dos identidades nuevas del CSPRNG del
sistema, el handshake de cuatro mensajes, sellado, ida y vuelta por el decoder
ordinario, apertura, un intento de replay, un intento de manipulación del tag, y
lo mismo en la dirección contraria.

Devuelve un código de salida estable por paso, documentado en
`rust/tools/qyro_crypto_smoke/include/qyro_crypto_smoke.h`, porque un runner lo
lee como estado de proceso y un código que se moviese entre versiones haría
ilegible un log antiguo.

No hay modo determinista. Una semilla fija demostraría que la plataforma sabe
reproducir una constante, que no es lo que se está preguntando de un toolchain
nuevo.

## Aislamiento

El harness enlaza el crate que guarda claves. No entra en el producto, y eso se
comprueba en dos mitades porque ninguna basta sola:

- `scripts/check_harness_isolation.{sh,ps1}` lee el árbol de fuentes en cada
  plataforma;
- `platform-builds.yml` abre el APK, el ZIP portable y el Runner.app y busca
  `qyro_crypto_smoke_run` dentro.

Un árbol limpio con un paso de copia que mete una biblioteca se ve idéntico a uno
correcto hasta que abres el bundle.

## Reproducir

    cargo test --package qyro_crypto
    cargo run --package qyro_crypto_smoke
    cargo run --package qyro_crypto_smoke -- --json

Para otra plataforma hace falta su toolchain; el workflow
`.github/workflows/crypto-platform.yml` es la receta completa.
