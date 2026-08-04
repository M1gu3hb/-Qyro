# Estrategia de pruebas

El inventario ejecutado y sus runs viven en [STATUS.md](STATUS.md).

## Regla

Todo comportamiento comprobable sigue rojo → verde → refactor. El test debe fallar por la causa prevista antes de producción.

## Capas obligatorias del baseline

- Rust: formato, Clippy sin warnings y tests.
- Flutter: formato, análisis y tests.
- FFI: biblioteca real en la plataforma disponible.
- Scripts: contratos Bash y PowerShell.
- Documentación: consistencia con STATUS.md.
- Seguridad/licencias: checks ejecutables, no afirmaciones Markdown.
- Intro: unit/widget/golden según corresponda.

## Honestidad de plataforma

- Un build no demuestra ejecución.
- Una biblioteca dentro de un APK no demuestra carga runtime.
- Un Runner.app sin firma no demuestra enlace FFI.
- Un test omitido no cuenta como éxito.
- N/A solo es válido para una plataforma no aplicable o funcionalidad futura registrada en STATUS.md.

## Rendimiento

No publicar cifras sin máquina, modo, versión, resolución y metodología. Los tests pueden detectar regresiones graves, no prometer FPS.
