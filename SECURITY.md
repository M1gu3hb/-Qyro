# Seguridad

Estado: diseño inicial; no hay transferencia real.

## Principios

- Primitivas revisadas, separación de identidad/sesión/contenido y nonces únicos.
- TLS 1.3 para red; capa de contenido consistente entre transportes.
- Evaluar X25519, HKDF-SHA256, XChaCha20-Poly1305 o AES-GCM, Ed25519, BLAKE3 y SHA-256.
- Claves privadas en Keystore, Keychain o DPAPI/CNG.
- Metadata, manifest, nombres y rutas cifrados.
- Longitudes y conteos se validan antes de reservar memoria.
- Un frame no puede declarar `ENCRYPTED` sin llevar tag: solo el sellado activa
  ese flag, y produce el tag en la misma operación.
- El nombre visible se deriva de la ruta; el peer no envía uno aparte.
- Todo archivo lleva digest final, incluidos los de cero bytes.
- Se rechazan rutas que un sistema de archivos real plegaría en una sola.
- Temporales .qyro-part y rename solo tras autenticidad, tamaño, flush e integridad.
- Sin autoaceptación de peers desconocidos.
- Logs locales, rotativos y redactados; sin contenido, claves o rutas completas.

## Reporte

No publicar vulnerabilidades con datos sensibles en issues. Hasta definir SECURITY.md con canal privado del propietario, describir solo el impacto mínimo y solicitar coordinación.

## Estado de auditoría

`cargo audit` es obligatorio en CI desde el sprint 2 y pasa. Este apartado decía
además que «el workspace no tiene dependencias externas»: era cierto hasta el
sprint 4A e incorrecto desde entonces. `qyro_crypto` depende de la pila dalek y
de RustCrypto, y `qyro_manifest` de `unicode-normalization`. El inventario por
crate, versión y licencia está en `docs/LICENSE_AUDIT.md`; la ruta de parsing de
`qyro_protocol` sigue sin dependencias de terceros.

**Sí hay KAT de criptografía**, en contra de lo que decía este archivo:

| Primitiva | Vectores | Archivo |
|---|---|---|
| Ed25519 | RFC 8032 §7.1, las cinco pruebas | `docs/security/test-vectors/rfc8032-ed25519.json` |
| X25519 | RFC 7748 §5 y §6.1 | `docs/security/test-vectors/rfc7748-x25519.json` |
| HMAC-SHA-256 | RFC 4231, los siete casos | `docs/security/test-vectors/rfc4231-hmac-sha256.json` |
| Identidad Qyro | construcción propia | `docs/security/test-vectors/identity-v1.json` |
| Handshake Qyro | ejecución completa | `docs/security/test-vectors/handshake-v1.json` |

Lo que **no** existe: campaña real de fuzzing —hay targets `cargo-fuzz` y un
corpus que CI reproduce como smoke, lo que protege contra regresiones conocidas
pero no explora entradas nuevas—, prueba de tráfico, revisión externa y
auditoría criptográfica independiente. Deben añadirse antes de afirmar seguridad
de transferencia.
