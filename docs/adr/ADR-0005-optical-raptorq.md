# ADR-0005: RaptorQ óptico

- Estado: **no implementada** — fuera de la v1.0, ver la enmienda al final
- Fecha: 2026-08-04

## Contexto

QR animado pierde, duplica y reordena frames.

## Decisión

Evaluar cberner/raptorq por epochs; integrar solo tras targets/benchmarks.

## Alternativas

Secuencia numerada; RLNC; libcimbar.

## Consecuencias

Recuperación sin todos los frames exactos.

## Riesgos

ARM y memoria; parámetros deben medirse.

---

## Enmienda de la fase 10 — 2026-08-16

**Esta decisión no describe la v1.0.** Se deja entera y sin reescribir: una ADR
que se corrige en silencio deja de ser un registro de decisiones y pasa a ser
una descripción del presente, que es lo que ya hace `docs/release/v1.0.md`.

**Qué decía:** evaluar `cberner/raptorq` para transferencia óptica y
integrarlo sólo tras targets y benchmarks.

**Qué existe:** nada. No hay `raptorq` en `Cargo.lock`, no hay canal óptico y no
hay cámara en la aplicación. La condición que la propia ADR ponía —integrar sólo
tras benchmarks— nunca se cumplió porque la evaluación no se hizo.

**Por qué no se hizo:** un canal óptico unidireccional obliga a poner la clave
delante de una cámara, y el modelo de amenazas ya recogía que un observador que
ve contenido y clave rompe la confidencialidad. Resolver eso bien es un proyecto,
no una fase, y la v1.0 no lo necesita: el camino manual funciona en el 100 % de
las redes.
