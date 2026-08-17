# ADR-0009: Alcance Bluetooth

- Estado: **no implementada** — fuera de la v1.0, ver la enmienda al final
- Fecha: 2026-08-04

## Contexto

Rendimiento/interoperabilidad no soportan archivos grandes universales.

## Decisión

Control plane/descubrimiento primero; archivos pequeños experimentales después.

## Alternativas

Transporte MVP principal; excluirlo para siempre.

## Consecuencias

No distrae de LAN.

## Riesgos

Expectativas de usuario; comunicar límites.

---

## Enmienda de la fase 10 — 2026-08-16

**Esta decisión no describe la v1.0.** Se deja entera y sin reescribir: una ADR
que se corrige en silencio deja de ser un registro de decisiones y pasa a ser
una descripción del presente, que es lo que ya hace `docs/release/v1.0.md`.

**Qué decía:** Bluetooth como plano de control y descubrimiento primero,
archivos pequeños experimentales después.

**Qué existe:** nada de Bluetooth. El descubrimiento es mDNS/NSD (ADR-0035) y el
plano de control viaja por el mismo socket TCP que los datos.

**Por qué:** Bluetooth habría traído permisos de tiempo de ejecución en Android
—incluida la familia de localización en varias versiones— a una aplicación cuyo
manifiesto declara **una** permission. Y lo que Bluetooth resolvía de verdad era
el descubrimiento con aislamiento de cliente; eso lo resuelve el código de
emparejamiento tecleado, sin permisos y en todas las redes.
