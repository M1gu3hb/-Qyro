# Base de datos local

Estado: diseñada, no implementada.

Motor previsto: SQLite desde Rust, sujeto a prueba en Android, iOS y Windows. No guarda contenido de archivos.

## Tablas

- devices: identidad pública, fingerprint, alias, plataforma, confianza y fechas.
- transfers: dirección, peer, transporte, estado, bytes, items, tiempos, error, versión y hash del manifest.
- transfer_items: ruta/nombre cifrados, tamaño, progreso, checksum, estado y referencias temporales/finales.
- resume_state: transfer, item, bitmap o chunk, estado, checksum y actualización.
- settings: clave, valor y actualización.
- schema_migrations: versión, fecha y checksum.

Índices previstos: fingerprint único activo; transfers por started_at/peer/state; items por transfer/state; resume por transfer/item.

## Seguridad y retención

Rutas, nombres e historial sensible se cifran con clave local protegida por Keystore, Keychain o DPAPI/CNG. El historial puede desactivarse, eliminarse por entrada o borrarse totalmente. Los dispositivos pueden revocarse. Diagnósticos se exportan redactados.

## Recuperación

Migraciones transaccionales e idempotentes. Chunks confirmados se persisten antes de anunciar ACK. Tras crash se revalidan temporales y checksums antes de reanudar. No hay migraciones todavía.
