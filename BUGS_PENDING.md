# Bugs y pendientes verificados

## QYR-0001 — Falta referencia visual de scramble

- Plataforma: todas
- Severidad: P2
- Esperado: design/reference/scramble-decode-reference.jpg
- Actual: activo no suministrado
- Workaround: tests deterministas sin golden visual
- Estado: abierto
- Dueño: propietario
- Fecha: 2026-08-04

## QYR-0002 — Runners Flutter no generados

- Plataforma: Android, iOS, Windows
- Severidad: P0
- Estado: resuelto
- Evidencia: commit 286d3d4 y builds run 30938946789
- Resolución: runners oficiales generados por Flutter 3.44.8
- Fecha: 2026-08-04

## QYR-0003 — Aviso de actions/checkout v4

- Plataforma: CI
- Severidad: P3
- Reproducción: ejecutar CI
- Esperado: cero avisos
- Actual: GitHub fuerza Node 24 porque la action declara Node 20
- Workaround: ninguno necesario; jobs pasan
- Estado: abierto; evaluar checkout v5 tras auditoría
- Dueño: release
- Fecha: 2026-08-04

## QYR-0004 — Builds no retenidos

- Plataforma: release
- Severidad: P1
- Esperado: artefactos debug descargables con checksums
- Actual: **parcialmente incorrecto tal y como estaba redactado.** El ZIP
  portable de Windows sí se retiene desde que existe el job `windows`
  (`qyro-windows-x64-portable-debug`, 14 días). El APK de Android y el
  `Runner.app` de iOS no: `platform-builds.yml` tiene un único paso de
  `upload-artifact` y es el de Windows
- Lo que sigue faltando: checksum SHA-256 distribuido **dentro** del paquete y
  la etiqueta DEVELOPMENT / NOT FOR PUBLIC RELEASE, en los tres. El digest que
  GitHub imprime al subir un artefacto identifica el ZIP de ese run, no el
  contenido que alguien descarga; usarlo como sustituto cambiaría en silencio lo
  que se afirma
- Evidencia: run 30938946789; `.github/workflows/platform-builds.yml:223`
- Workaround: volver a ejecutar builds para Android e iOS
- Estado: abierto, con el alcance corregido en el sprint 4C.1
- Dueño: release
- Fecha: 2026-08-04, corregido 2026-08-05

## QYR-0005 — Auditorías y suites avanzadas no disponibles

- Plataforma: CI
- Severidad: P1
- Esperado: cargo-audit, tests nativos y vectores de protocolo ejecutables
- Actual: test_all informa WARNING para cargo-audit y N/A para suites/corpus ausentes
- Workaround: las suites Rust/Flutter y el ledger de licencias sí se validan
- Estado: abierto
- Dueño: seguridad/protocolo
- Fecha: 2026-08-04

## QYR-0006 — iOS no compilaba por un storyboard ilegible

- Plataforma: iOS
- Severidad: P0
- Esperado: `flutter build ios --debug --no-codesign` produce Runner.app
- Actual: `Error (Xcode): The document "LaunchScreen.storyboard" could not be
  opened. (com.apple.InterfaceBuilder error -1.)`
- Causa: 67fa795 eliminó `toolsVersion`/`systemVersion` del elemento `<document>`
  al oscurecer la launch surface, dejando una `capability` con `minToolsVersion`
  sin versión de herramientas contra la que compararse
- Evidencia: runs 30960631901 (67fa795), 30961031089 (9bfb1cc) y 30961153321
  (e9ed7f3) fallan; 30953803079 (9104421) y 30956527561 (4f7ed01) pasaban
- Estado: resuelto
- Resolución: commit 565a78d restaura la estructura del documento que ya
  compilaba y añade validación estructural al contrato de launch surfaces
- Confirmación: run 30963011815 sobre ff933d9, los diez pasos en success,
  incluidos la verificación de símbolos con `nm -gU` y el XCTest en simulador
- Dueño: iOS
- Fecha: 2026-08-05

## QYR-0007 — STATUS.md pudo derivar 58 commits sin detección

- Plataforma: CI/documentación
- Severidad: P1
- Esperado: el job documental detecta que la fuente canónica quedó obsoleta
- Actual: `check_docs_consistency` validaba solo la estructura de STATUS.md, así
  que `Verified commit: 7ca3973` sobrevivió 58 commits declarando funciones ya
  implementadas como NOT_IMPLEMENTED y 9 tests cuando la suite ejecuta 51
- Estado: resuelto
- Resolución: commit 5825b50 añade la regla de frescura (SHA mal formado,
  inalcanzable o con más de `QYRO_MAX_STATUS_COMMIT_LAG` commits de retraso) en
  Bash y PowerShell, y el job documental usa `fetch-depth: 0`
- Dueño: documentación
- Fecha: 2026-08-05

## QYR-0008 — Run de Android runtime atascado sin runner

- Plataforma: CI/Android
- Severidad: P2
- Esperado: el run concluye o falla
- Actual: run 30961153377 (e9ed7f3) sigue `in_progress` desde 2026-08-04T23:47Z
  con `total_ms: 0`; nunca obtuvo runner
- Workaround: no se canceló porque `concurrency: android-runtime-${{ github.ref }}`
  con `cancel-in-progress: true` lo desplaza en el próximo push a esa ref
- Impacto: ninguno ya sobre el estado actual. El runtime ABI de Android quedó
  confirmado en esta rama por el run 30963016390 sobre ff933d9
- Estado: cerrado por obsolescencia; el run atascado sigue en la otra rama
- Dueño: CI/Android
- Fecha: 2026-08-05

## QYR-0009 — ADR-0016 prometía compatibilidad que el código no tenía

- Plataforma: protocolo
- Severidad: P0
- Esperado: un tipo de mensaje desconocido es recuperable
- Actual: `FrameDecoder` envenenaba el stream ante cualquier error de cabecera,
  así que un peer con una versión menor más nueva mataba la conexión
- Impacto adicional: `header_len > 48` se aceptaba y los bytes de extensión se
  descartaban, rompiendo la reserialización byte-exacta; `ENCRYPTED` y
  `COMPRESSED` eran ajustables públicamente
- Estado: resuelto
- Resolución: ADR-0018 y commits 30fe57e (contratos) y cc38554 (implementación)
- Fecha: 2026-08-05

## QYR-0010 — El manifest permitía un nombre visible engañoso

- Plataforma: manifest
- Severidad: P0
- Esperado: el nombre mostrado corresponde al archivo que se escribirá
- Actual: `display_name` viajaba aparte de la ruta, así que `factura.pdf.exe`
  podía presentarse como `factura.pdf` con un manifest técnicamente válido
- Estado: resuelto
- Resolución: ADR-0019, campo eliminado del wire, `MANIFEST_VERSION` a 2
- Fecha: 2026-08-05

## QYR-0011 — Archivos sin digest y colisiones portables aceptadas

- Plataforma: manifest
- Severidad: P0
- Esperado: todo archivo tiene digest final; dos items no pueden ser el mismo
  archivo en el receptor
- Actual: `HashMetadata::none()` era válido para archivos, y `Foto.jpg` junto a
  `foto.jpg` se aceptaban, sobrescribiéndose en Windows o macOS
- Estado: resuelto
- Resolución: digest obligatorio en el constructor y `PortableCollisionKey`
- Fecha: 2026-08-05

## QYR-0012 — Aserción de travesía incorrecta desde el sprint 2

- Plataforma: pruebas
- Severidad: P2
- Esperado: la travesía se comprueba por segmento
- Actual: property tests y targets de fuzzing comprobaban `".."` como subcadena,
  lo que rechaza el nombre legítimo `notes..txt` y no dice nada útil sobre
  travesía real
- Estado: resuelto
- Resolución: aserciones por segmento en property tests y targets
- Fecha: 2026-08-05

## QYR-0013 — El repositorio no podía clonarse en Windows

- Plataforma: Windows
- Severidad: P0
- Esperado: `actions/checkout` obtiene el árbol en el runner de Windows
- Actual: `error: invalid path 'rust/fuzz/corpus/relative_path/nul.txt'`,
  `git.exe` salía con 128 y el job moría en el paso 2, antes de compilar nada
- Causa: el caso de corpus del **byte** NUL se nombró por su contenido, y `NUL`
  es un nombre de dispositivo reservado en Windows. Sus hermanos sí llevaban
  prefijo (`reserved_con.txt`, `reserved_com1_ext.txt`), así que el riesgo se
  conocía para CON y COM1 y se pasó por alto para NUL
- Alcance: desde que se añadió el corpus en el sprint 2. La última evidencia de
  Windows en STATUS era de `e9ed7f3`, anterior al corpus, así que el fallo
  quedó fuera de vista durante tres sprints
- Resolución: renombrado a `nul_byte.txt`; el contenido (`a\0b`) no cambia,
  porque lo que un corpus de fuzzing aporta son bytes, no nombres
- Prevención: `scripts/check_repo_portability.{sh,ps1}` rechaza cualquier ruta
  rastreada que Windows no pueda extraer, con contratos en ambos shells y en
  CI. Es la misma regla que `qyro_manifest` aplica a una transferencia: un
  proyecto que rechaza el nombre no portable de un peer y comete uno propio no
  está aplicando su propio estándar
- Estado: resuelto
- Evidencia: run 30976026135 (fallo, job `windows`), contrato en rojo y verde
- Fecha: 2026-08-05

## QYR-0014 — Cuatro afirmaciones de ADR-0022 no las cubría ninguna prueba

- Plataforma: pruebas, criptografía
- Severidad: P2
- Esperado: la derivación del AEAD liga la clave a la dirección, al
  `auth_transcript` y al `SessionId`, y el prefijo de nonce se expande aparte
- Actual: borrar `direction.label()` de `info_for` —de modo que las dos
  direcciones derivaran bajo la misma etiqueta— dejaba pasar las treinta y tres
  pruebas. Quitar el transcript o el `SessionId` de cada `info`, también
- Causa: las pruebas extremo a extremo no pueden ejercitar el caso que la
  afirmación cubre. Los dos secretos de tráfico ya difieren, porque el schedule
  del handshake los deriva bajo etiquetas propias, y dos sesiones de prueba
  difieren en todo a la vez. La propiedad estaba sostenida una capa más arriba
- Resolución: cuatro pruebas unitarias sobre la derivación misma, más las
  etiquetas fijadas contra ADR-0022 y no contra la función que las produce
- Prevención: cada mutación se volvió a aplicar después; las cuatro fallan
- Estado: resuelto
- Evidencia: `docs/audits/SPRINT4C_AEAD_AUDIT.md`, hallazgo H-1
- Fecha: 2026-08-05

## QYR-0015 — Tres variantes de error inalcanzables en ADR-0022

- Plataforma: criptografía
- Severidad: P3
- Esperado: cada variante de `AeadError` la produce alguna ruta
- Actual: `NotEncrypted`, `PayloadTooLarge` e `InvalidNonceState` no las puede
  provocar nada. Un `EncryptedEnvelope` no existe sin el flag `ENCRYPTED`, un
  `Frame` no excede `MAX_PAYLOAD_LEN`, e «estado de nonce inválido» era
  `SequenceExhausted` con otro nombre
- Causa: la lista se congeló en la ADR antes de que existiera el código, que es
  deliberado y correcto; lo que faltaba era revisarla al implementarla
- Resolución: eliminadas del enum, con enmienda registrada en ADR-0022
- Estado: resuelto
- Evidencia: `docs/audits/SPRINT4C_AEAD_AUDIT.md`, hallazgo H-2
- Fecha: 2026-08-05

## QYR-0016 — Cuatro workflows en verde no probaban `qyro_crypto` en ninguna plataforma

- Plataforma: Android, iOS, Windows, CI
- Severidad: P1
- Esperado: evidencia de que `qyro_crypto` compila y corre en las tres
  plataformas del producto
- Actual: `platform-builds.yml`, `android-runtime.yml` e `ios-runtime.yml`
  construían y ejecutaban `qyro_ffi`. `qyro_ffi` depende de `qyro_core` y de
  nada más —hay una prueba que falla si alguien añade `qyro_crypto`—, así que
  ninguno de esos runs tocaba una línea de criptografía fuera de x86_64 Linux
- Causa: la evidencia se leía por plataforma y no por paquete. Un job llamado
  `android` en verde parecía cubrir «Android», y cubría un crate concreto
- Resolución: `crypto-platform.yml` con cuatro jobs que compilan `qyro_crypto`
  por target explícito y ejecutan un harness aislado en Linux, Windows, emulador
  Android y simulador iOS
- Prevención: `scripts/check_crypto_platform_evidence.{sh,ps1}` exige que el
  nombre del paquete y el `--target` aparezcan **juntos**, y su contrato incluye
  a propósito una sustitución `qyro_ffi`/`qyro_crypto` para comprobar que el
  checker la detecta
- Estado: resuelto
- Evidencia: `docs/audits/SPRINT4C1_CRYPTO_PLATFORM_AUDIT.md`
- Fecha: 2026-08-05

## QYR-0017 — La ruta AEAD podía abortar el proceso con datos de un peer

- Plataforma: criptografía
- Severidad: P1
- Esperado: ningún input de peer provoca un pánico
- Actual: `unreachable!`, `assert!` e indexado sin comprobar en el camino de
  `seal` y `open`
- Causa adicional: un `assert!` no era un control. `debug_assertions` está
  apagado en release, así que la comprobación desaparecía justo en la
  compilación que se distribuye
- Resolución: cada estado pasa a ser una variante de `AeadError`
  (`FrameTemplateRejected`, `EnvelopeConstructionFailed`,
  `AssociatedDataMismatch`, `ReplayStateCorrupt`, `SealerPoisoned`), bajo `deny`
  de `clippy::panic`, `unwrap_used`, `expect_used`, `unreachable`, `todo`,
  `unimplemented` e `indexing_slicing`
- Segundo defecto que destapó: devolver `Err` sin más deja un sealer que puede
  haber consumido ya su secuencia. Un reintento reutilizaría el nonce. Ahora
  cualquier error lo envenena de forma permanente
- Estado: resuelto
- Evidencia: `docs/audits/SPRINT4C1_CRYPTO_PLATFORM_AUDIT.md`
- Fecha: 2026-08-05

## QYR-0018 — El texto claro descifrado quedaba en memoria sin borrar

- Plataforma: criptografía
- Severidad: P1
- Esperado: el texto claro autenticado se borra al soltarlo
- Actual: `AuthenticatedFrame::payload` era un `Vec<u8>` e `into_payload` lo
  entregaba desnudo. Los búferes temporales de `seal` y `open` tampoco se
  borraban
- Actual, segunda parte: las features `zeroize` de `sha2` y `hmac` estaban
  apagadas, así que el estado de compresión de cada transcript y el estado con
  clave de cada MAC de confirmación y de cada expansión HKDF quedaban en memoria
  liberada
- Causa: nadie había recorrido los secretos uno por uno. La feature se supuso
  activa por el nombre en lugar de comprobarse en `Cargo.lock`
- Resolución: `Zeroizing<Vec<u8>>` en los tres sitios, `into_zeroizing_payload`
  en lugar de `into_payload`, y ambas features activadas
- Límite registrado, no cerrado: swap, hibernación, core dumps y registros
  quedan fuera del alcance de cualquier `Drop`, y nada de esto se ha observado
  ocurriendo. Ver `docs/security/secret-lifecycle-audit.md`
- Estado: resuelto
- Evidencia: `docs/audits/SPRINT4C1_CRYPTO_PLATFORM_AUDIT.md`
- Fecha: 2026-08-05

## QYR-0019 — Ningún target de `cargo-fuzz` podía construirse

- Plataforma: pruebas
- Severidad: P2
- Esperado: los targets documentados en `parser-threats.md` se ejecutan con el
  recetario que ese archivo publica
- Actual: dos fallos encadenados. `rust/fuzz/Cargo.toml` decía «excluded from
  the main workspace» y nada lo excluía —el manifest raíz ni lo listaba ni lo
  excluía, y el paquete no declaraba `[workspace]` propio—, así que Cargo
  respondía «current package believes it's in a workspace when it's not». Detrás
  de eso, `frame_decoder` seguía usando una API que cambió en el sprint 2
- Tercer fallo, en la documentación: el recetario omitía `--fuzz-dir rust/fuzz`,
  sin el cual cargo-fuzz busca `<raíz>/fuzz` y falla con un mensaje que no dice
  cuál es el problema
- Causa: lo único que CI ejecutaba sobre estos archivos era `rustfmt --check`,
  que no necesita tipos para pasar
- Resolución: `[workspace]` propio, target reparado, tres targets nuevos y
  `crypto-fuzz.yml`
- Estado: resuelto
- Evidencia: `docs/audits/SPRINT4C1_CRYPTO_PLATFORM_AUDIT.md`
- Fecha: 2026-08-05

## QYR-0020 — El repositorio se extraía con CRLF en Windows

- Plataforma: Windows, CI
- Severidad: P2
- Esperado: los archivos rastreados tienen los mismos bytes en las tres
  plataformas
- Actual: sin `.gitattributes`, Git aplicaba su conversión por defecto en
  Windows y tres pruebas fallaban allí y solo allí: las dos que regeneran
  vectores byte a byte y la que recorre el fuente buscando constructores
  deterministas
- Causa: las pruebas comparan bytes, y el checkout los cambiaba. El fallo no
  tenía nada que ver con el código que señalaba
- Resolución: `.gitattributes` con `* text=auto eol=lf` y una lista explícita de
  extensiones binarias, más una prueba nombrada que rechaza un `\r` en los
  vectores comprometidos
- Estado: resuelto
- Evidencia: run 31049534832 (fallo, job `windows-crypto`)
- Fecha: 2026-08-05

## QYR-0021 — Categoría Unicode `Cf` aceptada en rutas de manifest

- Plataforma: todas
- Severidad: P0
- Esperado: un `.exe` no puede presentarse como `.pdf` (ADR-0019, `lib.rs:20`,
  THREAT_MODEL.md)
- Actual: `RelativePath::parse` filtraba con `char::is_control()`, que es la
  categoría general Unicode `Cc` y nada más. La categoría `Cf` pasaba entera.
  `RelativePath::parse("invoice\u{202E}fdp.exe")` devolvía `Ok`, `as_str()` lo
  entregaba tal cual y sobrevivía a `codec::encode`/`codec::decode` byte a byte.
  Todo renderizador consciente de bidi muestra ese nombre como `invoiceexe.pdf`,
  incluidos los selectores de archivo donde un receptor confirma la
  transferencia. Aceptados también `U+202D`, `U+2066`, `U+200B`, `U+200D`,
  `U+FEFF`, `U+00AD`
- Resolución: tabla de veintiún rangos transcrita de
  `extracted/DerivedGeneralCategory.txt` de Unicode 16.0.0, 170 puntos de código,
  citada en el fuente y comprobada contra el archivo. Variante propia
  `PathError::FormatCharacter`, que imprime `U+202E` y nunca el carácter. Sin
  dependencias nuevas. `U+200C`/`U+200D` rechazados como decisión explícita
- Estado: resuelto
- Evidencia: rojo en `cff6a1a` (5 pruebas); enmienda a ADR-0019;
  `tests/unicode_path_contract.rs`
- Fecha: 2026-08-07

## QYR-0022 — El iniciador no estaba autenticado por ninguna prueba

- Plataforma: todas
- Severidad: P1
- Esperado: un peer que presenta la `PublicIdentity` de un tercero y no puede
  firmar el transcript se rechaza con `SignatureVerificationFailed`
- Actual: el control existía y ninguna prueba lo cubría. Borrar la llamada a
  `verify_transcript` en `handshake/mod.rs::receive_initiator_finish` dejaba
  `cargo test --package qyro_crypto` en 124 passed, 0 failed. Es el único
  control que autentica al iniciador; el espejo del lado iniciador sí estaba
  cubierto por tres pruebas
- Resolución: `an_unsigned_peer_cannot_present_another_identity`, que construye
  el hello con la identidad de un tercero y también reintenta con una firma
  real hecha con la clave del atacante
- Estado: resuelto
- Evidencia: con el control borrado la prueba falla con
  `Some(FinishedVerificationFailed)` en vez de
  `Some(SignatureVerificationFailed)`
- Fecha: 2026-08-07

## QYR-0023 — `verify_strict` sin prueba que lo distinguiera de `verify`

- Plataforma: todas
- Severidad: P1
- Esperado: una firma que `verify` aceptaría y `verify_strict` rechaza no
  verifica
- Actual: sustituir `verify_strict` por `verify` en `identity.rs` dejaba la
  suite en 124 passed, 0 failed. Todas las demás pruebas de firma usan firmas
  que produjo este crate, y ambos verificadores las aceptan
- Resolución: `a_non_strict_signature_is_refused`, con una firma de `R` de orden
  pequeño sobre la clave de RFC 8032 §7.1 TEST 1. Los bytes se derivan en vez de
  citarse, y el fuente explica por qué: `verify` de este crate firma sobre su
  propia entrada con separación de dominio, así que ninguna terna publicada
  puede presentársele
- Estado: resuelto
- Evidencia: con `verify` la aserción pasa de `Err(SignatureVerificationFailed)`
  a `Ok(())`
- Fecha: 2026-08-07

## QYR-0025 — El transcript se verificaba llamándose a sí mismo

- Plataforma: todas
- Severidad: P1
- Esperado: los valores registrados se verifican contra las primitivas, como
  afirma STATUS.md
- Actual: `handshake/vectors.rs:498,525` llamaban a `base_transcript` y
  `auth_transcript`, y `:575` a `hmac_sha256`. Eso demuestra que el código
  coincide consigo mismo. `every_recorded_value_verifies_against_the_primitives`
  pasaba además tras reencaminar el `info` de HKDF al salt en `schedule.rs`
- Resolución: ambos transcripts recalculados con SHA-256 sobre concatenación
  literal, HMAC escrito desde RFC 2104, las dos entradas de firma registradas
  comprobadas contra el ADR, y `Schedule::derive` fijado contra los valores ya
  verificados. Prueba nueva
  `the_transcript_is_what_the_specification_says_it_is`, que no toca
  `transcript.rs` para construir lo esperado
- Estado: resuelto
- Evidencia: borrar los prefijos `u32` BE hace fallar la prueba nueva;
  reencaminar el `info` al salt hace fallar la del archivo de vectores con «the
  key schedule this crate runs disagrees with the primitives»
- Fecha: 2026-08-07

## QYR-0026 — Ningún workflow se disparaba en la rama de trabajo

- Plataforma: CI
- Severidad: P1
- Esperado: los seis workflows corren solos sobre la rama que lleva el trabajo
- Actual: `ci.yml` y `platform-builds.yml` solo con `main`;
  `android-runtime.yml` e `ios-runtime.yml` con `audit/baseline-hardening`, que
  dejó de recibir commits cuatro sprints antes. «CI está en verde» significaba
  «alguien se acordó de lanzarlo a mano»
- Resolución: la rama añadida al `push: branches:` de los seis, y las dos
  referencias muertas corregidas. Se eligió `push` y no una pull request: un run
  de `pull_request` se ejecuta sobre un commit de fusión que solo existe dentro
  del run, así que su ID no puede citarse como evidencia de un commit de la rama
- Estado: resuelto
- Evidencia: los seis runs finales de STATUS.md son de evento `push`
- Fecha: 2026-08-07

## QYR-0028 — Un archivo podía ser también un directorio

- Plataforma: todas
- Severidad: P2
- Esperado: `TransferManifest::new` rechaza un par ancestro/descendiente
- Actual: aceptaba `file("a")` + `file("a/b")` y `file("a")` + `file("A/b")`.
  Las claves de colisión se comparaban por igualdad, y `"a"` y `"a\0b"` son dos
  cadenas distintas. Un receptor tendría que crear `a` como archivo y `a` como
  directorio, y lo segundo pierde lo primero después de haber aceptado
- Resolución: regla de prefijo en frontera NUL sobre las claves ordenadas,
  aplicada solo cuando el ancestro es `File`. Formulación exacta en la enmienda
  a ADR-0017
- Estado: resuelto
- Evidencia: rojo en `52391a5` (3 pruebas);
  `tests/ancestor_collision_contract.rs`
- Fecha: 2026-08-07

## QYR-0029 — Nombres de dispositivo de Windows no cubiertos

- Plataforma: Windows
- Severidad: P2
- Esperado: un manifest no puede nombrar un dispositivo reservado
- Actual: `RelativePath::parse` aceptaba `COM¹`, `LPT²`, `COM0`, `LPT0`,
  `CONIN$`, `CONOUT$`, `CLOCK$` y `com0.txt`
- Resolución **parcial**: añadidos `COM¹`, `COM²`, `COM³`, `LPT¹`, `LPT²` y
  `LPT³`, con la página de Microsoft Learn citada en el fuente, incluida la nota
  de que Windows lee los superíndices ISO/IEC 8859-1 como dígitos dentro de un
  nombre de dispositivo
- **Sigue abierto**: `COM0`, `LPT0`, `CONIN$`, `CONOUT$` y `CLOCK$`. Esa página
  no los lista y no se comprobó ninguna otra fuente. Añadir una regla sin
  evidencia rechaza nombres legítimos, que es el mismo error en la otra
  dirección. Una prueba fija que hoy se aceptan, para que la respuesta no cambie
  por accidente
- Estado: **abierto** (parcialmente resuelto)
- Dueño: quien tenga acceso a un Windows real para medirlo
- Evidencia: rojo en `02e1e44`; `windows_superscript_device_names_are_rejected`
  y `names_that_merely_resemble_a_device_are_still_accepted`
- Fecha: 2026-08-07

## QYR-0030 — La frontera FFI se comprobaba partiendo texto

- Plataforma: todas
- Severidad: P2
- Esperado: `qyro_ffi` no puede alcanzar `qyro_crypto`
- Actual: `c_abi_contract.rs` partía el manifest por la cadena
  `"[dependencies]"` y buscaba en la sección siguiente. Una tabla
  `[target.'cfg(target_os = "android")'.dependencies]` con `qyro_crypto` es otra
  sección y no se miraba nunca, mientras la prueba se anunciaba como estructural
- Resolución: el cierre transitivo se pide a `cargo metadata`, sin
  `--filter-platform`, excluyendo dev-dependencies e incluyendo
  build-dependencies. Dos aserciones: una lista nombrada de crates de cripto, y
  igualdad exacta con `{qyro_ffi, qyro_core}`. Los dos scripts de guarda en
  shell descartan además las líneas de comentario antes de buscar
- Estado: resuelto
- Evidencia: con `qyro_crypto` bajo una tabla `cfg(target_os)` la prueba falla
  nombrando el crate; la ventana que leía la prueba anterior contiene con esa
  misma edición solo `qyro_core = { path = "../qyro_core" }`
- Fecha: 2026-08-07

## QYR-0031 — Seis sitios de documentación contradecían al código

- Plataforma: todas
- Severidad: P2
- Actual: rutas descritas como «normalized» con un campo llamado igual, cuando
  se guardan verbatim; bytes de cabecera desconocidos descritos como «se
  saltan», cuando se rechazan (ADR-0018); trailer descrito como cero para
  QYRO/1.0, cuando un frame sellado exige `1..=64`; `cfg(test)` donde el
  atributo es `cfg(any(test, fuzzing))`, en tres sitios; y tres filas de
  THREAT_MODEL.md
- Resolución: los seis corregidos y marcados como corregidos, no reescritos en
  silencio. El campo `normalized` pasa a llamarse `verbatim`. `fuzzing.rs` dice
  ahora que `--cfg fuzzing` es activable por `RUSTFLAGS` en todo el workspace
- Estado: resuelto
- Evidencia: `docs/audits/SPRINT4C2_AUDIT_CLOSURE.md`, tabla de QYR-0031
- Fecha: 2026-08-07

## QYR-0032 — Cuatro controles de la ruta de decode sin prueba

- Plataforma: todas
- Severidad: P2
- Actual: los cuatro podían borrarse con `cargo test --workspace` en verde: el
  total declarado contra la suma (`model.rs`), el orden canónico (`model.rs`),
  la cota del prefijo de longitud (`codec.rs`) y la suma comprobada
  (`model.rs`). La prueba de desbordamiento existente usaba un solo `u64::MAX`,
  y el límite total salta antes de alcanzar el desbordamiento
- Resolución: `tests/decode_guard_contract.rs`, cuatro pruebas construidas byte
  a byte porque la API de construcción rechaza el manifest antes de codificarlo.
  La de desbordamiento usa 1 y `u64::MAX`, que suman exactamente `u64::MAX + 1`,
  con total declarado cero
- Estado: resuelto
- Evidencia: cada control borrado por turno hace fallar su propia prueba
- Fecha: 2026-08-07

## QYR-0033 — La guarda anti-pánico solo leía `src/aead/`

- Plataforma: todas
- Severidad: P2
- Esperado: ninguna ruta de producción puede terminar el proceso
- Actual: `aead/guards.rs` recorría `["mod.rs","error.rs","replay.rs"]` bajo
  `src/aead/`. Fuera de ahí, `handshake/transcript.rs:88` tenía un `expect(...)`
  y `handshake/schedule.rs:166` un `unreachable!(...)`, ambos alcanzables desde
  bytes de un peer
- Resolución: `crate::guards` recorre los doce archivos de producción y falla
  ante `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` y
  la familia `assert!`, con `every_production_file_is_listed` para que un módulo
  nuevo no quede fuera. Clippy deniega además la familia de pánico y
  `indexing_slicing` en `handshake/`, `identity.rs`, `signature.rs` y
  `fingerprint.rs`. Los dos pánicos se eliminaron sin añadir un error muerto
- Estado: resuelto
- Evidencia: rojo en `6b02db6`; catorce indexaciones sin comprobar eliminadas
- Fecha: 2026-08-07

## QYR-0034 — Codificaciones X25519 con `u >= p`

- Plataforma: todas
- Severidad: P2
- Actual: este crate las acepta y la aritmética las reduce, conforme a
  RFC 7748 §5, pero ADR-0021 no registraba la decisión y no había prueba en
  ninguna dirección
- Resolución **parcial**: decisión registrada en la enmienda A a ADR-0021 —se
  aceptan— con `a_non_canonical_x25519_encoding_is_accepted_and_reduced`
- **Sigue abierto**: la auditoría afirma que libsodium y CryptoKit las rechazan.
  Eso **no se ha verificado en este repositorio** y no se toma como hecho. Si
  resulta cierto, la resolución correcta es rechazar también en Rust, porque
  este proyecto rechaza en todas las plataformas lo que rechaza en una
- Estado: **abierto** (decisión registrada, verificación pendiente)
- Dueño: quien escriba el lado Swift
- Fecha: 2026-08-07

## QYR-0035 — Cuatro variantes de `HandshakeError` que nada construía

- Plataforma: todas
- Severidad: P3
- Actual: `UnexpectedRole`, `InvalidEphemeralPublicKey`, `TranscriptMismatch` y
  `SequenceViolation` declaradas, formateadas y listadas en ADR-0021 y en
  `docs/security/handshake-state-machine.md` como controles del handshake, sin
  ningún sitio de construcción. Un llamante podía hacer `match` sobre un control
  que no existía
- Resolución: eliminadas, con el motivo de cada una escrito donde está el enum,
  y `every_handshake_error_has_a_construction_site` para que no vuelva a pasar
- Estado: resuelto
- Evidencia: volver a añadir una, con su brazo de `Display`, hace fallar esa
  guarda por nombre
- Fecha: 2026-08-07

## QYR-0036 — Sin denegación de pánico ni de indexado en los dos crates de parsing

- Plataforma: todas
- Severidad: P2
- Esperado: ninguna ruta que analice bytes de un peer puede terminar el proceso
- Actual: el sprint 4C.2 denegó la familia de pánico y `clippy::indexing_slicing`
  en `qyro_crypto` y no en `qyro_protocol` ni en `qyro_manifest`, que son la
  primera superficie que toca esos bytes. No se encontró ningún pánico concreto;
  lo que faltaba era el control que impediría el próximo
- Workaround mientras estuvo abierto: ninguno; los decoders ya estaban acotados y
  con pruebas de corpus
- Resolución: denegación en los dos, más la guarda estructural. Aparecieron
  **33 infracciones en `qyro_protocol`** (29 en `header.rs`, 3 en `envelope.rs`,
  1 en `frame.rs`) y **22 en `qyro_manifest`** (18 en `model.rs`, 2 en
  `codec.rs`, 2 en `path.rs`). Ninguna se silenció con `allow` salvo los módulos
  de prueba dentro de cada crate. La guarda encontró además un
  `debug_assert_eq!` en `codec.rs` que ningún lint de Clippy cubre, duplicando
  un test que ya corre en todos los perfiles
- Estado: resuelto en el sprint 4C.3
- Evidencia: reintroducir `.expect(` en cualquier archivo de producción de
  cualquiera de los dos hace fallar su guarda por nombre
- Fecha: registrado 2026-08-07 (4C.2), resuelto 2026-08-07 (4C.3)
- Nota de procedencia: este hallazgo estuvo en el ledger **dos veces**, una
  diciendo `abierto` y otra `resuelto`. La primera se registró al detectarlo y
  nunca se actualizó al corregirlo. Las dos entradas se fusionaron aquí en el
  sprint 4D.1 (QYR-0046)

## QYR-0024 — El decoder drenaba cada frame con un memmove del búfer entero

- Plataforma: todas
- Severidad: P1
- Esperado: recibir frames cuesta un trabajo proporcional a los bytes que llegan
- Actual: `next_frame` reclamaba el frame que acababa de entregar con
  `self.buffer.drain(..total)`, que memmovea todo lo que queda detrás. Llenar el
  búfer de frames mínimos y drenarlo cuesta Θ(n²/48). Medido con un contador
  instrumentado: 21 868 heartbeats, 1 049 664 bytes empujados,
  **11 476 501 344 bytes movidos**, 10 935 veces la entrada. Es tráfico válido:
  ningún frame está mal formado y ningún limitador basado en validez lo vería
- Resolución: cursor de lectura y compactación amortizada. Un frame entregado
  solo mueve el cursor; el espacio se reclama en `compact`, y solo cuando los
  bytes entrantes no caben o al menos la mitad del búfer está consumida. Esa
  segunda condición es la amortización: una compactación mueve como mucho la
  mitad del búfer y no puede repetirse hasta que se haya consumido otro tanto
- Estado: resuelto
- Evidencia: rojo en `9c4a1a2`. Después: 0 bytes movidos en el llenado-drenado, y
  2 359 296 contra 2 596 608 empujados en el bucle con backlog, que es donde la
  compactación corre de verdad. Enmienda a ADR-0016
- Fecha: 2026-08-07

## QYR-0027 — La capacidad del búfer llegaba al doble de su límite

- Plataforma: todas
- Severidad: P2
- Esperado: `buffer_capacity() <= MAX_BUFFER_LEN` siempre
- Actual: `push` acotaba `len`, y `Vec::extend_from_slice` crece
  geométricamente, así que goteando un byte por push la capacidad llegaba a
  2 097 152 frente a `MAX_BUFFER_LEN` de 1 049 664. `wire_contract.rs:353` y
  `property.rs:191` ya afirmaban lo contrario y pasaban porque nunca llenan el
  búfer
- Resolución: `reserve_for` conserva el doblado —que es lo que mantiene un push
  amortizado O(1)— y lo recorta al techo. `reserve_exact` a secas habría sido
  peor que el defecto: con pushes de un byte reasigna en cada byte
- Estado: resuelto
- Evidencia: `the_buffer_never_reserves_more_than_its_limit`, que llena el búfer
  de verdad, rojo en `9c4a1a2`
- Fecha: 2026-08-07


## QYR-0037 — Correcciones aplicadas sin numerar

- Plataforma: todas
- Severidad: P3
- Origen: auditoría externa del sprint 4C.1. **El identificador nunca entró en
  este repositorio**, así que el contenido de esta entrada se reconstruye a
  partir de la descripción del prompt del sprint 4C.3 y no de la auditoría
  original, que no está aquí
- Actual: parte del hallazgo se corrigió en el sprint 4C.2 sin numerarlo: la
  comprobación redundante de `U+007F` eliminada, la documentación que decía
  `cfg(test)` donde el atributo es `cfg(any(test, fuzzing))` corregida, y la
  nota sobre `RUSTFLAGS` añadida a `fuzzing.rs`. Corregir sin numerar deja el
  trabajo hecho y la trazabilidad rota: nadie puede comprobar qué se cerró
- Resolución: registrado aquí y enlazado desde
  `docs/audits/SPRINT4C2_AUDIT_CLOSURE.md`, que describe los tres cambios sin
  haberles puesto identificador. La regla nueva de `check_docs_consistency`
  impide que vuelva a ocurrir: un identificador citado sin entrada es un
  BLOCKER
- Estado: resuelto
- Fecha: 2026-08-07

## QYR-0038 — Cotas inalcanzables presentadas como cotas

- Plataforma: todas
- Severidad: P3
- Origen: auditoría externa. Como QYR-0037, el identificador nunca entró en este
  repositorio y el contenido se reconstruye del prompt del sprint 4C.3
- Actual: `MAX_HASH_LEN` en `qyro_manifest` y `FrameError::FrameTooLarge` en
  `qyro_protocol` se presentaban como límites vivos sin serlo
- Resolución: distintas, porque los dos casos son distintos.
  `MAX_HASH_LEN` **sí es alcanzable**, por el constructor y nunca por el cable
  —`decode` lee exactamente `digest_len()` bytes, que son 0 o 32—, y ahora hay
  dos pruebas que lo dicen y lo demuestran. La comprobación de `FrameTooLarge`
  en `FrameHeader::parse` **no puede dispararse**: las cotas de payload y
  trailer ya sujetan la suma. Se conserva, se documenta como inalcanzable con su
  razón, y una aserción `const` fija la aritmética para que subir una constante
  detenga el build en vez de convertir una rama muerta en una viva sin que nadie
  se entere. La variante no está muerta: `FrameDecoder::next_frame` la construye
  cuando un total declarado no cabe en un `usize`, inalcanzable en 64 bits y
  alcanzable en 16
- Estado: resuelto
- Fecha: 2026-08-07

## QYR-0039 — CI compila `cargo-audit` desde fuente en cada run, con pin exacto

- Plataforma: CI
- Severidad: P3
- Esperado: la herramienta que vigila el perímetro de dependencias no amplía ese
  mismo perímetro sin que nadie lo mire
- Actual: `ci.yml:53` hace `cargo install cargo-audit --locked --version 0.22.2`
  en cada ejecución. Dos consecuencias: es lento, y mete alrededor de un centenar
  de crates en el perímetro de confianza de CI que nada audita —la herramienta de
  auditoría es la parte del sistema que menos se audita a sí misma—
- Segundo problema, del pin exacto: un pin exacto caduca. La versión 0.21.2 ya no
  puede parsear el advisory DB actual, que trae entradas CVSS 4.0. Cuando eso
  pasa, el job **falla cerrado**, que es el comportamiento correcto y no una
  emergencia; pero falla por obsolescencia de la herramienta y no por una
  vulnerabilidad, y eso hay que saber leerlo
- Estado: **abierto y programado**. Este sprint le da contenido; no lo corrige,
  porque cambiar cómo CI obtiene su herramienta de auditoría no es trabajo de un
  sprint de almacenamiento seguro
- Acción concreta cuando se aborde: binario preconstruido con checksum, o una
  acción cacheada, o un rango de versiones en vez de un pin exacto. Las tres
  tienen contrapartidas distintas y la decisión merece su propia nota
- Procedencia: el enunciado lo aporta el prompt del sprint 4D.1. La auditoría
  externa original **sigue sin estar en este repositorio**; lo comprobable aquí
  —el pin exacto y su versión— se verificó leyendo `ci.yml`. Ver QYR-0047
- Fecha: registrado sin contenido 2026-08-07, descrito 2026-08-07

## QYR-0040 — El disparador de CI llevaba el nombre de la rama escrito a mano

- Plataforma: CI
- Severidad: P2
- Esperado: los seis workflows corren solos sobre cualquier rama de trabajo
- Actual: el sprint 4C.2 cerró QYR-0026 escribiendo el nombre de la rama de
  entonces en los seis YAML. Eso hizo de «CI corre sobre la rama de trabajo» una
  propiedad de *esa* rama y no del repositorio, y STATUS.md lo registró como lo
  segundo. La rama siguiente heredaba el defecto original intacto.
  `crypto-fuzz.yml` y `crypto-platform.yml` acumulaban ya dos nombres cada uno
- Resolución: `branches: [main, 'claude/**']` en los seis. Un patrón, no un
  nombre. `claude/**` y no `**`: cubrir cualquier rama del repositorio gasta
  minutos de runner en trabajo que este proyecto no empezó. Regla nueva en
  `check_docs_consistency` (Bash y PowerShell) que rechaza un nombre literal en
  cualquier `branches:`
- Estado: resuelto
- Evidencia: los seis runs finales se dispararon por `push` sobre
  `claude/qyro-resource-bounds-4c3`, un nombre que ningún YAML menciona
- Fecha: 2026-08-07

## QYR-0041 — Fecha incorrecta en la cita de Unicode 16.0.0

- Plataforma: todas
- Severidad: P3
- Actual: `path.rs` y ADR-0019 databan Unicode 16.0.0 con la marca de tiempo
  interna de `DerivedGeneralCategory.txt`, que es cuando se generó el archivo y
  no cuando se publicó la versión. Unicode 16.0.0 se publicó el 2024-09-10
- Resolución: fecha corregida en los dos sitios. La tabla en sí era y sigue
  siendo correcta, comprobada punto por punto contra el archivo: 170 puntos de
  código, veintiún rangos
- Estado: resuelto
- Fecha: 2026-08-07

## QYR-0042 — La lista de exenciones de la guarda se satisfacía borrando el gate

- Plataforma: todas
- Severidad: P3
- Esperado: quitar `#[cfg(test)]` de un módulo exento hace fallar la guarda
- Actual: `guards.rs` llevaba un array `TEST_ONLY` de diez nombres escritos a
  mano y `every_production_file_is_listed` aceptaba la pertenencia a ese array
  como suficiente. Quitar `#[cfg(test)]` de `mod schema;` habría convertido
  `schema.rs` en un archivo de producción exento de la guarda sin que nada
  fallara: la misma forma de defecto que la guarda existe para cerrar
- Resolución: las exenciones se derivan de las declaraciones `mod` mismas. El
  gate *es* la exención, así que quitarlo mueve el archivo al conjunto de
  producción en vez de eximirlo
- Estado: resuelto
- Evidencia: quitar `#[cfg(test)]` de `mod schema;` falla con «src/schema.rs is
  compiled into a release build and no guard covers it»
- Fecha: 2026-08-07

## QYR-0043 — Identificadores sin entrada en el registro

- Plataforma: todas
- Severidad: P3
- Actual: QYR-0037 y QYR-0038 tenían cero menciones en `BUGS_PENDING.md`,
  `NEXT_STEPS.md`, `STATUS.md` y la auditoría del sprint. QYR-0024 y QYR-0027
  estaban registrados en STATUS y NEXT_STEPS pero no en el registro, donde están
  todos los demás. QYR-0039 se citaba como no objetivo y no existía en ninguna
  parte
- Resolución: una entrada por identificador, y una regla en
  `check_docs_consistency` (Bash y PowerShell) que hace BLOCKER de cualquier
  `QYR-00xx` citado sin entrada
- Estado: resuelto
- Evidencia: la regla falló con ocho hallazgos antes de escribirse estas
  entradas, en `a1b61c4`
- Fecha: 2026-08-07

## QYR-0044 — «Regenera el vector» como consejo incondicional

- Plataforma: todas
- Severidad: P3
- Actual: bajo la mutación del prefijo `u32`-BE del transcript, la suite imprimía
  seis fallos: uno decía que el código debe cambiar y tres decían «the committed
  vector is stale; regenerate it with …». La trampa sustancial ya estaba cerrada
  —`the_transcript_is_what_the_specification_says_it_is` compara contra bytes
  literales de ADR-0021 y regenerar no deja la suite en verde—, pero el mensaje
  es lo que se lee primero, y decirle a alguien que regenere es decirle que
  registre lo que el código produce ahora
- Resolución: el mensaje del handshake es condicional de verdad, evaluado sobre
  lo único que distingue los dos casos: si este build todavía calcula el
  transcript que ADR-0021 especifica, comprobado con SHA-256 sobre bytes
  literales. Si no lo calcula, dice «Do not regenerate». El documento AEAD no
  tiene una comprobación equivalente de una línea, así que su consejo se
  reformula como «if and only if» con la condición escrita
- Estado: resuelto
- Evidencia: bajo la mutación, el mensaje del handshake cambia a «**and this
  build no longer computes the transcript ADR-0021 specifies**. Do not
  regenerate»
- Fecha: 2026-08-07

## QYR-0045 — Filtros de rutas que no cubren el código que el workflow construye

- Plataforma: CI
- Severidad: P2
- Esperado: si cambia el código que un workflow compila, ese workflow corre
- Actual: encontrado al comprobar QYR-0040. Un push documental disparó cuatro de
  los seis, lo cual es correcto, pero al mirar por qué aparecieron dos huecos
  reales:
  - `android-runtime.yml` e `ios-runtime.yml` compilan `qyro_ffi`, que depende
    de `qyro_core`, y `rust/crates/qyro_core/**` no estaba en ninguno de los
    dos. Un cambio en el único crate que puede alterar lo que Dart enlaza no
    disparaba la comprobación de ABI.
  - `crypto-platform.yml` y `crypto-fuzz.yml` no listaban `rust/guards/**`, que
    este mismo sprint introdujo y que se `include!` en el build de pruebas de
    los tres crates que esos workflows ejercitan. Un cambio en el analizador
    compartido cambia su resultado y no los disparaba
- Resolución: las cuatro rutas añadidas. `platform-builds.yml` ya lo cubría con
  `rust/**` y `ci.yml` no tiene filtro; los dos lo dicen ahora, porque
  `rust/guards/` es un directorio nuevo fuera de `rust/crates/` y un lector que
  compruebe la cobertura merece encontrar la respuesta
- Estado: resuelto
- Evidencia: los seis runs finales de STATUS.md
- Fecha: 2026-08-07

## QYR-0046 — QYR-0036 estaba en el ledger dos veces con dos estados

- Plataforma: documentación
- Severidad: P2
- Esperado: un identificador, una entrada, un estado
- Actual: `## QYR-0036` aparecía dos veces. La de arriba decía «Estado: abierto»
  y era la que registró el hallazgo en el sprint 4C.2; la de abajo decía
  «Estado: resuelto» y era la del 4C.3. La primera nunca se actualizó al
  corregirse el defecto, así que si el hallazgo estaba abierto o cerrado dependía
  de a cuál llegara antes quien lo consultara
- Causa: la regla del ledger comprueba «todo identificador citado tiene entrada»,
  no «tiene exactamente una». Construye el conjunto de entradas con `sort -u` en
  Bash y con un `HashSet` en PowerShell, y las dos estructuras colapsan el
  duplicado antes de que nada pueda verlo. El comentario sobre la regla decía
  «exactly one entry» mientras el código comprobaba «al menos una»
- Resolución: entradas fusionadas en una que conserva las dos mitades de la
  historia —el hallazgo tal como se registró y su resolución con los recuentos de
  infracciones—, más una regla nueva que cuenta las cabeceras leyendo el archivo
  en vez del conjunto deduplicado
- Prevención: `check_docs_consistency` en Bash y en PowerShell trata como BLOCKER
  cualquier `## QYR-00xx` repetido, nombrando el identificador y cuántas veces
  aparece
- Estado: resuelto
- Evidencia: la regla se escribió antes de fusionar y disparó sobre el duplicado
  real en los dos shells: «QYR-0036 has 2 entries in BUGS_PENDING.md»
- Fecha: 2026-08-07

## QYR-0047 — Tres hallazgos externos se registraron reconstruidos, no leídos

- Plataforma: documentación, CI
- Severidad: P3
- Esperado: una auditoría externa citada por identificador está en el
  repositorio, y su entrada en el ledger se escribe leyéndola
- Actual: QYR-0037, QYR-0038 y QYR-0039 se registraron a partir de la
  descripción del prompt de un sprint posterior, no de la auditoría original,
  que nunca entró aquí. El propio ledger lo dice, y en QYR-0039 admite no saber
  qué describe el hallazgo
- Resolución: `docs/audits/external/` existe y HANDOFF.md pide que toda auditoría
  externa se comprometa ahí antes de citarse. El contenido de QYR-0039 lo aporta
  el prompt del sprint 4D.1 y queda registrado en su propia entrada; las
  auditorías de 4C.1 que originaron QYR-0037 y QYR-0038 **siguen sin estar en el
  repositorio**, así que esas dos permanecen reconstruidas y se marcan como tales
- Estado: resuelto en lo que este sprint puede resolver; el hueco de procedencia
  de QYR-0037 y QYR-0038 solo lo cierra quien tenga el documento original
- Fecha: 2026-08-07

## QYR-0048 — La entropía especificada era circular y no se podía implementar

- Plataforma: Windows, especificación
- Severidad: P1
- Esperado: la regla de entropía congelada en ADR-0024 se puede ejecutar
- Actual: `entropía = QYRO_IDENTITY_ENTROPY_V1 ‖ cabecera[0..16]`, y la cabecera
  lleva `wrapped_len` en el offset 12. Para componer la entropía hace falta
  `wrapped_len`; para conocerlo hay que haber llamado ya a `CryptProtectData`; y
  esa llamada necesita la entropía. Circular
- Sin escapatoria por predicción: la propia ADR dice «`N` lo elige DPAPI y no es
  constante», y la referencia de Microsoft que cita dice «Being opaque,
  application developers do not need to parse or understand the format at all»
- Causa: los dos documentos especificaban un **orden de lectura** y ninguno un
  orden de escritura. Al leer, los dieciséis bytes ya están en disco y la regla
  se aplica sin esfuerzo; el hueco solo existe al escribir, y no había nada
  escrito sobre escribir. Es la misma forma que QYR-0024: una ruta que nadie
  especificó es una ruta que nadie revisó
- Resolución: `entropía = QYRO_IDENTITY_ENTROPY_V1 ‖ cabecera[0..12]` —todo menos
  `wrapped_len`—, más un orden de escritura numerado en ADR-0024 y en
  `docs/security/identity-storage.md`
- Lo que sobrevive: «voltear un bit en cualquier posición produce un error
  tipado» sigue siendo cierto por **tres** caminos —`0..12` por la entropía,
  `12..16` por `LengthMismatch`, `16..` por el MAC de DPAPI—, y las pruebas dicen
  cuál esperan en cada tramo. Y la razón de ligar la cabecera sigue en pie: liga
  la **interpretación** del envoltorio, no su longitud
- Alternativa descartada: poner `wrapped_len` a cero al componer la entropía en
  ambas fases. Equivalente, con más ceremonia y con un campo que miente
- Estado: resuelto en la especificación; la implementación llega después
- Fecha: 2026-08-07

## QYR-0049 — La rama tenía runs en verde y STATUS.md no los nombraba

- Plataforma: documentación, CI
- Severidad: P3
- Esperado: STATUS.md registra la evidencia ejecutada de la rama en curso
- Actual: cuatro runs de CI en verde sobre `claude/qyro-secure-storage-4d1` y
  ninguna tabla de 4D.1 que los nombrara. `Verified commit` seguía apuntando a
  `c21dd72`, del sprint anterior, mientras el archivo describía ADR-0024, que
  allí no existe
- Causa: los otros cinco workflows filtran por rutas y el sprint empezó por
  documentación, así que solo corrió CI. Sin tabla, «solo corrió uno» y «no se
  miró» se leen igual
- Resolución: tabla de runs de 4D.1 en STATUS.md con la frase que explica por qué
  los otros cinco no corrieron. `Verified commit` se mueve cuando este sprint
  tenga sus propios seis en verde sobre un mismo commit, no antes: moverlo a un
  commit con un solo workflow ejecutado sería exactamente la sustitución que el
  campo existe para impedir
- Estado: resuelto en la parte documental; el ancla se mueve al cerrar el sprint
- Fecha: 2026-08-07

## QYR-0050 — La ruta del blob depende de un nombre de producto provisional

- Plataforma: Windows, producto
- Severidad: P3
- Esperado: la ubicación del almacén no depende de un valor que STATUS lista
  como bloqueante para empaquetar
- Actual: `docs/security/identity-storage.md` fija
  `%LOCALAPPDATA%\Qyro\identity.bin`, y STATUS sigue listando el clearance del
  nombre «Qyro» y la base `com.owner.qyro` entre los valores provisionales. El
  formato tiene byte de versión para el **contenido** y nada para la
  **ubicación**
- Consecuencia si el nombre cambia: los blobs quedan huérfanos y el código nuevo
  lee «no hay identidad», que es justo el caso que el orden de lectura se
  esfuerza en separar de «hay una y no se puede leer». La peor forma de fallar de
  este diseño, alcanzada sin que nadie toque un byte del blob
- Resolución de este sprint: **ninguna en código**, porque el crate de plataforma
  todavía no existe. Queda decidido y anotado: la ruta vivirá en una sola
  constante del crate de plataforma, y un cambio de nombre exige migración
  explícita —leer con la ruta antigua, escribir con la nueva, borrar la antigua—
  y no un cambio de literal
- Estado: **abierto**, con la decisión tomada y la implementación pendiente del
  crate de plataforma
- Fecha: 2026-08-07

## QYR-0051 — La rama quedó en rojo por una política que este archivo se inventó

- Plataforma: CI, documentación
- Severidad: P1
- Esperado: la rama de trabajo no se deja en rojo
- Actual: CI #115 (run 31206358256) sobre `940b49d` falló en el job
  `documentation`: «Stale verified commit: HEAD is 11 commits ahead of the
  verified commit (limit 10)». `3527db7` estaba a exactamente diez y pasó por un
  margen de uno
- Causa: `Verified commit` seguía en `c21dd72`, del sprint 4C.3, sostenido por una
  política escrita en STATUS.md —«se moverá cuando este sprint tenga sus propios
  seis en verde sobre un mismo commit»— que **no puede coexistir** con el límite
  de diez commits en un sprint largo. La política era invención de este sprint;
  `HANDOFF.md` ya decía lo contrario: «STATUS.md debe actualizarse dentro del
  mismo tramo de trabajo, no al final»
- Agravante: la tabla de runs listaba cuatro, todos success, y se detenía en
  `e0786ee`. Un lector concluía que la rama estaba sana
- Resolución: ancla movida a un commit de esta rama, política escrita en STATUS.md
  —se mueve **por tramo**, y no es una afirmación de que seis workflows corrieran
  sobre él, que es lo que dice la tabla fila por fila—, y el fallo registrado con
  su run ID y su causa
- Estado: resuelto
- Fecha: 2026-08-07

## QYR-0052 — La ligadura de la cabecera a la entropía no la comprobaba nada

- Plataforma: Windows, pruebas
- Severidad: P2
- Esperado: la razón que la enmienda de QYR-0048 da para meter la cabecera en la
  entropía —«liga el envoltorio a ese `version`, ese `wrap` y ese `reserved`»—
  está cubierta por una prueba
- Actual: sustituir los doce bytes de cabecera por doce ceros en `entropy_for`
  —misma longitud, ninguna ligadura— dejaba **toda la suite en verde**
- Causa: el único test comprobaba (a) la longitud y (b) que
  `entropy_for(V, W) == entropy_for(V, W)`, que es la misma función pura con los
  mismos argumentos: una tautología
- Patrón, no incidente: es la **tercera** vez con esta forma exacta. QYR-0025 (un
  transcript verificado llamándose a sí mismo) y la aserción tautológica del
  target `encrypted_envelope` son las otras dos
- Estado: abierto al inicio de este tramo
- Fecha: 2026-08-07

## QYR-0053 — La guarda de material de clave no veía la semilla en claro

- Plataforma: criptografía, pruebas
- Severidad: P2
- Esperado: un `pub fn` que devuelva la semilla hace fallar la guarda
- Actual: añadir `pub fn leak_raw(&self) -> Zeroizing<[u8; 32]>` a `identity.rs`
  dejaba `every_public_path_returning_key_material_is_listed` **en verde**: el
  retorno no contiene ninguno de los cinco marcadores
- Causa: `[u8; 32]` se excluyó a propósito porque un fingerprint también mide
  treinta y dos bytes, y el comentario que lo explica es correcto. La conclusión
  no: excluirlo sin más deja fuera justo lo que la guarda vigila. Una lista de
  marcadores es una lista de permitidos disfrazada de prohibidos
- Estado: abierto al inicio de este tramo
- Fecha: 2026-08-07

## QYR-0054 — Nada comprobaba `forbid(unsafe_code)`

- Plataforma: workspace, pruebas
- Severidad: P2
- Esperado: STATUS afirma «todos los crates conservan `forbid(unsafe_code)`,
  incluido el nuevo» y algo lo comprueba
- Actual: quitar `#![forbid(unsafe_code)]` de `qyro_identity_store` no rompía
  nada. La afirmación descansaba en que nadie lo hubiera hecho
- Urgencia: la guarda tiene que existir **antes** del crate de plataforma. Si
  llega después, añadir la excepción es indistinguible de un `forbid` que nunca
  estuvo
- Estado: abierto al inicio de este tramo
- Fecha: 2026-08-07

## QYR-0055 — Tres afirmaciones de STATUS.md que el repositorio contradecía

- Plataforma: documentación
- Severidad: P3
- Actual, las tres:
  1. «la enmienda va en el commit `df9f574`, anterior al primer commit de
     implementación». Falso: `0ff21bd` —`feat:`, 217 líneas de Rust— es anterior.
     La intención se cumplió; la frase no era la intención
  2. El encabezado decía «lo que existe a este commit es decisión y
     especificación, **no código**» y tres viñetas después listaba el crate
  3. `Updated UTC` marcaba las 07:10 con el último commit a las 18:18
- Causa: las tres son del mismo tipo —una frase escrita cuando era cierta y no
  revisada cuando dejó de serlo—, y ninguna regla las cubre porque las tres son
  prosa
- Resolución: corregidas diciendo lo que sí es cierto, no borradas
- Estado: resuelto
- Fecha: 2026-08-07

## QYR-0058 — Las dos guardas de aislamiento nombraban un solo harness

- Plataforma: CI, pruebas
- Severidad: P2
- Esperado: ningún harness de pruebas puede alcanzar el producto
- Actual: `check_harness_isolation.{sh,ps1}` tenían `harness="qyro_crypto_smoke"`
  escrito a mano. Al añadir `qyro_store_smoke` en el sprint 4D.1, las dos guardas
  siguieron en verde sobre un segundo harness **completamente sin vigilar**
- Causa: una guarda que nombra *una instancia* de una categoría deja de cubrir la
  categoría en cuanto aparece la segunda. Es la misma forma que QYR-0045 —un
  filtro de rutas que enumera crates— y que QYR-0053 —una lista de marcadores de
  tipo—, y las tres fallan en la dirección silenciosa
- Resolución: las dos guardas iteran sobre una lista de harnesses
- Evidencia: hacer que `qyro_crypto` dependa de `qyro_store_smoke` hace fallar
  las dos por nombre, comprobado en Bash y en PowerShell
- Estado: resuelto
- Fecha: 2026-08-07

## QYR-0056 — La guarda de material de clave no ve `Vec<u8>` ni `String`

- Plataforma: criptografía, pruebas
- Severidad: P2
- Esperado: cualquier camino público que devuelva la semilla hace fallar la guarda
- Actual: `every_public_path_returning_key_material_is_listed` clasifica por
  **forma del tipo de retorno**, así que un `pub fn` que devolviera la semilla
  como `Vec<u8>` o `String` no dispara ningún marcador
- Solución medida, para cuando se aborde: dejar de adivinar formas y congelar los
  **sitios de origen**. `signing_key.to_` aparece una sola vez en producción, en
  `identity.rs` dentro de `export_secret`. Una prueba que enumere esos sitios y
  falle si aparece otro es la misma técnica que
  `every_handshake_error_has_a_construction_site`
- Estado: abierto
- Fecha: 2026-08-07

## QYR-0057 — Tres entradas del ledger usan un `Estado` que no es un estado

- Plataforma: documentación
- Severidad: P3
- Actual: QYR-0052, QYR-0053 y QYR-0054 dicen `Estado: abierto al inicio de este
  tramo`, que describe una historia y no un estado. Las tres están resueltas
- Solución: `Estado` debe ser uno de un conjunto cerrado —`abierto`, `resuelto`,
  `parcial`, `cerrado por obsolescencia`—, con la narración en otra línea, y una
  regla de `check_docs_consistency` que rechace cualquier otra cosa
- Estado: abierto
- Fecha: 2026-08-07

## QYR-0059 — DPAPI no autentica todos los bytes de su propio blob

- Plataforma: Windows
- Severidad: **P3** (registrado como P1 hasta responder la pregunta de abajo)
- Esperado: voltear un bit en cualquier posición del blob produce un error
  tipado. Para el tramo `16..` —el envoltorio de DPAPI— se daba por hecho que lo
  atraparía el MAC que DPAPI documenta: «The function also adds a Message
  Authentication Code (MAC) (keyed integrity check) to the encrypted data to
  guard against data tampering»
- Actual: el barrido de 448 posiciones **contra DPAPI real** encuentra **128
  posiciones supervivientes: los bytes 20..36 del blob, los ocho bits de cada
  uno**. Son dieciséis bytes contiguos en el offset 4 del envoltorio, es decir
  **el GUID del provider**: DPAPI ni lo autentica ni lo consulta al desproteger.
  Runs 31211959010 y 31213769557, job `windows-crypto`
- **Corrección de una medición propia:** la primera versión de este hallazgo
  decía «byte 20, bit 0», en singular. Era cierto y estaba incompleto: la prueba
  entraba en pánico en la primera superviviente, así que esa era la única que
  alguien había visto. Al recogerlas todas aparecieron 128. Una cota elegida para
  encajar con una observación no es una propiedad
- Lo que esto invalida: la afirmación de `docs/security/identity-storage.md` de
  que el tramo `16..` lo cubre «el MAC propio de DPAPI sobre el envoltorio` es
  **falsa tal como está escrita**. El MAC cubre los datos cifrados, no cada byte
  de la estructura que los rodea; el blob lleva cabecera propia —versión, GUID
  del provider, sal— y al menos un byte de esa zona no está autenticado
- **Respondido: la identidad que sale es la MISMA, en las 128.** Run 31212494494
  lo mostró para la primera; el run 31213769557 lo comprueba para todas, porque
  la aserción de fingerprint corre dentro del bucle y ninguna falló. El byte alterado descifra a la misma semilla, así que el blob es
  **maleable en un campo que DPAPI ignora** y **no** es un camino para sustituir
  en silencio la identidad de un dispositivo, que era el resultado que habría
  sido grave. La prueba se modificó para responder esta pregunta en vez de
  relajarse
- **Severidad revisada: P3.** Lo que queda es que un atacante con acceso de
  escritura al archivo puede alterar un byte sin que nada lo note; no gana
  lectura de la semilla, no gana sustitución de identidad, y ya tenía acceso de
  escritura. Lo que **sí** hay que corregir es la afirmación, no el formato:
  `identity-storage.md` decía que el MAC de DPAPI cubre todo el tramo `16..` y
  no es cierto
- **Decisión, tomada**: la prueba acepta el conjunto exacto con su razón escrita.
  La alternativa era que Qyro dejara de tratar el envoltorio como opaco y lo
  cubriera con un MAC propio, y eso contradice «no inventes criptografía» y la
  decisión de ADR-0024 §2 de no añadir uno. Lo que la prueba fija no es una cota
  ni un permiso: es el conjunto `20..36 × 0..8`, y **cualquier** otro conjunto
  —una posición nueva, una que deje de sobrevivir— la pone en rojo. Eso es lo
  que la convierte en una medición vigilada y no en una excepción
- **Lo que sigue abierto**: el conjunto es una observación sobre el
  `windows-latest` de hoy, no un contrato de Microsoft, que dice explícitamente
  que el formato es opaco y no debe parsearse. Otra versión de Windows puede
  moverlo, y entonces esta prueba falla. Fallar es el comportamiento correcto:
  quien la vea fallar tiene que volver aquí y decidir de nuevo, no ampliar el
  rango
- Lo que **no** se hizo: ajustar la aserción para que pase. El prompt del sprint
  lo dice y es lo correcto: si un tramo cae por otro camino, eso es el hallazgo
- Estado: abierto
- Fecha: 2026-08-07

## QYR-0060 — STATUS.md afirmaba la persistencia arriba y la negaba abajo

- Plataforma: documentación
- Severidad: **P2**
- Esperado: STATUS.md es la fuente canónica del estado ejecutable, así que dos
  párrafos suyos no pueden decir cosas incompatibles sobre la misma capacidad
- Actual: en `91355a8` la línea `Milestone` decía «una identidad sobrevive al
  cierre del proceso en Windows … **IMPLEMENTED solo en Windows**», y la sección
  «Sprint 4D.1 en curso», ochenta líneas más abajo, seguía abriendo con «**No hay
  persistencia en ninguna plataforma**» y listando bajo «lo que no existe
  todavía»: «No hay crate de plataforma y no hay DPAPI», «No hay harness de dos
  procesos ni paso de CI que ejecute persistencia», «No hay `storage-v1.json`» y
  «**No hay `unsafe` en ninguna parte del producto**». Las cinco eran falsas en
  ese mismo commit
- Causa: la cabecera se actualizó con la evidencia nueva y el cuerpo no. Es la
  misma forma que QYR-0055, que se registró en este mismo sprint por tres
  afirmaciones de este mismo archivo, y volvió a ocurrir **doce commits después**
  de haberla registrado. Registrar una forma de fallo no la previene
- Alcance real: nadie fue engañado hacia arriba —el cuerpo era más conservador
  que la cabecera, no al revés—, pero eso es suerte de esta ocurrencia y no una
  propiedad. La misma omisión con los signos cambiados es una capacidad
  reclamada sin evidencia
- Lo que **no** se hizo: escribir una regla de `check_docs_consistency` que
  compare la línea `Milestone` con el cuerpo. No hay forma honesta de comprobar
  con un `grep` que dos párrafos en prosa concuerdan, y una regla que finja
  hacerlo es una guarda que no guarda —el defecto que QYR-0052, QYR-0053 y
  QYR-0054 documentan en este mismo sprint—. Queda como disciplina: la sección
  de sprint en curso se reescribe **en el mismo commit** que mueve la cabecera
- Estado: cerrado
- Fecha: 2026-08-07

## QYR-0061 — Dos filas de la tabla de runs no resistían una comprobación

- Plataforma: documentación
- Severidad: **P2**
- Esperado: cada fila de las tablas de runs de STATUS.md nombra un run que
  existe y su conclusión real. Es la única forma de evidencia que este proyecto
  acepta, así que una fila que no se puede comprobar no es una fila débil: es
  una fila falsa
- Actual, dos filas de «Runs de 4D.1»:
  - `Crypto platform #14` sobre `3f25874` figuraba como **success**. Fue
    **cancelled**, por el grupo de concurrencia `cancel-in-progress`
  - `CI` sobre `0cb18ec` citaba el run **31207659962**, que **no existe**: la API
    responde `404 Not Found`. El run real de ese commit es 31207950941
- Causa: las dos filas se escribieron desde la memoria de la sesión en vez de
  desde la lista de runs de la rama. Una cancelación y un éxito se parecen mucho
  cuando lo que se recuerda es «ese commit estaba bien», y un identificador de
  once dígitos no se verifica solo por releerlo
- Además, la tabla **omitía cuatro fallos** —`Crypto platform` #20, #21 y #22, y
  `CI` #127— y siete runs en verde. La versión actual lista **todos** los `push`
  de la rama, obtenidos por API
- Impacto: ninguna conclusión cambia. El run cancelado se sustituyó por
  `Crypto platform #15` sobre `940b49d`, que sí pasó, y el commit con el
  identificador equivocado sí tuvo su run en verde. Eso es exactamente por qué
  merece registrarse: los dos errores eran invisibles precisamente porque no
  rompían nada
- Lo que **no** se hizo: una regla automática. `check_docs_consistency` no puede
  llamar a la API de GitHub —el job documental corre sin red garantizada y una
  guarda que depende de un servicio externo falla por razones que no son el
  defecto que vigila—. Lo que sí queda es el método: reconstruir la tabla desde
  `actions_list` sobre la rama, no desde lo que uno recuerda
- Estado: cerrado
- Fecha: 2026-08-07

## QYR-0062 — `NEXT_STEPS.md` dice que QYR-0039 no tiene contenido, y sí lo tiene

- Plataforma: documentación
- Severidad: P3
- Esperado: un archivo canónico no contradice al ledger sobre si un hallazgo
  tiene enunciado
- Actual: `NEXT_STEPS.md` decía «**QYR-0039**: recuperar el enunciado del
  hallazgo … su contenido no está en este repositorio, así que no se puede ni
  cerrar ni evaluar», mientras `BUGS_PENDING.md:694` lleva el enunciado completo
  desde el sprint 4D.1 —`cargo-audit` compilado desde fuente en cada run, con
  pin exacto— y **el propio `NEXT_STEPS.md`, treinta y cuatro líneas más abajo,
  lo describe bien**. El archivo se contradecía a sí mismo
- Causa: al reescribir la sección de sprint en 4D.1 arrastré el bullet viejo
  literal en vez de leerlo. Copiar un párrafo es más rápido que comprobarlo, y
  ésa es exactamente la diferencia
- Corrección: el bullet obsoleto se sustituye por lo que el ledger dice
- Estado: cerrado
- Fecha: 2026-08-07

## QYR-0064 — El harness de binario empujado no puede alcanzar Android Keystore

- Plataforma: Android
- Severidad: **P1** para el sprint 4D.2a; es un hallazgo de especificación, no
  un defecto del código
- Esperado: el prompt de 4D.2a §8.4 pide demostrar la persistencia en Android
  «empujado por `adb` como hace `android_crypto_smoke.sh`», es decir con la
  misma forma de harness que 4D.1 usó en Windows
- Actual: **no se puede.** `android_crypto_smoke.sh` empuja un ejecutable nativo
  a `/data/local/tmp` y lo lanza con `adb shell`. Ese proceso no tiene runtime
  ART con las clases del framework y no corre con un UID de aplicación
- Fuentes, consultadas 2026-08-07:
  - La lista de APIs nativas estables del NDK **no incluye keystore, keychain ni
    gestión de claves** (comprobado como ausencia en la lista, no como cita)
  - «`AndroidKeyStore` … consists of Java code that runs in the app's own process
    space» y «fulfills app requests for Keystore behavior by forwarding them to
    the keystore daemon» (AOSP, Hardware-backed Keystore)
  - «the UID of the caller is also included to disambiguate keys from different
    apps» (AOSP, Hardware-backed Keystore)
- Consecuencia: la evidencia de persistencia en Android exige un **test
  instrumentado** ejecutado con `am instrument` dentro de un proceso de
  aplicación. Dos invocaciones son dos procesos, que es lo que la propiedad
  pide. Sigue siendo `adb`, sigue siendo el emulador y sigue siendo un harness
  aislado según ADR-0023; lo que cambia es que el proceso es una app
- Coste real, dicho antes de empezarlo y no descubierto a mitad: andamiaje
  Gradle nuevo —módulo, manifiesto, runner de instrumentación, empaquetado de la
  `.so` en `jniLibs`— más la capa JNI en Rust
- Alternativa descartada: cliente AIDL escrito a mano contra `keystore2` sobre
  binder del NDK. Más superficie `unsafe` que todo el sprint 4D.1 junto, contra
  una interfaz de sistema versionada que no promete estabilidad a las apps
- Registrado en: ADR-0025 §1.2
- Estado: abierto
- Fecha: 2026-08-07

## QYR-0065 — Sin fuente verbatim sobre la invalidación de claves sin autenticación

- Plataforma: Android
- Severidad: P2
- Esperado: ADR-0025 §3.2 decide **no** exigir autenticación de usuario para la
  clave que envuelve la identidad. Esa decisión debería apoyarse en la página de
  referencia de `KeyGenParameterSpec.Builder`
- Actual: esa página, la de `KeyPermanentlyInvalidatedException` y la de
  `KeyProtection.Builder` **se renderizan con JavaScript y no se pudieron
  obtener** en esta sesión. Lo que hay de `setInvalidatedByBiometricEnrollment`
  y de la invalidación al quitar el bloqueo de pantalla viene de resúmenes de
  buscador **sobre** esas páginas, no de su texto
- Lo que **no** se hizo: citar el resumen como si fuera la página. ADR-0025 lo
  marca como fuente secundaria y **no apoya ninguna decisión en él**; se eligió
  deliberadamente el camino que no necesita el dato que falta
- Lo que falta confirmar: que una clave **sin** `setUserAuthenticationRequired`
  sobrevive a quitar y volver a poner el bloqueo de pantalla. Si no sobreviviera,
  la identidad de Qyro se perdería en un cambio de PIN y ADR-0025 cambia
- Estado: abierto
- Fecha: 2026-08-07

## QYR-0066 — No está medido qué error da Keystore cuando el alias ya no existe

- Plataforma: Android
- Severidad: P2
- Esperado: el paso 1 del orden de lectura distingue «no hay identidad» de «hay
  una y no se puede leer». Confundirlos genera una identidad nueva en silencio
  sobre una que seguía ahí, que es el peor resultado que este formato puede
  producir (ADR-0024 §3)
- Actual: tras un restore en un dispositivo nuevo, el blob envuelto puede llegar
  y la clave de Keystore no —es no exportable y ligada al dispositivo—. **No
  está medido** qué observa la aplicación en ese caso: ausencia del alias,
  `KeyPermanentlyInvalidatedException`, o un fallo de tag en `Cipher.doFinal`.
  Cada uno mapea a una variante distinta de `StoreError`
- Lo que **no** se hizo: suponerlo. La página de Keystore no cubre backup ni
  restore y la de Auto Backup no menciona Keystore; las dos se comprobaron
- Cómo se cierra: midiéndolo contra el emulador cuando exista el harness de
  §QYR-0064, o declarándolo explícitamente como no medido
- Estado: abierto
- Fecha: 2026-08-07

## QYR-0067 — La especificación del blob se quedó atrás del código en 4D.2a

- Plataforma: documentación
- Severidad: P2
- Esperado: `docs/security/identity-storage.md` es la referencia byte a byte que
  leería una segunda implementación. Si discrepa del código, la segunda
  implementación construye algo que la primera no acepta
- Actual, tres desajustes introducidos por el sprint 4D.2a:
  - la tabla de la cabecera decía `wrap 0x01 = DPAPI ámbito de usuario` y no
    registraba el `0x02` que ADR-0025 §5 añadió;
  - el orden de lectura tenía nueve pasos y **no incluía la comparación de
    `wrap`** que el código hace entre el 7 y el 8;
  - el paso 8 decía `CryptUnprotectData` como si sólo hubiera un envoltorio, y
    la fila `wrapped` decía «salida opaca de CryptProtectData»
- Causa: 4D.2a añadió el byte y la comprobación en el mismo commit y no tocó
  este archivo. Es la forma de QYR-0055 y QYR-0060 otra vez: el código avanza y
  el documento que lo describe se queda
- Corrección: la tabla lista los dos valores, el orden de lectura tiene diez
  pasos con `WrapMismatch` como paso 8, y el paso 9 nombra el envoltorio que
  corresponda en vez de uno concreto
- Estado: cerrado
- Fecha: 2026-08-08

## QYR-0068 — La cabecera QYRO/1 lleva identificadores que nadie puede rellenar

- Plataforma: protocolo
- Severidad: P2
- Esperado: la cabecera de 48 bytes reserva `transfer_id` (u64), `stream_id`
  (u32) e `item_id` (u32), y los tres viajan **dentro de los datos asociados
  autenticados** de cada frame sellado. Estar en la AAD significa que el peer no
  los puede alterar sin romper el tag, que es exactamente la propiedad por la
  que valdría la pena ponerlos ahí
- Actual: corregido el registro. `Frame::new` los inicia en cero, pero
  `Frame::with_identifiers` y `FrameHeader::with_identifiers` ya eran públicas
  desde `cc38554`; ADR-0029 congela esa API real y define cero como valor sin
  ámbito asignado por la capa superior
- Cómo se encontró: escribiendo ADR-0026 §1 decidí repetir `item_id` en el
  cuerpo de `DataChunk`, y al implementarlo descubrí que la cabecera ya lo
  llevaba. Es el desajuste que el sprint 5A existía para destapar: dos piezas
  probadas por separado, con un campo que una declara y la otra no puede usar
- Lo que **no** se hizo: no se añadió un tercer setter ni `FrameIdentifiers`, y
  no se movió `item_id` fuera del cuerpo de `DataChunk`; eso pertenece al
  contrato de ADR-0026 y a un crate fuera del alcance de esta fase
- Resolución: ADR-0029 fue congelada en `b4faf2e` antes del código. Los tres
  campos sobreviven al seal/open, alterar `transfer_id` rompe el tag y el layout
  se compara contra un vector literal de 48 bytes. Los IDs desconocidos se
  rechazan con errores tipados en la capa receptora después de autenticar
- Estado: cerrado
- Fecha: 2026-08-08
- Evidencia: contratos nominales en `62c82b8`; las mutaciones de setter, AAD y
  offset hicieron fallar cada prueba con nombre. CI 31534679436 pasó

## QYR-0069 — Un crate externo no puede construir un handshake determinista

- Plataforma: criptografía
- Severidad: P3
- Esperado: `qyro_transfer` necesita un sealer y un opener reales para probarse.
  Los obtiene de un handshake real, que es lo correcto
- Actual: `send_hello_with_entropy` y `receive_initiator_hello` son
  `pub(crate)`, así que desde fuera sólo existen `send_hello` y
  `receive_initiator_hello_from_system`, que toman entropía del sistema. Un
  crate dependiente **no puede** fijar la entropía, y por tanto no puede
  reproducir una sesión byte a byte
- Por qué probablemente esté bien: un constructor determinista público es un
  constructor que alguien acaba usando en producción, que es la razón por la que
  `from_test_seed` también es privado. Las pruebas de 5A no necesitan
  determinismo —lo que se prueba es el motor, no el handshake—, así que aquí no
  cuesta nada
- Cuándo costará: cuando haga falta un vector interoperable de una transferencia
  completa, como los de `handshake-v1.json`. Ahí sí hace falta reproducir la
  sesión, y entonces habrá que decidir entre un `cfg(feature)` de pruebas o
  vectores generados dentro de `qyro_crypto`
- Estado: abierto
- Fecha: 2026-08-08

## QYR-0070 — Dos veredictos de integridad sin una sola prueba que los produjera

- Plataforma: transferencia
- Severidad: P2
- Esperado: cada variante de `ItemVerdict` la produce alguna entrada, y borrar el
  control que la produce rompe alguna prueba
- Actual: `SizeMismatch` e `Incomplete` no aparecían en ninguna prueba y borrar
  sus dos controles dejaba la suite entera en verde. No era un agujero —un
  archivo truncado caía a `DigestMismatch` y se rechazaba igual— pero 5B.1
  construye reanudación encima de la distinción entre «incompleto» y «corrupto»
- **Lo que las pruebas descubrieron al escribirse:** la infra-entrega **nunca
  llega a la fase de veredicto**. El control de `Complete` la rechaza antes con
  `CompleteBeforeAllItems`, así que el único camino a `SizeMismatch` es un peer
  que envía **más** de lo que el manifest declara. La prueba se llama
  `an_over_delivered_item_is_a_size_mismatch` y parece lo contrario de lo que su
  nombre sugiere, por eso
- **`Incomplete` es inalcanzable, y se puede demostrar.** Con `k` chunks
  contiguos el receptor ha tomado como mucho `k · CHUNK_SIZE` bytes, y
  `k < ceil(size / CHUNK_SIZE)` acota eso por debajo de `size`. Las dos
  condiciones que exige —`received >= size` y `next_expected < expected_chunks`—
  no pueden darse a la vez
- Resolución: `SizeMismatch` tiene prueba y su mutación la rompe. `Incomplete`
  queda **exento por nombre y con el argumento escrito** en
  `VERDICTS_WITH_NO_CONSTRUCTION_SITE`, no borrado: el byte `3` de
  `IntegrityResult` está congelado en ADR-0026 §1 y quitar un valor de un formato
  congelado es un cambio de formato que este sprint no tiene mandato para hacer
- Efecto colateral, y el motivo de que se encontrara: la guarda de sitios de
  construcción se llevó al análisis compartido y destapó **dos variantes más**
  que 5A había declarado y nadie construía, `TransferError::UnsupportedMessage` y
  `TransferError::WindowExhausted`. Las dos **borradas**; la segunda además tenía
  un comentario que afirmaba que se reportaba, y era falso
- Estado: cerrado
- Fecha: 2026-08-08

## QYR-0071 — El análisis de guardas leía la mitad de un archivo y nadie lo notaba

- Plataforma: guardas
- Severidad: **P1**
- Esperado: `production_source` devuelve el archivo entero menos lo que está
  detrás de un `#[cfg(test)]`. Todas las guardas estructurales del proyecto
  —anti-pánico, completitud, sitios de construcción— se construyen encima
- Actual: `item_end` sabía terminar un item en `;` o en el `}` de un cuerpo, y
  **no en la coma de un campo**. `#[cfg(test)] peak_content_held: usize,` en
  `qyro_transfer` no tiene ni cuerpo ni punto y coma, así que el escaneo pasó de
  largo, desincronizó el conteo de llaves y **se comió el resto del archivo**
- Medido: **13 401 bytes analizados de 30 861**. Menos de la mitad. Desde el
  sprint 5A, `no_production_path_can_panic` sobre `session.rs` cubría el 43 % del
  archivo mientras decía cubrirlo entero
- Cómo se encontró: la guarda de sitios de construcción, recién llevada al
  análisis compartido, dijo que `WindowGrantTooLarge` no se construía en ninguna
  parte — y sí se construye, en `session.rs:441`. La contradicción entre lo que
  la guarda decía y lo que el `grep` mostraba es lo que destapó el truncamiento
- Corrección, en dos partes porque una sola no basta: `item_end` termina también
  en una coma a profundidad cero; y `assert_analysis_reached_the_end` compara la
  última línea no vacía del archivo con lo que sobrevivió al análisis. Lo segundo
  es lo que importa: atrapa **cualquier** forma de item futura que el stripper no
  conozca, no sólo ésta
- Es la cuarta vez que este proyecto encuentra una guarda que dejó de guardar,
  tras QYR-0025, QYR-0036 y QYR-0052/0053/0054. La diferencia es que esta vez el
  fallo no estaba en la guarda sino **en el análisis que comparten todas**
- Estado: cerrado
- Fecha: 2026-08-08

## QYR-0072 — La carrera intermedia no se cierra con comprobaciones por nombre

- Plataforma: filesystem
- Severidad: P2
- Esperado: ningún componente de la ruta materializada es un enlace simbólico, y
  la comprobación no tiene ventana
- Actual: ADR-0027 §1 comprueba cada componente con `symlink_metadata` y abre el
  `.qyro-part` con `O_NOFOLLOW`. Eso cierra por completo la carrera del **último**
  componente —comprobar y abrir son la misma llamada— y **no** la de los
  intermedios: entre comprobar que `fotos/` no es un enlace y abrir
  `fotos/x.qyro-part` hay una ventana
- Cerrarla exige abrir cada directorio por descriptor y resolver relativo a él
  —`openat` con `O_NOFOLLOW`, o `dirfd`—, que no está en `std` y que en Windows
  es otro mecanismo. Traerlo significaría una dependencia nueva o `unsafe`, y las
  dos merecen su propia decisión
- A quién afecta, dicho con precisión: un atacante con escritura en el directorio
  de destino **ya puede escribir lo que quiera ahí**. Lo que las comprobaciones
  impiden es que use Qyro para escribir **fuera** de ahí. La ventana devuelve
  parte de ese privilegio durante un instante
- Decisión: opción (c), mitigación parcial sin dependencias. `FileSink` conserva
  la raíz canonicalizada y `open_part` vuelve a canonicalizar el padre después
  de obtener el handle y antes de truncar, borrar o escribir. Un padre que sigue
  fuera produce `FsError::EscapesRoot`. La opción (a) por descriptor es la única
  que cerraría la carrera, pero exige APIs fuera de `std` y dos implementaciones;
  no se añadió `libc` ni un cuarto crate con `unsafe`. La opción (b) dejaría sin
  implementar ADR-0027 §1.5 y se descartó
- Límite aceptado: un atacante puede hacer un doble cambio —fuera durante el
  `open`, dentro durante la canonicalización— y dejar un handle exterior que la
  comprobación por nombre no detecta. La creación puede dejar un archivo vacío
  fuera; `digest`, `rename` y `remove_file` conservan ventanas propias. Sólo la
  opción (a) completa puede cerrar esas propiedades
- Estado: resuelto por decisión de riesgo y mitigación; la TOCTOU no se declara
  cerrada
- Fecha: 2026-08-08
- Evidencia: enmienda congelada en `01133a8`, código `5deb51a`; sin la
  comprobación post-open,
  `an_opened_part_outside_the_root_is_rejected_before_it_can_be_changed`
  devolvió `Ok(File)` y falló. CI 31537833116 pasó en Ubuntu, macOS y Windows

## QYR-0073 — `O_NOFOLLOW` no tiene una prueba que ejerza el enlace final

- Plataforma: Linux, macOS y Windows probados en CI; Android e iOS sólo
  compilados, sin ejecución de filesystem en dispositivo
- Severidad: P1
- Esperado: una transferencia real mediante `FileSink` rechaza un
  `<destino>/<nombre>.qyro-part` que sea un enlace simbólico, no modifica el
  objetivo externo y devuelve el error tipado correspondiente
- Actual: la prueba existente compara dos veces el digest del mismo archivo y
  no construye el enlace en la ruta que abre producción. Sustituir
  temporalmente `O_NOFOLLOW` por `0` dejó las 388 pruebas Linux en verde
- Resolución: la prueba usa `FileSink` y coloca el enlace en la ruta
  `.qyro-part` que abre producción. Exige `SymlinkInPath`, conserva byte por
  byte el objetivo externo y no produce el nombre final. `open_part` clasifica
  el rechazo atómico Unix y el handle de reparse point Windows sin convertir
  una segunda consulta de ruta en el control de seguridad
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: CI 31529521600 y 31529821869 pasaron el test real en Ubuntu, macOS
  y Windows. Con `O_NOFOLLOW = 0`, CI 31529689978 falló el test nominal en
  Ubuntu con `the real FileSink path returned the wrong typed error: Ok(())`;
  el control quedó restaurado en `a9f21a9`

## QYR-0074 — La prueba de memoria del manifest mide una constante

- Plataforma: todas
- Severidad: P2
- Esperado: el contador de lectura del constructor registra los bytes que cada
  llamada real a `Read::read` devuelve, y la prueba distingue entradas pequeñas
  de grandes sin cargar el archivo completo
- Actual: corregido. `digest_of` registra el `count` devuelto por cada
  `Read::read`; `FileSource` registra los bytes realmente leídos y `FileSink`
  sólo registra una escritura después de que se haya completado con éxito.
  Las tres pruebas comparan tamaños distintos y exigen un pico estrictamente
  menor para la operación pequeña
- Resolución: la medición del builder se movió al bucle de lectura real y se
  aisló por hilo para que hashes paralelos no contaminen el pico. Se añadieron
  contratos equivalentes para source y sink; `read_to_end`, un contador
  constante y contar operaciones rechazadas rompen pruebas nominales
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: M2 original reproducida sobre `983ca71`; cierre en `f56435c`. Los
  barridos locales posteriores hicieron fallar
  `building_a_manifest_from_disk_does_not_load_the_file`,
  `file_source_peak_is_the_largest_completed_read_not_the_request` y
  `file_sink_peak_is_the_largest_successful_write_not_a_constant` al retirar o
  sustituir por constantes sus mediciones; todas las mutaciones se restauraron

## QYR-0075 — La política de recuperación congelada en ADR-0027 no se lee

- Plataforma: todas
- Severidad: P2
- Esperado: `FileSink` lee `.qyro-resume`, reanuda sólo el `transfer_id`
  coincidente truncando el parcial a `bytes_committed`, y elimina un parcial
  huérfano antes de empezar una transferencia nueva
- Actual: corregido. `FileSink::part_for` llama a `ResumeState::decode` cuando
  existe un parcial, reanuda sólo metadata coincidente, trunca a
  `bytes_committed` y elimina el parcial cuando ningún progreso lo describe
- Resolución: `an_interrupted_transfer_resumes_from_its_metadata` deja una cola
  no confirmada y exige que producción la trunque; la prueba de huérfanos
  comprueba por longitud el descarte tanto de 17 como de 8192 bytes. QYR-0101
  y la enmienda de ADR-0027 cubren el `transfer_id` discordante
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: M4 original encontró cero llamantes productivos y M3 falló por
  digest. Cierre en `4d7b6fd`: retirar la lectura productiva volvió a causar
  `DigestMismatch`; retirar `set_len` conservó 262243 bytes en vez de 131072;
  reutilizar el huérfano conservó 17 bytes en vez de 1. CI 31532723390 pasó

## QYR-0100 — El checker confunde límites de rangos reservados con hallazgos

- Plataforma: documentación; Bash y PowerShell
- Severidad: P2
- Esperado: `check_docs_consistency` exige una ficha para cada cita concreta de
  un hallazgo, pero acepta declaraciones de propiedad como
  `QYR-0076–QYR-0099` o `QYR-0100 en adelante` sin inventar fichas para los
  extremos
- Actual: el escaneo extrae cualquier texto con forma `QYR-NNNN`; el segundo
  prompt verbatim produjo tres bloqueos por 0076, 0099 y 0100 aunque sólo
  describían rangos reservados y 0076–0099 pertenecen a otro agente
- Resolución: ambos checkers eliminan del texto los rangos cerrados y los
  límites `onward`/`en adelante`/`+` antes de extraer citas. Las pruebas de
  contrato fijan que una reserva pasa y una cita concreta sin ficha sigue
  fallando
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: el contrato Bash falló antes del cambio en el fixture de rangos y
  pasó después. CI 31528281381 hizo fallar el contrato PowerShell por el caso
  vacío de `Get-Content -Raw`; tras convertir `$null` a texto vacío, Bash y
  PowerShell 7 pasaron en CI 31528757962

## QYR-0101 — Metadatos de otra transferencia no describen el parcial

- Plataforma: todas
- Severidad: P2
- Esperado: `FileSink` sólo reanuda un `.qyro-part` cuando el `transfer_id` de
  `.qyro-resume` coincide con el manifest actual; metadatos de otra transferencia
  convierten el parcial en huérfano y se descartan antes de escribir
- Actual: ADR-0027 §5 no definía la discordancia y producción no leía los
  metadatos. Un parcial de 8192 bytes acompañado por progreso de la transferencia
  99 se reutilizaba al iniciar la transferencia 42
- Resolución: la enmienda fechada de ADR-0027 define «lo describa» y
  `FileSink::committed_progress` exige el `transfer_id` actual antes de devolver
  `bytes_committed`; en caso contrario `part_for` abre con el guard de enlaces,
  elimina el parcial y crea uno nuevo
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: antes del cambio,
  `resume_metadata_for_another_transfer_makes_the_part_an_orphan` falló con
  longitud 8192 en vez de 1; al retirar después la comparación de
  `transfer_id`, falló con longitud 4096 en vez de 1

## QYR-0102 — QYR-0068 y `header.rs` negaban una API pública existente

- Plataforma: protocolo y documentación
- Severidad: P2
- Esperado: el ledger, el comentario de `FrameHeader::new` y la ADR vigente
  describen la superficie pública que un crate externo puede compilar y usar
- Actual: QYR-0068 y `header.rs` afirman que nada público puede rellenar los
  identificadores, pero `Frame::with_identifiers` y
  `FrameHeader::with_identifiers` son públicos desde `cc38554`; pruebas de
  integración y `qyro_crypto_smoke` ya los llaman con valores no cero
- Resolución: ADR-0029 congela la API real sin duplicarla; `header.rs` y
  `frame.rs` documentan cero, el sealer y el límite de autenticación. La
  evidencia genérica se convirtió en tres contratos nominales y QYR-0068 quedó
  corregida con el historial preservado
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: `git show 15934aa:rust/crates/qyro_protocol/src/header.rs` contiene
  `pub const fn with_identifiers`; `rg -n 'with_identifiers'` encuentra usos en
  tests externos y `rust/tools/qyro_crypto_smoke/src/lib.rs`. ADR `b4faf2e`,
  contratos `62c82b8` y CI verde 31534679436

## QYR-0103 — `FrameError::InvalidIdentifier` no tiene construcción posible

- Plataforma: protocolo
- Severidad: P3
- Esperado: cada variante pública de `FrameError` corresponde a un rechazo que
  el decoder o un constructor puede producir, o su ausencia está decidida y
  documentada sin prometer un control inexistente
- Actual: `InvalidIdentifier { field: IdentifierField }` sólo aparece en su
  declaración, `Display` y reexport. Ningún código lo construye; ADR-0029 decide
  además que framing acepta el rango entero, incluido cero
- Resolución: pendiente. No se reutiliza para routing: `qyro_protocol` no conoce
  transfers activos ni manifests. Fase 9 decidirá su compatibilidad al añadir
  la guarda de sitios de construcción que hoy falta en este crate
- Estado: abierto
- Fecha: 2026-08-11
- Evidencia: `rg -n 'InvalidIdentifier|IdentifierField' rust/crates/qyro_protocol`
  sólo devuelve `error.rs` y el reexport de `lib.rs`; ninguna rama productiva
  devuelve la variante

## QYR-0104 — El informe sumaba un test Linux inexistente desde Fase 3

- Plataforma: documentación y CI Linux
- Severidad: P3
- Esperado: §9 y §11 copian los totales que produce cada ejecución normal de
  `cargo test --workspace`, distinguidos por plataforma y obtenidos del log
- Actual: el informe decía 391/392/393 pruebas Linux tras Fases 3/4/5; los logs
  de los runs 31531722569, 31533293790 y 31535319037 contienen respectivamente
  390/391/392. Los totales Windows 396/397/398 sí eran correctos
- Resolución: se recontaron sólo las líneas `test result` del step exacto
  `Run cargo test --workspace`, excluyendo `--all-features` y doc tests. El
  informe corrige las tres cifras y fija Fase 6 en 398 Linux/404 Windows
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: extracción por API de los logs de CI; implementación de Fase 6 en
  31536398365: 38 resúmenes, 398 passed, 0 failed, 2 ignored

## QYR-0105 — El workspace no compilaba su superficie Windows en CI

- Plataforma: Windows y CI
- Severidad: P2
- Esperado: Clippy estricto y las pruebas normales del workspace compilan y
  corren tanto la superficie Linux como la superficie Windows en cada cambio
- Actual: el único job completo usaba `ubuntu-latest`. En Windows,
  `qyro_store_smoke::code::UNSUPPORTED_PLATFORM` quedaba sin usar y
  `cargo clippy --workspace --all-targets -- -D warnings` fallaba por
  `dead_code`; Linux no podía ver el defecto ni compilar el backend DPAPI
- Resolución: `UNSUPPORTED_PLATFORM` sólo se compila fuera de Windows, que es
  donde puede devolverse. CI añade un job `windows-latest` con el mismo Clippy
  estricto y `cargo test --workspace`. El coste es un runner adicional y una
  segunda suite; el beneficio es cubrir código productivo Windows y ocho tests
  funcionales `cfg(windows)` que el job Linux no puede ejecutar
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: local Windows Rust 1.88.0: Clippy PASS y 405 passed/2 ignored; las
  nueve pruebas de `qyro_win_dpapi` pasan, incluida la guarda de `unsafe` que
  también corre en Linux. Al retirar el `cfg`, Clippy falló nominalmente con
  `constant UNSUPPORTED_PLATFORM is never used`; restaurado después. CI
  31540971698 ejecutó Clippy y 405/2 en `windows-latest`; los ocho jobs pasaron

## QYR-0106 — STATUS presentaba un conteo Linux como universal

- Plataforma: documentación, Linux y Windows
- Severidad: P3
- Esperado: cada total de pruebas dice la plataforma y cualquier diferencia por
  `cfg` queda explicada por pruebas concretas
- Actual: `STATUS.md` declaraba 388 como «el» total, pero la misma base ejecutaba
  394 en Windows. La rama de Fase 7 ejecuta 399 en Linux y 405 en Windows
- Resolución: STATUS publica ambos conteos. El delta exacto de +6 en Windows es
  ocho tests funcionales DPAPI sólo Windows menos dos tests de symlinks sólo
  Unix; la novena prueba DPAPI, la guarda de bloques `unsafe`, corre en ambos y
  no contribuye al delta
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: comparación por nombre de `cargo test --workspace -- --list` en
  Windows con el step Linux de CI 31537833116; lista nominal en STATUS y en el
  informe 5C

## QYR-0107 — Los checkers de portabilidad no eran portables al host Windows

- Plataforma: Git Bash y Windows PowerShell
- Severidad: P2
- Esperado: ambos checkers y sus contratos terminan en el Windows incluido de
  fábrica sin exigir instalar PowerShell 7 ni lanzar un proceso por segmento
- Actual: los dos scripts PowerShell exigían 7.0 y Windows PowerShell 5.1 los
  rechazaba antes de ejecutarlos. El checker Bash hacía `printf | tr` por cada
  segmento versionado y agotó 120 s en Git Bash
- Resolución: los scripts PowerShell declaran 5.1 y el contrato invoca el mismo
  ejecutable que lo aloja; el Bash convierte mayúsculas con `${stem^^}`, sin
  subprocesos por segmento
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: antes, Windows PowerShell 5.1 produjo
  `ScriptRequiresUnmatchedPSVersion` y Bash siguió activo después de 120 s.
  Después: checker/contrato PowerShell 0.731 s/27.409 s y checker/contrato Bash
  0.860 s/19.262 s, todos con salida 0

## QYR-0108 — Las fixtures de portabilidad dependían de PowerShell 7 y Unix Git

- Plataforma: Git for Windows y Windows PowerShell 5.1
- Severidad: P3
- Esperado: el contrato llega a ejecutar el checker contra nombres hostiles
  mantenidos sólo en el índice, y la construcción de su ruta raíz funciona en
  PowerShell 5.1
- Actual: `Join-Path $PSScriptRoot '..' '..'` usa una forma no aceptada por 5.1;
  además Git for Windows rechazaba `NUL` antes de que el checker pudiera ser el
  componente bajo prueba
- Resolución: el contrato anida las dos llamadas `Join-Path`, reutiliza el
  ejecutable PowerShell actual y fija `core.protectNTFS=false` sólo dentro de
  cada repositorio temporal. El nombre hostil permanece únicamente en el índice
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: ambos contratos completos pasan en Windows; retirar cualquiera de
  las adaptaciones reproduce respectivamente el error de parámetros de
  `Join-Path` o el rechazo anticipado de Git for Windows

## QYR-0109 — El checker documental leía UTF-8 y Git de forma distinta en PowerShell 5.1

- Plataforma: Windows PowerShell 5.1
- Severidad: P2
- Esperado: el checker y su contrato aceptan el mismo repositorio y las mismas
  reservas de rango que PowerShell 7, incluidos UTF-8 y finales CRLF
- Actual: bajar el requisito de los scripts de portabilidad permitió ejecutar
  el checker documental en el host real. Éste leyó UTF-8 sin declarar encoding,
  no reconoció el en dash de `QYR-0076–QYR-0099` y exigió dos fichas ajenas.
  Después, `ErrorActionPreference=Stop` convirtió el stderr esperado de Git en
  excepción y los headings CRLF no satisficieron patrones anclados a `$`
- Resolución: todas las lecturas textuales declaran UTF-8; el regex de rangos
  usa escapes ASCII `\u2013`/`\u2014`; `Invoke-Git` captura salida y código sin
  promover estados no cero a excepciones; los headings aceptan LF y CRLF. El
  contrato usa el ejecutable PowerShell actual y fixtures UTF-8
- Estado: cerrado
- Fecha: 2026-08-11
- Evidencia: antes, el repositorio real falló con QYR-0076–QYR-0099 sin ficha.
  El contrato 5.1 reprodujo después el stderr nativo y los nueve headings CRLF;
  tras las correcciones, checker real y contrato completo terminan con salida 0
