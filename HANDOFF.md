# Handoff operativo

El estado actual completo está en [STATUS.md](STATUS.md). Este archivo no duplica commits, resultados ni capacidades para evitar desincronización.

## Reanudación

1. Leer STATUS.md.
2. Confirmar la rama `claude/qyro-protocol-manifest`, que continúa
   `claude/qyro-recovery-continuation-j53jgx` (recuperación) y añade el protocolo
   y el manifest.
3. Leer `docs/audits/CLAUDE_RECOVERY_AUDIT.md` para el contexto de recuperación,
   más ADR-0014 (logo), ADR-0015 (ramas), ADR-0016 (framing) y ADR-0017
   (manifest). Las especificaciones están en `docs/protocols/`.
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
    cargo audit --deny warnings          # obligatorio desde el sprint 2
    rustfmt --check --edition 2024 rust/fuzz/fuzz_targets/*.rs
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

## Estado del protocolo

`qyro_protocol` y `qyro_manifest` están implementados y probados, pero **nada los
usa todavía**: no hay sockets, transporte, cifrado ni escritura en disco. Que el
framing exista no significa que Qyro transfiera archivos. Los botones Enviar y
Recibir siguen deshabilitados a propósito.

`rust/fuzz` es un workspace aparte y exige nightly; no entra en la compilación
del producto.
