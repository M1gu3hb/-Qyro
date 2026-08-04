# ADR-0003: QYRO/1

- Estado: aceptada para desarrollo inicial
- Fecha: 2026-08-04

## Contexto

Se necesitan versión, límites, resume e integridad independientes de UI.

## Decisión

Protocolo binario propio versionado; evaluar CBOR canónico.

## Alternativas

REST sin resume; copiar LocalSend.

## Consecuencias

Control explícito y vectores/fuzz obligatorios.

## Riesgos

Diseño incorrecto sería costoso; mantener experimental hasta vectores.
