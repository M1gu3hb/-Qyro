# R6 — El estado base, verificado el 2026-08-12

**Estos números los reproduje yo** sobre el árbol fusionado de
`claude/qyro-net-6a` (`dd2099a`) y `codex/qyro-trust-5d` (`003dce4`). No están
copiados de ningún informe.

**Tu primera tarea es reproducirlos.** Si no coinciden, para y repórtalo: significa
que el árbol que tienes no es el que se planificó, y todo lo demás se apoya en eso.

---

## 1. Los números

| Comprobación | Valor | Comando |
|---|---|---|
| Tests | **527 passed, 0 failed, 2 ignored** | `cargo test --workspace` |
| Clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings` |
| Formato | PASS | `cargo fmt --all --check` |
| Paquetes | **63**, todos de primera parte | `grep -c '^\[\[package\]\]' Cargo.lock` |
| Coherencia docs | PASS | `bash scripts/check_docs_consistency.sh` |
| Ledger | **116 entradas, 24 abiertas** (5 P1, 13 P2, 6 P3) | ver §4 |

**Cero dependencias externas.** Siete sprints seguidos. `serde_json` está en el
grafo sólo como `dev-dependency` y no se enlaza en nada que Dart cargue.

---

## 2. Los diez crates y los tres tools

| Crate | Qué hace | Estado |
|---|---|---|
| `qyro_core` | Constantes compartidas | Hecho |
| `qyro_protocol` | Cabecera de 48 bytes, decodificador incremental acotado, identificadores autenticados (ADR-0029), guarda de progreso del decodificador | Hecho |
| `qyro_manifest` | Manifest canónico, SHA-256 obligatorio, validación dura de rutas (traversal, absolutas, Unicode `Cc` y `Cf`, reservados de Windows, colisiones portables) | Hecho |
| `qyro_crypto` | Ed25519 con `verify_strict`, handshake de 4 mensajes (X25519 + firmas sobre el transcript + HKDF-SHA256 + HMAC), ChaCha20-Poly1305 con la cabecera completa como AAD, nonce sin repetición, ventana de replay de 1024 | Hecho |
| `qyro_identity_store` | Formato del blob de identidad; **`known_peers`: confianza, `TrustVerdict`, `HumanFingerprint`, almacén sellado** (ADR-0031) | Hecho |
| `qyro_win_dpapi` | Envoltorio de Windows, `wrap = 0x01` | Hecho |
| `qyro_transfer` | Motor: chunks de 64 KiB, ventana de 16, go-back-N, pausa/reanudación/cancelación, veredicto SHA-256 por archivo, `Receiver::manifest()` | Hecho |
| `qyro_fs` | `FileSource`/`FileSink`, `.qyro-part`, rename tras digest verificado, política de symlinks con `O_NOFOLLOW`, reanudación desde `.qyro-resume`, **`history`: log append-only recuperable** | Hecho |
| `qyro_net` | Listener, dialer, `FrameStream` sobre el decodificador incremental, handshake sobre socket real, cinco finales tipados, límites antes de autenticar | Hecho |
| **`qyro_ffi`** | **Sólo dos funciones: `qyro_protocol_version_ptr` y `_len`. Depende únicamente de `qyro_core`.** | **Prácticamente vacío** |

Tools: `qyro_crypto_smoke`, `qyro_store_smoke`, `qyro_net_smoke`.

---

## 3. Lo que está demostrado, y con qué clase de evidencia

| Propiedad | Clase de evidencia |
|---|---|
| Un archivo de 8 MiB cruza **dos procesos reales** por un socket TCP y llega byte a byte idéntico | Probado entre procesos, Linux CI |
| Los cinco finales de una transferencia tienen error tipado y prueba que lo produce, incluido `Child::kill()` sobre un hijo real | Probado entre procesos, Linux |
| Ningún hilo ni descriptor sobrevive a una sesión terminada, **y hay una prueba que filtra cuatro descriptores a propósito para demostrar que la medida los ve** | Probado en integración, Linux |
| `O_NOFOLLOW` refuse un enlace en el componente final | Probado en **Linux, macOS y Windows** (job matriz `fs-final-component`) |
| Una transferencia interrumpida se reanuda leyendo `.qyro-resume` **desde producción** | Probado en unidad |
| La identidad sobrevive al reinicio | Probado en **Windows real** (DPAPI). **Android e iOS: NO implementado** |
| El workspace compila y pasa en Windows | Probado, job `rust-windows` |
| **Hardware físico** | **Ninguna. Cero pruebas en un teléfono o una máquina de verdad.** |

---

## 4. Las 24 entradas abiertas del ledger

**P1 — cinco:**

| ID | Qué | Dónde se cierra |
|---|---|---|
| QYR-0004 | Builds no retenidos | Fase 08 |
| QYR-0005 | Auditorías y suites avanzadas no disponibles | Fase 09 |
| QYR-0064 | El harness de binario empujado no alcanza Android Keystore | **Fase 06** |
| QYR-0078 | `qyro_net` no se ejecuta ni compila en Windows | **Fase 01** (comprobar si el job `rust-windows` ya lo cerró) |
| QYR-0295 | La materialización no prueba directamente todas sus barreras de integridad | Fase 09 |

**P2 — trece:** QYR-0001, 0052, 0053, 0054, 0056, 0065, 0066, **0088**
(`FileSink` no puede abandonar una transferencia), **0089** (`TransferReject`
existe y nadie lo emite), 0290, 0292, 0294, 0296.

**P3 — seis:** QYR-0003, 0057, 0059, 0069, 0090, 0092.

**QYR-0088 y QYR-0089 son las que más te van a estorbar**: la fase 05 necesita que
un receptor pueda **rechazar** una transferencia y que el emisor se entere, y hoy
el mensaje existe en el protocolo pero **nadie lo emite ni lo entiende**.

---

## 5. La infraestructura de verificación que ya existe — úsala, no la reinventes

- **`cargo-mutants 27.1.0`** está adoptado y **hay un job de CI que exige
  evidencia de mutación en el filesystem**. El barrido completo del 2026-08-11 está
  en `docs/reports/mutation-sweep-2026-08-11.md`, 1 172 líneas, con el alcance
  declarado por crate.
- **`assert_no_assertion_compares_a_call_to_itself`** — guarda estructural que
  falla si los dos lados de una aserción son textualmente idénticos. Mató un
  anti-patrón que este proyecto había producido cinco veces.
- **`every_workspace_crate_has_the_minimum_structural_guards_or_an_exact_exception`**
  — exige el conjunto mínimo de guardas por crate, y **las excepciones se
  auto-caducan**: si un crate exceptuado gana sus guardas, la guarda revienta
  pidiendo que se borre la excepción.
- **`assert_analysis_reached_the_end`** — comprueba que el análisis estructural
  leyó el archivo entero. Existe porque durante un sprint entero leyó 13 401 bytes
  de 30 861 y nadie lo notó (QYR-0071).
- **`check_docs_consistency`** en Bash y PowerShell — `STATUS.md` no puede nombrar
  un `Verified commit` a más de **diez commits** de HEAD, y **todo identificador
  `QYR-00xx` citado en cualquier archivo tiene que tener ficha en
  `BUGS_PENDING.md`**. Las dos reglas te van a morder si las ignoras.
- **Jobs de CI:** `ci.yml` (Linux + `rust-windows` + `fs-final-component` en tres
  sistemas), `platform-builds.yml`, `crypto-platform.yml`, `crypto-fuzz.yml`,
  `android-runtime.yml`, `ios-runtime.yml`.

---

## 6. La rama de la que sales

`claude/qyro-net-6a`, HEAD `dd2099a`, **con `codex/qyro-trust-5d` fusionada**.

Si esa fusión no está hecha cuando empieces, **hazla primero**: yo la probé y sale
con **un solo conflicto**, en `STATUS.md`, que se resuelve conservando los dos
lados. El árbol resultante da los 527 tests de §1.

---

## 7. Lo que este proyecto no ha hecho nunca, y hay que decirlo cada vez

- **Ninguna prueba en hardware físico.** Ni un teléfono, ni una tablet, ni una
  máquina Windows que no sea un runner de GitHub.
- **Ninguna transferencia entre dos máquinas distintas.** Dos procesos en
  `127.0.0.1` no son dos aparatos en una Wi-Fi: no hay pérdida de paquetes, ni
  MTU, ni suspensión de radio, ni aislamiento de cliente.
- **Ningún usuario ha usado esto.** Los botones siguen apagados.
- **Nada está firmado ni empaquetado.**
