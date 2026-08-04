# ADR-0001: Stack multiplataforma

- Estado: aceptada para desarrollo inicial
- Fecha: 2026-08-04

## Contexto

Tres plataformas requieren UI coherente y APIs nativas.

## Decisión

Flutter/Dart para UI; Rust para lógica compartida; módulos nativos estrechos.

## Alternativas

Tres apps nativas; Electron; UI Rust.

## Consecuencias

Máximo código compartido con runners por SO.

## Riesgos

Complejidad FFI y toolchains.
