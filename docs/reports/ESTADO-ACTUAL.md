# ESTADO ACTUAL

*Se reescribe entero al cerrar cada paso. Techo duro: 120 líneas.*

## 1. Dónde estoy

- **Fase 03**, paso 4 (Windows). Sesión iniciada el 2026-08-14.
- Rama `claude/qyro-net-6a`. Último commit propio: *(ninguno todavía)*.
- HEAD al arrancar: `9137220`.

## 2. Hecho en esta fase (antes de esta sesión)

| Paso | Qué | Commit |
|---|---|---|
| 1 | ADR-0034 congelada | `269f0fa` |
| 2 | Superficie FFI por descriptor | `b4ebac7` |
| 3 | Selector Android: `MethodChannel` propio, `"rw"` + `detachFd()` | `9137220` |

## 3. Lo siguiente, concreto

1. `apps/qyro/pubspec.yaml`: añadir `file_selector` (flutter.dev, BSD-3).
2. `apps/qyro/lib/ffi/qyro_file_picker.dart`: `QyroDesktopFilePicker` hoy exige un
   callback `openPaths` y lanza `UnsupportedError` si falta. Conectarlo de verdad
   a `file_selector.openFiles()` dejando la inyección para las pruebas.
3. Prueba `a_file_chosen_through_the_system_dialog_transfers_and_verifies`.
4. Informe `docs/reports/fase-03-selector-de-archivos.md` con las 16 secciones de
   `R5` y la puerta de 12 comprobaciones de `R2`.

## 4. Línea base reproducida (2026-08-14, sobre `9137220`, Windows 10)

| Comprobación | Comando | Resultado |
|---|---|---|
| Tests Rust | `cargo test --workspace` | **598 passed, 0 failed, 2 ignored** (50 suites). Esperado 593 en Linux; +5 por `cfg` de Windows |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit **0** |
| Formato | `cargo fmt --all --check` | exit **0** |
| Docs (Bash) | `bash scripts/check_docs_consistency.sh` | exit **0** |
| Docs (PS) | `powershell -File scripts\check_docs_consistency.ps1` | exit **0** |
| Paquetes | `Cargo.lock`: `[[package]]` / `source =` | **64 / 50** |
| Ledger | script de `R2` §1.10 | **142 fichas, 37 abiertas** |
| Tests Dart | `flutter test` en `apps/qyro` | **58 passed, 6 skipped** (los 6 piden `QYRO_FFI_LIBRARY_PATH`) |

**Divergencia con el prompt de sesión:** decía «142 fichas, 38 abiertas». El script
canónico de `R2` §1.10 devuelve **37** sobre este mismo commit. No bloquea; queda
registrado aquí y en el informe de fase.

## 5. El entorno de esta máquina (medido, no supuesto)

| Cosa | Dónde | Nota |
|---|---|---|
| Repo | `D:\Qyro\repo` | clonado en esta sesión |
| Rust | `1.88.0` activo por `rust-toolchain.toml` | el `stable` por defecto es 1.96.0 |
| Flutter | `D:\flutter\bin\flutter.bat` — 3.44.8 / Dart 3.12.2 | descargado en esta sesión; coincide con CI |
| `PUB_CACHE` | `D:\pub-cache` | hay que exportarlo en cada shell |
| `cargo-mutants` | `D:\tools\cargo\bin\` | instalado aislado con `--root` |
| Git Bash | `C:\Program Files\Git\bin\bash.exe` | `bash` a secas resuelve a WSL y **no hay distro** |
| PowerShell | 5.1 únicamente | **`pwsh` no existe**; ojo con `-Recurse -Include` (QYR-0311) |
| Java | `C:\Program Files\Microsoft\jdk-21.0.11.10-hotspot` | 21.0.11 |
| Android SDK (A) | `C:\Users\mighu\AppData\Local\Android\Sdk` | `adb`, platforms android-36, build-tools 35/36. **Sin** cmdline-tools, emulador ni system-images |
| Android SDK (B) | `D:\android-sdk` | `cmdline-tools/latest` (sdkmanager), android-34, build-tools 34.0.0. **Sin** emulador |
| `ANDROID_HOME` | **vacío** | hay que fijarlo por invocación |
| `gh` | autenticado como `M1gu3hb` | scopes: gist, read:org, repo, workflow |
| Espacio | C: 10.1 GB libres · D: 150.4 GB | **todo va a D:** |

**Modo Desarrollador: APAGADO.** `AllowDevelopmentWithoutDevLicense` no existe en
`HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock`. Consecuencia:
`flutter build` y `flutter run` con plugins fallan por symlinks (QYR-0324).
`flutter test` sí funciona. **No lo activo yo: es configuración del propietario.**

## 6. Decisiones tomadas todavía sin ADR

- Ninguna en esta sesión.

## 7. Qué NO hay que rehacer

- **No busques `flutter`, `dart`, `adb` ni `cargo-mutants` en el `PATH`.** No están.
  Las rutas absolutas están en §5.
- **No uses `bash` desde PowerShell.** Resuelve a WSL sin distro. Ruta absoluta de
  Git Bash, siempre.
- **No intentes `pwsh`.** Usa `powershell.exe -NoProfile -File`.
- **No clones en C:.** Quedan 10 GB.
- `flutter pub get` reescribe `apps/qyro/windows/flutter/generated_plugin_*` con
  CRLF. `git diff` sale vacío (hay `eol=lf`), pero `git status` los marca. Se
  arregla con `git checkout -- apps/qyro/windows/flutter/`.

## 8. Reglas mínimas (diez líneas — esto basta con poco contexto)

1. **Sólo un P0 detiene una fase.** P1/P2/P3 → `docs/reports/deuda-de-calidad.md`, se
   arreglan en la fase 09. Excepción: si un defecto *impide construir lo siguiente*.
2. **Autonomía total** salvo: tocar `main`/force-push/borrar rama, añadir dependencia
   de Rust (excepción concedida: `mdns-sd` 0.20.3 bajo `cfg(windows)` en la fase 04),
   gastar dinero, y lo que exija hardware o una segunda persona.
3. **Los botones Enviar/Recibir siguen `onPressed: null`** hasta la fase 05 y sólo con
   las cinco condiciones escritas y su evidencia.
4. **La puerta son 12 comprobaciones y se leen por CÓDIGO DE SALIDA**, no por el texto.
5. **Toda clase de evidencia se nombra.** «Compiló» nunca es «funciona».
6. **Nada de salida de herramienta en `BUGS_PENDING.md`.** Va al informe.
7. **Identificadores nuevos desde `QYR-0326`**, consecutivos.
8. Toda ADR se congela **antes** del código, en un commit propio.
9. **Nunca commits en `main`.** Sólo `claude/qyro-net-6a`.
10. Este archivo se reescribe y se commitea al cerrar cada paso.
