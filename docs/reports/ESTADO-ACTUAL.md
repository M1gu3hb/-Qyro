# ESTADO ACTUAL

*Se reescribe entero al cerrar cada paso. Techo duro: 120 líneas.*

## 1. Dónde estoy

- **Fase 03 CERRADA COMO PARCIAL.** Informe: `docs/reports/fase-03-selector-de-archivos.md`.
- **Fase 04, paso 1** (ADR-0035). Rama `claude/qyro-net-6a`.
- Último commit: **`62658d7`**. Base de la fase 03: `546dbf6`.

## 2. Hecho en esta sesión

| Qué | Commit |
|---|---|
| Línea base reproducida + archivo de estado | `00aea33` |
| ADR-0034 enmienda 1: `file_selector_windows`, no el paraguas | `54c66e2` |
| Corrección medida: el 2.º `pub get` seguido miente sobre el 1.º | `205204f` |
| Diálogo de Windows + las 4 pruebas del paso 2 + QYR-0327/0328/0329 | `867d3fa` |
| Las 3 pruebas `cfg(unix)` no compilaban (lo dijo CI, no yo) | `b8dbca5` |
| La prueba del manifiesto fusionado miraba donde Flutter no escribe | `274b504` |
| Informe de fase 03, 16 secciones, PARCIAL | `62658d7` |

## 3. Lo siguiente, concreto

**Fase 04, y en este orden** (el fallback manual va PRIMERO, `FASE-04` §3.4):

1. **ADR-0035** `docs/adr/ADR-0035-discovery-and-pairing.md`, congelada **antes**
   del código, en commit propio. Lo que tiene que decidir está en §6 de abajo.
2. **Paso 2 — el endpoint manual y el QR.** `ip:puerto` **más la huella** en la
   misma cadena, para que escanear sea emparejar. Cero dependencias: el tipo vive
   en Rust y se prueba con `cargo test`.
3. **Paso 3 — la confianza por el FFI.** Ver §6: hay dos restricciones
   estructurales que decide la ADR, no el código.
4. Pasos 4–6 (mdns-sd en Windows, NsdManager en Android, NWBrowser en iOS): si no
   caben, se declaran **no hechos** con su motivo. La 05 no los necesita.

## 4. Números actuales (comandos, no memoria)

| Comprobación | Comando | Valor |
|---|---|---|
| Tests Rust | `cargo test --workspace` | **603 passed, 0 failed, 2 ignored** (50 suites, Windows) |
| Tests Dart | `flutter test` con las 2 variables | **76 passed, 1 skipped** |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit **0** |
| Clippy `cfg(unix)` | `cargo clippy -p qyro_ffi --all-targets --target aarch64-linux-android -- -D warnings` | exit **0** |
| fmt Rust / Dart | `cargo fmt --all --check` · `dart format --set-exit-if-changed .` **desde `apps/qyro`** | exit **0** los dos |
| `flutter analyze` | en `apps/qyro` | exit **0** |
| Docs | `check_docs_consistency` en Bash y PowerShell | exit **0** los dos |
| Paquetes Rust / Dart | `Cargo.lock` `[[package]]`/`source =` · sección `packages:` de `pubspec.lock` | **64 / 50** · **45** |
| Ledger | script de `R2` §1.10 | **147 fichas, 39 abiertas** |

## 5. El entorno (medido)

| Cosa | Dónde |
|---|---|
| Repo | `D:\Qyro\repo` · Rust 1.88.0 por `rust-toolchain.toml` |
| Flutter | `D:\flutter\bin\flutter.bat` 3.44.8 / Dart 3.12.2 · `dart.exe` en `D:\flutter\bin\cache\dart-sdk\bin` |
| `PUB_CACHE` | `D:\pub-cache` — exportar en cada shell |
| `cargo-mutants` | `D:\tools\cargo\bin\cargo-mutants.exe` 27.1.0 (subcomando `mutants`) |
| Git Bash | `C:\Program Files\Git\bin\bash.exe` — `bash` a secas es WSL sin distro |
| PowerShell | 5.1 únicamente, **no hay `pwsh`** |
| Android SDK | `%LOCALAPPDATA%\Android\Sdk` y `D:\android-sdk`. **Ninguno tiene emulador** |
| Targets Rust | `x86_64-pc-windows-msvc` y `aarch64-linux-android` instalados |
| Pruebas FFI de Dart | `cargo build --release -p qyro_ffi -p qyro_net_smoke`, luego `QYRO_FFI_LIBRARY_PATH=…\target\release\qyro_ffi.dll` y `QYRO_NET_SMOKE_PATH=…\qyro_net_smoke.exe` |

**Modo Desarrollador APAGADO**: `flutter test` sí, `flutter build`/`run` no (QYR-0324).

## 6. Decisiones que la ADR-0035 tiene que tomar (medidas ya, sin ADR todavía)

1. **`qyro_net::Session` NO expone la identidad del peer.** `qyro_crypto` sí
   —`peer_identity()` en el estado establecido—, pero `qyro_net` la envuelve y no
   la republica. Sin ensanchar ahí, no hay huella que enseñar.
2. **`qyro_session` NO puede reexportar `TrustVerdict` ni `HumanFingerprint`.**
   `qyro_session_re_exports_nothing_it_does_not_own` (en
   `qyro_ffi/tests/c_abi_contract.rs`) sólo permite `pub use` de `crate`, `self`,
   `super`, `error` y `session`. Así que la fachada tiene que **poseer su propio
   vocabulario de confianza** y convertir por dentro. Eso es ADR-0032 haciendo su
   trabajo, no un obstáculo.
3. **`qyro_ffi` sólo puede nombrar `qyro_core` y `qyro_session`**, comprobado
   contra el resolvedor. Añadir `qyro_identity_store` a `qyro_session` es de
   primera parte y hay que actualizar `CLOSURE` en `c_abi_contract.rs`.
4. **Cuándo se decide la confianza**, la pregunta que ADR-0031 dejó abierta.
5. Qué va en el TXT de `_qyro._tcp`. Lo ve toda la red.

## 7. Qué NO hay que rehacer

- **`cargo` no reconstruye si restauras un archivo con una mtime vieja.** Tras
  `Copy-Item` de un backup: `(Get-Item ruta).LastWriteTime = Get-Date`.
- **Lo `cfg(unix)` no se compila aquí.** Verifícalo con `cargo clippy -p <crate>
  --all-targets --target aarch64-linux-android`; `check` no enlaza.
- **`check_docs_consistency` se corre sobre el commit FINAL**, no antes: la regla
  de frescura de `STATUS.md` se invalida sola con cada commit.
- Citar un `QYR-00xx` sin ficha bloquea. Para el «siguiente libre», escribe
  `QYR-nnnn+` **con el `+` pegado al número y dentro de las comillas**.
- `flutter pub get` sale 1 la primera vez tras cambiar un plugin y 0 la segunda.
  Reescribe `apps/qyro/windows/flutter/generated_plugin_*`: `git checkout --`.
- **`dart format --set-exit-if-changed .` desde la raíz falla** por
  `tools/branding_generator`, que CI no cubre. Está en el carril.
- `Select-String` de PowerShell 5.1 no tiene `-Recurse`, y `-match` ignora
  mayúsculas: `FAILED` casa con `failed`. Usa `-cmatch`.

## 8. Reglas mínimas (esto basta con poco contexto)

1. **Sólo un P0 detiene una fase.** El resto → `docs/reports/deuda-de-calidad.md`,
   fase 09. Excepción: si un defecto *impide construir lo siguiente*.
2. **Autonomía total** salvo: `main`/force-push/borrar rama, dependencia nueva de
   Rust (**excepción concedida: `mdns-sd` 0.20.3 bajo `cfg(windows)`**), gastar
   dinero, y lo que exija hardware o una segunda persona.
3. **Los botones Enviar/Recibir siguen `onPressed: null`** hasta la fase 05.
4. **Las comprobaciones de la puerta se leen por CÓDIGO DE SALIDA.** Son doce, más
   la decimotercera que añadió la fase 03: el clippy del target unix.
5. **Toda clase de evidencia se nombra.** «Compiló» nunca es «funciona».
6. **Nada de salida de herramienta en `BUGS_PENDING.md`.** Va al informe.
7. **Identificadores nuevos desde `QYR-0331+`**, consecutivos.
8. Toda ADR se congela **antes** del código, en un commit propio.
9. **Nunca commits en `main`.** Sólo `claude/qyro-net-6a`.
10. Este archivo se reescribe y se commitea al cerrar cada paso.
