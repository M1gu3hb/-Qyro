# ESTADO ACTUAL

*Se reescribe entero al cerrar cada paso. Techo duro: 120 líneas.*

## 1. Dónde estoy

- **Fase 05 CERRADA. LOS BOTONES ESTÁN ENCENDIDOS.** Las cinco condiciones
  cumplidas con su prueba con nombre — `docs/reports/fase-05-la-interfaz-y-los-botones.md`.
- Fase 03 cerrada como PARCIAL. Fase 04a cerrada. **04b va ahora.**
- Rama `claude/qyro-net-6a`, último commit **`d7d8b50`**.

## 2. Lo siguiente, en orden

1. **04b** — `mdns-sd` 0.20.3 bajo `cfg(windows)` (pre-autorizada, delta de
   `Cargo.lock` + `cargo audit` en el informe) y `NsdManager` con
   `FLAG_SHOW_PICKER` en Android. **La trampa está en ADR-0035 enmienda 1**:
   cualquier cosa que no sea `NsdManager` necesita `WifiManager.MulticastLock`,
   porque el stack Wi-Fi filtra el multicast por debajo del socket y
   `join_multicast_v4` tiene éxito sin recibir nada y **sin error**.
2. **06** — Android Keystore. QYR-0064: test **instrumentado bajo `am instrument`**,
   no un binario en `/data/local/tmp`. `jni-sys` está pre-autorizada. Sin esto la
   confianza y la identidad no sobreviven a un reinicio.
3. **08** — permisos, APK firmado, `.exe`, nombre de paquete y clave de firma
   (decido yo). QYR-0050 y QYR-0004.
4. **09** — cerrar o descartar **toda** la deuda: 37 fichas abiertas y
   `docs/reports/deuda-de-calidad.md` entero.
5. **10** — ADR superadas marcadas, `THREAT_MODEL.md` reescrito contra lo que
   existe, `docs/release/v1.0.md`, etiqueta `v1.0.0`, artefactos con SHA-256.
6. **`docs/testing/hardware-protocol.md`** — veinte escenarios con su comando
   literal, listos para que el propietario conecte un teléfono. **No se inventa
   evidencia de hardware.**

## 3. Números actuales (comandos, no memoria)

| Comprobación | Comando | Valor |
|---|---|---|
| Tests Rust | `cargo test --workspace` | **623 passed, 0 failed, 2 ignored** (52 suites, Windows) |
| Tests Dart | `flutter test` con las 2 variables | **94 passed** |
| Clippy · fmt | `--workspace --all-targets -D warnings` · `fmt --all --check` | exit **0** |
| Clippy target unix | `-p <crate> --all-targets --target aarch64-linux-android` | exit **0** |
| `flutter analyze` · `dart format` | en `apps/qyro` | exit **0** |
| Docs | `check_docs_consistency` en Bash y PowerShell | exit **0** |
| Paquetes Rust / Dart | `[[package]]`/`source =` · sección `packages:` | **64 / 50** · **45** |
| Ledger | script de `R2` §1.10 | **147 fichas, 37 abiertas** |
| Símbolos C | `extern "C" fn` en `qyro_ffi/src` | **19** |

## 4. El entorno (medido)

| Cosa | Dónde |
|---|---|
| Repo | `D:\Qyro\repo` · Rust 1.88.0 por `rust-toolchain.toml` |
| Flutter | `D:\flutter\bin\flutter.bat` 3.44.8 / Dart 3.12.2 · `dart.exe` en `D:\flutter\bin\cache\dart-sdk\bin` |
| `PUB_CACHE` | `D:\pub-cache` — exportar en cada shell |
| `cargo-mutants` | `D:\tools\cargo\bin\cargo-mutants.exe` 27.1.0 (subcomando `mutants`) |
| Git Bash | `C:\Program Files\Git\bin\bash.exe` — `bash` a secas es WSL sin distro |
| PowerShell | 5.1 únicamente, **no hay `pwsh`** |
| Android SDK | `%LOCALAPPDATA%\Android\Sdk` (adb, android-36) y `D:\android-sdk` (cmdline-tools, android-34). **Sin emulador** |
| Java | `C:\Program Files\Microsoft\jdk-21.0.11.10-hotspot` |
| Targets Rust | `x86_64-pc-windows-msvc` y `aarch64-linux-android` |
| Pruebas FFI de Dart | `cargo build --release -p qyro_ffi -p qyro_net_smoke`, luego `QYRO_FFI_LIBRARY_PATH=…\target\release\qyro_ffi.dll` y `QYRO_NET_SMOKE_PATH=…\qyro_net_smoke.exe` |

**Modo Desarrollador APAGADO**: `flutter test` sí, `flutter build`/`run` no (QYR-0324).

## 5. Lo que ya está resuelto y no hay que reinvestigar

1. **La superficie C son diecinueve símbolos** y ninguno cruza un tipo. El
   contrato de texto vive en `emit_text`: `out_len` siempre, y **nada escrito**
   si no cabe.
2. **`qyro_session` posee su vocabulario**: `PeerTrust`, `RejectReason`,
   `TrustBook`, `parse_pairing`. No reexporta nada ajeno, y la guarda que lo
   exige **deriva** los módulos del `lib.rs` de la fachada.
3. **Las pantallas hablan con `QyroTransferService`**, no con el FFI, porque
   `stepBlocking` bloquea sin cota. `NativeTransferService` corre la sesión en
   `Isolate.run` y sólo devuelve enteros y texto.
4. **La confianza no persiste** hasta la fase 06: `seal_known_peers` necesita un
   `SecretWrapper` y en Android no hay.
5. **`cargo-mutants` no puede mutar una `extern "C" fn` de forma observable**: el
   cuerpo sustituido devuelve `0`, que es `QYRO_OK`. Esas se mutan a mano.

## 6. Qué NO hay que rehacer

- **`cargo` no reconstruye con una mtime vieja.** Tras restaurar un backup:
  `(Get-Item ruta).LastWriteTime = Get-Date`.
- **Lo `cfg(unix)` no se compila aquí.** `--target aarch64-linux-android`.
- **`check_docs_consistency` se corre sobre el commit FINAL** y hay que actualizar
  `Verified commit` en `STATUS.md` cada pocas confirmaciones.
- Citar un `QYR-00xx` sin ficha bloquea. Para «el siguiente libre» escribe
  `QYR-nnnn+` con el `+` pegado al número, dentro de las comillas.
- `flutter pub get` sale 1 la primera vez tras cambiar un plugin y 0 la segunda.
  Reescribe `apps/qyro/windows/flutter/generated_plugin_*`: `git checkout --`.
- `dart format --set-exit-if-changed .` **desde `apps/qyro`**, como CI; desde la
  raíz falla por `tools/branding_generator`.
- `Select-String` de PS 5.1 no tiene `-Recurse` y `-match` ignora mayúsculas.
- **Una prueba de widget necesita un `Scaffold`**: `TextField` y `Card` piden un
  `Material` ancestro y un `home:` pelado no lo tiene.
- **El analizador de Dart rechaza un U+202E crudo en un literal.** Escríbelo como
  `'\u202E'`.

## 7. Reglas mínimas

1. **Sólo un P0 detiene.** El resto → `deuda-de-calidad.md`, y la **09 la vacía**.
   Excepción: lo que impide construir lo siguiente.
2. **Autonomía total** salvo `main`, dinero, aparato físico o segunda persona.
3. **Ningún informe termina con «falta esto».** Dos destinos: HECHA, o
   cerrada/descartada con argumento en la 09.
4. **Trece comprobaciones por puerta, por CÓDIGO DE SALIDA.**
5. **Toda clase de evidencia se nombra.** «Compiló» nunca es «funciona».
6. **NO se inventa evidencia de hardware.** Es lo único que arruinaría el proyecto.
7. **Identificadores nuevos desde `QYR-0348+`.**
8. Toda ADR se congela **antes** del código, en commit propio.
9. **Nunca commits en `main`.** Sólo `claude/qyro-net-6a`.
10. Este archivo se reescribe y se commitea al cerrar cada paso.
