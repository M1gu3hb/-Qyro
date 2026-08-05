# Third-party notices

Qyro no reutiliza código de repositorios investigados.

## Distribuido o usado por la app

Flutter/Dart SDK (BSD-3-Clause) y sus paquetes fijados en apps/qyro/pubspec.lock. material_color_utilities, clock y fake_async usan Apache-2.0; los demás paquetes Dart actuales usan BSD-3-Clause. flutter_lints y dependencias de test son herramientas de desarrollo.

Desde el sprint 4A el workspace Rust sí tiene crates externos; ver la sección al final y `docs/LICENSE_AUDIT.md`. Los textos completos se incorporarán a release/notices/LICENSES antes del primer release.

## Activo del propietario

design/brand/source/logo.png fue suministrado por el propietario y es el único logo autorizado. Falta registrar autoría/licencia antes de publicación. El archivo design/brand/source/"no usar este logo" es el marcador anterior rechazado y no debe distribuirse.

Consulta docs/LICENSE_AUDIT.md y docs/RESEARCH_NOTES.md.

## Dependencias Rust (sprint 4A)

`qyro_manifest` usa `unicode-normalization` (MIT/Apache-2.0, unicode-rs) para
normalización canónica real.

`qyro_crypto` usa la pila Ed25519/X25519 de dalek-cryptography (BSD-3-Clause) y
RustCrypto (MIT/Apache-2.0): `ed25519-dalek`, `x25519-dalek`, `curve25519-dalek`,
`sha2`, `hkdf`, `hmac`, `chacha20poly1305`, `zeroize`, `subtle` y `getrandom`.
Solo para pruebas añade `serde_json` (MIT/Apache-2.0) y su cierre, que no se
enlaza en la biblioteca. El detalle por crate, versión y licencia está en
`docs/LICENSE_AUDIT.md`. Ninguna es copyleft.
