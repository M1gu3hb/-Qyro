# Handoff operativo

- Actualizado: 2026-08-04 18:36 UTC
- Rama: main
- Commit actual/comprobado por CI: f6bea027e86122e05f880b7552cf72ce6f5db8ad
- Commit de builds multiplataforma: 6812c6aafaea2e3844bb42bc078a03ffc83ac9ce
- Hito: 0 con base técnica alcanzada; Hito 1 parcial

## Funciona

- App Flutter con runners Android/iOS/Windows.
- Boot oscuro con logo, scramble QYRO, duración 5.5 s, guardia de 1 s, toque, Enter/Espacio/Escape y reduced motion.
- Home muestra Enviar/Recibir deshabilitados y declara que faltan transportes.
- qyro_core informa readiness y QYRO/1.
- qyro_ffi exporta puntero/longitud de QYRO/1 con memoria estática.
- CI ejecuta formato, lint, análisis y 11 tests.
- Builds debug Android, Windows e iOS sin firma.

## No funciona todavía

Enlace Dart↔qyro_ffi real, scripts doctor/bootstrap, branding dinámico, selección, protocolo, criptografía, LAN, base de datos y modo óptico.

## Pruebas ejecutadas

GitHub Actions CI run 30939294346:

- cargo fmt --all --check: éxito.
- cargo clippy --workspace --all-targets -- -D warnings: éxito.
- cargo test --workspace: 4 tests, 0 fallos.
- dart format: 7 archivos, 0 cambios.
- flutter analyze: sin issues.
- flutter test: 7 tests, 0 fallos.
- Jobs: Rust 15 s; Flutter 38 s.

GitHub Actions Platform builds run 30938946789:

- flutter build apk --debug: éxito, 2 min 57 s.
- flutter build windows --debug: éxito, 46 s.
- flutter build ios --debug --no-codesign: éxito, 1 min 28 s.
- Jobs completos: Android 3 min 20 s; Windows 3 min 36 s; iOS 3 min 49 s.

## Builds generados

Rutas dentro de runners efímeros, no retenidas como release:

- apps/qyro/build/app/outputs/flutter-apk/app-debug.apk
- apps/qyro/build/windows/x64/runner/Debug/qyro.exe
- apps/qyro/build/ios/iphoneos/Runner.app

No existe IPA firmado, MSIX, AAB ni artefacto de release descargable.

## Bloqueos

- No se probó ejecución en hardware Android/iOS ni en escritorio interactivo.
- iOS no está firmado.
- Falta identidad legal, bundle IDs finales y aprobación de licencia.
- Falta scramble-decode-reference.jpg y declaración de autoría/licencia del logo.
- actions/checkout@v4 emite aviso de Node 20 forzado a Node 24.

## Próxima tarea exacta

Implementar y probar el enlace Dart↔Rust real para leer QYRO/1 en Android y Windows, detrás de una interfaz inyectable.

Criterio: test nativo usa la biblioteca qyro_ffi compilada, CI Android/Windows pasa y la UI no bloquea el hilo principal.
