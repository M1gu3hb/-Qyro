# Fase 03 — El selector de archivos

**Base de la fase:** `546dbf6`. **Último commit:** `2fd36bb`.
**Rama:** `claude/qyro-net-6a`. **Escrito durante la fase, no al final.**

---

## 1. Objetivo y alcance

> **Que el usuario elija qué mandar y dónde recibir, con el selector de su propio
> sistema, sin que Qyro pida ni un permiso de almacenamiento.**

**No objetivos declarados por el documento de fase:** descubrimiento, UI,
emparejamiento, Keystore, empaquetado, permisos de red, y persistir URIs para el
historial. **iOS queda fuera de la v1.0 por ADR-0039**, así que la mitad iOS de
esta fase está aplazada, no cancelada.

**No objetivo añadido por esta sesión:** nada de lo que aparece en
`docs/reports/deuda-de-calidad.md` se arregla aquí. Rige la regla del carril.

---

## 2. Qué se hizo

Contra los cuatro pasos de `FASE-03 §5`:

| Paso | Estado | Dónde |
|---|---|---|
| 1 — resolver el bloqueador de §4.3 y congelar ADR-0034 | **hecho** (sesión anterior) | `269f0fa`, ficha QYR-0323 |
| 2 — la superficie FFI por descriptor | **hecho** (código en `b4ebac7`, **pruebas en esta sesión**) | `867d3fa` |
| 3 — Android: selector desde Dart con `"rw"` y `detachFd()` | **hecho** (sesión anterior); **sin prueba en emulador** | `9137220` |
| 4 — Windows: `file_selector` desde Dart, ruta a Rust | **hecho en esta sesión** | `54c66e2`, `867d3fa` |

Lo concreto de esta sesión:

1. **ADR-0034, enmienda 1.** La dependencia de Windows es
   `file_selector_windows` 0.9.3+5 y no el paraguas `file_selector`. Congelada
   en `54c66e2`, **antes** del código, con los conteos medidos.
2. **El diálogo de Windows conectado.** `QyroDesktopFilePicker` pedía un callback
   y lanzaba `UnsupportedError` si no se lo daban; hoy `QyroWindowsFilePicker`
   llama al diálogo real por defecto y deja `openPaths` como costura de prueba.
3. **`pickerForPlatform` rechaza por nombre** Linux, macOS, iOS y cualquier otra
   cosa, en vez de devolver una lista vacía — que es lo que devuelve un selector
   cancelado, y no se pueden distinguir.
4. **Las cuatro pruebas que la fase pide por nombre para el paso 2**, que no
   existían: el commit `b4ebac7` aterrizó la superficie por descriptor sin tocar
   un solo archivo de test.
5. **Doce pruebas de Dart para el selector**, que tampoco existían.
6. **Tres defectos encontrados por el camino**, arreglados: QYR-0327, QYR-0328
   (P1) y QYR-0329 (P1). §4 los cuenta.

---

## 3. Cómo se hizo

### La decisión del paquete, y la alternativa descartada

ADR-0034 §1 nombraba `file_selector`. Se midieron las dos opciones antes de
elegir, con `flutter pub get` y el conteo de la sección `packages:` del
`pubspec.lock`:

| Opción | Paquetes Dart | Delta |
|---|---|---|
| baseline | 37 | — |
| `file_selector` 1.0.3 | 52 | **+15** |
| `file_selector_windows` 0.9.3+5 | 45 | **+8** |

Los siete de diferencia: `file_selector`, `file_selector_android`,
`file_selector_ios`, `file_selector_linux`, `file_selector_macos`,
`file_selector_web`, `flutter_web_plugins`.

**El que decide es `file_selector_android`**: *es* la implementación que copia el
archivo entero a la caché (QYR-0323), que es el motivo por el que este
repositorio escribió su propio `MethodChannel`. Depender del paraguas mete ese
Java en el APK de una aplicación que nunca debe llamarlo, y *una dependencia que
hay que acordarse de no llamar es una trampa para quien venga después*.

**No es una decisión de permisos.** El `AndroidManifest.xml` de
`file_selector_android` 0.5.2+9 está vacío —leído del paquete en la caché, no de
su documentación—, así que el criterio 6 no corría peligro por ninguna de las dos
vías. Es una decisión de no embarcar el código que copia.

**Lo que cuesta:** no hay selector en Linux ni macOS. Ninguna es plataforma de la
v1.0.

### La costura de la prueba

`QyroWindowsFilePicker.openPaths` tiene por defecto el diálogo real, así que
producción no necesita cableado. Una prueba lo sustituye, porque `flutter test`
corre en la VM de Dart sin ventana y un diálogo modal de Win32 necesita una. La
alternativa —no probar nada de esto— era peor.

### El nombre de la prueba de cierre, corregido

El documento de fase pide
`a_file_chosen_through_the_system_dialog_transfers_and_verifies`. **Ese nombre no
se puede escribir con honestidad aquí**: ningún diálogo se abre en esta máquina
ni en CI. La prueba se llama
`a_file_chosen_through_the_picker_transfers_and_verifies` y ejerce todo lo que
hay aguas abajo del diálogo. Un nombre que enuncia una propiedad que el cuerpo no
ejerce es el anti-patrón 3 de este repositorio.

---

## 4. Qué se encontró que no estaba en el plan

| # | Hallazgo | Dónde | Gravedad | Cómo se descubrió |
|---|---|---|---|---|
| 1 | El paso 2 aterrizó **sin una sola prueba**; la fase pide cuatro por nombre | `b4ebac7` | P1 de proceso | `git show --stat b4ebac7`: cinco archivos, ninguno de test |
| 2 | Un **byte NUL crudo** en `session_abi.rs` hacía que ripgrep saltara el archivo entero | `qyro_ffi` | P2 — QYR-0327 | Una búsqueda del símbolo devolvió cero con la función delante, y me hizo concluir que el paso 2 no existía |
| 3 | `item_end` no saltaba literales de carácter: un `'}'` cerraba el módulo de pruebas y **el análisis de panicos leía un archivo truncado** | `rust/guards/source_guard.rs` | **P1 — QYR-0328** | Añadí una prueba al módulo y `no_production_path_can_panic` falló señalando un `.expect(` que estaba dentro de `#[cfg(test)]` |
| 4 | La prueba del **manifiesto fusionado no corrió nunca**, en ningún sitio | `platform-builds.yml` | **P1 — QYR-0329** | Buscando dónde se verifica el criterio 6: el único job que construye un APK no ejecutaba pruebas después |
| 5 | El rebobinado tras el digest es **código muerto** y el commit que lo introdujo afirma lo contrario | `qyro_fs::manifest_from_open_files` | P3 — QYR-0330 | Mutación manual: borrarlo deja 600 tests en verde |
| 6 | `http` entra en el árbol de Dart de una aplicación que promete no hablar con la nube | `apps/qyro` | P3 — QYR-0326 | Conteo del `pubspec.lock` al medir las dos opciones |
| 7 | El ledger tenía **37** abiertas y el prompt de sesión decía 38 | `BUGS_PENDING.md` | informativo | Script canónico de `R2` §1.10 sobre `9137220` |
| 8 | `dart format --set-exit-if-changed .` desde la raíz **falla hoy** por `tools/branding_generator`; CI no lo cubre porque corre en `apps/qyro` | `.github/workflows/ci.yml` | P3, carril | Al correr la comprobación 1 de la puerta desde la raíz |
| 9 | `assert_analysis_reached_the_end` compara la última línea no vacía, y en un `.rs` esa línea es `}` | `source_guard.rs` | P3, carril | Es la razón por la que el hallazgo 3 pasó desapercibido |

**Y un error propio, escrito porque es lo que hace creíble el resto.** Al medir
la consecuencia de añadir el plugin escribí en la ADR que `flutter pub get` sale
1 en esta máquina. Luego una corrida salió **0** y empecé a corregir la ADR por
lo que parecía un error mío. La corrida en verde era la que no probaba nada: con
`.dart_tool/` al día Flutter **se salta** el paso de symlinks. La afirmación
original era correcta y la corrección habría sido falsa. Está escrito en
`205204f` con el orden exacto que lo reproduce.

---

## 5. Qué se arregló y qué no

| Ficha | Sev | Qué | ¿Arreglado? | Por qué |
|---|---|---|---|---|
| QYR-0327 | P2 | Byte NUL crudo invisible a grep | **sí, cerrada** | Impedía buscar en el archivo central de esta fase |
| QYR-0328 | P1 | `'}'` truncaba el análisis de guardas | **sí, cerrada** | **Bloqueo**: no se podían añadir pruebas a ese módulo sin que escaparan del strip |
| QYR-0329 | P1 | La prueba del manifiesto fusionado no corría | **sí, cerrada** | Es el criterio 6 de esta fase; sin esto no se puede afirmar |
| QYR-0323 | P1 | `file_selector_android` copia | **sí, cerrada** en `9137220` | Bloqueo del paso 1, cerrado en la sesión anterior |
| QYR-0326 | P3 | `http` en el árbol de Dart | **no** | Carril → fase 09. Haría falta una guarda de importaciones, o escribir `IFileOpenDialog` a mano, que ADR-0034 §4.2 rechaza |
| QYR-0330 | P3 | Rebobinado muerto | **no** | Carril → fase 09. Matarlo exige un handle no buscable, que en Windows no se monta trivialmente |
| QYR-0324 | P2 | Sin Modo Desarrollador | **no** | Depende del propietario. `start ms-settings:developers` abre el panel; es configuración del sistema y no la toca esta sesión |

**Sin ficha propia, en el carril:** el `dart format` de la raíz y la vacuidad de
`assert_analysis_reached_the_end` (hallazgos 8 y 9). Los dos están en
`docs/reports/deuda-de-calidad.md`.

---

## 6. A qué afectaba cada defecto

- **QYR-0327.** A quien audita, incluido yo. La mitad de la verificación de este
  proyecto es textual —guardas, revisiones, búsquedas— y un archivo que ninguna
  herramienta de texto lee no está cubierto por nada de eso. **Escenario
  concreto:** busqué `open_sender_fd` en todo `rust/`, obtuve cero resultados, y
  di por hecho que el paso 2 no tenía implementación. Lo tenía.
- **QYR-0328.** A cada crate que incluye `source_guard.rs`, es decir a siete.
  **Escenario concreto:** `session_abi.rs` tiene 1 075 líneas y el análisis de
  panicos leía hasta la línea ~790. Un `.unwrap()` escrito después de ese punto,
  en producción, habría pasado la guarda. Es la misma forma que QYR-0071, que
  hizo que cuatro sprints midieran menos de lo que decían.
- **QYR-0329.** Al criterio 6 entero. **Escenario concreto:** un plugin de
  Flutter añade `READ_MEDIA_IMAGES` al manifiesto fusionado; el APK lo pide; la
  prueba que existe para detectarlo se salta y el salto se cuenta como no-fallo.
  Nadie se entera hasta que alguien instala el APK y ve la pantalla de permisos.
- **QYR-0326.** A nadie hoy. Es una dependencia que viaja sin que nadie la llame.
  Se registra porque el argumento entero del producto es que no habla con nadie.
- **QYR-0330.** A nadie. Es código defensivo redundante; lo que estaba mal era la
  afirmación escrita sobre él.

---

## 7. Resultado contra el objetivo

Los once criterios de aceptación de `FASE-03 §7`, uno a uno:

| # | Criterio | Veredicto |
|---|---|---|
| 1 | El bloqueador de §4.3 resuelto con evidencia **medida** | **Cumplido** — QYR-0323, leído del Java del paquete fijado |
| 2 | ADR-0034 congelada antes del código | **Cumplido** — `269f0fa` precede a `b4ebac7`; la enmienda 1 (`54c66e2`) precede a `867d3fa` |
| 3 | La superficie por fd existe, con `SAFETY:` escrito y la lista de exentos actualizada | **Cumplido** — y la lista de exentos **no creció**: `qyro_fs` y `qyro_session` conservan `#![forbid(unsafe_code)]` |
| 4 | **El fd se cierra exactamente una vez**, con prueba | **Parcial** — probado que se **libera** (`the_descriptor_is_closed_exactly_once`, `cfg(unix)`); la mitad «no dos veces» no es observable dentro del proceso y descansa en `the_crate_closes_no_descriptor_by_hand` + la propiedad de `Drop`. Dicho en el propio doc-comment |
| 5 | Android abre en `"rw"`, con prueba o argumento escrito del seek | **Parcial** — el argumento está escrito (ADR-0034 §1) y el Kotlin lo hace; **nadie lo ha visto correr** |
| 6 | El manifiesto de Android no declara ningún permiso de almacenamiento | **Cumplido sobre el manifiesto fuente; pendiente de la corrida de CI sobre el fusionado**, que hasta hoy no existía (QYR-0329) |
| 7 | Un archivo elegido por el usuario se transfiere y verifica **en emulador Android y en Windows** | **No hecho** — ver §15. No hay emulador en esta máquina y el Modo Desarrollador está apagado |
| 8 | Cero crates de Rust nuevos; en Dart sólo `file_selector` de flutter.dev. **Di los dos conteos** | **Cumplido con una desviación declarada** — Rust: **64 → 64**. Dart: **37 → 45**. La desviación es el paquete, no el publisher: `file_selector_windows` en vez del paraguas, ADR-0034 enmienda 1 |
| 9 | Barrido con `cargo-mutants`, alcance declarado | **Cumplido** — §10 |
| 10 | `R2` en todas las puertas; informe según `R5` | **Cumplido** — §9 y este documento |
| 11 | Los botones siguen `onPressed: null` | **Cumplido** — sin tocar; `Home keeps transfer actions visibly disabled` sigue en verde |

**Veredicto de fase: PARCIAL.** Ocho cumplidos, dos parciales, uno no hecho. El
no hecho es el criterio 7 y su causa es de hardware, no de código: sin emulador y
sin Modo Desarrollador nadie puede ver ni el selector de Android ni el de
Windows. La fase **no se declara cumplida**.

---

## 8. Clase de evidencia por afirmación

| Afirmación | Clase | Plataforma | Dónde |
|---|---|---|---|
| Un archivo abierto por descriptor se lee igual que uno abierto por ruta | **Probado en integración** | Windows 10 | `a_file_opened_by_descriptor_reads_identically_to_one_opened_by_path` |
| Una transferencia conducida por descriptor llega byte a byte | **Probado en integración** | Windows 10 | `a_transfer_driven_by_descriptor_arrives_byte_identical` |
| Los nombres que sólo el selector conoce viajan correctos | **Probado en integración** | Windows 10 | mismo test: llega `holiday.jpg` y **no** `first.bin` |
| Un descriptor que deja de dar bytes termina la sesión y no cuelga | **Probado en integración** | Windows 10 | `a_revoked_descriptor_mid_transfer_is_a_typed_error_not_a_hang`, con `recv_timeout` de 60 s |
| El descriptor entregado se libera, también en el camino de fallo | **Compilado** aquí; **probado en unidad** en CI | Linux (`cfg(unix)`) | `the_descriptor_is_closed_exactly_once` — **no corre en Windows** |
| Un argumento rechazado cierra igualmente lo que recibió | **Compilado** aquí; **probado en unidad** en CI | Linux | `a_rejected_argument_still_closes_what_it_was_handed` |
| Nada del crate cierra un descriptor a mano | **Probado en unidad** | Windows 10 | `the_crate_closes_no_descriptor_by_hand` |
| Un archivo elegido por el selector se transfiere entre dos procesos y se verifica | **Probado entre procesos** | Windows 10 | `a_file_chosen_through_the_picker_transfers_and_verifies` — **el diálogo está sustituido por su costura** |
| El canal de Android decodifica descriptores, nombres y tamaños | **Probado en unidad** | Windows 10 (VM de Dart, canal simulado) | `qyro_file_picker_test.dart` — **el `MethodChannel` real no corre** |
| El nombre que viaja nunca es una ruta | **Probado en unidad** | Windows 10 | `leafName`, con entradas hostiles |
| Una plataforma no soportada se rechaza por nombre | **Probado en unidad** | Windows 10 | `pickerForPlatform(operatingSystem: …)` |
| El manifiesto **fuente** no declara permiso de almacenamiento | **Probado en unidad** | Windows 10 | `android_manifest_test.dart`, primer test |
| El manifiesto **fusionado** no declara permiso de almacenamiento | **Pendiente de CI** | Linux | segundo test, ahora con `QYRO_REQUIRE_MERGED_MANIFEST=1` |
| El diálogo de Windows se abre y devuelve una ruta | **Ninguna** | — | Nadie lo ha visto. `flutter build windows` no corre aquí (QYR-0324) |
| El SAF de Android devuelve un fd que Rust puede leer | **Ninguna** | — | Nadie lo ha visto. Sin emulador ni teléfono |

---

## 9. Las puertas

### Puerta del paso 4 y de la fase — 2026-08-14, sobre `2fd36bb`

| # | Comprobación | Comando | Veredicto |
|---|---|---|---|
| 1 | Formato Rust | `cargo fmt --all --check` | **exit 0** |
| 2 | Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** |
| 3 | Tests | `cargo test --workspace` | **exit 0** — 603 passed, 0 failed, 2 ignored, 50 suites. Los 2 ignorados son los mismos de la línea base (regeneran vectores) — **sin ignorados nuevos** |
| 4 | Barrido de mutación | §10 | **Cumplido, con alcance declarado** |
| 5 | Lectura de aserciones | manual + `assert_no_assertion_compares_a_call_to_itself` | **Cumplido** — §9.1 |
| 6 | Lectura de contadores | — | **No aplica**: esta fase no añadió ningún contador `cfg(test)` |
| 7 | La medida se ve fallar | §9.2 | **Cumplido** |
| 8 | Lectura de nombres | §9.3 | **Cumplido, con una corrección** |
| 9 | Coherencia del informe | relectura contra el código | **Cumplido** — §9.4 |
| 10 | El ledger sigue legible | script de `R2` §1.10 | **147 fichas, 39 abiertas** (era 142/37). **+5 fichas**, bajo el techo de diez |
| 11 | Coherencia documental | `check_docs_consistency` en Bash y PowerShell | **exit 0 los dos** — a la segunda. Ver §14, causa 3 |
| 12 | Escribir el resultado | este documento | hecho |

**Y una decimotercera que esta fase añade a la puerta**, porque su ausencia dejó
llegar a CI tres pruebas que no compilaban:

| # | Comprobación | Comando | Veredicto |
|---|---|---|---|
| 13 | El código `cfg(unix)` compila | `cargo clippy -p qyro_ffi --all-targets --target aarch64-linux-android -- -D warnings` | **exit 0** |

*`check` no enlaza, así que basta con tener el `std` del target: no hace falta un
enlazador de Android ni una máquina Linux. Sin esto, en Windows, todo lo que vive
detrás de `cfg(unix)` es invisible para las doce comprobaciones anteriores.*

**Y las tres que la fase añade sobre Dart**, porque el producto es mitad Dart:

| Comprobación | Comando | Veredicto |
|---|---|---|
| Formato Dart | `dart format --output=none --set-exit-if-changed .` desde `apps/qyro` | **exit 0** |
| Analizador | `flutter analyze` | **exit 0**, «No issues found» |
| Tests Dart | `flutter test` con `QYRO_FFI_LIBRARY_PATH` y `QYRO_NET_SMOKE_PATH` | **exit 0** — 76 passed, 1 skipped |

*El único saltado es `reads QYRO/1 from the compiled Rust library`, que pide otra
variable de entorno; corre en el job de Windows de `platform-builds.yml`.*

### 9.1 — Lectura de aserciones

Los dos lados difieren en todas las nuevas. Las que merecen decirse:

- `assert_eq!(arrived_by_path, arrived_by_handle)` — dos archivos distintos,
  producidos por dos caminos distintos del motor. No es una llamada consigo misma.
- `assert_ne!(read_all(&arrived_first).len(), read_all(&arrived_second).len())` —
  dos tamaños escritos a propósito distintos, que es lo que permite ver una
  colisión de nombres.
- `expect(first.descriptor, isNot(second.descriptor))` y lo mismo con el tamaño:
  una implementación que leyera la primera entrada dos veces satisface todo lo
  demás y falla aquí.
- `assert_ne!(what_the_descriptor_points_at(descriptor), identity)` — el lado
  derecho se capturó **antes** de la entrega, del `File` original.

### 9.2 — La medida se ve fallar

Cuatro mediciones nuevas, cuatro contra-pruebas:

| Medición | Prueba que la ve fallar |
|---|---|
| «ningún `.rs` lleva un NUL crudo» | La propia guarda comprueba que detecta un NUL en `b"…\0…"` **y** que no marca el escape `\0`, así que no confunde el arreglo con el defecto |
| «el descriptor fue liberado» | `a_descriptor_that_was_not_handed_over_stays_visible_to_this_measurement` mantiene uno vivo a propósito y exige que la misma medición lo vea. Sin ella, la aserción negativa pasaría gratis si la medición estuviera rota |
| «el strip no se pasa de largo» | Los dos casos nuevos de `the_analysis_actually_strips`: un `'}'` dentro de un módulo cerrado, y dos tiempos de vida en una línea que **no** deben tomarse por literales |
| «nada del app importa el paraguas» | El escáner exige haber leído más de 10 archivos y comprueba que **sí** encuentra `package:file_selector_windows/` donde debe estar |

Y una que **no** tiene contra-prueba propia, dicho en voz alta:
`a_revoked_descriptor_mid_transfer_is_a_typed_error_not_a_hang` afirma una
**negativa** —«no termina en `Completed`»— y una mutación que rompa la entrega la
satisface igual. Su control positivo es indirecto: las otras dos pruebas del
mismo arnés **sí** completan, así que el arnés no está roto de fábrica. Se dice
aquí en vez de dejarlo implícito.

### 9.3 — Lectura de nombres

Cada test nuevo ejerce lo que su nombre dice. **Una corrección:** el nombre que
pedía la fase, `a_file_chosen_through_the_system_dialog_transfers_and_verifies`,
habría sido falso — ningún diálogo se abre. Se llama
`a_file_chosen_through_the_picker_transfers_and_verifies`, y su comentario dice
exactamente qué no cubre.

### 9.4 — Coherencia del informe

Reobtenidos en la puerta, no citados de memoria: los 603 tests, los 76 de Dart,
los 64/50 paquetes de Rust, los 37→45 de Dart, y las 147/39 fichas. El conteo de
Dart se recalculó con un script que lee **sólo** la sección `packages:`, porque
el patrón suelto que usó QYR-0324 cuenta además las dos entradas de `sdks:` y por
eso aquella ficha dice 39→54 donde éste dice 37→52. Misma medición, denominador
distinto; no se edita ficha ajena, se explica.

---

## 10. Tabla de mutación

*(Rellenada al cerrar la puerta; ver §10.2 para el alcance.)*

### 10.1 — Mutaciones aplicadas a mano, con su resultado

| Control | Mutación aplicada | Resultado | Test que falló | Commit |
|---|---|---|---|---|
| El separador escrito como escape y no como byte | Reintroducir el NUL crudo en `session_abi.rs` | **Muerto** | `guards::no_rust_source_carries_a_raw_nul_byte`, exit 101 | `867d3fa` |
| `item_end` salta literales de carácter | Borrar el bloque `if bytes[index] == b'\''` de `source_guard.rs` | **Muerto** | `guards::the_analysis_actually_strips`, exit 101 | `867d3fa` |
| `descriptors_by_item` entrega los handles | Vaciar el bucle: devolver un mapa vacío | **Muerto** | `a_file_opened_by_descriptor_reads_identically_to_one_opened_by_path` **y** `a_transfer_driven_by_descriptor_arrives_byte_identical` | `867d3fa` |
| Rebobinado tras el digest en `manifest_from_open_files` | Borrar el `seek(SeekFrom::Start(0))` posterior | **SUPERVIVIENTE** | ninguno — 17 passed | QYR-0330 |

**El superviviente va en la tabla y no escondido.** Es un control redundante: la
propiedad «se lee desde cero» la sostiene el `seek(offset)` de
`FileSource::try_read`, no este rebobinado. La afirmación del commit `b4ebac7`
—«olvidarlo envía un archivo vacío con un digest correcto»— es **falsa** para
cualquier handle que admita búsqueda, y ADR-0034 exige `"rw"` precisamente para
que la admita.

### 10.2 — Barrido con `cargo-mutants` 27.1.0

**Alcance declarado.** No es un barrido de los once crates: es un barrido de los
**tres archivos que esta fase tocó en Rust**, en Windows 10, con `--timeout 90`
y `--timeout 120`. Lo que queda fuera: todo lo demás del workspace, y las tres
pruebas `cfg(unix)` de `qyro_ffi`, que en esta plataforma no se compilan y por
tanto no pueden matar a nadie.

```
cargo-mutants mutants --package qyro_fs      --file manifest_builder.rs --file io.rs --timeout 90
cargo-mutants mutants --package qyro_session --file session.rs                       --timeout 120
```

| Archivo | Mutantes | caught | missed | unviable | timeout |
|---|---|---|---|---|---|
| `qyro_fs/src/{io.rs, manifest_builder.rs}` | **60** | 39 | **12** | 9 | 0 |
| `qyro_session/src/session.rs` | **39** | 28 | **3** | 7 | 0 |

**Y un hallazgo sobre el propio barrido, que es lo más útil que produjo.**
`descriptors_by_item -> BTreeMap::new()` salió **missed** aquí y sin embargo mi
mutación manual del mismo cuerpo mató dos pruebas por nombre. La diferencia es el
alcance: `--package qyro_fs` corre **sólo las pruebas de `qyro_fs`**, y la
cobertura de esa función vive en `qyro_session/tests/session_behaviour.rs`. Con
la suite entera no hay ningún superviviente:

```
cargo-mutants mutants --package qyro_fs --file manifest_builder.rs --test-workspace true --timeout 180
# 7 mutants tested in 3m: 3 caught, 4 unviable   -- 0 missed
```

> **Un barrido por paquete subestima la cobertura de toda función cuyas pruebas
> viven aguas abajo.** Un `missed` así no es un hueco: es el barrido mirando por
> una ventana más estrecha que el código.

**Los quince `missed` restantes, clasificados por familia (`R3` §3):**

| Familia | Mutantes | Veredicto |
|---|---|---|
| Política de symlinks en `open_part` — `metadata_is_link_or_reparse_point`, `libc_o_nofollow`, los dos `match guard`, el `delete !` | **8** | **Fuera del alcance de esta fase.** Código preexistente de `qyro_fs`, con mitades `cfg(unix)` y `cfg(windows)`: en Windows los controles de Unix no se ejercen y viceversa. Ya está cubierto por QYR-0295 |
| `error.kind() == NotFound` en `committed_progress` y `part_for` | 2 | **Fuera del alcance.** Preexistente, ruta de reanudación |
| `replace < with <=` en `FileSource::try_read` | 1 | **Equivalente.** Con `<=`, al llegar a `filled == out.len()` la porción es vacía, `read` devuelve `Ok(0)` y el bucle rompe igual. Mismo comportamiento observable |
| `replace descriptors_by_item -> BTreeMap::new()` | 1 | **Falso positivo de alcance.** Caught con `--test-workspace`, arriba |
| `replace > with >=` en `Emitter::step_for` | 1 | **Frontera del presupuesto de emisiones (ADR-0033).** Preexistente de la fase 02, no de ésta. Registrado aquí, no arreglado |
| `RefusingSink::write_at` con `()` | 1 | **Ruido.** Es el sumidero que un emisor usa para no escribir nada; sustituir un cuerpo vacío por un cuerpo vacío |
| `impl Debug for Session` | 1 | **Ruido** por `R3` §3: nadie prueba el texto de un `Debug`, ni debe |

**Ninguno de los quince es un control de esta fase que sobreviva a su propio
borrado.** Los cuatro controles que esta fase introdujo están en §10.1, y tres de
los cuatro mueren con nombre.

---

## 11. Tests antes y después

| Suite | Antes (`9137220`) | Después (`2fd36bb`) | Comando |
|---|---|---|---|
| Rust, Windows 10 | 598 passed / 2 ignored | **603 passed / 2 ignored** | `cargo test --workspace` |
| Rust, Linux (CI) | 593 passed / 2 ignored | pendiente de la corrida — **+3 `cfg(unix)`** que aquí no compilan | job `rust` de `ci.yml` |
| Dart, Windows 10 | 63 passed / 1 skipped | **76 passed / 1 skipped** | `flutter test` con las dos variables |

**Una línea por test nuevo:**

*Rust, `qyro_session/tests/session_behaviour.rs` (portables, corren en los dos SO):*

1. `a_file_opened_by_descriptor_reads_identically_to_one_opened_by_path` — mueve
   el mismo archivo por ruta y por handle a dos destinos y compara los tres.
2. `a_transfer_driven_by_descriptor_arrives_byte_identical` — dos handles, dos
   nombres que sólo el selector conoce, y comprueba que **no** llega el nombre
   del archivo de origen.
3. `a_revoked_descriptor_mid_transfer_is_a_typed_error_not_a_hang` — trunca el
   archivo después de construir el manifiesto y exige que la sesión **termine**,
   con un `recv_timeout` de 60 s que convierte un cuelgue en un fallo con texto.

*Rust, `qyro_ffi/src/session_abi.rs` (`cfg(unix)`, no corren en Windows):*

4. `the_descriptor_is_closed_exactly_once` — entrega un fd por el camino que
   falla y comprueba que ya no nombra el archivo que se le dio.
5. `a_descriptor_that_was_not_handed_over_stays_visible_to_this_measurement` — la
   contra-prueba de la anterior.
6. `a_rejected_argument_still_closes_what_it_was_handed` — dos nombres y un
   descriptor: el rechazo llega **después** de tomar la propiedad, a propósito.

*Rust, `qyro_ffi/src/guards.rs` (corren en los dos SO):*

7. `no_rust_source_carries_a_raw_nul_byte` — recorre todo `rust/` menos `target`.
8. `the_crate_closes_no_descriptor_by_hand` — la mitad estructural de «una sola
   vez».

*Dart, `apps/qyro/test/ffi/qyro_file_picker_test.dart` (doce):* el enrutado por
plataforma, el rechazo por nombre de las cuatro no soportadas, la decodificación
del canal de Android —descriptores, nombres, tamaños, cancelación, nombre
ausente—, el mapeo de Windows —tamaño leído del disco, archivo ausente,
cancelación—, `leafName` con las dos separaciones y con entradas hostiles, y la
guarda de que nada del app importa el paraguas.

*Dart, `qyro_session_transfer_test.dart` (uno):*
`a_file_chosen_through_the_picker_transfers_and_verifies`.

---

## 12. Delta de dependencias

**Rust: sin cambios.**

```
grep -c '^\[\[package\]\]' Cargo.lock   # 64  antes y después
grep -c '^source = ' Cargo.lock         # 50  antes y después
git diff 546dbf6..HEAD -- Cargo.lock    # vacío
```

`Cargo.lock` no aparece en `git diff --name-only 546dbf6..HEAD` (§13), que es la
forma más corta de decir que el diff está vacío.

**Dart: +8 paquetes.**

| Paquete | Versión | Licencia | Por qué |
|---|---|---|---|
| `file_selector_windows` | 0.9.3+5 | BSD-3, publisher `flutter.dev` verificado | El diálogo de Windows |
| `file_selector_platform_interface` | 2.7.0 | BSD-3, `flutter.dev` | Dependencia directa del anterior |
| `cross_file` | 0.3.5+4 | BSD-3, `flutter.dev` | `XFile`, el tipo que devuelve `openFiles` |
| `plugin_platform_interface` | 2.1.8 | BSD-3, `flutter.dev` | Base del anterior |
| `http` | 1.6.0 | BSD-3, `dart.dev` | **Transitivo, nunca llamado.** QYR-0326 |
| `http_parser` | 4.1.2 | BSD-3, `dart.dev` | De `http` |
| `typed_data` | 1.4.0 | BSD-3, `dart.dev` | De `http_parser` |
| `web` | 1.1.1 | BSD-3, `dart.dev` | De `cross_file` |

Alternativa descartada y **medida**: `file_selector` 1.0.3, +15 paquetes, siete de
ellos evitables y uno de esos siete es la implementación que copia (§3).
Alternativa descartada sin medir: escribir `IFileOpenDialog` a mano — ADR-0034
§4.2, ~29 huecos de vtable cuyo orden Microsoft no publica en la web, y un hueco
desplazado es UB silencioso.

---

## 13. Archivos tocados

```
git diff --name-only 546dbf6..HEAD
```

```
.github/workflows/platform-builds.yml
BUGS_PENDING.md
apps/qyro/android/app/src/main/kotlin/com/owner/qyro/FilePickerChannel.kt
apps/qyro/android/app/src/main/kotlin/com/owner/qyro/MainActivity.kt
apps/qyro/lib/ffi/qyro_file_picker.dart
apps/qyro/lib/ffi/qyro_session_api.dart
apps/qyro/pubspec.lock
apps/qyro/pubspec.yaml
apps/qyro/test/android_manifest_test.dart
apps/qyro/test/ffi/qyro_file_picker_test.dart
apps/qyro/test/ffi/qyro_session_transfer_test.dart
docs/adr/ADR-0034-file-selection.md
docs/reports/ESTADO-ACTUAL.md
docs/reports/deuda-de-calidad.md
rust/crates/qyro_ffi/src/guards.rs
rust/crates/qyro_ffi/src/session_abi.rs
rust/crates/qyro_fs/src/io.rs
rust/crates/qyro_fs/src/lib.rs
rust/crates/qyro_fs/src/manifest_builder.rs
rust/crates/qyro_session/src/session.rs
rust/crates/qyro_session/tests/session_behaviour.rs
rust/guards/source_guard.rs
```

**Los ocho commits de la fase**, del más viejo al más nuevo:

```
269f0fa docs: freeze ADR-0034 and open the quality-debt lane
b4ebac7 feat(ffi): a descriptor can cross the boundary, because Android never hands out a path
9137220 feat(picker): the person picks the file, and Android never copies it
00aea33 docs: el archivo de estado, porque el contexto se acaba y el disco no
54c66e2 docs: la enmienda que decide el paquete de Windows antes de escribir el codigo
205204f docs: el segundo `pub get` seguido miente sobre el primero
867d3fa feat(picker): el dialogo de Windows conectado, y cuatro pruebas que faltaban
2fd36bb docs: el estado al cerrar el paso 4 de la fase 03
```

---

## 14. Runs de CI

**Todos los de la rama sobre commits de esta fase, sin filtrar.** Los fallos y las
cancelaciones también: una lista de la que se pueden caer los fallos no es
evidencia, es un resumen favorable.

| Commit | Workflow | Run | Conclusión |
|---|---|---|---|
| `269f0fa` | — | — | *(commit sólo documental; los filtros de rutas no dispararon nada, y `ci.yml` sí corrió sobre el siguiente)* |
| `b4ebac7` | CI | 31775740522 | **success** |
| `b4ebac7` | Platform builds | 31775740546 | **success** |
| `b4ebac7` | iOS runtime ABI | 31775740453 | **success** |
| `b4ebac7` | Android runtime ABI | 31775740520 | **cancelled** — el grupo de concurrencia lo cancela al llegar el push siguiente |
| `9137220` | CI | 31776013352 | **success** |
| `9137220` | Platform builds | 31776013296 | **success** |
| `9137220` | Android runtime ABI | 31776013392 | **success** |
| `867d3fa` | Crypto fuzz | 31843125990 | **success** |
| `867d3fa` | Android runtime ABI | 31843125971 | **success** |
| `867d3fa` | **CI** | 31843125905 | **FAILURE** — jobs `rust` y `documentation` |
| `867d3fa` | **Platform builds** | 31843125902 | **FAILURE** — job `android` |
| `867d3fa` | iOS runtime ABI | 31843125957 | **cancelled** |
| `b8dbca5` | iOS runtime ABI | 31843688151 | **success** |
| `b8dbca5` | **Platform builds** | 31843688177 | **FAILURE** — job `android`, el mismo defecto |
| `b8dbca5` | Android runtime ABI | 31843688357 | **cancelled** |
| `b8dbca5` | CI | 31843688168 | cancelled por el push siguiente |
| `274b504` | **Platform builds** | 31843897147 | **success** — *la primera vez que la aserción del manifiesto fusionado se ejecuta de verdad* |
| `274b504` | **CI** | 31843897139 | **FAILURE** — sólo `documentation`; `rust`, `flutter`, `scripts`, `rust workspace (windows)` y las tres guardas de `fs`: **success** |
| `274b504` | Android runtime ABI | 31843897198 | en curso al escribir esto |

### Qué falló y por qué, sin adornos

**1. `867d3fa` / CI / job `rust`: cinco errores de tipo.** Las tres pruebas
`cfg(unix)` del descriptor **no compilaban**. `file.metadata().map(...)` devuelve
un `Result`, no un `Option`, y yo las comparaba con `Option<(u64, u64)>`. En
Windows están detrás de `cfg(unix)`, así que `cargo test --workspace` en verde no
dijo absolutamente nada sobre ellas.

*Es la lección de este proyecto en su forma inversa: «compiló en Windows» no es
«compila».* Arreglado en `b8dbca5`, y la puerta de esta fase incorpora la
comprobación que lo habría evitado sin tener Linux:

```
cargo clippy -p qyro_ffi --all-targets --target aarch64-linux-android -- -D warnings   # exit 0
```

`check` no enlaza, así que no hace falta un enlazador de esa plataforma.

**2. `867d3fa` y `b8dbca5` / Platform builds / job `android`: la guarda nueva
falló en su primera corrida, y encontró lo que existía para encontrar.** El paso
que añadí para QYR-0329 —con `QYRO_REQUIRE_MERGED_MANIFEST=1`, donde un salto es
un fallo— dijo que no había ningún manifiesto fusionado bajo
`android/app/build/intermediates`. **Tenía razón: nunca lo hubo.** El plugin de
Gradle de Flutter mueve el directorio de build fuera del proyecto Android
(`rootProject.buildDir` pasa a `../build`), así que todo se escribe en
`build/app/intermediates`. La prueba llevaba desde el paso 3 mirando una ruta que
no existe, que es exactamente por qué se saltaba. Arreglado en `274b504`, y
`Platform builds` pasó a **success**.

*Una guarda que falla la primera vez que corre y destapa que la aserción que
protegía nunca había leído un archivo es la guarda funcionando, no rompiéndose.*

**3. `867d3fa`, `b8dbca5` y `274b504` / CI / job `documentation`.** Dos causas,
las dos de este mismo informe:
`Stale verified commit` —`STATUS.md` apuntaba a un commit a más de diez de HEAD—
y «*is cited but has no entry*», por escribir en `ESTADO-ACTUAL.md` el siguiente
identificador libre, `QYR-0331+`, sin el sufijo que el checker exime. Las dos se
corrigen en el commit de cierre de esta fase.

*(Y la regla mordió una tercera vez mientras escribía este párrafo: citar el
identificador sin el `+` **dentro del propio informe** vuelve a dispararla. La
guarda no distingue una cita de una mención, y eso es correcto — distinguirlas
sería adivinar.)*

**Y un error de proceso mío, que es el que las dejó llegar a CI:** corrí la
comprobación 11 de la puerta cuando escribí las fichas y **no volví a correrla
sobre el commit final**. `R2` §1.9 pide exactamente eso —releer lo que la fase
pudiera haber invalidado— y la regla de frescura de `STATUS.md` se invalida sola
con cada commit. Está escrito aquí porque omitirlo daría una impresión más limpia
que la real.

**Las cuatro cancelaciones** son el grupo de concurrencia de
`android-runtime.yml` e `ios-runtime.yml` cancelando el run anterior sobre la
misma ref cuando llega un push nuevo. No son fallos y no se usan como evidencia.

---

## 15. Qué NO debe leerse como progreso

**Esta es la sección importante.**

- **Nadie ha visto el selector.** Ni el de Android ni el de Windows. No hay
  emulador en esta máquina, no hay teléfono conectado, y el Modo Desarrollador
  está apagado, así que `flutter build windows` y `flutter run` no corren
  (QYR-0324). Todo lo que esta fase prueba está **aguas abajo** del diálogo.
- **El criterio 7 de la fase no está cumplido**, y por eso la fase se cierra como
  PARCIAL en vez de como cumplida.
- **`the_descriptor_is_closed_exactly_once` no corre en esta máquina.** Es
  `cfg(unix)`. Su clase de evidencia aquí es «compilado», y sólo pasa a «probado
  en unidad» cuando la corrida de Linux lo confirme.
- **La prueba del manifiesto fusionado tampoco ha corrido todavía.** El paso de
  CI existe desde hoy; hasta que una corrida lo ejecute, el criterio 6 está
  verificado sólo sobre el manifiesto que escribimos nosotros.
- **Sigue sin haber descubrimiento.** La IP y el puerto van a mano. Esta fase no
  toca eso; es la fase 04.
- **Sigue sin haber ninguna pantalla** de envío, recepción, progreso o peers.
- **Los botones Enviar y Recibir siguen `onPressed: null`**, y así se quedan
  hasta la fase 05.
- **La confianza sigue sin llamarse desde ninguna parte.** `decide_trust` existe
  y está probada, y nada la invoca.
- **Nada se ha probado en hardware físico**, y **dos procesos en `127.0.0.1` no
  son dos aparatos en una Wi-Fi.**
- **Que las guardas se hayan arreglado no significa que el código que cubrían
  estuviera mal.** QYR-0328 dice que el análisis leía menos de lo que decía; al
  arreglarlo, `cargo test --workspace` siguió en verde. Lo que se recuperó es la
  **garantía**, no un defecto oculto.

---

## 16. Ledger y handoff

### Fichas

| ID | Sev | Título | Al empezar | Al cerrar |
|---|---|---|---|---|
| QYR-0323 | P1 | `file_selector_android` copia el archivo a la caché | abierto | **cerrado** (en `9137220`) |
| QYR-0326 | P3 | Un cliente HTTP viaja en el árbol de dependencias de Dart | — | **abierto** (nueva) |
| QYR-0327 | P2 | Un byte NUL crudo hacía que grep saltara un archivo entero | — | **cerrado** (nueva) |
| QYR-0328 | P1 | Un `}` escrito como carácter truncaba el análisis de guardas | — | **cerrado** (nueva) |
| QYR-0329 | P1 | La prueba del manifiesto fusionado no se ejecutó nunca | — | **cerrado** (nueva) |
| QYR-0330 | P3 | El rebobinado tras el digest es código muerto | — | **abierto** (nueva) |

**Balance: 142 fichas y 37 abiertas al empezar la sesión; 147 y 39 al cerrarla.**
Sube 5 en total y 2 en abiertas: cinco fichas nuevas, de las cuales tres se
cierran en el mismo tramo con su mutación escrita. Bajo el techo de diez de
`R2` §1.10.

*(El 37 es el número que devuelve el script canónico de `R2` §1.10 sobre
`9137220`. El prompt de sesión decía 38.)*

### Qué documentación quedó desfasada

- **`ADR-0034 §1`** decía `file_selector`. Corregido por la enmienda 1 en el
  mismo archivo, no en otro sitio.
- **El mensaje del commit `b4ebac7`** afirma que olvidar el rebobinado envía un
  archivo vacío. Es falso; QYR-0330 lo recoge. Un mensaje de commit no se
  reescribe.
- **El comentario de `item_end`** decía «estos crates no tienen llaves en
  literales de carácter». Reescrito, y la premisa ya no es premisa.
- **`R3` §2** dice que «hay un job de CI que ya exige evidencia para `qyro_fs`»
  con `cargo-mutants`. **No lo hay:** `grep -i mutant .github/workflows/*.yml`
  no devuelve nada. Queda registrado aquí; no se arregla en esta fase.

### Qué necesita saber la fase 04

1. **`pickerForPlatform({String? operatingSystem})`** acepta la plataforma por
   parámetro para poder probarse desde cualquier máquina. La UI de la 05 debe
   llamarla sin argumento.
2. **`QyroSession.sendDescriptors`** ya existe en Dart y **lanza
   `UnsupportedError` en Windows por diseño**: el símbolo no está en la DLL. La
   UI tiene que ramificar por el tipo de `QyroPicked`, no por la plataforma.
3. **El nombre que ve el receptor sale de `QyroPicked.name`**, no del archivo. En
   Android lo elige el proveedor de contenido y `FilePickerChannel.sanitise` lo
   recorta; en Windows sale de `leafName`. **La fase 05 tiene que asumir que ese
   nombre es hostil** al pintarlo.
4. **El entorno para correr las pruebas FFI de Dart** está en
   `docs/reports/ESTADO-ACTUAL.md` §5. Sin esas dos variables, seis pruebas se
   saltan y el salto no es un fallo.
5. **`mdns-sd` 0.20.3 bajo `cfg(windows)` está pre-autorizada** para la 04. Será
   la primera dependencia externa de Rust desde el sprint 4A, así que el delta de
   `Cargo.lock` y `cargo audit` van en el informe de esa fase, medidos con
   `cargo tree`.
6. **La trampa que no da error:** cualquier cosa que no sea `NsdManager` necesita
   `WifiManager.MulticastLock` en Android. `join_multicast_v4` tiene éxito y no
   recibe nada, sin error.
