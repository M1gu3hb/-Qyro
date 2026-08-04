# ADR-0012: Branding validado en tiempo de build

- Estado: aceptada
- Fecha: 2026-08-04

## Contexto

La configuración de marca existía solo como JSON de ejemplo y la aplicación usaba valores hardcodeados. Leer archivos fuera del bundle durante runtime sería frágil y ampliaría innecesariamente el acceso al filesystem.

## Decisión

`tools/branding_generator` valida el JSON local `config/branding.json`, con fallback de desarrollo a `config/branding.example.json`, y emite `apps/qyro/lib/generated/branding.g.dart`. La aplicación consumirá únicamente constantes generadas durante build.

El generador rechaza campos ausentes, controles Unicode peligrosos, longitudes excesivas, bundle IDs inválidos y colores fuera de `#RRGGBB`. Los marcadores `REPLACE_WITH_*` y `com.owner.qyro` producen `isProvisional = true`; sus valores visibles se vacían para no inventar propietario. `--require-final` bloquea el empaquetado público.

El archivo local `config/branding.json` permanece ignorado. El archivo Dart generado sí se versiona y CI ejecuta `--check` para detectar deriva.

## Consecuencias

Los builds son reproducibles sin acceso runtime al repositorio. Debug continúa con un banner provisional; un paquete público no puede usar el fallback. Cambiar branding exige regenerar y revisar el diff.

## Alternativas descartadas

- Leer JSON en runtime: dependencia de rutas externas y fallo tardío.
- Variables hardcodeadas por plataforma: deriva entre Flutter, Android, iOS y Windows.
- Aceptar placeholders silenciosamente: atribución falsa y riesgo de publicación.
