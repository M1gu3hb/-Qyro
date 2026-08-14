# ESTADO ACTUAL

*Se reescribe entero al cerrar cada paso. Techo duro: 120 líneas.*

## 1. Dónde estoy

- **Fase 03 CERRADA COMO PARCIAL** — `docs/reports/fase-03-selector-de-archivos.md`.
- **Fase 04 ABIERTA**, paso 2 a medias — `docs/reports/fase-04-descubrimiento-y-emparejamiento.md`.
- Rama `claude/qyro-net-6a`. Base de la fase 04: `62658d7`.

## 2. Hecho en esta sesión (2026-08-14)

| Qué | Commit |
|---|---|
| Línea base reproducida + archivo de estado | `00aea33` |
| ADR-0034 enmienda 1: `file_selector_windows`, no el paraguas | `54c66e2` |
| El 2.º `pub get` seguido miente sobre el 1.º | `205204f` |
| Diálogo de Windows + las 4 pruebas del paso 2 + QYR-0327/0328/0329 | `867d3fa` |
| Las 3 pruebas `cfg(unix)` no compilaban (lo dijo CI, no yo) | `b8dbca5` |
| La prueba del manifiesto fusionado miraba donde Flutter no escribe | `274b504` |
| Informe de fase 03, PARCIAL | `62658d7` |
| **ADR-0035 congelada** antes del código | `39f645c` |
| **La cadena de emparejamiento** `QYRO1\|addr\|32hex`, con 7 pruebas | `67dd8da` |

## 3. Lo siguiente, concreto y en orden

1. **Cierra el paso 2**: `two_processes_connected_by_a_manual_endpoint_transfer_a_file`.
   `qyro_net_smoke serve` ya imprime `LISTENING <puerto>` y vacía antes de
   aceptar; falta que imprima además la cadena, que `send` la acepte, y que
   **rechace por tipo si la huella autenticada no es la prometida** — ADR-0035
   §2.1. Necesita el punto 1 de §6.
2. **Paso 3, la confianza por el FFI.** Empieza por `qyro_net`, no por el FFI.
3. Pasos 4–6 (mdns-sd, NsdManager, NWBrowser): la fase 05 **no los necesita**. Si
   no caben, decláralos no hechos con su motivo, como ya hace el informe.

*(La puerta del paso 2 pasa sus trece comprobaciones, barrido de mutación
incluido: 27 mutantes, 20 caught, 2 missed —uno ruido, uno equivalente
demostrado—, 5 unviable. Y CI está en verde sobre `67dd8da` y `7c83134`.)*

## 4. Números actuales (comandos, no memoria)

| Comprobación | Comando | Valor |
|---|---|---|
| Tests Rust | `cargo test --workspace` | **611 passed, 0 failed, 2 ignored** (51 suites, Windows) |
| Tests Dart | `flutter test` con las 2 variables | **76 passed, 1 skipped** |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit **0** |
| Clippy target unix | `cargo clippy -p <crate> --all-targets --target aarch64-linux-android -- -D warnings` | exit **0** |
| fmt Rust / Dart | `cargo fmt --all --check` · `dart format --set-exit-if-changed .` **desde `apps/qyro`** | exit **0** |
| `flutter analyze` | en `apps/qyro` | exit **0** |
| Docs | `check_docs_consistency` en Bash y PowerShell | exit **0** |
| Paquetes Rust / Dart | `[[package]]`/`source =` · sección `packages:` | **64 / 50** · **45** |
| Ledger | script de `R2` §1.10 | **147 fichas, 39 abiertas** |
| CI verde sobre | `62658d7`, `f153d61`, `39f645c` | **success** |

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
| Targets Rust | `x86_64-pc-windows-msvc` y `aarch64-linux-android` |
| Pruebas FFI de Dart | `cargo build --release -p qyro_ffi -p qyro_net_smoke`, luego `QYRO_FFI_LIBRARY_PATH=…\target\release\qyro_ffi.dll` y `QYRO_NET_SMOKE_PATH=…\qyro_net_smoke.exe` |

**Modo Desarrollador APAGADO**: `flutter test` sí, `flutter build`/`run` no (QYR-0324).

## 6. Restricciones estructurales medidas, que ADR-0035 ya decidió

1. **`qyro_net::Session` no publica la identidad del peer.** `qyro_crypto` sí
   —`peer_identity()`—, `qyro_net` la envuelve y no la republica. **Sin ensanchar
   ahí no hay huella que enseñar.** ADR-0035 §5(a) lo autoriza: accesor de sólo
   lectura a la identidad **pública**, que no es material de clave.
2. **`qyro_session` no puede reexportar `TrustVerdict` ni `HumanFingerprint`.**
   `qyro_session_re_exports_nothing_it_does_not_own` sólo admite `pub use` de
   `crate`, `self`, `super`, `error` y `session`. La fachada **posee su propio
   vocabulario** y convierte por dentro. ADR-0035 §5(b).
3. **`qyro_session` gana `qyro_identity_store`**, de primera parte, y hay que
   actualizar `CLOSURE` en `qyro_ffi/tests/c_abi_contract.rs`.
4. **Toda enum de error nueva necesita su comprobación de sitio de construcción**
   o una meta-guarda de `qyro_identity_store` pone el workspace en rojo. Se
   exime con el argumento escrito, como se hizo con `PairingError`.

## 7. Qué NO hay que rehacer

- **`cargo` no reconstruye si restauras un archivo con una mtime vieja.** Tras
  `Copy-Item` de un backup: `(Get-Item ruta).LastWriteTime = Get-Date`.
- **Lo `cfg(unix)` no se compila aquí.** Verifícalo con `--target
  aarch64-linux-android`; `check` no enlaza, no hace falta enlazador.
- **`check_docs_consistency` se corre sobre el commit FINAL**: la regla de
  frescura de `STATUS.md` se invalida sola con cada commit.
- Citar un `QYR-00xx` sin ficha bloquea. Para «el siguiente libre» escribe
  `QYR-nnnn+`, con el `+` pegado al número **dentro** de las comillas.
- `flutter pub get` sale 1 la primera vez tras cambiar un plugin y 0 la segunda.
  Reescribe `apps/qyro/windows/flutter/generated_plugin_*`: `git checkout --`.
- **`dart format --set-exit-if-changed .` desde la raíz falla** por
  `tools/branding_generator`, que CI no cubre. Está en el carril.
- `Select-String` de PS 5.1 no tiene `-Recurse`, y `-match` ignora mayúsculas
  (`FAILED` casa con `failed`). Usa `-cmatch`.
- **Un barrido `cargo-mutants --package` subestima la cobertura** de toda función
  cuyas pruebas viven aguas abajo. Usa `--test-workspace true`.

## 8. Reglas mínimas (esto basta con poco contexto)

1. **Sólo un P0 detiene una fase.** El resto → `docs/reports/deuda-de-calidad.md`,
   fase 09. Excepción: si un defecto *impide construir lo siguiente*.
2. **Autonomía total** salvo: `main`/force-push/borrar rama, dependencia nueva de
   Rust (**excepción concedida: `mdns-sd` 0.20.3 bajo `cfg(windows)`**), gastar
   dinero, y lo que exija hardware o una segunda persona.
3. **Los botones Enviar/Recibir siguen `onPressed: null`** hasta la fase 05.
4. **Las comprobaciones de la puerta se leen por CÓDIGO DE SALIDA.** Son doce más
   la decimotercera que añadió la fase 03: el clippy del target unix.
5. **Toda clase de evidencia se nombra.** «Compiló» nunca es «funciona».
6. **Nada de salida de herramienta en `BUGS_PENDING.md`.** Va al informe.
7. **Identificadores nuevos desde `QYR-0331+`**, consecutivos.
8. Toda ADR se congela **antes** del código, en un commit propio.
9. **Nunca commits en `main`.** Sólo `claude/qyro-net-6a`.
10. Este archivo se reescribe y se commitea al cerrar cada paso.
