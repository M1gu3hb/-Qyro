# ADR-0013: Coordinación determinista del arranque

- Estado: aceptada
- Fecha: 2026-08-04

## Contexto

La pantalla de boot anterior avanzaba con un `AnimationController` de 5.5 segundos, independientemente de que branding, assets o ABI estuvieran disponibles. Eso podía declarar la interfaz lista después de un fallo nativo.

## Decisión

`StartupCoordinator` es una máquina de estados observable e independiente del render. Ejecuta en orden:

1. Validación/carga del branding generado.
2. Comprobación de assets requeridos.
3. Apertura del puente nativo y lectura compatible de `QYRO/1`.
4. Inicialización mínima de interfaz.

Cada tarea se registra como completada únicamente después de resolver. El coordinador conserva reduced motion y ciclo de vida, admite cancelación y retry, y aplica un timeout total configurable de ocho segundos. Una tarea pendiente mantiene el boot; failure o timeout exponen diagnóstico sanitizado y no navegan al Home.

La UI depende de `NativeBridge`, no de `dart:ffi`. El adaptador `QyroNativeApi` queda en infraestructura.

## Consecuencias

La duración visual y la preparación funcional quedan separadas: terminar la animación no implica estar ready, y estar ready no elimina la duración visual mínima. Volver de background solo actualiza lifecycle; no reinicia un startup ya completado.

## Alternativas descartadas

- Timer fijo como fuente de verdad: puede ocultar fallos o carreras.
- Future único sin estados: no permite mostrar tarea pendiente ni diagnóstico.
- Acceso FFI desde widgets: acopla UI a plataforma y dificulta pruebas.
