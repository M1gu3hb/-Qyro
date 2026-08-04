# ADR-0004: Seguridad de red

- Estado: aceptada para desarrollo inicial
- Fecha: 2026-08-04

## Contexto

TLS por sí solo no unifica LAN/óptico ni confianza de peers.

## Decisión

TLS 1.3 más cifrado de contenido, SAS/QR y claves del sistema.

## Alternativas

HTTP local; TLS solamente.

## Consecuencias

Defensa coherente entre transportes.

## Riesgos

Complejidad de ciclo de claves; requiere auditoría.
