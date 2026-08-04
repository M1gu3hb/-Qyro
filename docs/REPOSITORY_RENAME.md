# Recomendación de renombrado del repositorio

El remoto actual es `M1gu3hb/-Qyro`. Se recomienda renombrarlo a `M1gu3hb/qyro` después de integrar y estabilizar esta rama. Esta sesión no cambia el remoto.

## Impacto esperado

GitHub mantiene redirecciones para URLs web y operaciones Git durante un periodo, pero integraciones externas pueden conservar el nombre antiguo. Deben revisarse:

- Badges y enlaces Markdown.
- URLs de clone/fetch en documentación.
- GitHub Actions y scripts que contengan el nombre literal.
- Webhooks, tokens con scope por repositorio y reglas externas.
- Protección de rama, Pages, paquetes y releases.
- Marcadores locales `origin`.

## Comando sugerido

    gh repo rename qyro --yes

Ejecutarlo solo con autorización explícita del propietario y desde el repositorio correcto.

## Checklist posterior

- [ ] Confirmar que https://github.com/M1gu3hb/qyro responde.
- [ ] Confirmar redirección desde la URL antigua.
- [ ] Actualizar badges, enlaces y clone URLs.
- [ ] Revisar workflows, secrets, environments y reglas.
- [ ] Actualizar integraciones externas y webhooks.
- [ ] Ejecutar CI y builds desde el nuevo remoto.
- [ ] Actualizar STATUS.md, RELEASES.md y documentación de soporte.
