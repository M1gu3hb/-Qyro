# Instrucciones canónicas para agentes

## Objetivo

Construir Qyro: transferencia privada de archivos entre Android, iOS y Windows, con un monorepo Flutter/Rust y capacidades nativas aisladas por plataforma.

## Reglas no negociables

- Sin cuentas, anuncios, analítica, trackers, nube, backend o relay obligatorio.
- Ningún archivo, nombre, ruta, clave o diagnóstico sale a internet.
- Toda recepción de un peer desconocido requiere confirmación.
- Cifrado, integridad, reanudación y escritura temporal preceden al rename final.
- No inventar builds, pruebas, rendimiento, compatibilidad, propiedad de marca ni seguridad.
- No copiar interfaz o identidad de proyectos de referencia.
- No agregar Firebase, Supabase, AWS, Azure, Sentry, Crashlytics ni equivalentes.
- No agregar GPL, AGPL, LGPL o licencia desconocida sin autorización explícita.

## Arquitectura

- Flutter/Dart: presentación, navegación, accesibilidad y coordinación de UI.
- Rust: protocolo, criptografía, manifiestos, streaming, persistencia y selección de transporte.
- qyro_ffi: frontera nativa estrecha; ninguna lógica de negocio vive allí.
- Kotlin, Swift y C++/WinRT: capacidades exclusivas del sistema operativo.
- Trabajo pesado fuera del hilo de UI.
- Dependencias apuntan hacia el dominio; el dominio no conoce Flutter ni APIs nativas.

## Flujo obligatorio

1. Leer HANDOFF.md y NEXT_STEPS.md.
2. Revisar este archivo y ADR relevantes.
3. Ejecutar doctor cuando exista y las pruebas de base.
4. Para comportamiento nuevo: test rojo, comprobar el fallo correcto, implementación mínima, verde, refactor.
5. Ejecutar formato, lint, análisis y tests tras cada cambio.
6. Actualizar CHANGELOG.md, PROJECT_CONTEXT.md, HANDOFF.md, FILE_MAP.md y NEXT_STEPS.md cuando cambie el estado.
7. Registrar fallos reales en BUGS_PENDING.md; no esconderlos en TODO.
8. Commit Conventional Commit, pequeño y coherente.

## Comandos actuales

    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cd apps/qyro
    flutter pub get
    dart format --output=none --set-exit-if-changed .
    flutter analyze
    flutter test

Los scripts doctor, bootstrap, test_all y build_all son trabajo P0 pendiente; no documentar resultados de ellos antes de implementarlos.

## Seguridad y dependencias

- Usar primitivas revisadas; nunca criptografía propia.
- Limitar longitudes antes de reservar memoria.
- Validar rutas, tamaños, versiones, sesiones, expiraciones y nonces.
- Redactar logs; nunca claves, contenido ni rutas completas.
- Fijar versiones, actualizar docs/LICENSE_AUDIT.md y THIRD_PARTY_NOTICES.md.
- Revisar licencia, actividad, arquitectura y issues antes de integrar una referencia.

## Definición de terminado

Una función está terminada solo si el comportamiento real existe, sus tests relevantes pasan, se validó en plataformas declaradas, la seguridad fue revisada y la documentación refleja límites. UI, mock, placeholder o TODO no cuentan.

## Handoff

Registrar fecha UTC, rama, commit, hito, funciones reales, comandos/resultados, builds, bloqueos, archivos modificados y una única próxima tarea verificable. Nunca usar porcentajes inventados.
