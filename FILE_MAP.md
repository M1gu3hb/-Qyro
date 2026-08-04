# Mapa de archivos

| Ruta | Propósito | Responsable | Puede depender de | No puede depender de |
|---|---|---|---|---|
| apps/qyro/lib | UI/estado Flutter | Presentación | paquetes Dart, frontera FFI | detalles internos de transporte |
| apps/qyro/android | runner/plugin Kotlin | Android | APIs Android, contratos | dominio concreto |
| apps/qyro/ios | runner/plugin Swift | iOS | APIs Apple, contratos | dominio concreto |
| apps/qyro/windows | runner/plugin C++ | Windows | Win32/WinRT, contratos | dominio concreto |
| rust/crates/qyro_core | dominio compartido | Núcleo | std | Flutter, plataforma |
| rust/crates/qyro_ffi | ABI estrecha | Integración | qyro_core | UI, reglas de negocio |
| native | módulos nativos futuros | Plataforma | APIs oficiales | dominio concreto |
| config | branding/features ejemplo | Producto | nada | secretos |
| design | fuentes de marca/referencia | Diseño | assets del propietario | logos de terceros |
| docs/adr | decisiones | Arquitectura | contexto | código ejecutable |
| docs/security | privacidad | Seguridad | threat model | secretos |
| .github/workflows | CI/builds remotos | Release/QA | actions auditadas | credenciales embebidas |
| release | staging/checksums/SBOM | Release | builds comprobados | certificados |
