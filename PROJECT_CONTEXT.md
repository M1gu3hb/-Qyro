# Contexto del proyecto

## Visión

Qyro mueve archivos, carpetas y texto de forma privada y directa entre dispositivos cercanos, sin cuentas, nube ni internet.

## Plataformas

Android, iOS y Windows son obligatorias y ya tienen runners Flutter oficiales. Linux/web quedan fuera del alcance inicial.

## Alcance

Selección múltiple, LAN/IP/QR, cifrado, integridad, pausa/reanudación, historial local opcional, confianza revocable, modo óptico FEC e integraciones nativas graduales.

## Fuera de alcance inicial

Backend, cuentas, almacenamiento remoto, publicación automática, Bluetooth principal y QUIC crítico.

## Decisiones del propietario

Qyro/Kiro; monorepo Flutter/Rust; Apache-2.0 provisional; logo PNG suministrado; trabajo de repositorio solo en GitHub.

## Estado actual

Base técnica del Hito 0 comprobada. La app compila debug para Android/Windows e iOS sin firma. Boot/Home y tests existen. ABI Rust existe pero Dart aún no la carga. No hay selección, red, cifrado, persistencia, transferencia ni óptico.
