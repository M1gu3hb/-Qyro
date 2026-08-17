# ADR-0004: Seguridad de red

- Estado: **superada** por ADR-0021 y ADR-0022 — ver la enmienda al final
- Fecha: 2026-08-04

## Contexto

TLS por sí solo no unifica LAN/óptico ni confianza de peers.

## Decisión

TLS 1.3 más cifrado de contenido, SAS/QR y claves del sistema.

## Alternativas

HTTP local; TLS solamente.

## Consecuencias

Defensa coherente entre transportes.

## Riesgos

Complejidad de ciclo de claves; requiere auditoría.

---

## Enmienda de la fase 10 — 2026-08-16

**Esta decisión no describe la v1.0.** Se deja entera y sin reescribir: una ADR
que se corrige en silencio deja de ser un registro de decisiones y pasa a ser
una descripción del presente, que es lo que ya hace `docs/release/v1.0.md`.

**Qué decía:** «TLS 1.3 más cifrado de contenido, SAS/QR y claves del sistema».

**Qué existe:** **no hay TLS.** El transporte es un socket TCP desnudo
(ADR-0028) y toda la protección la da el protocolo propio: handshake autenticado
de cuatro mensajes con firma Ed25519 sobre un transcript que incluye ambas
identidades, ambos nonces y ambas efímeras (ADR-0021), y ChaCha20-Poly1305 por
frame con claves separadas por dirección (ADR-0022).

**Por qué se fue TLS.** Un TLS sin PKI necesita certificados que alguien tiene
que emitir, y entre dos aparatos sin cuentas ni servidor no hay quién los emita;
lo que queda es TLS con certificados autofirmados y la autenticación real hecha
aparte — es decir, el handshake propio, con TLS encima aportando complejidad y
una superficie de configuración que nadie iba a auditar.

**Lo que se pierde y se dice:** no hay las décadas de escrutinio de una pila TLS
madura. Lo que sostiene la elección es que la parte criptográfica es pequeña,
está construida sobre primitivas estándar, y cada fila del modelo de amenazas
nombra la prueba que la cubre.

**SAS/QR:** no hay QR ni cámara (QYR-0348). Lo que existe es la comparación de
huella en voz alta y el código de emparejamiento tecleado (ADR-0035).
