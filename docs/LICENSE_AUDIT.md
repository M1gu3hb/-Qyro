# Auditoría de licencias

Fecha: 2026-08-04. Apache-2.0 es provisional y requiere aprobación antes de 1.0.

## Producto

| Dependencia | Versión | Origen | Licencia | Función | Riesgo |
|---|---:|---|---|---|---|
| Rust std | 1.88.0 | rust-lang/rust | MIT/Apache-2.0 | núcleo/ABI | bajo |
| Flutter/Dart SDK | Flutter 3.44.8 | flutter/flutter, dart-lang/sdk | BSD-3-Clause | UI/runtime | bajo |
| integration_test | Flutter 3.44.8 | Flutter SDK | BSD-3-Clause | instrumentación móvil | dev |
| flutter_lints | 5.0.0 | pub.dev | BSD-3-Clause | lint dev | bajo |
| async | 2.13.1 | pub.dev | BSD-3-Clause | tests SDK transitivos | dev |
| boolean_selector | 2.1.2 | pub.dev | BSD-3-Clause | tests | dev |
| characters | 1.4.1 | pub.dev | BSD-3-Clause | Flutter | bajo |
| clock | 1.1.2 | pub.dev | Apache-2.0 | tests | dev |
| collection | 1.19.1 | pub.dev | BSD-3-Clause | Flutter | bajo |
| fake_async | 1.3.3 | pub.dev | Apache-2.0 | tests | dev |
| leak_tracker | 11.0.2 | pub.dev | BSD-3-Clause | tests | dev |
| leak_tracker_flutter_testing | 3.0.10 | pub.dev | BSD-3-Clause | tests | dev |
| leak_tracker_testing | 3.0.2 | pub.dev | BSD-3-Clause | tests | dev |
| lints | 5.1.1 | pub.dev | BSD-3-Clause | lint | dev |
| matcher | 0.12.19 | pub.dev | BSD-3-Clause | tests | dev |
| material_color_utilities | 0.13.0 | pub.dev | Apache-2.0 | Flutter Material | bajo |
| meta | 1.18.0 | pub.dev | BSD-3-Clause | anotaciones | bajo |
| path | 1.9.1 | pub.dev | BSD-3-Clause | tests SDK | dev |
| source_span | 1.10.2 | pub.dev | BSD-3-Clause | tests | dev |
| stack_trace | 1.12.1 | pub.dev | BSD-3-Clause | tests | dev |
| stream_channel | 2.1.4 | pub.dev | BSD-3-Clause | tests | dev |
| string_scanner | 1.4.1 | pub.dev | BSD-3-Clause | tests | dev |
| term_glyph | 1.2.2 | pub.dev | BSD-3-Clause | tests | dev |
| test_api | 0.7.11 | pub.dev | BSD-3-Clause | tests | dev |
| vector_math | 2.2.0 | pub.dev | BSD-3-Clause | Flutter | bajo |
| vm_service | 15.2.0 | pub.dev | BSD-3-Clause | tests/debug | dev |

sky_engine, flutter, flutter_test e integration_test vienen del SDK. Las versiones/hashes de paquetes alojados están fijadas en apps/qyro/pubspec.lock. Cargo no tiene crates externos.

## Infraestructura de CI

| Dependencia | Versión fijada | Licencia | Función | Riesgo |
|---|---|---|---|---|
| ReactiveCircus/android-emulator-runner | a421e43855164a8197daf9d8d40fe71c6996bb0d (v2.38.0) | Apache-2.0 | emulador Android para prueba ABI real | dev |
| actions/upload-artifact | ea165f8d65b6e75b540449e92b4886f43607fa02 (v4) | MIT | publicación del ZIP portable de Windows | dev |

Las versiones y licencias se verificaron contra los tags y archivos LICENSE publicados. Los SHA completos evitan cambios implícitos de las actions fijadas.

## Política y pendiente

Permitidas tras revisión: Apache-2.0, MIT, BSD-2/3, ISC, Zlib y OFL. GPL/AGPL/LGPL, MPL y desconocidas requieren autorización. Falta automatizar verificación y revisar licencias de actions/checkout, flutter-action y rust-toolchain (CI, no distribuidas).

## Sprint 2 — 2026-08-05

`qyro_protocol` y `qyro_manifest` se añadieron **sin dependencias externas**. El
workspace sigue con cero paquetes de terceros, así que no hay licencias nuevas
que registrar.

`cargo audit 0.22.2` es obligatorio en CI desde este sprint y pasa con 0
vulnerabilidades sobre 4 crates, todas propias. No hay excepciones ni advisories
diferidas.

`proptest` se evaluó para las pruebas basadas en propiedades. Licencia
MIT/Apache-2.0, aceptable, pero **no se añadió**: arrastra 39 paquetes
transitivos por una herramienta que solo se usa en desarrollo. El razonamiento
está en `TESTING.md`.

`libfuzzer-sys` aparece únicamente en `rust/fuzz/Cargo.toml`, un workspace
separado que no forma parte de la compilación ni del `Cargo.lock` del producto, y
que exige nightly. No entra en ningún artefacto distribuible.

Pendiente: SBOM y `cargo-deny` para bans, fuentes y duplicados.

## Sprint 4A — 2026-08-05

El workspace deja de tener cero dependencias externas. Es un cambio deliberado y
acotado: implementar Ed25519, SHA-256 o normalización Unicode a mano sería mucho
peor que auditar implementaciones revisadas.

### Normalización Unicode (`qyro_manifest`)

| Crate | Versión | Licencia | Fuente |
|---|---|---|---|
| unicode-normalization | 0.1.25 | MIT OR Apache-2.0 | unicode-rs |
| tinyvec | 1.12.0 | Zlib OR Apache-2.0 OR MIT | Lokathor |
| tinyvec_macros | 0.1.1 | MIT OR Apache-2.0 OR Zlib | Soveu |

`default-features = false`, feature `std`. Sustituye una tabla de plegado escrita
a mano que eliminaba diacríticos y provocaba colisiones falsas.

### Identidad criptográfica (`qyro_crypto`)

| Crate | Versión | Licencia | Fuente |
|---|---|---|---|
| ed25519-dalek | 3.0.0 | BSD-3-Clause | dalek-cryptography |
| curve25519-dalek | 5.0.0 | BSD-3-Clause | dalek-cryptography |
| ed25519 | 3.0.0 | Apache-2.0 OR MIT | RustCrypto |
| signature | 3.0.0 | Apache-2.0 OR MIT | RustCrypto |
| sha2 | 0.11.0 | MIT OR Apache-2.0 | RustCrypto |
| digest / crypto-common / block-buffer | 0.11.x / 0.2.x / 0.12.x | MIT OR Apache-2.0 | RustCrypto |
| zeroize | 1.9.0 | Apache-2.0 OR MIT | RustCrypto |
| subtle | 2.6.1 | BSD-3-Clause | dalek-cryptography |
| getrandom | 0.4.3 | MIT OR Apache-2.0 | rust-random |
| rand_core | 0.10.1 | MIT OR Apache-2.0 | rust-random (transitiva) |
| fiat-crypto | 0.3.0 | MIT OR Apache-2.0 OR BSD-1-Clause | mit-plv |

`ed25519-dalek` con `default-features = false` y solo `rand_core` y `zeroize`;
sin `serde`, `pkcs8`, `pem`, `batch` ni `hazmat`.

### Handshake autenticado (`qyro_crypto`, sprint 4B)

| Crate | Versión | Licencia | Fuente |
|---|---|---|---|
| x25519-dalek | 3.0.0 | BSD-3-Clause | dalek-cryptography |
| hkdf | 0.13.0 | MIT OR Apache-2.0 | RustCrypto |
| hmac | 0.13.0 | MIT OR Apache-2.0 | RustCrypto |
| rand_core | 0.10.1 | MIT OR Apache-2.0 | rust-random (transitiva) |
| cmov | 0.5.4 | Apache-2.0 OR MIT | RustCrypto (transitiva de digest) |
| ctutils | 0.4.2 | Apache-2.0 OR MIT | RustCrypto (transitiva de digest) |

`x25519-dalek` comparte `curve25519-dalek 5.0` con `ed25519-dalek`, y
`hkdf`/`hmac` comparten `digest 0.11` con `sha2`: no entra ninguna versión
duplicada de una primitiva. `subtle` y `rand_core` ya estaban en el árbol como
transitivas y ahora se declaran directamente, porque este crate compara secretos
e implementa un `CryptoRng` él mismo. `cmov` y `ctutils` llegan por `digest`, no
por elección propia.

`x25519-dalek` con `default-features = false` y solo `precomputed-tables`,
`static_secrets` y `zeroize`; sin `serde`, `reusable_secrets` ni `getrandom`.

`static_secrets` se activa porque es el único constructor que acepta bytes
directamente, y construir el secreto desde bytes ya obtenidos es lo que permite
que esta ruta falle cerrada: `EphemeralSecret::random_from_rng` exige un
`CryptoRng` infalible, así que ningún adaptador que lo alimente puede informar de
agotamiento. El tipo se envuelve localmente para recuperar la garantía de un solo
uso que `StaticSecret` cede.

`getrandom` se omite a propósito: su `random()` hace pánico si el CSPRNG falla, y
aquí eso debe ser `EntropyUnavailable`.

`rand_core` deja de ser dependencia directa: existía solo para el adaptador RNG
que se eliminó. Sigue en el árbol como transitiva de la pila dalek.

`qyro_crypto` depende de `qyro_protocol` desde el sprint 4B.1, para compartir
`SessionId`. No hay dependencia en sentido contrario: `qyro_protocol` no conoce
la criptografía.

### AEAD de frames (`qyro_crypto`, sprint 4C)

| Crate | Versión | Licencia | Fuente |
|---|---|---|---|
| chacha20poly1305 | 0.11.0 | Apache-2.0 OR MIT | RustCrypto |
| chacha20 | 0.10.1 | MIT OR Apache-2.0 | RustCrypto (transitiva) |
| poly1305 | 0.9.1 | Apache-2.0 OR MIT | RustCrypto (transitiva) |
| aead | 0.6.1 | MIT OR Apache-2.0 | RustCrypto (transitiva) |
| cipher | 0.5.2 | MIT OR Apache-2.0 | RustCrypto (transitiva) |
| inout | 0.2.2 | MIT OR Apache-2.0 | RustCrypto (transitiva) |
| universal-hash | 0.6.1 | MIT OR Apache-2.0 | RustCrypto (transitiva) |

Una sola dependencia directa nueva. Las seis restantes llegan por ella, no por
elección propia, y todas están bajo la misma doble licencia permisiva.

Toda la pila comparte `crypto-common 0.2.2`, `hybrid-array 0.4.14`, `zeroize
1.9.0` y `cpufeatures 0.3.0` con lo que ya había: `cargo tree -d` no encuentra
ninguna versión duplicada de ninguna primitiva. `ctutils` y `cmov`, que ya
entraban por `digest`, ahora también llegan por `universal-hash`; siguen siendo la
misma versión.

`chacha20poly1305` con `default-features = false` y solo `zeroize`. Se apagan sus
dos features por defecto:

- `alloc`: la ruta de sellado usa la API detached in-out, que escribe sobre un
  búfer que ya existe y no reserva nada. Solo la API `Aead` de conveniencia
  necesita `alloc`, y esa no se usa.
- `getrandom`: por el mismo motivo que en `x25519-dalek`. Ninguna dependencia de
  este crate obtiene entropía por su cuenta; el crate la pide él mismo y reporta
  el fallo como error en lugar de dejar que otro haga pánico.

`zeroize` se activa para que el cifrador borre su clave al soltarla.

Mantenimiento: RustCrypto/AEADs es el mismo grupo que publica `sha2`, `hkdf`,
`hmac` y `digest`, ya evaluados arriba. `cargo audit --deny warnings` sobre el
`Cargo.lock` resultante (55 dependencias) no reporta ningún advisory.

### Solo desarrollo (`dev-dependencies` de `qyro_crypto`)

No se enlazan en la biblioteca ni en ningún artefacto distribuible: solo
compilan al ejecutar las pruebas.

| Crate | Versión | Licencia | Fuente |
|---|---|---|---|
| serde_json | 1.0.151 | MIT OR Apache-2.0 | serde-rs |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | serde-rs |
| itoa | 1.0.18 | MIT OR Apache-2.0 | dtolnay |
| memchr | 2.8.3 | Unlicense OR MIT | BurntSushi |
| zmij | 1.0.23 | MIT | dtolnay |

Necesarias porque los archivos de vectores pasaron a leerse como JSON en vez de
rasparse buscando subcadenas `"clave": "valor"`. `Cargo.lock` también registra
`serde` y `serde_derive`, pero `cargo tree -e normal,dev` confirma que no entran
en el grafo de compilación: `serde_json` depende de `serde_core`, y las entradas
de `serde` quedan solo como resolución de features opcionales.

Total del workspace: 48 crates en `Cargo.lock`, 5 de ellos exclusivos de
pruebas. **Ninguna licencia GPL, AGPL ni LGPL.** Todas son permisivas
(BSD-3-Clause, MIT, Apache-2.0, Zlib, BSD-1-Clause, Unlicense).

`cargo audit --deny warnings` pasa sobre los 48 crates. `cargo-deny` sigue
pendiente.
