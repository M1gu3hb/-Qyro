# ADR-0008: Arranque en dos capas

- Estado: aceptada para desarrollo inicial
- Fecha: 2026-08-04

## Contexto

Se debe evitar blanco y mostrar inicialización real.

## Decisión

Launch estático nativo seguido de boot Flutter saltable/reduced motion.

## Alternativas

Animación nativa completa; splash largo fijo.

## Consecuencias

Consistencia y accesibilidad.

## Riesgos

Boot puede ocultar lentitud; estados deben ser reales.
