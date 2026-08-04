# Contexto del proyecto

## Fuente de verdad

El estado actual, evidencia, plataformas y bloqueos viven exclusivamente en [STATUS.md](STATUS.md).

## Visión

Qyro pretende mover archivos, carpetas y texto de forma privada y directa entre dispositivos cercanos, sin cuentas, nube ni internet. La visión no implica que esas funciones existan hoy.

## Plataformas

Android, iOS y Windows son obligatorias. Linux solo se usa como host de CI.

## Arquitectura aprobada

- Monorepo Flutter/Rust.
- Flutter para UI y coordinación.
- Rust para dominio/protocolo futuro.
- Integraciones nativas aisladas por plataforma.
- Apache-2.0 y los identificadores siguen provisionales.
- El logo PNG fue suministrado por el propietario, con autoría/licencia pendiente.

## Fuera del sprint baseline

Transferencia, selección, manifest, red, cifrado, persistencia, QR óptico, Wi-Fi Direct y Bluetooth. Consultar NEXT_STEPS.md para el orden futuro.
