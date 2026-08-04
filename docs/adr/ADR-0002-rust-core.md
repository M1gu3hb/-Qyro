# ADR-0002: Núcleo Rust

- Estado: aceptada para desarrollo inicial
- Fecha: 2026-08-04

## Contexto

Streaming, seguridad y FEC requieren control de memoria/rendimiento.

## Decisión

Reglas de dominio viven en crates Rust pequeños.

## Alternativas

Dart puro; C++ compartido.

## Consecuencias

Una implementación de protocolo para todas las plataformas.

## Riesgos

Tamaño de builds y cross-compilation.
