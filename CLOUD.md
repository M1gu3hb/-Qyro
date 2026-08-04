# Política de ausencia de nube

Qyro no usa nube, backend central, almacenamiento remoto, relay predeterminado ni telemetría. El funcionamiento normal debe ser local y peer-to-peer.

Ningún agente puede añadir Firebase, Supabase, AWS, Azure, Sentry, Analytics, Crashlytics o equivalentes sin autorización explícita.

Una futura propuesta de relay requiere simultáneamente:

- feature flag desactivado por defecto;
- consentimiento explícito;
- cifrado de extremo a extremo;
- servidor opcional y autohospedable;
- ADR;
- revisión de privacidad y amenaza;
- prueba de que Qyro sigue funcionando sin él.
