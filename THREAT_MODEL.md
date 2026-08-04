# Modelo de amenazas

## Activos

Contenido, nombres/rutas, claves, identidad local, confianza, historial, temporales y disponibilidad.

## Adversarios

- atacante en la misma LAN;
- MITM;
- peer malicioso;
- cámara u observador óptico;
- dispositivo confiable comprometido;
- proceso local con acceso indebido;
- inputs corruptos o gigantes.

## Amenazas y controles previstos

| Amenaza | Control |
|---|---|
| MITM/replay | TLS 1.3, huella/SAS, sesión, nonce, expiración |
| Manifest/path traversal | rutas relativas normalizadas, límites, rechazo de symlink/junction |
| Corrupción | AEAD por chunk y SHA-256 final |
| Memoria/disco agotados | límites previos, streaming, cuota y preflight |
| Decompression bomb | ratio/tamaño máximo y streaming |
| QR de otra sesión | IDs, versión y autenticación |
| Clave óptica visible | peer confiable o emparejamiento bidireccional; advertencia en una cámara |
| Logs/backups | redacción, retención y cifrado local |
| Cámara activa | indicador y permiso just-in-time |
| Discovery rastreable | alias y session ID rotatorios, metadata mínima |
| DoS | timeouts, rate limits, presupuestos y cancelación |

## Riesgo residual

Un observador que ve contenido y clave en un flujo óptico unidireccional puede romper la confidencialidad. iOS puede suspender tareas. Un dispositivo comprometido puede leer datos ya descifrados. Estas limitaciones deben mostrarse sin lenguaje engañoso.
