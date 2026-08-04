# Mapa de archivos

| Ruta | Propósito | Responsable | Puede depender de | No puede depender de |
|---|---|---|---|---|
| apps/qyro | Aplicación Flutter | Presentación | paquetes Dart, qyro_ffi | detalles internos de transporte |
| rust/crates/qyro_core | dominio compartido | Núcleo | std | Flutter, plataforma |
| rust/crates/qyro_ffi | ABI estrecha | Integración | qyro_core | UI, reglas de negocio |
| native | capacidades por SO | Plataforma | APIs oficiales, contratos | dominio concreto |
| config | branding/features de ejemplo | Producto | nada | secretos |
| design | fuentes de marca/referencia | Diseño | assets del propietario | logos de terceros |
| docs/adr | decisiones inmutables | Arquitectura | contexto | código ejecutable |
| docs/security | procedimientos de privacidad | Seguridad | threat model | secretos |
| .github/workflows | verificación remota | Release/QA | scripts públicos fijados | credenciales embebidas |
| release | manifiestos y artefactos generados | Release | builds comprobados | certificados |
