# Fuente de marca

## Logo canónico

`logo.png` es el único logo autorizado para producción. Lo suministró el
propietario el 2026-08-04 en la rama `main`. Antes de publicación debe
registrarse autoría, licencia y permiso de distribución. No sustituirlo por
logos de terceros.

- SHA-256: `e8413410d53958fe399c3e37ed73e85030b41c1dbe456ca3a5bad2491e6d4f39`
- Consumidores: `tools/logo_ascii_generator/generate.py` (fuente para el ASCII
  de arranque) y `apps/qyro/assets/brand/qyro-logo.png` (copia byte a byte).
- El checksum está fijado en `apps/qyro/assets/generated/logo_ascii.json` y se
  comprueba en `tools/logo_ascii_generator/test_logo_ascii_generator.py`.

El PNG no se dibuja como logo principal del arranque: solo es la fuente desde la
que se genera el ASCII determinista.

## Archivo rechazado

`no usar este logo` es el marcador provisional anterior. El propietario lo
renombró para marcarlo como inutilizable y se conserva con sus bytes originales
por trazabilidad.

- SHA-256: `52107d9e88fcc50838e7c9fcef928592529eea6aaed367597fcfc4547488258d`
- No debe entrar en assets, previews, generación ASCII, empaquetado ni releases.
- Una prueba comprueba que sus bytes no aparecen en `apps/qyro/assets`.

La decisión completa está en `docs/adr/ADR-0014-canonical-logo.md`.
