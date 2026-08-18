# Auditoría de recuperación — Claude Code

Reporte de recuperación del estado real del repositorio antes de continuar el
trabajo que Codex dejó inconcluso. Todo lo que aparece aquí fue comprobado
ejecutando comandos o consultando la API de GitHub Actions. Lo que no pude
comprobar está marcado como NO VERIFICADO.

- Fecha UTC: 2026-08-05T00:13:46Z
- Sistema: Linux 6.18.5-fc-v18 (x86_64), contenedor efímero
- Herramientas presentes al iniciar: git 2.x, Rust 1.88.0, Python 3.11.15, Java 21
- Herramientas instaladas por esta sesión: Flutter 3.44.8 / Dart 3.12.2
  (la versión exacta que fija CI) y PowerShell 7.4.6
- Herramientas ausentes: Xcode/macOS, SDK/emulador Android, host Windows

## 1. Estado de las ramas

Los tres SHA del prompt maestro se confirmaron sin cambios:

| Ref | SHA | Comentario |
|---|---|---|
| `origin/main` | `e0041de377f787481d60886f83ab26f9211aab0f` | `Rename qyro-logo.png to no usar este logo` |
| `origin/audit/baseline-hardening` | `e9ed7f3951d7a42dae958048d4dc74e886fe1c7d` | `build: lock Flutter localization dependencies` |
| Merge base | `7ca3973cd1928ffaa3e7b112d121587d83d5092c` | `docs: record verified native ABI integration` |

    git rev-list --left-right --count origin/main...origin/audit/baseline-hardening
    2	58

Divergencia confirmada: `audit` está 58 commits adelante y 2 detrás. No se hizo
force-push, no se reescribió `main` y no se descartó ningún commit.

### Rama de trabajo

El prompt maestro pedía `claude/complete-qyro`. Las instrucciones de sesión
asignan `claude/qyro-recovery-continuation-j53jgx` como rama obligatoria, así que
se usó esa. Es el mismo punto de partida y la misma estrategia: se recreó desde
`origin/audit/baseline-hardening` (que concentra el trabajo) y luego se integró
`origin/main` por merge.

Respaldos locales antes de tocar nada:

- `backup/main-e0041de`
- `backup/audit-e9ed7f3`

## 2. Cambios del propietario y decisión sobre el logo

Los dos commits exclusivos de `main` son del propietario:

- `9596322` `logo real` — añadió `design/brand/source/logo.png`.
- `e0041de` — renombró `design/brand/source/qyro-logo.png` a
  `design/brand/source/no usar este logo`.

Checksums medidos (`sha256sum` sobre los blobs de cada rama):

| Archivo | SHA-256 | Significado |
|---|---|---|
| `main:design/brand/source/logo.png` | `e8413410…4f39` | Logo real |
| `audit:design/brand/source/qyro-logo.png` | `e8413410…4f39` | **Mismos bytes** |
| `main:design/brand/source/no usar este logo` | `52107d9e…258d` | Marcador rechazado |
| `7ca3973:design/brand/source/qyro-logo.png` | `52107d9e…258d` | Marcador original |

Hallazgo central: **las dos ramas ya contenían el mismo logo real**. Codex lo
había sustituido en `41f13a7 fix: replace truncated provisional logo source`
bajo el nombre antiguo; el propietario lo añadió en `main` bajo el nombre nuevo.
No había pérdida de contenido, solo un conflicto de nombres.

### Conflicto silencioso detectado en el merge automático

`git merge origin/main` terminó **sin conflictos**, pero produjo un árbol
incorrecto: Git combinó el renombrado de `main` con la modificación de contenido
de `audit` y dejó `design/brand/source/no usar este logo` con los bytes del
**logo real** (`e8413410…4f39`) en lugar del marcador rechazado.

Es exactamente el fallo silencioso contra el que advertía el prompt. Se corrigió
restaurando el archivo byte a byte desde `origin/main`, de modo que el árbol
final reproduce la intención del propietario:

- `design/brand/source/logo.png` → `e8413410…4f39` (ruta canónica de producción)
- `design/brand/source/no usar este logo` → `52107d9e…258d` (conservado, excluido)
- `design/brand/source/qyro-logo.png` → eliminado (absorbido por el renombrado)

La decisión completa está en `docs/adr/ADR-0014-canonical-logo.md`. El generador
ASCII, la documentación de marca y `THIRD_PARTY_NOTICES.md` apuntan ahora a
`design/brand/source/logo.png`, y cinco pruebas nuevas fijan los checksums e
impiden que el marcador rechazado vuelva a `apps/qyro/assets`.

Al regenerar los activos ASCII solo cambió una línea (`"source"`), lo que
demuestra que el arte generado ya provenía del logo real.

## 3. Documentación desactualizada

`STATUS.md` es la fuente canónica declarada, y estaba equivocado:

- Fijaba `Verified commit: 7ca3973`, **58 commits por detrás** del HEAD real.
- Declaraba `NOT_IMPLEMENTED` seis funciones que sí existen en el código:
  runtime ABI de Android, staticlib de iOS, branding generado, secuencia ASCII y
  StartupCoordinator, localización, y artefactos retenidos.
- Reportaba «9 tests» de Flutter cuando la suite real ejecuta **51**.

Causa raíz identificada: `scripts/check_docs_consistency.{sh,ps1}` validaba la
*estructura* de STATUS.md (que los encabezados existieran) pero nunca comprobaba
que `Verified commit` correspondiera al HEAD. Por eso la deriva pasó 58 commits
sin que CI lo notara. Se cerró esa brecha (sección 6).

Otros documentos desalineados:

- `DECISIONS.md` listaba ADR-0001..0010; en `docs/adr/` ya existían ADR-0012 y
  ADR-0013 (no hay ADR-0011).
- `HANDOFF.md` indicaba continuar en `audit/baseline-hardening`.
- `design/brand/source/README.md` y `THIRD_PARTY_NOTICES.md` nombraban el archivo
  antiguo `qyro-logo.png`.

## 4. Código real existente

Verificado leyendo las fuentes, no los Markdown:

| Área | Estado real |
|---|---|
| `rust/crates/qyro_core` | Real. `protocol_version()` → `QYRO/1`, `ReadinessReport` con componentes obligatorios. |
| `rust/crates/qyro_ffi` | Real. ABI C con `qyro_protocol_version_ptr` / `_len`, memoria estática sin transferir propiedad. |
| `apps/qyro/lib/ffi/qyro_native_api.dart` | Real y robusto. Fallos tipados: biblioteca ausente, símbolo ausente, puntero nulo, longitud inválida, UTF-8 inválido, versión incompatible. Sanea rutas a basename. |
| `apps/qyro/lib/startup/startup_coordinator.dart` | Real. Tareas obligatorias, generaciones para cancelación, timeout, retry, reduced motion, ciclo de vida, diagnóstico tipado. |
| `apps/qyro/lib/boot/` | Real. `AsciiLogoModel` (validación estricta), painters, `BootSequenceController`, `ScrambleDecodeEngine`, `CipherRainPainter`, `BootScreen`. |
| `tools/logo_ascii_generator/` | Real. Codificador/decodificador PNG propio sin dependencias, determinista, con modo `--check`. |
| `tools/branding_generator/` | Real. Valida configuración y emite `branding.g.dart`; marca `isProvisional`. |
| `apps/qyro/lib/l10n/` | Real. Catálogos `app_en.arb` / `app_es.arb` con `flutter_localizations`. |
| Transferencia de archivos | **No existe.** No hay protocolo, manifest, red, cifrado, base de datos ni modo óptico. |

Dos contratos importantes se confirmaron por lectura y por prueba:

- `BootSequenceController.canFinish` exige `isVisualComplete && startupReady`, así
  que la intro **no** termina solo por un temporizador.
- `skip()` solo adelanta el progreso visual; no marca tareas obligatorias como
  completadas. La prueba «skip never bypasses an obligatory startup task» lo fija.

## 5. Pruebas ejecutadas en esta sesión

Todas en el host Linux, con Flutter 3.44.8 (la versión que fija CI).

| Comando | Resultado | Detalle |
|---|---|---|
| `cargo fmt --all --check` | PASS | Sin diferencias |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | Sin avisos |
| `cargo test --workspace` | PASS | 4 tests (3 `qyro_core`, 1 `qyro_ffi`) |
| `flutter pub get --enforce-lockfile` | PASS | El lockfile satisface `pubspec.yaml` |
| `dart tools/branding_generator/bin/generate.dart --check` | PASS | Branding generado al día |
| `dart format --output=none --set-exit-if-changed .` | PASS | 27 archivos, 0 cambiados |
| `flutter analyze` | PASS | «No issues found!» |
| `flutter test` | PASS | **51 tests**, ~5 s, con `QYRO_FFI_LIBRARY_PATH` a `libqyro_ffi.so` |
| 5 contratos Bash | PASS | doctor, bootstrap, test_all, docs_consistency, launch_surface |
| 6 contratos PowerShell | PASS | los mismos más `windows_package` |
| `python3 -m unittest tools/logo_ascii_generator/…` | PASS | **7 tests** (2 previos + 5 nuevos del logo) |
| `bash/pwsh scripts/check_docs_consistency` | PASS | tras reescribir STATUS.md |

`flutter test` incluye «reads QYRO/1 from the compiled Rust library», es decir,
un paso real Dart→Rust por FFI en este host, no un mock.

## 6. Defectos reales encontrados y corregidos

### 6.1 iOS no compila desde `67fa795` (regresión bloqueante)

Evidencia — historial completo del workflow «iOS runtime ABI»:

| Run | SHA | Conclusión |
|---|---|---|
| 30953803079 | `9104421` | success |
| 30956527561 | `4f7ed01` | success |
| 30960631901 | `67fa795` | **failure** |
| 30961031089 | `9bfb1cc` | **failure** |
| 30961153321 | `e9ed7f3` (HEAD) | **failure** |

Log del run 30961153321, paso «Build unsigned iOS application with qyro_ffi»:

    Error (Xcode): The document "LaunchScreen.storyboard" could not be opened.
    The operation couldn't be completed. (com.apple.InterfaceBuilder error -1.)

Causa: `67fa795 feat: darken native launch surfaces` reescribió el storyboard y
eliminó los atributos `toolsVersion` y `systemVersion` del elemento `<document>`
(y la `version` del plugIn), a la vez que declaraba una `<capability>` con
`minToolsVersion`. Sin `toolsVersion`, `ibtool` no puede abrir el documento y
falla la compilación completa antes de ejecutar ningún código Dart.

Corrección: se restauró la estructura del documento que ya había compilado con
éxito antes de `67fa795`, conservando el fondo oscuro de Qyro y sin reintroducir
`LaunchImage`. Los pasos 8–10 de ese workflow (verificación de símbolos, arranque
del simulador y XCTest) quedaron en `skipped` por el fallo del paso 7, así que
**la vinculación de qyro_ffi en iOS sigue sin verificarse en HEAD**.

### 6.2 STATUS.md podía derivar sin límite

`check_docs_consistency` no comparaba `Verified commit` con el HEAD. Se añadió la
regla por TDD en Bash y PowerShell:

- SHA ausente o mal formado → `[BLOCKER] Malformed verified commit`
- SHA que no existe o no es alcanzable desde HEAD → `[BLOCKER] Unknown verified commit`
- HEAD más de 10 commits por delante → `[BLOCKER] Stale verified commit`

El límite es configurable con `QYRO_MAX_STATUS_COMMIT_LAG`. La tolerancia existe
porque STATUS.md no puede contener el SHA del commit que lo introduce. La regla
se omite (SKIP) en clones superficiales y fuera de un árbol Git, y el job
`documentation` de CI pasó a `fetch-depth: 0` para que la comprobación sea real.

Comprobación de que la regla detecta el defecto original:

    $ bash scripts/check_docs_consistency.sh   # con el STATUS.md antiguo
    [BLOCKER] Stale verified commit: HEAD is 58 commits ahead of the verified commit (limit 10)

### 6.3 El contrato de launch surfaces no detectaba un storyboard ilegible

Solo comprobaba cadenas de color. Se añadió validación estructural en Bash y
PowerShell: XML bien formado, elemento raíz `<document>`, atributos
`toolsVersion`/`targetRuntime`/`initialViewController` presentes, `launchScreen`
activo, y ninguna `capability` por encima de `toolsVersion`. Verificado en rojo
contra el storyboard de `67fa795` y en verde contra el corregido, en ambos shells.

### 6.4 Archivos generados sin ignorar

`apps/qyro/lib/l10n/generated/` y `__pycache__/` aparecían como no rastreados.
Añadidos a `.gitignore`.

## 7. Estado de los workflows

Último HEAD `e9ed7f3` de `audit/baseline-hardening`:

| Workflow | Run | Conclusión |
|---|---|---|
| CI | 30961157153 | success |
| iOS runtime ABI | 30961153321 | **failure** (§6.1) |
| Android runtime ABI | 30961153377 | **`in_progress` desde 2026-08-04T23:47Z** |

El run de Android lleva más de 24 h en `in_progress`. `get_workflow_run_usage`
devuelve `total_ms: 0`, es decir, **nunca llegó a ejecutarse**: quedó esperando
runner. No se canceló porque el workflow declara
`concurrency: android-runtime-${{ github.ref }}` con `cancel-in-progress: true`,
así que el próximo push a esa ref lo desplaza automáticamente. Queda registrado
como pendiente, no como evidencia.

Historial de «Android runtime ABI»: 40 runs, **un solo `success`** (run
30957598982, SHA `c971c9a`). La gran mayoría figuran como `cancelled` porque cada
commit nuevo cancelaba el anterior por concurrencia — el patrón que generó la
avalancha de correos descrita en el prompt. El último run completado
(30961031082, SHA `9bfb1cc`) falló por otra causa:

    Unable to satisfy `pubspec.yaml` using `pubspec.lock`.

Ese fallo ya está resuelto: el commit siguiente (`e9ed7f3`) actualizó el
lockfile, y en esta sesión `flutter pub get --enforce-lockfile` pasa en HEAD.

Nota de disparo: `android-runtime.yml` e `ios-runtime.yml` solo se activan por
push a `audit/baseline-hardening` (o por `workflow_dispatch`), y `ci.yml` por
push a `main` o por pull request. Empujar la rama de trabajo **no dispara nada**,
lo que mantiene el ruido bajo; obtener evidencia de iOS/Android exige
`workflow_dispatch` explícito o un pull request.

## 7 bis. Confirmación en CI de esta rama

Tras empujar `ff933d9`, se lanzaron ambos workflows de runtime con
`workflow_dispatch` sobre `claude/qyro-recovery-continuation-j53jgx`:

| Workflow | Run | Conclusión |
|---|---|---|
| iOS runtime ABI | 30963011815 | **success**, 10/10 pasos |
| Android runtime ABI | 30963016390 | **success**, 8/8 pasos |

En iOS pasaron por primera vez desde `67fa795` los tres pasos que antes quedaban
en `skipped`:

- «Build unsigned iOS application with qyro_ffi» — confirma la corrección del
  storyboard.
- «Verify native symbols in the unsigned application» — `nm -gU` sobre
  `Runner` y `Runner.debug.dylib` encuentra `_qyro_protocol_version_ptr` y
  `_qyro_protocol_version_len`, es decir, el staticlib **sí** queda enlazado.
- «Execute qyro_ffi XCTest through the Runner host» — XCTest real en simulador.

En Android, el paso «Execute native ABI smoke test in an Android emulator»
ejecutó `integration_test/native_abi_smoke_test.dart` en un emulador API 35
`google_apis` x86_64 con KVM, recuperando una verificación que en `e9ed7f3` no
tenía ninguna ejecución válida.

Esto convierte las dos entradas NOT_VERIFIED de la sección 8 en verificadas.

## 8. Riesgos abiertos

1. Ninguna plataforma se ha probado en **hardware físico**: solo emulador,
   simulador y host. Un simulador no equivale a un dispositivo.
2. `ci.yml` no se ha ejecutado en esta rama porque solo se dispara por push a
   `main` o por pull request. Su contenido se reprodujo íntegro en el host Linux.
3. Este entorno no tiene macOS, SDK de Android ni Windows: la evidencia de las
   tres plataformas obligatorias depende de CI.
4. La marca sigue provisional (`REPLACE_WITH_*`, `com.owner.qyro`), lo que debe
   seguir bloqueando cualquier empaquetado público.
5. Autoría y licencia del logo siguen sin registrar.
6. `cargo audit` todavía no es obligatorio y no hay SBOM ni lockfile de licencias.
7. No existe ninguna función de transferencia: el producto no es usable todavía.

## 9. Plan de continuación

Hito A (esta sesión) cierra con: ramas reconciliadas, logo canónico, regresión de
iOS corregida y cubierta, brecha de deriva documental cerrada, y STATUS/HANDOFF/
NEXT_STEPS reescritos con evidencia real.

Además, iOS y Android quedaron confirmados en CI sobre esta rama (sección 7 bis).

Orden siguiente, sin saltarse ninguno:

1. Hito C: `qyro_protocol`, `qyro_manifest` y `qyro_transfer` por TDD, con
   validación de rutas, límites antes de reservar memoria y corpus de vectores.
2. Terminar el Hito 1 visual: golden tests (0/20/50/80/100 %, teléfono, tablet,
   Windows, reduced motion, fallo de FFI, branding provisional) y benchmark.
3. Hitos D–L según el prompt maestro.

Ninguna función de transferencia debe declararse antes de existir, y los botones
Enviar/Recibir deben seguir deshabilitados hasta que haya transporte real.
