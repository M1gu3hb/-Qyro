# Estado actual — v1.0

**Última actualización:** 2026-08-16. **Rama:** `claude/qyro-net-6a`.
**Nunca `main`. Nunca force-push. Nunca reescribir historia.**

---

## 1. Dónde está esto

**Terminado. Fases 00 a 10 cerradas, `v1.0.0` etiquetada.** Lo único que queda no
es código: es la **fase 07**, que necesita dos aparatos y una persona, y está
escrita escenario a escenario en `docs/testing/hardware-protocol.md`.

**Nada se ha ejecutado nunca en hardware físico.** No se inventa.

---

## 2. Qué existe

| | |
|---|---|
| Motor | Rust 1.88.0, edition 2024, once crates. Transferencia completa, cifrada y verificada |
| Frontera | 23 símbolos C, **ninguno cruza un tipo** (ADR-0032 + enmiendas 1 y 2) |
| Interfaz | Cuatro pantallas, dos idiomas, **botones encendidos** (fase 05, ADR-0036) |
| Descubrimiento | `NsdManager` + `FLAG_SHOW_PICKER`; `mdns-sd` bajo `cfg(windows)` (fase 04b) |
| Identidad | **Persiste entre procesos** (ADR-0040). DPAPI en Windows; en Android sin envolver, bajo el sandbox por UID. Keystore descartado para la v1.0 con argumento |
| Paquete | `dev.qyro.app`, firmado con clave propia (fase 08) |
| Pruebas | Rust **633 / 0 / 2 ignoradas**; Dart **90 pasadas, 10 saltadas** |
| Dependencias | `Cargo.lock` **80**; `pubspec.lock` **45**. Cero externas de Rust en Android |
| Ledger | **154 fichas, 0 abiertas** |

---

## 3. Las reglas que no cambian

1. **Nunca commits en `main`.** Nunca force-push, nunca borrar una rama.
2. **No se inventa evidencia de hardware.** Es lo único que arruinaría esto.
3. **ADR congelada antes del código, en su propio commit.**
4. **Nunca volcar salida de herramienta en `BUGS_PENDING.md`.**
5. **La puerta se pasa por exit code**, no por leer la salida.
6. **Dos destinos para una ficha: cerrada, o descartada con argumento.** Nunca
   «pendiente».
7. IDs nuevos desde **QYR-0355 en adelante**.

---

## 4. La puerta — trece comprobaciones, por exit code

```bash
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p qyro_session -p qyro_ffi --all-targets --target aarch64-linux-android -- -D warnings
cargo audit --deny warnings
```

```bash
cd apps/qyro && flutter analyze && flutter test && dart format --set-exit-if-changed .
```

```bash
bash scripts/check_docs_consistency.sh
powershell -NoProfile -File scripts/check_docs_consistency.ps1
```

Y después: `gh run list --branch claude/qyro-net-6a` en verde. **Un job rojo es
una afirmación sin evidencia** — es QYR-0350, que estuvo dos commits en rojo
mientras la fase 06 daba su test instrumentado por hecho.

---

## 5. El entorno de esta máquina

| | |
|---|---|
| Repo | `D:\Qyro\repo` (C: no tiene espacio) |
| Flutter | `D:\flutter`, 3.44.8 / Dart 3.12.2 — **no está en PATH** |
| Android SDK | `D:\android-sdk`, build-tools 34.0.0. **Sin NDK** |
| Keystore | `D:\qyro-release\qyro-release.jks`, alias `qyro` |
| Rust | 1.88.0, targets `aarch64-linux-android` y `x86_64-pc-windows-msvc` |

```powershell
$env:PATH="D:\flutter\bin;"+$env:PATH
```

---

## 6. Las trampas de esta máquina, ya pagadas

1. **`flutter build` no corre aquí.** «Building with plugins requires symlink
   support» — el Modo Desarrollador está apagado y es configuración del
   propietario (QYR-0324). Los artefactos los construye `release.yml` en CI.
2. **`Copy-Item` conserva el mtime del backup**, así que cargo no recompila.
   `(Get-Item ruta).LastWriteTime = Get-Date`.
3. **PowerShell lee el stderr de `git` como `NativeCommandError`.** No es un
   fallo: comprobar el resultado, no el flujo.
4. **`cargo test` en Windows no compila lo que está tras `cfg(unix)`.** Por eso
   la comprobación 13 usa `clippy --target aarch64-linux-android`, que no
   necesita linker de Android.
5. **Las guardas textuales pierden contra la sintaxis.** Ya ha pasado tres
   veces: QYR-0328 en Rust, QYR-0348 en Dart, y el falso positivo de la regla de
   hardware. Si una guarda lee fuente, **que salte comentarios y literales** y
   que tenga su control en los dos sentidos.
6. **Un `Progress` que cruza el FFI también cruza sus comentarios.** Una frase
   equivocada en Rust se lee igual de mal en Dart (QYR-0318).
7. `apps/qyro/lib/l10n/generated/` está en `.gitignore`: tras tocar un `.arb`,
   `flutter gen-l10n`.

---

## 7. Lo que sigue, y sólo hay una cosa

**La fase 07.** Dos aparatos, una Wi-Fi, una persona, y los veinte escenarios de
`docs/testing/hardware-protocol.md` anotados — **incluidos los que fallen**.

Lo que la fase 07 encuentre decide la v1.1, y no al revés. Escribir hoy una lista
de mejoras sería adivinar antes de la única medición que falta.
