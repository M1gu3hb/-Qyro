# Contexto del proyecto

## Visión

Qyro resuelve el envío privado y directo de archivos, carpetas y texto entre dispositivos cercanos sin cuentas, nube ni internet.

## Usuarios y plataformas

Personas que necesitan mover datos entre Android, iOS y Windows con control local. Las tres plataformas son obligatorias; Linux y web no forman parte del alcance inicial.

## Alcance

- selección múltiple y carpetas;
- LAN, IP manual y emparejamiento QR;
- cifrado, integridad, pausa y reanudación;
- historial local opcional y dispositivos confiables revocables;
- modo óptico con QR animado y FEC;
- integraciones nativas graduales.

## Fuera de alcance inicial

- backend central, almacenamiento remoto o cuentas;
- publicación automática en tiendas;
- Bluetooth como transporte principal;
- QUIC como dependencia crítica;
- promesas de background indefinido en iOS.

## Decisiones del propietario

- Nombre visible provisional: Qyro, pronunciado Kiro.
- Monorepo Flutter/Rust.
- Android, iOS y Windows.
- Código abierto; Apache-2.0 provisional.
- Logo PNG suministrado el 2026-08-04.
- Trabajo de repositorio exclusivamente en GitHub.

## Terminología

Peer: dispositivo remoto. Offer: propuesta antes de aceptar. Item: archivo o carpeta del manifest. Chunk: unidad reanudable. Epoch: bloque óptico FEC. SAS: código corto de autenticación.

## Estado actual

Hito 0 en progreso. Rust y pruebas Flutter pasan en CI. Existen estado de arranque, motor determinista y ABI C mínima. No existen todavía runners de plataforma, UI ejecutable, persistencia, red, cifrado ni transferencia real.
