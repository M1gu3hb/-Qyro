# Índice de decisiones

- ADR-0001: Flutter + Rust para multiplataforma.
- ADR-0002: Rust como núcleo compartido.
- ADR-0003: protocolo QYRO/1 versionado.
- ADR-0004: TLS 1.3 más cifrado de contenido.
- ADR-0005: RaptorQ para modo óptico, pendiente de benchmark.
- ADR-0006: SQLite local desde Rust.
- ADR-0007: política sin nube.
- ADR-0008: launch estático y boot Flutter.
- ADR-0009: Bluetooth limitado a control/experimental.
- ADR-0010: paquetes por plataforma y releases reproducibles.
- ADR-0012: branding generado en tiempo de build.
- ADR-0013: StartupCoordinator y tareas obligatorias de arranque.
- ADR-0014: ruta canónica del logo de Qyro.
- ADR-0015: reconciliación de ramas divergentes.
- ADR-0016: framing binario de QYRO/1.
- ADR-0017: codificación canónica del manifest.
- ADR-0018: errores estructurales frente a eventos semánticos.
- ADR-0019: nombre visible derivado de la ruta.
- ADR-0020: fundación de identidad de dispositivo.
- ADR-0021: handshake autenticado de cuatro mensajes.
- ADR-0022: cifrado autenticado de frames QYRO/1.

El sprint 4B.1 cerró el handshake sin cambiar ninguna decisión: unificó el
`SessionId` en ocho bytes, añadió `ResponderFinishPending`, sacó las claves de
la API pública y comprometió vectores. Está registrado como enmienda dentro de
ADR-0021.

No existe ADR-0011.

Consulta docs/adr/.
