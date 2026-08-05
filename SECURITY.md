# Seguridad

Estado: diseño inicial; no hay transferencia real.

## Principios

- Primitivas revisadas, separación de identidad/sesión/contenido y nonces únicos.
- TLS 1.3 para red; capa de contenido consistente entre transportes.
- Evaluar X25519, HKDF-SHA256, XChaCha20-Poly1305 o AES-GCM, Ed25519, BLAKE3 y SHA-256.
- Claves privadas en Keystore, Keychain o DPAPI/CNG.
- Metadata, manifest, nombres y rutas cifrados.
- Longitudes y conteos se validan antes de reservar memoria.
- Temporales .qyro-part y rename solo tras autenticidad, tamaño, flush e integridad.
- Sin autoaceptación de peers desconocidos.
- Logs locales, rotativos y redactados; sin contenido, claves o rutas completas.

## Reporte

No publicar vulnerabilidades con datos sensibles en issues. Hasta definir SECURITY.md con canal privado del propietario, describir solo el impacto mínimo y solicitar coordinación.

## Estado de auditoría

`cargo audit` es obligatorio en CI desde el sprint 2 y pasa: el workspace no
tiene dependencias externas, así que la ruta de parsing no expone código de
terceros. Las amenazas sobre los parsers y su cobertura están en
`docs/security/parser-threats.md`.

No se ha ejecutado una campaña de fuzzing: existen targets `cargo-fuzz` y un
corpus de 65 entradas que CI reproduce como smoke, lo que protege contra
regresiones conocidas pero no explora entradas nuevas. Tampoco hay KAT de
criptografía, prueba de tráfico ni revisión externa, porque esas funciones aún
no existen. Deben añadirse antes de afirmar seguridad de transferencia.
