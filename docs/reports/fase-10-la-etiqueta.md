# Fase 10 — La etiqueta, y las tres promesas rotas que encontró

**Base:** `63f4ca2`. **Rama:** `claude/qyro-net-6a`.

---

## 1. Objetivo y alcance

> **Que la documentación diga lo que el código hace, y etiquetar.** ADR superadas
> marcadas, `THREAT_MODEL.md` reescrito contra lo que existe, `STATUS.md` sin
> mentiras, artefactos con su SHA-256, y `v1.0.0`.

**No objetivo:** función nueva. Esta fase no debía escribir código.

**Escribió código en tres sitios**, y los tres son la misma clase de hallazgo:
comparar un documento con el código encuentra defectos **en el código**, porque
el documento fue una decisión que alguien tomó y nadie ejecutó.

---

## 2. Qué se hizo

1. **Siete ADR marcadas** con una enmienda fechada cada una, sin reescribir el
   original.
2. **`THREAT_MODEL.md` reescrito entero**, con las amenazas sin control en su
   propia sección.
3. **`STATUS.md` corregido en once entradas** que afirmaban lo contrario del
   estado real.
4. **Tres defectos encontrados y arreglados**: QYR-0348, QYR-0349, QYR-0350.
5. **`release.yml`**: los artefactos de la v1.0, con SHA256SUMS dentro del
   paquete.
6. **La regla de deriva de capacidades sustituida**, porque había expirado.
7. **`docs/release/v1.0.md`**, `ESTADO-ACTUAL.md`, y la etiqueta.

---

## 3. Cómo se hizo

### Las ADR no se reescriben, se enmiendan

Cada una lleva un bloque `## Enmienda de la fase 10 — 2026-08-16` con la misma
estructura: **qué decía**, **qué existe**, **por qué cambió** y **qué se pierde**.
El original queda intacto. Una ADR corregida en silencio deja de ser un registro
de decisiones y pasa a ser una descripción del presente, y para eso ya está
`docs/release/v1.0.md`.

| ADR | Destino |
|---|---|
| 0004 Seguridad de red | **Superada** por ADR-0021 + ADR-0022. No hay TLS |
| 0005 RaptorQ óptico | **No implementada.** No hay canal óptico ni cámara |
| 0006 SQLite local | **Superada** por los tres formatos propios de `qyro_fs` |
| 0009 Bluetooth | **No implementada.** El código tecleado resuelve lo que Bluetooth resolvía, sin permisos |
| 0010 Empaquetado | **Superada** por el empaquetado real: APK y ZIP; ni AAB, ni MSIX, ni IPA, ni SBOM |
| 0025 Keystore Android | **Superada en el mecanismo** por ADR-0037. Sus cuatro sub-decisiones siguen rigiendo |
| 0031 Confianza | **Cumplida**, con la tabla de dónde aterrizó cada línea de su política |

### El modelo de amenazas: dos tablas, no una

La versión anterior mezclaba controles implementados con controles previstos en
una sola tabla. Eso se lee como una lista de garantías, y **tres de sus filas no
lo eran**:

- «MITM/replay en transporte — **TLS 1.3**». No hay TLS. La fila llevaba «no
  implementado» al lado, que es mejor que mentir y peor que quitarla: dejaba en
  la tabla de controles algo que no era un control.
- «Discovery rastreable — **alias y session ID rotatorios**». Falso. El
  descubrimiento anuncia la huella pública y es **estable**.
- «Memoria/disco agotados — límites, streaming, **cuota y preflight**». Hay lo
  primero; **no hay cuota ni preflight**.

Ahora §4 sólo admite filas que nombren dónde está el control, y §5 es la lista de
lo que este producto **no** defiende. Una fila que no puede nombrar su archivo no
pasa de §5 a §4.

### La regla que había expirado

`check_docs_consistency` prohibía a cuatro documentos decir «file transfer:
implemented». Fue correcta durante seis sprints, porque la transferencia no
existía. Ahora existe, y **una regla vencida que sigue bloqueando es peor que no
tener regla**: impide decir la verdad.

Se sustituye por la que sí sigue viva, y no es cuestión de opinión: **ningún
documento puede afirmar evidencia de hardware mientras
`docs/testing/hardware-protocol.md` tenga un hueco sin marcar.** En Bash y en
PowerShell, con su control en los dos sentidos.

---

## 4. Qué se encontró que no estaba en el plan

### QYR-0348, P1 — un botón con icono de escáner que no escanea

El botón de la pantalla de peers llevaba `Icons.qr_code_scanner` y la etiqueta
«Escanear un código», y lo que hacía era leer el campo de texto de encima. **No
hay cámara**: ni paquete en `pubspec.yaml`, ni permiso, ni decodificador. Y el
texto de ayuda decía «o escanea el del otro aparato».

Es el defecto que la fase 05 quitó del Home —un control que afirma algo que el
producto no hace— entrado por la puerta de al lado. Y estaba **también en el
documento de release que esta misma fase estaba escribiendo**, que es cómo se
encontró.

La guarda que lo impide tiene tres pruebas, y la segunda es la que la sostiene:
no comprueba sólo que el icono no esté, sino que **no hay ningún paquete en el
grafo que pudiera dar una cámara**. Un icono se renombra; una cámara no se cuela
sin dependencia.

### QYR-0349, P1 — una decisión de ADR que nadie escribió

ADR-0025 §3.4 decidió `android:allowBackup=false`. **El atributo no estaba en el
manifiesto**, así que regía el `true` por defecto de Android: Auto Backup habría
copiado el blob de identidad envuelto a Google Drive. Una aplicación cuya primera
promesa es «sin nube» subiendo parte de sí misma a una nube, por omisión.

No es una catástrofe —el blob va envuelto por una clave de Keystore que no sale
del aparato, así que una copia restaurada es inservible— y eso es exactamente por
qué era fácil de no ver. La promesa es «nada sale de esta red»; «sale pero no
sirve» es otra promesa.

Faltaba además `dataExtractionRules`: en API 31+ `allowBackup` dejó de gobernar
la transferencia aparato-a-aparato.

### QYR-0350, P1 — el job de Android llevaba dos commits en rojo

`:app:mergeDebugAndroidTestAssets` moría con «Cannot find a version of
`androidx.test:runner` that satisfies the version constraints», así que el test
instrumentado de Keystore —la evidencia entera de la fase 06— **no se había
ejecutado nunca**.

La causa no era un artefacto que faltara. `integration_test` de Flutter declara
`api("androidx.test:runner:1.2+")`, eso entra en `debugRuntimeClasspath` y
resuelve a 1.3.0, y la **resolución consistente** de AGP lo reimpone como
`strictly 1.3.0` sobre el classpath de androidTest: un `1.6.2` escrito a mano no
puede satisfacer un `strictly`.

**Lo que enseña es más importante que el arreglo.** La fase 06 dio el test por
hecho porque el código estaba escrito y `flutter test` pasaba. El único sitio
donde ese test puede correr es CI, CI estaba en rojo, y la fase se cerró sin
mirar. **Un job rojo es una afirmación sin evidencia, aunque el código sea
correcto** — y aquí lo era.

### La guarda nueva falló en su primera ejecución, contra su propio comentario

`promised_capabilities_test.dart` busca `Icons.qr_code_scanner` en `lib/`, y lo
primero que encontró fue el comentario que explica por qué el icono ya no está.
Es **QYR-0328 en el otro lenguaje**: una comprobación que no distingue una
mención de un uso. Se arregló igual —`codeOnly` recorre la fuente saltando
comentarios y literales— y lleva su control en los dos sentidos.

Van tres veces que este proyecto tropieza con lo mismo. Está escrito como trampa
número 5 en `ESTADO-ACTUAL.md`.

---

## 5. Qué se arregló y qué no

**Arreglado:** los tres P1, más la regla vencida y su falso positivo.

**No arreglado, y dicho:** todo lo que está en `THREAT_MODEL.md` §5. La lista es
más larga que antes de esta fase, y eso es el resultado: no aparecieron amenazas
nuevas, aparecieron filas que se hacían pasar por controles.

---

## 6. A qué afectaba cada defecto

| Ficha | A qué afectaba | Quién lo habría notado |
|---|---|---|
| QYR-0348 | Una persona apunta la cámara a un código y no pasa nada | La primera persona que lo usara |
| QYR-0349 | El blob de identidad sube a Google Drive | Nadie, nunca, porque no rompe nada |
| QYR-0350 | La evidencia entera de la fase 06 | Sólo quien mirara CI |

Los tres son **invisibles desde dentro**: el código compila, las pruebas locales
pasan y la aplicación arranca. Lo que los encontró fue comparar dos documentos
que se contradecían.

---

## 7. Resultado contra el objetivo — **CUMPLIDO**

Siete ADR marcadas, el modelo de amenazas reescrito contra el código, `STATUS.md`
corregido en once entradas, artefactos con su SHA-256 en `docs/release/v1.0.md`,
y `v1.0.0` etiquetada.

---

## 8. Clase de evidencia por afirmación

| Afirmación | Clase |
|---|---|
| Las ADR marcadas describen lo que existe | **Verificado contra el código**: `raptorq`, `sqlite`, `bluetooth` y `tls` no aparecen en el árbol |
| No hay cámara | **Probado**, `promised_capabilities_test.dart`, con control |
| Nada se respalda ni se transfiere | **Probado en unidad** sobre el manifiesto y el archivo de reglas. **No observado** en un teléfono restaurando un backup |
| El test instrumentado de Keystore corre | **En CI, en emulador.** Un emulador no es un TEE de verdad |
| Los artefactos existen y sus hashes son ésos | **Ejecutado en CI**, hashes copiados del run |
| El APK firmado con la clave release | **Ejecutado localmente**, `apksigner verify --print-certs` |
| Algo de esto funciona en un teléfono | **Ninguna.** Veinte huecos en blanco |

---

## 9. La puerta — 2026-08-16

| # | Comprobación | Exit |
|---|---|---|
| 1 | `cargo test --workspace` | 0 — **633 pasados, 0 fallados, 2 ignorados** |
| 2 | `cargo fmt --all --check` | 0 |
| 3 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| 4 | `cargo clippy -p qyro_session -p qyro_ffi --target aarch64-linux-android -- -D warnings` | 0 |
| 5 | `cargo audit --deny warnings` | 0, sobre 80 paquetes |
| 6 | `flutter analyze` | 0 |
| 7 | `flutter test` | 0 — **90 pasadas, 10 saltadas** |
| 8 | `dart format --set-exit-if-changed .` | 0 |
| 9–12 | `check_docs_consistency` en Bash y en PowerShell | 0 y 0 |
| 13 | CI en verde sobre el commit etiquetado | ver §14 |

La comprobación 9–12 se vio **fallar** dos veces antes de pasar, las dos por
razones correctas: el SHA de siete caracteres y el falso positivo de la regla de
hardware.

---

## 10. Tabla de mutación

**Ninguna sobre Rust.** Esta fase no toca lógica de Rust: cambia un comentario y
documentación.

La guarda nueva de Dart lleva su equivalente: `codeOnly` tiene un control que
comprueba que **distingue un uso de una mención** en los dos sentidos, y un
tercer caso que comprueba que un `//` dentro de un literal no se traga la línea.
Un test que sólo verificara el estado actual pasaría igual con la función
devolviendo su entrada tal cual.

---

## 11. Tests antes y después

| | Antes | Después |
|---|---|---|
| Rust | 633 pasados, 2 ignorados | 633 pasados, 2 ignorados |
| Dart | 86 pasadas, 10 saltadas | **90 pasadas**, 10 saltadas |

Las cuatro nuevas de Dart: tres de `promised_capabilities_test.dart` y una de
`android_manifest_test.dart`.

---

## 12. Delta de dependencias

Ninguna en Rust: `Cargo.lock` sigue en **80**. En Android, las de androidTest
**pierden su versión** —la provee la restricción de resolución consistente— y
`androidx.test.ext:junit` baja de 1.2.1 a 1.1.2, que es la que empareja con
runner 1.3.0. Ninguna dependencia nueva.

---

## 13. Archivos tocados

| Archivo | Qué |
|---|---|
| `THREAT_MODEL.md` | Reescrito entero |
| `STATUS.md` | Cabecera nueva, once entradas corregidas, bloqueadores y siguiente tarea |
| 7 × `docs/adr/ADR-00xx` | Enmienda de la fase 10 |
| `docs/release/v1.0.md` | Completado con los hashes |
| `apps/qyro/lib/transfer/transfer_screens.dart` + 2 catálogos | QYR-0348 |
| `apps/qyro/test/promised_capabilities_test.dart` | **Nuevo** |
| `AndroidManifest.xml` + `res/xml/data_extraction_rules.xml` | QYR-0349 |
| `apps/qyro/test/android_manifest_test.dart` | La prueba de backup |
| `apps/qyro/android/app/build.gradle.kts` | QYR-0350 |
| `.github/workflows/release.yml` | **Nuevo** |
| `.github/workflows/platform-builds.yml` | El `BUILD-INFO.txt` que mentía |
| `scripts/check_docs_consistency.{sh,ps1}` | La regla vencida, sustituida |
| `docs/reports/ESTADO-ACTUAL.md` | Reescrito |

---

## 14. Runs de CI

En `STATUS.md`, sección «Fase 10 — runs de cierre», con su commit y su
conclusión. La etiqueta `v1.0.0` apunta a un commit cuyos tres workflows
terminaron en **success**, y `release.yml` corre sobre la etiqueta.

---

## 15. Qué NO debe leerse como progreso

**Etiquetar no prueba nada.** `v1.0.0` es un nombre para un commit. Lo que ese
commit contiene está probado en unidad, en integración, entre procesos y en CI, y
**no está probado en un teléfono**.

**Corregir un documento no arregla el software que describía mal.** De los tres
defectos de esta fase, dos existían desde hacía fases y uno hacía que la
evidencia de la fase 06 no existiera. Lo que esta fase demuestra es que **el
proyecto no sabía tres cosas sobre sí mismo**, y eso es una afirmación sobre las
nueve fases anteriores, no sólo sobre ésta.

**Un modelo de amenazas más honesto es un modelo de amenazas con más agujeros
escritos.** §5 es más larga que antes. Nadie añadió amenazas: se movieron desde
la tabla donde parecían resueltas.

**El APK no lo ha instalado nadie.** Existe, tiene su hash y está firmado con una
clave real. Ningún Android lo ha ejecutado nunca.

---

## 16. Ledger y handoff

- `BUGS_PENDING.md`: **150 fichas, 0 abiertas.** Tres nuevas en esta fase —
  QYR-0348, QYR-0349, QYR-0350—, las tres cerradas con su arreglo.
- IDs siguientes desde **QYR-0351**.
- `docs/reports/ESTADO-ACTUAL.md` reescrito, 117 líneas.
- Siguiente, y única: **la fase 07**, en `docs/testing/hardware-protocol.md`.
