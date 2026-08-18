# Instrucciones canónicas para agentes

## Fuente de verdad

Leer [STATUS.md](STATUS.md) antes de trabajar. STATUS.md es la única fuente de verdad para funciones implementadas, plataformas ejecutadas, tests, artefactos y bloqueos. HANDOFF.md enlaza ese estado; NEXT_STEPS.md mantiene el backlog.

## Objetivo

Construir Qyro como transferencia privada entre Android, iOS y Windows con monorepo Flutter/Rust. El alcance actual llega hasta la criptografía en memoria —identidad, handshake autenticado y AEAD de frames— y **no** incluye transferencia, transporte, LAN, base de datos, almacenamiento seguro ni modo óptico. Cifrar un frame en memoria no acerca ninguno de ellos por sí solo: Qyro sigue sin transferir archivos.

## Reglas no negociables

- Sin cuentas, anuncios, analítica, trackers, nube, backend o relay obligatorio.
- No inventar builds, pruebas, rendimiento, compatibilidad, marca ni seguridad.
- No copiar interfaces, identidad o assets de proyectos de referencia.
- No agregar servicios remotos ni criptografía propia.
- No agregar GPL, AGPL, LGPL o licencia desconocida sin autorización.
- Una función está terminada solo con implementación, test, plataforma y documentación coherentes.

## Arquitectura

- Flutter/Dart: presentación, accesibilidad y coordinación.
- Rust: dominio compartido y protocolo futuro.
- qyro_ffi: frontera estrecha sin lógica de negocio.
- Kotlin, Swift y C++: capacidades exclusivas de plataforma.
- Dependencias hacia el dominio; el dominio no conoce Flutter ni APIs nativas.
- Cambios de arquitectura requieren ADR.

## Flujo obligatorio

1. Leer STATUS.md, HANDOFF.md y NEXT_STEPS.md.
2. Ejecutar doctor y baseline relevante.
3. Para comportamiento nuevo: test rojo, fallo correcto, implementación mínima, verde y refactor.
4. Ejecutar formato, análisis y tests tras cada cambio.
5. Actualizar STATUS.md y documentos afectados sin duplicar estado.
6. Registrar fallos reales en BUGS_PENDING.md.
7. Usar commits Conventional Commits pequeños.

## Comandos

    bash scripts/doctor.sh
    bash scripts/bootstrap.sh
    bash scripts/test_all.sh

PowerShell dispone de doctor.ps1, bootstrap.ps1 y test_all.ps1 equivalentes. La existencia y estado actual de scripts se registra únicamente en STATUS.md.

## Seguridad

- Limitar longitudes antes de reservar memoria.
- Validar rutas, tamaños, versiones y entradas externas.
- Redactar diagnósticos; no exponer claves, contenido ni rutas completas.
- Fijar versiones y actualizar LICENSE_AUDIT/THIRD_PARTY_NOTICES.
