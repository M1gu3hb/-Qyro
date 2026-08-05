# Próximos pasos

## P0

1. Golden tests de la secuencia de arranque.
   - Aceptación: 0/20/50/80/100 %, teléfono, tablet, Windows ancho, reduced
     motion, branding provisional, branding válido con firma, fallo de
     biblioteca, fallo de asset, timeout y retry. Seeds deterministas,
     dimensiones fijas, assets locales y ninguna dependencia de hora o red.
     Archivos golden versionados y documentado cómo actualizarlos.
2. Benchmark de arranque documentado.
   - Aceptación: `docs/benchmarks/boot-baseline.md` con tiempo de preparación
     del modelo, tiempo de `frameAt`, build y paint por frame, tamaño del asset
     ASCII, y declaradas máquina, SO, versión de Flutter, modo, resolución y
     número de muestras. Sin afirmar 60 FPS sin medirlo.
3. Retener artefactos de desarrollo con SHA-256, etiquetados
   DEVELOPMENT / NOT FOR PUBLIC RELEASE.

## P1

- **AEAD de sesión (ChaCha20-Poly1305) y protección de replay.** El handshake ya
  deriva claves direccionales y un identificador de sesión; nada las usa. Es el
  paso que convierte `EncryptedEnvelope` en algo con un tag real, y el que
  permite que `SealedFrame` y `AuthenticatedFrame` existan con constructores
  privados.
- **Vectores interoperables del handshake.** Hoy el transcript está fijado por
  ADR-0021 y por tests de Rust; un test en Rust prueba que Rust es consistente
  consigo mismo, no que el formato esté definido sin ambigüedad. Debe existir un
  `handshake-v1.json` con una ejecución completa antes de que haya una segunda
  implementación.
- Almacenamiento local seguro de la identidad, sin iniciar LAN hasta cerrar los
  vectores criptográficos.
- Ejecutar una campaña real de `cargo-fuzz` y añadir los hallazgos al corpus.
- Probar en hardware físico: hasta ahora solo emulador, simulador y host.
- SBOM y `cargo-deny` para licencias, fuentes, duplicados y bans.
- Selección de archivos y construcción del manifest desde el filesystem real.

## P2

- SQLite/migración 0001.
- LAN e IP manual.
- Emparejamiento por QR e identidad local.

## P3

- RaptorQ/QR adaptativo.
- Wi-Fi Direct, Multipeer y Bluetooth experimental.

## Completado el 2026-08-05 (sprint 4B, handshake autenticado)

- Handshake autenticado de cuatro mensajes en memoria: X25519, Ed25519 en el
  dominio `HandshakeTranscript`, HKDF-SHA256 y HMAC-SHA256, con máquina de
  estados de estados consumidos (ADR-0021).
- Cerradas las invariantes que quedaban: cabecera protegida fuera de `Frame`,
  plantilla de sobre probada por tipo, claves Ed25519 de orden bajo rechazadas,
  `verify_strict`, firma solo falible, fingerprint con dos escrituras canónicas,
  identidad pública de 33 bytes y constructor determinista fuera de la API.
- KAT completos: RFC 8032 §7.1 (5 vectores) y RFC 4231 (7 vectores), extraídos
  del texto de los RFC.
- Corregidas afirmaciones obsoletas sobre el plegado Unicode y sobre qué ABI
  nativas estaban verificadas.

## Completado el 2026-08-05 (sprint 2, protocolo y manifest)

- `qyro_protocol`: framing QYRO/1 con decoder incremental acotado; 29 contratos
  de wire y 4 property tests.
- `qyro_manifest`: manifest canónico y `RelativePath` estricto; 40 contratos y
  5 property tests.
- Targets `cargo-fuzz`, corpus de 65 entradas y smoke en CI.
- `cargo audit` obligatorio, en verde con cero dependencias externas.
- Wordmark, tagline y firma configurable mediante scramble, sin inventar nombre.
- ADR-0016, ADR-0017 y las especificaciones de wire y manifest.
- CI en verde sobre la rama: run 30964542743.

## Completado el 2026-08-05 (Hito A, recuperación)

- Reconciliadas main y audit/baseline-hardening sin force-push ni pérdida de
  commits; ambos cambios del propietario preservados.
- Logo canónico fijado en design/brand/source/logo.png por checksum, con el
  marcador rechazado excluido del producto y cubierto por pruebas (ADR-0014).
- Corregida la regresión que impedía compilar iOS desde 67fa795, con contrato
  estructural del storyboard verificado en rojo y verde.
- Cerrada la brecha que permitió a STATUS.md derivar 58 commits sin detección.
- Baseline completo reproducido en host Linux: Rust, Flutter (51 tests),
  11 contratos de script, 7 tests Python y el job documental.
- iOS y Android confirmados en CI sobre esta rama: run 30963011815 (iOS, 10/10
  pasos, incluye verificación de símbolos y XCTest) y run 30963016390 (Android,
  smoke test de ABI en emulador).

## Completado el 2026-08-04

- doctor/bootstrap/test_all en Bash y PowerShell mediante TDD.
- bootstrap crea configuración desde examples sin sobrescribir al usuario.
- Puente Dart↔qyro_ffi con lectura real QYRO/1 en Linux y Windows.
- APK contiene bibliotecas Rust arm64-v8a/x86_64 y Windows distribuye la DLL junto al ejecutable.
