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
- Actual: outputs existen solo en runners efímeros
- Evidencia: run 30938946789
- Workaround: volver a ejecutar builds
- Estado: abierto
- Dueño: release
- Fecha: 2026-08-04

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
