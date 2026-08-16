# ESTADO ACTUAL

*Se reescribe entero al cerrar cada paso. Techo duro: 120 líneas.*

## 1. Dónde estoy

- **Fase 03 cerrada como PARCIAL.** **Fase 04 partida en 04a y 04b** por
  ADR-0035 enmienda 1: la 04b (`NsdManager`, `mdns-sd`) va **después** de la 05.
- **Fase 04a**: pasos 1–3 hechos en Rust. Falta el FFI y la prueba entre procesos.
- Rama `claude/qyro-net-6a`, último commit **`0f87a4e`**.

## 2. LOS CINCO REQUISITOS DE LOS BOTONES — el estado exacto

| # | Condición | Estado | Qué falta, en concreto |
|---|---|---|---|
| 1 | Dart conduce una transferencia verificada | **CUMPLIDA** — fase 02 | — |
| 2 | El usuario elige el archivo con el selector de su sistema | **CUMPLIDA en código; compilada y probada en CI aguas abajo del diálogo, NO vista abrirse por nadie** | Nada bloquea. La clase de evidencia se escribe tal cual |
| 3 | Dos aparatos se encuentran, o hay camino manual | **PARCIAL** | La cadena `QYRO1\|addr\|32hex` existe y está probada como **texto**. Falta usarla para conectar: `qyro_net_smoke serve` imprime la cadena, `send` la acepta, y **rechaza por tipo si la huella autenticada no es la prometida** (ADR-0035 §2.1) |
| 4 | La huella se ve y una clave cambiada se rechaza | **PARCIAL** | El núcleo está hecho y probado en Rust: `Session::peer_fingerprint()`, `TrustBook`, `a_known_peer_whose_key_changed_is_refused_by_name`. **Falta cruzarlo por el FFI y por Dart** |
| 5 | El receptor puede rechazar | **NO CUMPLIDA** | QYR-0089 y QYR-0088. `TransferReject` está en el protocolo y **nadie lo emite ni lo entiende**; `FileSink` no puede abandonar una transferencia |

**Los botones NO se encienden hoy.** Faltan la 3 en su mitad de conexión, la 4 en
su mitad de FFI, y la 5 entera.

## 3. Lo siguiente, en orden

1. **Condición 5 — QYR-0089 y QYR-0088.** Es la única *no cumplida* y la que hace
   que la pantalla de recibir no sea una mentira. Prueba de las dos juntas: un
   receptor rechaza a mitad, el emisor recibe el motivo exacto y para, y el
   destino queda **sin un solo archivo nuevo**, comprobado listando el directorio.
2. **Condición 4 — el FFI de confianza.** Cinco símbolos:
   `qyro_session_peer_fingerprint`, `qyro_session_peer_trust`,
   `qyro_session_remember_peer`, `qyro_trust_forget_peer`,
   `qyro_trust_list_peers`. El libro es `qyro_session::TrustBook`, **en memoria**.
   Después, `a_known_peer_whose_key_changed_is_refused_by_name` **desde Dart**.
3. **Condición 3 — la cadena entre dos procesos.**
   `two_processes_connected_by_a_manual_endpoint_transfer_a_file`.
4. **ADR de la UI** (`ADR-0036-transfer-ui.md`), congelada antes de dibujar.
   Lo que tiene que decidir: los estados feos, **qué pasa cuando llega una
   transferencia de un desconocido** —si se acepta por defecto, Qyro es un buzón
   abierto para cualquiera en la Wi-Fi—, qué ve el receptor antes de decidir, y
   los dos idiomas.
5. **Las cuatro pantallas**: peers, enviar, recibir, historial. `qyro_fs::history`
   ya tiene `latest`, `for_peer` y `with_status`: exponer, no reescribir.
6. **Encender los botones** y **quitar el texto que explica por qué están
   apagados**, que deja de ser cierto.

## 4. Números actuales (comandos, no memoria)

| Comprobación | Comando | Valor |
|---|---|---|
| Tests Rust | `cargo test --workspace` | **615 passed, 0 failed, 2 ignored** (51 suites, Windows) |
| Tests Dart | `flutter test` con las 2 variables | **76 passed, 1 skipped** |
| Clippy · fmt | `--workspace --all-targets -D warnings` · `fmt --all --check` | exit **0** |
| Clippy target unix | `-p <crate> --all-targets --target aarch64-linux-android` | exit **0** |
| Docs | `check_docs_consistency` en Bash y PowerShell | exit **0** |
| Paquetes Rust / Dart | `[[package]]`/`source =` · sección `packages:` | **64 / 50** · **45** |
| Ledger | script de `R2` §1.10 | **147 fichas, 39 abiertas** |
| CI verde sobre | `62658d7`, `f153d61`, `39f645c`, `67dd8da`, `7c83134` | success |

## 5. El entorno (medido)

| Cosa | Dónde |
|---|---|
| Repo | `D:\Qyro\repo` · Rust 1.88.0 por `rust-toolchain.toml` |
| Flutter | `D:\flutter\bin\flutter.bat` 3.44.8 / Dart 3.12.2 · `dart.exe` en `D:\flutter\bin\cache\dart-sdk\bin` |
| `PUB_CACHE` | `D:\pub-cache` — exportar en cada shell |
| `cargo-mutants` | `D:\tools\cargo\bin\cargo-mutants.exe` 27.1.0 (subcomando `mutants`) |
| Git Bash | `C:\Program Files\Git\bin\bash.exe` — `bash` a secas es WSL sin distro |
| PowerShell | 5.1 únicamente, **no hay `pwsh`** |
| Targets Rust | `x86_64-pc-windows-msvc` y `aarch64-linux-android` |
| Pruebas FFI de Dart | `cargo build --release -p qyro_ffi -p qyro_net_smoke`, luego `QYRO_FFI_LIBRARY_PATH=…\target\release\qyro_ffi.dll` y `QYRO_NET_SMOKE_PATH=…\qyro_net_smoke.exe` |

**Modo Desarrollador APAGADO**: `flutter test` sí, `flutter build`/`run` no (QYR-0324).

## 6. Lo que ya está resuelto y no hay que volver a investigar

1. **`qyro_net::Session::peer_identity()` existe.** Y `peer_fingerprint()` ya
   existía desde 6A: lo que faltaba era la identidad, que es lo que
   `decide_trust` necesita — el almacén guarda la identidad completa, no su hash.
2. **`qyro_session` posee su vocabulario**: `PeerTrust` (Known/Changed/New, con
   `code()` escrito a mano) y `TrustBook`. No reexporta nada ajeno.
3. **La guarda de re-exportación ahora deriva** los módulos del `lib.rs` de la
   fachada en vez de listarlos, así que añadir un módulo ya no la pone roja.
   `CLOSURE` en `c_abi_contract.rs` incluye `qyro_identity_store`.
4. **La confianza NO persiste**, y es correcto: `seal_known_peers` necesita un
   `SecretWrapper` y en Android no hay hasta la fase 06. El libro muere con el
   proceso. Funciona la decisión, no su memoria.

## 7. Qué NO hay que rehacer

- **`cargo` no reconstruye con una mtime vieja.** Tras restaurar un backup:
  `(Get-Item ruta).LastWriteTime = Get-Date`.
- **Lo `cfg(unix)` no se compila aquí.** `--target aarch64-linux-android`; `check`
  no enlaza, no hace falta enlazador.
- **`check_docs_consistency` se corre sobre el commit FINAL.**
- Citar un `QYR-00xx` sin ficha bloquea. Para «el siguiente libre» escribe
  `QYR-nnnn+` con el `+` pegado al número, dentro de las comillas.
- `flutter pub get` sale 1 la primera vez tras cambiar un plugin y 0 la segunda.
  Reescribe `apps/qyro/windows/flutter/generated_plugin_*`: `git checkout --`.
- `dart format --set-exit-if-changed .` **desde la raíz** falla por
  `tools/branding_generator`. Córrelo desde `apps/qyro`, como CI.
- `Select-String` de PS 5.1 no tiene `-Recurse` y `-match` ignora mayúsculas.
- **`cargo-mutants --package` subestima** la cobertura de funciones cuyas pruebas
  viven aguas abajo. Usa `--test-workspace true`.

## 8. Reglas mínimas

1. **Sólo un P0 detiene una fase.** El resto → `deuda-de-calidad.md`, fase 09.
   Excepción: lo que *impide construir lo siguiente*.
2. **Autonomía total** salvo `main`, dinero, aparato físico o segunda persona.
3. **Los botones sólo se encienden con las cinco condiciones de §2 escritas y con
   su evidencia.** Y entonces se quita el texto que explica por qué están apagados.
4. **Las comprobaciones de la puerta son trece y se leen por CÓDIGO DE SALIDA.**
5. **Toda clase de evidencia se nombra.** «Compiló» nunca es «funciona».
6. **Nada de salida de herramienta en `BUGS_PENDING.md`.**
7. **Identificadores nuevos desde `QYR-0331+`.**
8. Toda ADR se congela **antes** del código, en commit propio.
9. **Nunca commits en `main`.** Sólo `claude/qyro-net-6a`.
10. Este archivo se reescribe y se commitea al cerrar cada paso.
