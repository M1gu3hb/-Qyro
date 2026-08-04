# Auditoría de licencias

Fecha: 2026-08-04. Apache-2.0 es provisional y requiere aprobación antes de 1.0.

## Producto

| Dependencia | Versión | Origen | Licencia | Función | Riesgo |
|---|---:|---|---|---|---|
| Rust std | 1.88.0 | rust-lang/rust | MIT/Apache-2.0 | núcleo/ABI | bajo |
| Flutter/Dart SDK | Flutter 3.44.8 | flutter/flutter, dart-lang/sdk | BSD-3-Clause | UI/runtime | bajo |
| integration_test | Flutter 3.44.8 | Flutter SDK | BSD-3-Clause | instrumentación móvil | dev |
| flutter_lints | 5.0.0 | pub.dev | BSD-3-Clause | lint dev | bajo |
| async | 2.13.1 | pub.dev | BSD-3-Clause | tests SDK transitivos | dev |
| boolean_selector | 2.1.2 | pub.dev | BSD-3-Clause | tests | dev |
| characters | 1.4.1 | pub.dev | BSD-3-Clause | Flutter | bajo |
| clock | 1.1.2 | pub.dev | Apache-2.0 | tests | dev |
| collection | 1.19.1 | pub.dev | BSD-3-Clause | Flutter | bajo |
| fake_async | 1.3.3 | pub.dev | Apache-2.0 | tests | dev |
| leak_tracker | 11.0.2 | pub.dev | BSD-3-Clause | tests | dev |
| leak_tracker_flutter_testing | 3.0.10 | pub.dev | BSD-3-Clause | tests | dev |
| leak_tracker_testing | 3.0.2 | pub.dev | BSD-3-Clause | tests | dev |
| lints | 5.1.1 | pub.dev | BSD-3-Clause | lint | dev |
| matcher | 0.12.19 | pub.dev | BSD-3-Clause | tests | dev |
| material_color_utilities | 0.13.0 | pub.dev | Apache-2.0 | Flutter Material | bajo |
| meta | 1.18.0 | pub.dev | BSD-3-Clause | anotaciones | bajo |
| path | 1.9.1 | pub.dev | BSD-3-Clause | tests SDK | dev |
| source_span | 1.10.2 | pub.dev | BSD-3-Clause | tests | dev |
| stack_trace | 1.12.1 | pub.dev | BSD-3-Clause | tests | dev |
| stream_channel | 2.1.4 | pub.dev | BSD-3-Clause | tests | dev |
| string_scanner | 1.4.1 | pub.dev | BSD-3-Clause | tests | dev |
| term_glyph | 1.2.2 | pub.dev | BSD-3-Clause | tests | dev |
| test_api | 0.7.11 | pub.dev | BSD-3-Clause | tests | dev |
| vector_math | 2.2.0 | pub.dev | BSD-3-Clause | Flutter | bajo |
| vm_service | 15.2.0 | pub.dev | BSD-3-Clause | tests/debug | dev |

sky_engine, flutter, flutter_test e integration_test vienen del SDK. Las versiones/hashes de paquetes alojados están fijadas en apps/qyro/pubspec.lock. Cargo no tiene crates externos.

## Infraestructura de CI

| Dependencia | Versión fijada | Licencia | Función | Riesgo |
|---|---|---|---|---|
| ReactiveCircus/android-emulator-runner | a421e43855164a8197daf9d8d40fe71c6996bb0d (v2.38.0) | Apache-2.0 | emulador Android para prueba ABI real | dev |
| actions/upload-artifact | ea165f8d65b6e75b540449e92b4886f43607fa02 (v4) | MIT | publicación del ZIP portable de Windows | dev |

Las versiones y licencias se verificaron contra los tags y archivos LICENSE publicados. Los SHA completos evitan cambios implícitos de las actions fijadas.

## Política y pendiente

Permitidas tras revisión: Apache-2.0, MIT, BSD-2/3, ISC, Zlib y OFL. GPL/AGPL/LGPL, MPL y desconocidas requieren autorización. Falta automatizar verificación y revisar licencias de actions/checkout, flutter-action y rust-toolchain (CI, no distribuidas).
