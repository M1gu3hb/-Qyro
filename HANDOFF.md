# Handoff operativo

El estado actual completo está en [STATUS.md](STATUS.md). Este archivo no duplica commits, resultados ni capacidades para evitar desincronización.

## Reanudación

1. Leer STATUS.md.
2. Confirmar la rama `claude/qyro-transfer-engine-5a`, que continúa
   `claude/qyro-android-keystore-4d2a`.

   **Existe un motor de transferencia y no mueve archivos.** `qyro_transfer`
   lleva una transferencia completa —varios archivos, varios chunks, digest
   verificado, ACK con ventana, pausa, reanudación, cancelación y
   retransmisión— entre dos extremos del **mismo proceso** que sólo intercambian
   `Vec<u8>` de frames sellados. No hay sockets, no hay disco, y `qyro_ffi` no
   depende ni de `qyro_crypto` ni de `qyro_transfer`, así que la aplicación no
   ejercita nada de esto. Los botones siguen deshabilitados.

   Lo que 5B tiene que respetar: `ContentSource` y `ContentSink` son la costura
   por la que entra el filesystem, y **no deberían cambiar**. Si cambian, estaban
   mal, y eso es un hallazgo antes que un cambio — igual que `SecretWrapper`
   aguantó la segunda plataforma sin ensancharse.

   **El sprint 4D.2a está aparcado, no cancelado.** ADR-0025 sigue congelada y
   sigue siendo buena; lo que la paró es QYR-0064, que no es un defecto suyo.

   El sprint 4D.1 fue el primero desde 4A que añadió función: persistencia de
   identidad, en Windows.

   **La función existe en Windows y no existe en Android ni en iOS.** Una
   identidad generada por un proceso la carga otro proceso distinto, ejecutado
   en CI sobre `windows-latest` con DPAPI de ámbito de usuario. En las otras dos
   plataformas no hay nada, y cerrar el proceso sigue perdiendo la identidad.
   Nada del producto llama al almacén todavía: lo que persiste en CI es un
   harness aislado, no la aplicación.

   Lo que abrió el sprint y hay que tratar con cuidado es el **accesor de
   semilla**. Hasta 4D.1, `identity.rs` decía «there is no accessor for the seed
   or the private key» y esa frase era toda la protección. Ahora existe
   `DeviceIdentity::export_secret`, así que cualquier crate que dependa de
   `qyro_crypto` puede pedir la semilla de una identidad que tenga en la mano, y
   lo único que lo contiene es que haya que poseer el `DeviceIdentity`. La
   guarda `every_public_path_returning_key_material_is_listed` enumera los
   caminos por nombre y falla si aparece un cuarto. Su historia —una versión que
   fallaba abierta— está en `docs/audits/SPRINT4D1_SECURE_STORAGE.md`, y merece
   leerse antes de tocar la frontera. Ver «Next task» en STATUS.md.

   **Los workflows ya no nombran ninguna rama.** Disparan sobre
   `[main, 'claude/**']`, así que una rama nueva con ese prefijo tiene CI sin
   tocar un solo YAML. Hasta el sprint 4C.3 el nombre estaba escrito a mano en
   los seis archivos, y cada sprint heredaba el defecto del anterior
   (QYR-0040). Una regla de `check_docs_consistency` rechaza ahora un nombre
   literal en cualquier `branches:`.
3. Leer `docs/audits/CLAUDE_RECOVERY_AUDIT.md` para el contexto de recuperación,
   más ADR-0014 (logo), ADR-0015 (ramas), ADR-0016 (framing), ADR-0017
   (manifest), ADR-0020 (identidad, con su enmienda del sprint 4B), ADR-0021
   (handshake), ADR-0022 (AEAD de frames, con sus tres enmiendas) y ADR-0023
   (harness de criptografía por plataforma) y ADR-0024 (almacenamiento seguro de
   identidad, congelada antes de su implementación y enmendada tres veces
   después: QYR-0048, QYR-0054 y QYR-0059). Las especificaciones están
   en `docs/protocols/` y `docs/security/` —el formato del blob, byte a byte, en
   `docs/security/identity-storage.md`—; las auditorías de los últimos sprints,
   en `docs/audits/SPRINT4C1_CRYPTO_PLATFORM_AUDIT.md`,
   `docs/audits/SPRINT4C2_AUDIT_CLOSURE.md`,
   `docs/audits/SPRINT4C3_RESOURCE_BOUNDS.md` y
   `docs/audits/SPRINT4D1_SECURE_STORAGE.md`. La de 4C.3 lleva los números de
   coste del decoder antes y después, medidos con el mismo método, y la
   advertencia que costó descubrir: una mutación que no toca el camino que la
   prueba recorre sale verde y no prueba nada. La de 4C.2 lleva su propia tabla
   de mutación —hallazgo, mutación aplicada, prueba que falló, commit en que
   estuvo roja—, y la de 4D.1 la suya, más los caminos públicos que devuelven
   material de clave antes y después. Léelas antes de tocar `qyro_manifest`,
   `qyro_crypto` o la frontera de identidad; explican por qué varias
   comprobaciones están escritas de una forma que parece rebuscada.

   ADR-0016 lleva la enmienda de coste del sprint 4C.3, y ADR-0017, ADR-0019 y
   ADR-0021 las del 4C.2 —con la fecha de Unicode corregida en 4C.3. Las
   afirmaciones anteriores que corrigen están marcadas como corregidas, no
   borradas.
4. Leer NEXT_STEPS.md y ADR relacionadas.
5. Ejecutar doctor y tests relevantes.
6. Continuar con la única “Next task” de STATUS.md.

## Reproducir el baseline

Este entorno no trae Flutter preinstalado. El baseline completo exige Flutter
3.44.8 (la versión que fija CI), Rust 1.88.0, Python 3 y PowerShell 7 para los
contratos `.ps1`.

    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cargo test --workspace --all-features
    cargo test --doc --workspace
    cargo audit --deny warnings          # obligatorio desde el sprint 2
    cargo run --package qyro_crypto_smoke -- --json   # el harness, en el host
    cargo build --package qyro_ffi
    cd apps/qyro && flutter pub get --enforce-lockfile
    dart tools/branding_generator/bin/generate.dart --check   # desde la raíz
    dart format --output=none --set-exit-if-changed .
    flutter analyze
    QYRO_FFI_LIBRARY_PATH=<repo>/target/debug/libqyro_ffi.so flutter test

## Auditorías externas: comprometer antes de citar

Una auditoría que no produjo este repositorio se comparte en
`docs/audits/external/` **antes** de que ningún `QYR-00xx` derivado de ella se
cite en el ledger o en un prompt.

QYR-0037, QYR-0038 y QYR-0039 se registraron a partir de la descripción de un
prompt posterior, no del documento original, que nunca entró aquí. QYR-0039 llegó
a decir en el propio ledger que no se sabía qué describía el hallazgo. Un
identificador cuyo enunciado no está en ninguna parte no se puede cerrar ni
evaluar, porque no hay contra qué comprobarlo (QYR-0047).

Si el documento no se puede comprometer, la entrada del ledger tiene que decir
que está reconstruida. Lo que no vale es reconstruir sin marcarlo: esa entrada se
lee igual que una verificada.

## Regla de entrega

Al cerrar una unidad, actualizar STATUS.md con evidencia real y registrar defectos en BUGS_PENDING.md. No declarar transferencia, seguridad o compatibilidad que no hayan sido ejecutadas.

El job documental ahora falla si `Verified commit` de STATUS.md no es alcanzable
desde HEAD o queda más de 10 commits por detrás, así que STATUS.md debe
actualizarse dentro del mismo tramo de trabajo, no al final.

## Estado del protocolo y la criptografía

`qyro_protocol`, `qyro_manifest` y `qyro_crypto` están implementados y probados,
y desde el sprint 5A **`qyro_transfer` los usa a los tres a la vez**. Lo que
sigue sin existir es lo de fuera: no hay sockets, ni transporte, ni escritura en
disco.
Cifrado sí hay, desde el sprint 4C, y no mueve un solo byte a ninguna parte. Que el framing y el handshake existan no significa que Qyro
transfiera archivos. Los botones Enviar y Recibir siguen deshabilitados a
propósito, y el README sigue diciendo que Qyro todavía no transfiere archivos.

Concretamente, después del sprint 4C.1:

- El handshake **corre entre dos valores en un proceso**. No hay socket, ni
  descubrimiento, ni integración con el framing. El `SessionId` que deriva sí es
  ya el tipo que lleva la cabecera QYRO/1, así que conectarlo no exigirá
  inventar ninguna conversión.
- Sus claves de sesión **sí** cifran desde el sprint 4C: `qyro_crypto::aead` sella
  y abre frames QYRO/1 con ChaCha20-Poly1305, con nonces monotónicos y una
  ventana de replay de 1024. Sigue sin haber transporte que los mueva.
- `EncryptedEnvelope` sigue siendo una forma de cable que no afirma nada, y eso
  es deliberado: los tipos que afirman son `SealedFrame` y `AuthenticatedFrame`,
  en `qyro_crypto`, con constructores privados.
- Las claves de sesión viven **solo en memoria**, en las tres plataformas, y así
  debe seguir: son efímeras por diseño.
- La **identidad** persiste en Windows desde el sprint 4D.1 —`qyro_win_dpapi`,
  `%LOCALAPPDATA%\Qyro\identity.bin`, DPAPI de ámbito de usuario— y **no**
  persiste en Android ni en iOS. Ese desequilibrio es el estado actual, no una
  simplificación de esta nota.
- `qyro_crypto` **se ejecuta** en Linux, Windows, emulador Android y simulador
  iOS, y se compila además para Android arm64 e iOS device. Antes del sprint
  4C.1 no había evidencia de ninguna de las tres plataformas: los workflows en
  verde construían `qyro_ffi`, que no depende de `qyro_crypto`. Emulador y
  simulador **no son hardware**, y nada se ha medido en un teléfono.
- La ruta AEAD de producción no puede entrar en pánico y el texto claro
  autenticado se borra al soltarlo. Ninguna de las dos cosas era cierta antes
  del sprint 4C.1.

`rust/fuzz` es un workspace aparte y exige nightly; no entra en la compilación
del producto. Desde el sprint 4C.1 **se puede construir**, que hasta entonces no
era el caso: el paquete no declaraba `[workspace]` propio y Cargo se negaba a
hacer nada en ese directorio. Los targets se ejecutan así, desde la raíz:

    rustup toolchain install nightly
    cargo install cargo-fuzz --locked --version 0.13.1
    cargo +nightly fuzz run --fuzz-dir rust/fuzz frame_opener \
        -- -max_total_time=120 -print_final_stats=1

`--fuzz-dir` no es opcional; sin él cargo-fuzz busca `<raíz>/fuzz` y falla con un
mensaje que no explica el problema.

## El harness de criptografía no entra en el producto

`rust/tools/qyro_crypto_smoke` existe para responder «¿corre `qyro_crypto` en
esta plataforma?» y no forma parte de la aplicación. Es `publish = false`, no lo
depende ningún crate del producto, y dos guardas lo mantienen fuera: uno busca su
símbolo en los bundles que CI construye, y otro rechaza que cualquier workflow
que no sea `crypto-platform.yml` lo compile y suba como artefacto. Ver ADR-0023.

## Dónde vive el `unsafe`, y por qué hay

Hasta el sprint 4D.1 no había ninguno. Ahora hay **tres funciones en un solo
crate**, `qyro_win_dpapi`: `ffi.rs::take_and_free`, `store.rs::wrap` y
`store.rs::unwrap`. Están enumeradas por nombre en `src/guards.rs` de ese crate,
y añadir un bloque en cualquier otra función pone la guarda en rojo con el nombre
de la función en el mensaje.

Dos cosas que hay que saber antes de tocarlo:

- **`cargo check` no enlaza.** Se añadió el target de Windows en local para que
  el `extern` se comprobara de tipos, salió limpio, y CI falló con `LNK2019`
  porque faltaba `#[link(name = "Crypt32")]`. Comprobar tipos no es comprobar
  que el símbolo existe.
- La lista de crates que pueden relajar `#![forbid(unsafe_code)]` tiene **tres**
  entradas y una prueba la vigila desde `qyro_identity_store`. `qyro_ffi` y
  `qyro_crypto_smoke` no pueden llevar el atributo —`#[unsafe(no_mangle)]` es un
  atributo unsafe en edición 2024— y `qyro_win_dpapi` es la excepción argumentada
  en ADR-0024 §1. Cualquier otro crate del workspace tiene que llevarlo.

`qyro_win_dpapi` no lleva `#![cfg(windows)]` en la raíz sino en cada módulo, a
propósito: así su guarda —que lee los archivos como texto— se ejecuta también en
Linux y en macOS. Una guarda que solo corre en una plataforma está apagada en la
mayoría de CI.

## Cómo se comprueba una invariante en este repositorio

Las últimas sesiones encontraron varios defectos que un razonamiento cuidadoso
no habría encontrado. El patrón que funcionó, en los tres casos, fue el mismo:
**borrar la corrección y comprobar que alguna prueba falla.** Cuando ninguna
falla, la propiedad no estaba cubierta, por convincente que fuera el argumento.

Así se descubrió que el enlace de la firma del iniciador sobre la del
respondedor no aporta nada con Ed25519 determinista, que cinco de doce
codificaciones «de orden bajo» de X25519 no lo son, y que `[0xFF; 32]` es una
clave Ed25519 perfectamente válida.

El sprint 4B.1 lo aplicó también a las propias reglas del verificador
documental, y encontró dos que no comprobaban lo que decían: una búsqueda de
`SI` insensible a mayúsculas encontraba el «si» dentro de «físico», y una prueba
de entropía fallaba contra el comentario que explica por qué ese constructor se
rechaza. Ninguna era un defecto del producto; las dos habrían quedado como
reglas que parecen estrictas y no lo son.

El sprint 4D.1 añadió dos formas más, las dos caras:

- **Una guarda escrita antes de que exista lo que vigila se puede comprobar
  gratis.** `the_unsafe_blocks_are_the_ones_we_listed` se escribió con la lista
  vacía cuando no había ni un bloque `unsafe`; el primero la puso en rojo. Igual
  con la de material de clave: lista vacía, verde, y el accesor de semilla la
  puso en rojo con exactamente los dos caminos esperados. Escribirla después
  habría sido escribirla contra el resultado.
- **Una edición que no se aplica se ve igual que una que sí.** `str.replace`
  devuelve la cadena sin cambios cuando el ancla no coincide, y en este sprint
  eso produjo un commit vacío de contenido seguido de una lectura del error
  anterior de CI como si fuera nuevo. Toda edición programática de un fuente
  lleva ahora `assert` sobre el ancla, y se lee el archivo después.

Y una tercera que no es sobre pruebas: **una tabla de evidencia también hay que
comprobarla**. Dos filas de la tabla de runs de STATUS.md se escribieron desde la
memoria de la sesión; una contaba un run cancelado como éxito y la otra citaba un
identificador que no existe (QYR-0061). Las tablas de runs se reconstruyen
listando los runs de la rama por API, no recordándolos.

## Conectar dos piezas probadas destapa lo que ninguna prueba sola veía

El sprint 5A fue el primero que usó el framing, el manifest, el handshake y el
AEAD juntos, y encontró dos cosas que cinco sprints de pruebas por separado no
podían encontrar:

- **QYR-0068**: la cabecera de 48 bytes reserva `transfer_id`, `stream_id` e
  `item_id` dentro de los datos asociados autenticados, y **no hay forma pública
  de rellenarlos**. `Frame::new` los pone a cero. Son tres campos autenticados
  que hoy no dicen nada, y se descubrió al escribir un cuerpo de mensaje que
  duplicaba uno de ellos sin saberlo.
- **QYR-0069**: los constructores deterministas del handshake son `pub(crate)`,
  así que un crate dependiente no puede reproducir una sesión byte a byte.

Ninguno se arregló. Los dos están registrados con la decisión pendiente escrita,
porque ensanchar una superficie congelada como efecto secundario de otro sprint
es cómo se pierde el control de un formato.

La lección operativa, para el siguiente que conecte dos piezas: **espera
desajustes y regístralos antes de arreglarlos.** Añadir la segunda plataforma en
4D.2a destapó que `open_identity` nunca comparaba el byte `wrap`; conectar el
protocolo con el sellado destapó estos dos. Es la forma en que este proyecto
descubre lo que sus pruebas no cubrían.
