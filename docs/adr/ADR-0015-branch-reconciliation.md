# ADR-0015: Reconciliación de ramas divergentes

- Estado: aceptada
- Fecha: 2026-08-05
- Contexto de recuperación: `docs/audits/CLAUDE_RECOVERY_AUDIT.md`

## Contexto

Al recuperar el proyecto, `audit/baseline-hardening` (`e9ed7f3`) estaba 58
commits por delante de `main` (`e0041de`) y 2 por detrás, con merge base
`7ca3973`. `audit` concentraba prácticamente todo el trabajo de ingeniería;
`main` contenía dos commits del propietario sobre el logo.

Restricciones: no hacer force-push, no sustituir `main`, no descartar trabajo de
ninguna rama, y no abrir PR hasta tener baseline verde y documentación
sincronizada.

## Decisión

1. Trabajar en `claude/qyro-recovery-continuation-j53jgx`, recreada desde
   `origin/audit/baseline-hardening`. El prompt maestro proponía
   `claude/complete-qyro`; las instrucciones de sesión fijan este otro nombre.
   La estrategia es idéntica y el nombre queda registrado aquí para trazabilidad.
2. Integrar `origin/main` mediante **merge**, no rebase ni cherry-pick, para que
   los dos commits del propietario permanezcan alcanzables con su autoría intacta.
3. Etiquetar el estado previo antes de tocar nada: `backup/main-e0041de` y
   `backup/audit-e9ed7f3`.
4. Revisar el árbol resultante archivo por archivo en lugar de confiar en que
   «el merge no dio conflictos».
5. No modificar `main` ni `audit/baseline-hardening`.

## Justificación del merge frente al rebase

Un rebase de 58 commits sobre `main` habría reescrito toda la historia de Codex y
roto los SHA que STATUS.md, la auditoría y los runs de CI referencian. El merge
conserva ambas historias y deja una única resolución explícita y auditable.

## Consecuencia crítica registrada

El merge automático **no reportó conflicto** y aun así produjo un árbol
incorrecto: Git emparejó el renombrado de `main`
(`qyro-logo.png` → `no usar este logo`) con la modificación de contenido de
`audit` sobre `qyro-logo.png`, y dejó el archivo marcado como inutilizable con
los bytes del logo real.

Se corrigió restaurando ese archivo byte a byte desde `origin/main`. La lección
queda fijada como regla operativa: **ausencia de conflicto no es prueba de
corrección** cuando una rama renombra y la otra modifica el mismo archivo. La
verificación por checksum de ADR-0014 existe precisamente para detectarlo.

## Estado de disparo de CI

`ci.yml` se dispara por push a `main` y por pull request; `android-runtime.yml` e
`ios-runtime.yml`, por push a `audit/baseline-hardening` o `workflow_dispatch`.
Empujar la rama de trabajo no dispara ningún workflow, lo que mantiene bajo el
ruido de correos. Obtener evidencia de iOS o Android exige `workflow_dispatch`
explícito o abrir el pull request.
