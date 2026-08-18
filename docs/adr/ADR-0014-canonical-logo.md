# ADR-0014: Ruta canónica del logo de Qyro

- Estado: aceptada
- Fecha: 2026-08-05
- Contexto de recuperación: `docs/audits/CLAUDE_RECOVERY_AUDIT.md`

## Contexto

`main` y `audit/baseline-hardening` divergieron sobre el mismo activo de marca.

En el merge base `7ca3973` existía `design/brand/source/qyro-logo.png` con un PNG
provisional truncado (`sha256:52107d9e…258d`).

Después:

- El propietario, en `main`, añadió el logo real como
  `design/brand/source/logo.png` (`sha256:e8413410…4f39`) y renombró el
  provisional a `design/brand/source/no usar este logo` para marcarlo inutilizable.
- Codex, en `audit`, sustituyó el contenido de `design/brand/source/qyro-logo.png`
  por **esos mismos bytes reales** (`41f13a7`).

Es decir, ambas ramas ya contenían el logo correcto; solo discrepaban en el
nombre. Además, `git merge` resolvió el caso **sin conflicto pero mal**: combinó
el renombrado de `main` con la modificación de `audit` y dejó el archivo
«no usar este logo» conteniendo el logo real.

## Decisión

1. La ruta canónica de producción es **`design/brand/source/logo.png`**, el
   nombre que eligió el propietario, con `sha256:e8413410…4f39`.
2. `design/brand/source/no usar este logo` se conserva con sus bytes originales
   (`sha256:52107d9e…258d`) por trazabilidad, y no puede entrar en assets,
   previews, generación ASCII, empaquetado ni releases.
3. `design/brand/source/qyro-logo.png` desaparece: queda absorbido por el
   renombrado del propietario.
4. `apps/qyro/assets/brand/qyro-logo.png` sigue siendo la copia empaquetada y
   debe ser byte a byte idéntica al logo canónico. Se mantiene su nombre para no
   tocar `pubspec.yaml` ni los contratos de `bootstrap`; su identidad queda fijada
   por checksum en vez de por nombre.
5. El PNG no se dibuja como logo principal del arranque. Es únicamente la fuente
   desde la que `tools/logo_ascii_generator` produce el ASCII determinista.

## Cumplimiento

`tools/logo_ascii_generator/test_logo_ascii_generator.py` fija la decisión con
cinco pruebas:

- el logo canónico tiene el SHA-256 esperado;
- el activo empaquetado es byte a byte idéntico al canónico;
- `logo_ascii.json` apunta a `design/brand/source/logo.png` y fija su checksum;
- el archivo rechazado sigue existiendo y conserva sus bytes originales;
- ningún archivo bajo `apps/qyro/assets` tiene los bytes del archivo rechazado.

El generador toma `design/brand/source/logo.png` por defecto y `--check` falla si
los activos generados quedan obsoletos.

## Consecuencias

- No se pierde ningún cambio del propietario ni de Codex.
- El marcador rechazado no puede volver a producción sin romper pruebas.
- Regenerar los activos tras el cambio de ruta modificó una sola línea
  (`"source"`), lo que confirma que el arte ASCII ya derivaba del logo real.
- Autoría, licencia y permiso de distribución del logo siguen **pendientes** y
  continúan bloqueando cualquier empaquetado público.
