# ADR-0023: harness aislado de pruebas criptográficas por plataforma

Estado: aceptada. Depende de ADR-0020 (identidad), ADR-0021 (handshake) y
ADR-0022 (AEAD de frames).

## Contexto

Al cerrar el sprint 4C, cuatro workflows estaban en verde sobre el mismo commit:
CI, Platform builds, Android runtime ABI e iOS runtime ABI. Ninguno probaba nada
sobre `qyro_crypto` en Android, iOS ni Windows.

El motivo es una decisión anterior y correcta: `qyro_ffi` —la biblioteca que
carga Dart— **no depende de `qyro_crypto`**, y una prueba lo mantiene así. Por
tanto `cargo build --package qyro_ffi --target aarch64-linux-android` compila
`qyro_ffi` y `qyro_core`, y nada más. El handshake, el key schedule, el AEAD y la
ventana de replay se habían compilado y ejecutado **solo en x86_64 Linux**.

«Es Rust portable» es un argumento, no evidencia. Los sitios donde un argumento
así falla son conocidos: tamaño de palabra, endianness, alineación, disponibilidad
de instrucciones (`cpufeatures` elige rutas distintas en aarch64 y x86_64), y la
diferencia entre un toolchain de host y uno de cross-compilation.

## Decisión

Existe un **harness de pruebas aislado**, `rust/tools/qyro_crypto_smoke`, que:

- depende de `qyro_crypto` y `qyro_protocol`;
- ejecuta un flujo completo —handshake, sellado, wire, apertura, replay,
  tamper— usando **solo API pública**;
- se compila y ejecuta en las cuatro plataformas del producto;
- **no entra jamás en el producto**.

El aislamiento es la parte que importa. Un harness que pudiera acabar dentro del
APK, del Runner.app o del ZIP de Windows sería una superficie de ataque añadida
para obtener una prueba, y la prueba no vale eso.

## Aislamiento

| Regla | Cómo se sostiene |
|---|---|
| No se publica | `publish = false` |
| No lo enlaza el producto | Nadie lo declara como dependencia; `qyro_ffi` y `qyro_core` siguen sin ver `qyro_crypto` |
| No aparece en artefactos | Un guard busca `qyro_crypto_smoke_run` en el APK, en Runner.app y en el ZIP portable, y falla si lo encuentra |
| No expone secretos | Un solo export C que devuelve `int32_t`; ninguna función devuelve bytes |
| No inventa criptografía | No implementa primitivas ni constructores; llama a `qyro_crypto` |
| No usa claves fijas | Entropía del CSPRNG del sistema; los constructores deterministas siguen siendo `cfg(test)` y privados del crate |

## Superficie C

```c
int32_t qyro_crypto_smoke_run(void);
```

Nada más. Ni punteros a claves, ni semillas, ni nonces, ni texto claro, ni
longitudes que revelen contenido. El valor de retorno es un código de paso, y los
códigos están documentados y son estables porque un runner los lee.

Que la superficie sea un entero es deliberado. Cualquier función que devolviera
bytes tendría que documentar de quién son y quién los libera, y la respuesta
correcta para material criptográfico es que no cruce.

## Ejecución por plataforma

| Plataforma | Cómo se compila | Cómo se ejecuta |
|---|---|---|
| Linux | binario nativo | se ejecuta en el runner |
| Windows x64 | binario nativo | se ejecuta en el runner, más `cargo test -p qyro_crypto` |
| Android x86_64 | binario para `x86_64-linux-android` | `adb push` al emulador y `adb shell` |
| Android arm64 | `aarch64-linux-android` | **solo compila**; el runner es x86_64 |
| iOS simulator | `staticlib` para `aarch64-apple-ios-sim` | enlazado a un target XCTest, `xcodebuild test` |
| iOS device | `aarch64-apple-ios` | **solo compila**; no hay hardware |

Las dos filas marcadas «solo compila» se declaran así en STATUS.md y en el
reporte. Compilar no es ejecutar, y llamarlo ejecución sería exactamente el
error que este ADR corrige.

## Alternativas descartadas

- **Añadir `qyro_crypto` a `qyro_ffi`.** Resolvería la evidencia y rompería la
  frontera: pondría claves al alcance de Dart para poder probar que las claves
  funcionan.
- **Un target de test dentro de `qyro_crypto`.** `cargo test --target` necesita un
  runner para ejecutar el binario de test en el dispositivo, y el binario de test
  de un crate no es algo que se pueda empujar con `adb` sin más. Un binario
  propio con un `main` es más simple y su aislamiento es más fácil de demostrar.
- **Confiar en que Rust es portable.** Es lo que se venía haciendo sin decirlo.
- **Probar en hardware físico.** No hay hardware. Emulador y simulador son lo que
  hay, y el documento lo dice en lugar de insinuar otra cosa.

## No objetivos

Persistencia de identidad, Keystore, Keychain, DPAPI/CNG, FFI criptográfico de
producción, sockets, transporte y transferencia. El harness prueba que el núcleo
criptográfico funciona en cada plataforma; no lo conecta a nada.

Qyro sigue sin transferir archivos.
