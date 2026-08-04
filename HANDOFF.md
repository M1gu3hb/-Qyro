# Handoff operativo

- Actualizado: 2026-08-04 19:27 UTC
- Rama: main
- Commit actual/comprobado: 8360c984897084a4b7b70a8c27019a568f54e921
- Hito: 0 con base técnica alcanzada; Hito 1 parcial

## Funciona

- App Flutter con runners Android/iOS/Windows.
- Boot oscuro con logo, scramble QYRO, duración 5.5 s, guardia de 1 s, toque, teclado y reduced motion.
- Home muestra Enviar/Recibir deshabilitados y declara que faltan transportes.
- qyro_core informa readiness y QYRO/1.
- qyro_ffi exporta puntero/longitud de QYRO/1 con memoria estática.
- QyroNativeApi carga la biblioteca, valida puntero/longitud y decodifica UTF-8.
- CI Linux y Platform builds Windows leen QYRO/1 desde bibliotecas Rust compiladas.
- Android empaqueta libqyro_ffi.so para arm64-v8a y x86_64 dentro del APK.
- Windows empaqueta qyro_ffi.dll junto a qyro.exe.
- doctor, bootstrap y test_all equivalentes en Bash y PowerShell.
- bootstrap preserva configuraciones locales; test_all valida suites disponibles y ledger de licencias.

## No funciona todavía

Lectura FFI ejecutada dentro de un dispositivo/emulador Android, enlace estático iOS, branding dinámico, selección, manifest, criptografía, LAN, base de datos y modo óptico.

## Evidencia

GitHub Actions CI run 30942981584:

- Rust fmt/Clippy/4 tests: éxito.
- Dart format/analyze/9 tests Flutter+ABI: éxito.
- 6 contratos Bash/PowerShell: éxito.
- Tres jobs verdes.

GitHub Actions Platform builds run 30942981789:

- Android: cross-compile Rust arm64/x86_64, APK debug y verificación de ambas .so: éxito; job 3 min 30 s.
- Windows: build DLL, test Dart→DLL QYRO/1, build qyro.exe y copia de DLL: éxito; job 2 min 46 s.
- iOS: build Runner.app debug sin firma: éxito; job 2 min 42 s.

## Builds generados

Rutas dentro de runners efímeros, no retenidas como release:

- apps/qyro/build/app/outputs/flutter-apk/app-debug.apk
- apps/qyro/build/windows/x64/runner/Debug/qyro.exe
- apps/qyro/build/windows/x64/runner/Debug/qyro_ffi.dll
- apps/qyro/build/ios/iphoneos/Runner.app

No existe IPA firmado, MSIX, AAB ni artefacto de release descargable.

## Bloqueos

- Android tiene la biblioteca empaquetada, pero aún no se leyó QYRO/1 dentro de un emulador/dispositivo.
- iOS compila la capa Dart, pero qyro_ffi todavía no está enlazada al proceso.
- cargo-audit no está instalado en CI.
- No existen suites nativas dedicadas ni corpus de vectores.
- Falta identidad legal, bundle IDs finales y aprobación de licencia.
- Falta scramble-decode-reference.jpg y declaración de autoría/licencia del logo.
- actions/checkout@v4 emite aviso de Node 20 forzado a Node 24.

## Próxima tarea exacta

Crear smoke test Android que cargue libqyro_ffi.so en emulador y lea QYRO/1; después enlazar la staticlib en iOS.

Criterio: Android ejecuta el contrato dentro del runtime móvil y iOS puede resolver los dos símbolos desde DynamicLibrary.process().
