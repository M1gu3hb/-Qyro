# Mapa de archivos

| Ruta | Propósito | Responsable | Puede depender de | No puede depender de |
|---|---|---|---|---|
| apps/qyro/lib | UI/estado Flutter | Presentación | paquetes Dart, frontera FFI | detalles internos de transporte |
| apps/qyro/lib/ffi | carga/validación de ABI nativa | Integración | dart:ffi, símbolos qyro_ffi | reglas internas Rust |
| apps/qyro/android | runner y empaquetado .so | Android | APIs Android, contratos | dominio concreto |
| apps/qyro/ios | runner/plugin Swift | iOS | APIs Apple, contratos | dominio concreto |
| apps/qyro/windows | runner y distribución DLL | Windows | Win32/WinRT, contratos | dominio concreto |
| rust/crates/qyro_core | dominio compartido | Núcleo | std | Flutter, plataforma |
| rust/crates/qyro_ffi | ABI estrecha | Integración | qyro_core | UI, reglas de negocio |
| scripts | diagnóstico/bootstrap/pruebas | DevEx/QA | toolchains declaradas | secretos |
| native | módulos nativos futuros | Plataforma | APIs oficiales | dominio concreto |
| config | branding/features ejemplo | Producto | nada | secretos |
| design | fuentes de marca/referencia | Diseño | assets del propietario | logos de terceros |
| docs/adr | decisiones | Arquitectura | contexto | código ejecutable |
| docs/audits | auditorías de recuperación y estado | QA | evidencia ejecutada | afirmaciones sin prueba |
| tools/branding_generator | branding validado a Dart | Producto | config/ | secretos |
| tools/logo_ascii_generator | logo canónico → ASCII determinista | Diseño/Build | design/brand/source/logo.png | assets rechazados |
| docs/security | privacidad | Seguridad | threat model | secretos |
| .github/workflows | CI/builds remotos | Release/QA | actions auditadas | credenciales embebidas |
| release | staging/checksums/SBOM | Release | builds comprobados | certificados |
