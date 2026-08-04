# ADR-0006: SQLite local

- Estado: aceptada para desarrollo inicial
- Fecha: 2026-08-04

## Contexto

Resume, confianza e historial requieren persistencia crash-safe.

## Decisión

SQLite desde Rust con migraciones y campos sensibles cifrados.

## Alternativas

Archivos JSON; DB por plataforma.

## Consecuencias

Semántica uniforme y transacciones.

## Riesgos

Bindings/compilación móvil y gestión de clave.
