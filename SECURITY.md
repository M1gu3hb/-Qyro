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

No se han ejecutado cargo audit, fuzzing, KAT de criptografía, prueba de tráfico ni revisión externa porque las dependencias/funciones aún no existen. Deben añadirse antes de afirmar seguridad de transferencia.
