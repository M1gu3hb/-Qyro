# ADR-0010: Empaquetado

- Estado: aceptada para desarrollo inicial
- Fecha: 2026-08-04

## Contexto

Cada SO tiene firma/distribución distinta.

## Decisión

APK/AAB, MSIX/ZIP y xcarchive; IPA solo firmado; SBOM/checksums.

## Alternativas

Un paquete universal; publicación automática.

## Consecuencias

Artefactos honestos y reproducibles.

## Riesgos

Credenciales/hardware externos bloquean algunos outputs.
