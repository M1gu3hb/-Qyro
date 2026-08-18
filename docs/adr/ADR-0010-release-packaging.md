# ADR-0010: Empaquetado

- Estado: **superada** por la realidad del empaquetado — ver la enmienda al final
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

---

## Enmienda de la fase 10 — 2026-08-16

**Esta decisión no describe la v1.0.** Se deja entera y sin reescribir: una ADR
que se corrige en silencio deja de ser un registro de decisiones y pasa a ser
una descripción del presente, que es lo que ya hace `docs/release/v1.0.md`.

**Qué decía:** «APK/AAB, MSIX/ZIP y xcarchive; IPA sólo firmado;
SBOM/checksums».

**Qué existe en la v1.0:**

| Prometido | Real | Por qué |
|---|---|---|
| APK | **Sí**, firmado con la clave de `key.properties` | — |
| AAB | **No** | El AAB existe para Play Store, y esto no se publica en una tienda |
| MSIX | **No** | MSIX quiere un certificado de firma de código, que cuesta dinero |
| ZIP portable | **Sí** | Descomprimir y ejecutar |
| xcarchive / IPA | **No** | iOS está fuera de la v1.0 por ADR-0039: Xcode exige macOS |
| Checksums | **Sí** | SHA-256 del APK y del ZIP en `docs/release/v1.0.md`, y `SHA256SUMS.txt` dentro del propio paquete |
| SBOM | **No** | `Cargo.lock` (80 paquetes) y `pubspec.lock` (45) están en el repositorio y son la lista completa, verificable y ya versionada. Un SBOM generado sería un tercer archivo que decir lo mismo y quedarse obsoleto aparte |
