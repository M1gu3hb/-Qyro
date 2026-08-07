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

## QYR-0036 — `indexing_slicing` no denegado en `qyro_protocol` ni `qyro_manifest`

- Plataforma: todas
- Severidad: P2
- Esperado: ninguna ruta que analice bytes de un peer puede entrar en pánico
- Actual: el sprint 4C.2 denegó la familia de pánico y `clippy::indexing_slicing`
  en `qyro_crypto`. `qyro_protocol` y `qyro_manifest` también analizan bytes
  elegidos por un peer —son, de hecho, la primera superficie que los toca— y no
  tienen ninguna de las dos denegaciones ni una guarda estructural equivalente.
  No se ha encontrado ningún pánico concreto; lo que falta es el control que
  impediría el próximo
- Workaround: ninguno; los decoders están acotados y con pruebas de corpus
- Estado: abierto
- Dueño: el sprint que toque esos dos crates
- Fecha: 2026-08-07

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

## QYR-0036 — Sin denegación de pánico ni de indexado en los dos crates de parsing

- Plataforma: todas
- Severidad: P2
- Esperado: ninguna ruta que analice bytes de un peer puede terminar el proceso
- Actual: el sprint 4C.2 denegó la familia de pánico y `clippy::indexing_slicing`
  en `qyro_crypto` y no en `qyro_protocol` ni en `qyro_manifest`, que son la
  primera superficie que toca esos bytes
- Resolución: denegación en los dos, más la guarda estructural. Aparecieron
  **33 infracciones en `qyro_protocol`** (29 en `header.rs`, 3 en `envelope.rs`,
  1 en `frame.rs`) y **22 en `qyro_manifest`** (18 en `model.rs`, 2 en
  `codec.rs`, 2 en `path.rs`). Ninguna se silenció con `allow` salvo los módulos
  de prueba dentro de cada crate. La guarda encontró además un
  `debug_assert_eq!` en `codec.rs` que ningún lint de Clippy cubre, duplicando
  un test que ya corre en todos los perfiles
- Estado: resuelto
- Evidencia: reintroducir `.expect(` en cualquier archivo de producción de
  cualquiera de los dos hace fallar su guarda por nombre
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

## QYR-0039 — Hallazgo sin contenido registrado

- Plataforma: desconocida
- Severidad: desconocida
- Actual: los prompts de los sprints 4C.2 y 4C.3 citan `QYR-0039` como no
  objetivo declarado, y **su contenido no está en ninguna parte de este
  repositorio**. No se sabe qué describe. Esta entrada existe para que el
  identificador deje de ser una referencia colgante, no porque el hallazgo se
  entienda
- Workaround: ninguno posible sin saber qué es
- Estado: abierto, **sin descripción**
- Dueño: quien tenga la auditoría externa original
- Acción concreta: copiar aquí el enunciado del hallazgo. Hasta entonces no se
  puede ni cerrar ni evaluar
- Fecha: 2026-08-07

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
