# Handoff operativo

El estado actual completo está en [STATUS.md](STATUS.md). Este archivo no duplica commits, resultados ni capacidades para evitar desincronización.

## Reanudación

1. Leer STATUS.md.
2. Confirmar la rama `claude/qyro-audit-closure-4c2-9a3v4j`, que continúa
   `claude/qyro-crypto-platform-hardening` y cierra una auditoría independiente
   sin añadir funcionalidad.

   El nombre lleva un sufijo de sesión. El prompt del sprint pedía
   `claude/qyro-audit-closure-4c2`; la rama asignada al agente fue
   `claude/qyro-audit-closure-4c2-9a3v4j`, y se usó esa, porque empujar a otra
   rama sin permiso explícito no es una decisión que un agente deba tomar por su
   cuenta. Los seis workflows nombran esta rama en su `push: branches:`.
3. Leer `docs/audits/CLAUDE_RECOVERY_AUDIT.md` para el contexto de recuperación,
   más ADR-0014 (logo), ADR-0015 (ramas), ADR-0016 (framing), ADR-0017
   (manifest), ADR-0020 (identidad, con su enmienda del sprint 4B), ADR-0021
   (handshake), ADR-0022 (AEAD de frames, con sus tres enmiendas) y ADR-0023
   (harness de criptografía por plataforma). Las especificaciones están en
   `docs/protocols/` y `docs/security/`; las auditorías de los tres últimos
   sprints, en `docs/audits/SPRINT4C_AEAD_AUDIT.md`,
   `docs/audits/SPRINT4C1_CRYPTO_PLATFORM_AUDIT.md` y
   `docs/audits/SPRINT4C2_AUDIT_CLOSURE.md`. Esta última lleva la tabla de
   mutación del sprint 4C.2: hallazgo, mutación aplicada, prueba que falló y
   commit en que estuvo roja. Léela antes de tocar `qyro_manifest` o
   `qyro_crypto`; explica por qué varias comprobaciones están escritas de una
   forma que parece rebuscada.

   ADR-0017, ADR-0019 y ADR-0021 llevan enmiendas del sprint 4C.2. Las
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

## Regla de entrega

Al cerrar una unidad, actualizar STATUS.md con evidencia real y registrar defectos en BUGS_PENDING.md. No declarar transferencia, seguridad o compatibilidad que no hayan sido ejecutadas.

El job documental ahora falla si `Verified commit` de STATUS.md no es alcanzable
desde HEAD o queda más de 10 commits por detrás, así que STATUS.md debe
actualizarse dentro del mismo tramo de trabajo, no al final.

## Estado del protocolo y la criptografía

`qyro_protocol`, `qyro_manifest` y `qyro_crypto` están implementados y probados,
pero **nada los usa todavía**: no hay sockets, transporte ni escritura en disco.
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
- La identidad y las claves viven **solo en memoria**. No hay almacenamiento
  seguro en ninguna plataforma.
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
