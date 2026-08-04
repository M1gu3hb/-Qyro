# Auditoría de licencias

Fecha: 2026-08-04. Apache-2.0 es provisional y requiere aprobación antes de 1.0.

| Dependencia | Versión | Repositorio | Licencia | Función | Riesgo | Revisado |
|---|---|---|---|---|---|---|
| Rust std | 1.88.0 | rust-lang/rust | Apache-2.0 y MIT | núcleo/ABI | bajo | 2026-08-04 |
| Flutter SDK | stable CI (3.44.8 observada) | flutter/flutter | BSD-3-Clause | UI/tests | fijación exacta pendiente | 2026-08-04 |
| Dart SDK | incluida en Flutter | dart-lang/sdk | BSD-3-Clause | lenguaje/runtime | fijación exacta pendiente | 2026-08-04 |
| flutter_lints | 5.0.0 | flutter/packages | BSD-3-Clause | lint dev | bajo | 2026-08-04 |
| flutter_test | SDK | flutter/flutter | BSD-3-Clause | tests dev | bajo | 2026-08-04 |

Cargo no tiene dependencias externas. El pubspec todavía no tiene lockfile versionado; CI observó dependencias transitivas y debe fijarlas/auditarlas antes del siguiente hito.

## Política

Permitidas tras revisión: Apache-2.0, MIT, BSD-2/3, ISC, Zlib y OFL. GPL/AGPL/LGPL, MPL y desconocidas requieren autorización. Ningún código de referencia se reutiliza actualmente.

## Pendiente

- fijar Flutter/FVM y generar pubspec.lock;
- verificar cada paquete transitivo;
- auditar licencias de actions/checkout, flutter-action y rust-toolchain (CI solamente);
- crear scripts check_licenses con tests;
- registrar commit/archivo si se reutiliza código.
