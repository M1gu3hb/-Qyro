# ESTADO ACTUAL

*Se reescribe entero al cerrar cada paso. Techo duro: 120 líneas.*

## 1. Dónde estoy

- **Fase 03**, cerrando. Sesión del 2026-08-14.
- Rama `claude/qyro-net-6a`. Último commit: **`867d3fa`**, ya empujado.
- Base de la fase para el `git diff`: **`9137220`**.

## 2. Hecho en esta sesión

| Paso | Qué | Commit |
|---|---|---|
| 0 | Línea base reproducida + archivo de estado | `00aea33` |
| 1 | ADR-0034 enmienda 1: `file_selector_windows`, no el paraguas | `54c66e2` |
| 1b | Corrección medida: el 2.º `pub get` seguido miente sobre el 1.º | `205204f` |
| 4 | Diálogo de Windows conectado + las 4 pruebas del paso 2 + 3 defectos | `867d3fa` |

## 3. Lo siguiente, concreto

1. Barrido de mutación en curso (`target/mutants-fs`, `target/mutants-session`).
2. Escribir `docs/reports/fase-03-selector-de-archivos.md`: 16 secciones de `R5`,
   puerta de 12 comprobaciones de `R2`, tabla de mutación, tabla de runs de CI.
3. Leer los runs de CI de `867d3fa` con `gh run list` — ahí corren las 3 pruebas
   `cfg(unix)` del descriptor y la del manifiesto fusionado, que aquí no corren.
4. Cerrar fase 03 como **PARCIAL** (criterio 7: nadie ha visto el diálogo) y abrir
   `docs/fase-implementacion/FASE-04-DESCUBRIMIENTO-Y-EMPAREJAMIENTO.md`.

## 4. Números actuales (comandos, no memoria)

| Comprobación | Comando | Valor |
|---|---|---|
| Tests Rust | `cargo test --workspace` | **603 passed, 0 failed, 2 ignored** (50 suites, Windows) |
| Tests Dart | `flutter test` en `apps/qyro` con las 2 variables | **76 passed, 1 skipped** |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit **0** |
| fmt Rust | `cargo fmt --all --check` | exit **0** |
| fmt Dart | `dart format --output=none --set-exit-if-changed .` **desde `apps/qyro`** | exit **0** |
| `flutter analyze` | en `apps/qyro` | exit **0**, sin issues |
| Docs | `check_docs_consistency` en Bash y en PowerShell | exit **0** los dos |
| Paquetes Rust | `Cargo.lock`: `[[package]]` / `source =` | **64 / 50** — sin cambios |
| Paquetes Dart | sección `packages:` de `pubspec.lock` | **37 → 45** (+8, `file_selector_windows`) |
| Ledger | script de `R2` §1.10 | **147 fichas, 39 abiertas** (era 142/37) |

## 5. El entorno (medido, no supuesto)

| Cosa | Dónde |
|---|---|
| Repo | `D:\Qyro\repo` |
| Rust | 1.88.0 activo por `rust-toolchain.toml`; el `stable` por defecto es 1.96.0 |
| Flutter | `D:\flutter\bin\flutter.bat` — 3.44.8 / Dart 3.12.2, igual que CI |
| `dart` | `D:\flutter\bin\cache\dart-sdk\bin\dart.exe` |
| `PUB_CACHE` | `D:\pub-cache` — exportar en cada shell |
| `cargo-mutants` | `D:\tools\cargo\bin\cargo-mutants.exe` 27.1.0 (subcomando: `... mutants`) |
| Git Bash | `C:\Program Files\Git\bin\bash.exe` — `bash` a secas resuelve a WSL sin distro |
| PowerShell | 5.1 únicamente, **no hay `pwsh`** |
| Java | `C:\Program Files\Microsoft\jdk-21.0.11.10-hotspot` |
| Android SDK | `%LOCALAPPDATA%\Android\Sdk` (adb, android-36) y `D:\android-sdk` (cmdline-tools, android-34). **Ninguno tiene emulador ni system-images** |
| `gh` | autenticado como `M1gu3hb` |
| Espacio | C: 10 GB · D: ~148 GB. **Todo va a D:** |
| Para las pruebas FFI de Dart | `QYRO_FFI_LIBRARY_PATH=D:\Qyro\repo\target\release\qyro_ffi.dll` y `QYRO_NET_SMOKE_PATH=...\qyro_net_smoke.exe`, tras `cargo build --release -p qyro_ffi -p qyro_net_smoke` |

**Modo Desarrollador: APAGADO** (`AllowDevelopmentWithoutDevLicense` no existe).
`flutter test` funciona; `flutter build`/`run` con plugins no (QYR-0324).

## 6. Decisiones tomadas todavía sin ADR

- Ninguna. La única de esta sesión —el paquete de Windows— está en ADR-0034,
  enmienda 1, congelada antes del código.

## 7. Qué NO hay que rehacer

- **`flutter pub get` sale 1 la primera vez tras cambiar un plugin** y 0 la
  segunda, porque se salta el paso de symlinks. `flutter test` funciona igual.
  No es un fallo nuevo: es QYR-0324 y está medido en ADR-0034.
- **`cargo` no reconstruye si restauras un archivo con una mtime vieja.**
  `Copy-Item` de un backup deja la mtime del backup y la prueba sigue fallando
  con el código ya arreglado. Tras restaurar:
  `(Get-Item ruta).LastWriteTime = Get-Date`. Me costó un diagnóstico entero.
- **`dart format --set-exit-if-changed .` desde la raíz falla** por
  `tools/branding_generator`, que CI no cubre (corre con
  `working-directory: apps/qyro`). No es tuyo: está en el carril.
- **`Select-String` de PowerShell 5.1 no tiene `-Recurse`**, y `-match` es
  insensible a mayúsculas, así que buscar `FAILED` casa con `failed`. Usa
  `-cmatch`.
- `flutter pub get` reescribe `apps/qyro/windows/flutter/generated_plugin_*`;
  `git diff` sale vacío pero `git status` los marca. `git checkout --` esa carpeta.
- **No busques símbolos con grep sin más en `qyro_ffi`**: ya no hace falta
  (QYR-0327 cerrado), pero si una búsqueda devuelve cero, comprueba que el
  archivo no lleve un byte NUL antes de concluir que el código no existe.

## 8. Reglas mínimas (esto basta con poco contexto)

1. **Sólo un P0 detiene una fase.** P1/P2/P3 → `docs/reports/deuda-de-calidad.md`,
   fase 09. Excepción: si un defecto *impide construir lo siguiente*.
2. **Autonomía total** salvo: `main`/force-push/borrar rama, dependencia nueva de
   Rust (excepción concedida: `mdns-sd` 0.20.3 bajo `cfg(windows)` en la 04),
   gastar dinero, y lo que exija hardware o una segunda persona.
3. **Los botones Enviar/Recibir siguen `onPressed: null`** hasta la fase 05.
4. **Las 12 comprobaciones se leen por CÓDIGO DE SALIDA**, no por el texto.
5. **Toda clase de evidencia se nombra.** «Compiló» nunca es «funciona».
6. **Nada de salida de herramienta en `BUGS_PENDING.md`.** Va al informe.
7. **Identificadores nuevos desde `QYR-0331+`**, consecutivos.
8. Toda ADR se congela **antes** del código, en un commit propio.
9. **Nunca commits en `main`.** Sólo `claude/qyro-net-6a`.
10. Este archivo se reescribe y se commitea al cerrar cada paso.
