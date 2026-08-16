# Fase 05 — La interfaz, y los botones

**Base:** `3bdf5f6`. **Cerrada en:** `0aef8ea`. **Rama:** `claude/qyro-net-6a`.

---

## 1. Objetivo y alcance

> **El producto.** Que una persona abra Qyro, vea a quién le está mandando algo,
> elija qué, y lo vea llegar verificado del otro lado.

**No objetivos:** temas, avatares, ajustes, notificaciones, ejecución en segundo
plano, cola de transferencias. Lo mínimo para que una persona lo use.

---

## 2. Qué se hizo

1. **El FFI, primero.** Las condiciones 3, 4 y 5 estaban probadas en Rust y no
   eran alcanzables desde la aplicación, que es lo mismo que no tenerlas. La
   superficie C pasa de once símbolos a diecinueve (ADR-0032 enmienda 1).
2. **ADR-0036 congelada** antes de una línea de interfaz.
3. **Las cuatro pantallas**: peers, enviar, recibir, historial.
4. **Los botones encendidos**, y el texto que explicaba por qué estaban apagados
   **borrado**.
5. **Los dos idiomas** con prueba mecanizada de cobertura y de traducción.

---

## 3. Cómo se hizo

### El FFI: nada cruza un tipo

Nueve operaciones nuevas, todas con la misma forma: `i32` de retorno, valores por
out-parámetro, texto en un búfer que el llamante presta. El contrato del texto
vive en **una** función, `emit_text`, porque cinco operaciones lo comparten y
cinco copias son cinco sitios donde un off-by-one se vuelve media huella.

**Y cuando no cabe, no se escribe nada.** Un búfer a medio escribir devuelto
junto a un código de error es exactamente cómo se lee media huella y se compara
en voz alta, y media huella que coincide no prueba nada.

### `TrustBook` es del proceso

Un `Mutex` estático, no una segunda tabla de handles: **una aplicación tiene un
libro**, y una tabla para un objeto único sólo se puede usar mal.

### `with_session_entry`: la versión sin la regla pegajosa

Pedirle la huella del peer a una sesión que falló tiene que responder **la
huella**, no el código con el que murió. La pegajosidad de ADR-0032 §5 existe
para que nadie crea que una *transferencia* se recuperó; una huella no es una
transferencia, y ocultarla tras un fallo escondería justo el dato que una persona
necesita para entender qué pasó.

### Las pantallas hablan con una interfaz, no con el FFI

`QyroSession.stepBlocking` bloquea sin cota (ADR-0032 §7). Una pantalla que
llamara a la frontera directamente congelaría el fotograma, y una pantalla que
sólo se pudiera probar con un socket vivo no se probaría. Así que:

- `QyroTransferService` es lo único que las pantallas ven;
- `NativeTransferService` corre la sesión entera dentro de `Isolate.run` y sólo
  devuelve enteros y texto — **nada que posea un puntero cruza**, porque una
  dirección de una isolate no significa nada en otra;
- las pruebas de widget usan un doble y alcanzan **todos** los estados feos.

---

## 4. Qué se encontró que no estaba en el plan

| # | Hallazgo | Gravedad | Cómo se descubrió |
|---|---|---|---|
| 1 | La guarda `qyro_session_re_exports_nothing_it_does_not_own` **enumeraba a mano** los módulos de la fachada, así que añadir uno la ponía roja por republicar algo propio | P2, arreglado | Al añadir `mod trust;` |
| 2 | El suelo de `every_extern_c_function_sits_behind_the_panic_guard` seguía en 8 con una superficie de 19 | P2, arreglado | Al añadir los símbolos |
| 3 | Una meta-guarda exige comprobación de sitio de construcción para **toda** enum de error nueva | informativo | `cargo test --workspace` en rojo al añadir `PairingError` |
| 4 | `TextField` y `Card` necesitan un `Material` ancestro: un `home:` pelado en una prueba de widget no lo tiene | trivial | Las pruebas de pantalla fallaron con `No Material widget found` |
| 5 | El analizador de Dart **rechaza un U+202E crudo en un literal** — por el mismo motivo por el que existe la prueba que lo usa | informativo | `flutter analyze` tras escribir la prueba del nombre hostil |

**El hallazgo 1 no es un obstáculo: es la guarda mal escrita, no la regla.** Se
arregló **derivando** los módulos del `lib.rs` de la fachada en vez de listarlos
—lo mismo que ya hace `gated_files()` con los `#[cfg(test)]`, y por el mismo
motivo—, y su contra-prueba
`a_foreign_re_export_from_qyro_session_would_be_visible_to_guard_two` sigue en
verde: la versión derivada sigue cazando lo ajeno. De hecho **me rechazó un
`pub use qyro_transfer::RejectReason`** mientras lo escribía, que es su trabajo.

**El hallazgo 5 merece decirse entero.** El analizador se niega a que el fuente
se renderice distinto de como lo lee el compilador; es exactamente el ataque que
`safeDisplayName` cierra, un piso más abajo. Los controles del test van como
escapes y la prueba sigue ejerciendo el carácter real.

---

## 5. Qué se arregló y qué no

Todo lo encontrado se arregló en el mismo tramo. **Ninguna ficha nueva.**

---

## 6. A qué afectaba cada defecto

- **Hallazgo 1.** A cualquier fase futura que añada un módulo a la fachada: la
  habría parado con un mensaje que dice lo contrario de lo que pasa.
- **Hallazgo 2.** **Escenario concreto:** con el suelo en 8, un cambio que
  rompiera el análisis y dejara al lector viendo tres funciones de diecinueve
  habría seguido en verde. Es la forma de QYR-0071 otra vez.
- **Hallazgos 3, 4 y 5:** ninguno afectaba a comportamiento.

---

## 7. Resultado contra el objetivo — **CUMPLIDO**

### Los cinco requisitos de los botones, con su evidencia

| # | Condición | Estado | Evidencia |
|---|---|---|---|
| 1 | Dart conduce una transferencia verificada | **CUMPLIDA** | `a_file_crosses_two_processes_driven_from_dart` — 8 MiB + 13 B entre dos procesos, byte a byte |
| 2 | La persona elige el archivo con el selector de su sistema | **CUMPLIDA en código** | `a_file_chosen_through_the_picker_transfers_and_verifies`, más las doce de `qyro_file_picker_test.dart`. **Clase de evidencia: probado en unidad y entre procesos aguas abajo del diálogo; el diálogo no se ha abierto en esta máquina** (QYR-0324) |
| 3 | Hay un camino para encontrar al otro extremo | **CUMPLIDA** | La cadena `QYRO1\|addr\|32hex` con 7 pruebas de contrato, `qyro_pairing_parse` por el FFI, y `a_pairing_string_round_trips_through_the_ffi` desde Dart |
| 4 | La huella se ve y una clave cambiada se rechaza | **CUMPLIDA** | `a_known_peer_whose_key_changed_is_refused_by_name` **desde Dart**, con dos procesos receptores y por tanto dos identidades bajo un nombre. Comprueba que el veredicto es `changed` y **no** se ablanda en `newPeer` |
| 5 | El receptor puede rechazar | **CUMPLIDA** | `a_receiver_that_refuses_stops_the_sender_and_leaves_nothing_behind`: el emisor termina en `Rejected`, **aprende el motivo exacto**, y el destino queda vacío comprobado listando el directorio |

**Los botones están encendidos.** El texto que explicaba por qué estaban apagados
está borrado del catálogo, de la pantalla y de las dos pruebas que lo esperaban.

### Los criterios de `FASE-05`

| Criterio | Veredicto |
|---|---|
| ADR de la UI congelada antes de dibujar | **Cumplido** — ADR-0036, commit propio |
| Pantallas de peers, enviar, recibir, historial | **Cumplido** |
| Entrada manual y QR siempre visibles | **Cumplido** — en la primera pantalla, sin «avanzado» |
| Peer con clave cambiada visualmente alarmante | **Cumplido** — y sin acción de aceptar |
| Nombre hostil no reordena la línea | **Cumplido** — `safeDisplayName` |
| Dos idiomas con prueba | **Cumplido** — 46 pares comparados, ninguno idéntico |
| Botones encendidos | **Cumplido** |
| QYR-0089 y QYR-0088 cerradas | **Cumplido** |

---

## 8. Clase de evidencia por afirmación

| Afirmación | Clase | Plataforma |
|---|---|---|
| Dart consulta confianza, huella, rechazo y cadena por el FFI | **Probado entre procesos** | Windows 10 |
| Una clave cambiada se refuta por nombre desde Dart | **Probado entre procesos** | Windows 10 |
| El receptor rechaza, el emisor sabe por qué, el destino queda vacío | **Probado en integración** | Windows 10 |
| Cada estado feo tiene su pantalla y su frase | **Probado en widget** | Windows 10, VM de Dart |
| Un peer con clave cambiada se ve distinto | **Probado en widget** | Windows 10 |
| Un nombre hostil no reordena la línea | **Probado en unidad** | Windows 10 |
| Las cadenas están en los dos idiomas | **Probado en unidad** | Windows 10 |
| Los botones llaman a algo | **Probado en widget** | Windows 10 |
| **La aplicación completa se ve y se toca** | **Ninguna** | `flutter build` no corre aquí (QYR-0324). El protocolo de hardware de la fase 07 es lo que la cubre |

---

## 9. La puerta — 2026-08-14, sobre `0aef8ea`

| # | Comprobación | Veredicto |
|---|---|---|
| 1 | `cargo fmt --all --check` | **exit 0** |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** |
| 3 | `cargo test --workspace` | **exit 0** — 623 passed, 0 failed, 2 ignored |
| 4 | Barrido de mutación | §10 |
| 5 | Lectura de aserciones | **Cumplido** — §9.1 |
| 6 | Lectura de contadores | **No aplica**: sin contadores nuevos |
| 7 | La medida se ve fallar | **Cumplido** — §9.2 |
| 8 | Lectura de nombres | **Cumplido** |
| 9 | Coherencia del informe | **Cumplido** |
| 10 | El ledger sigue legible | **147 fichas, 37 abiertas** — esta fase no añadió ninguna |
| 11 | `check_docs_consistency` en los dos shells | **exit 0** |
| 12 | Escribir el resultado | este documento |
| 13 | El código tras `cfg` compila | `cargo clippy -p qyro_ffi --all-targets --target aarch64-linux-android` — **exit 0** |

**Y las tres de Dart:** `dart format --set-exit-if-changed .` desde `apps/qyro`,
`flutter analyze` y `flutter test` — **exit 0** las tres, 94 pruebas.

### 9.1 — Lectura de aserciones

Las que merecen decirse:

- `expect(alarming.color, isNot(calm.color))` — **las dos tarjetas comparadas
  entre sí**. Afirmar sólo que la alarmante tiene color pasaría si ambas lo
  tuvieran.
- `expect(seen.length, cases.length)` en las seis frases de fallo — seis clases
  compartiendo una frase satisfacen todas las aserciones anteriores.
- `expect(untranslated, isEmpty)` — una cadena española idéntica a la inglesa es
  una clave que nadie tradujo, y es indistinguible de una traducida si sólo se
  comprueba que existe.
- `expect(trust.peerFingerprint(second), isNot(firstFingerprint))` — dos
  receptores distintos, no la misma llamada dos veces.

### 9.2 — La medida se ve fallar

| Medición | Prueba que la ve fallar |
|---|---|
| «el búfer no se escribe cuando no cabe» | El búfer se rellena con `0xAA` y se exige que **siga** siendo `0xAA` tras el rechazo; y con capacidad exacta **sí** entra, así que el rechazo es por el tamaño |
| «un nombre hostil se limpia» | Se afirma que la entrada **sí** contiene el override, así que un `safeDisplayName` que devolviera su argumento fallaría |
| «un código de emparejamiento inválido se rechaza» | El control positivo en la misma prueba: un código válido resuelve y borra el error |
| «los dos catálogos están completos» | Se exige comparar más de 40 pares, así que una lista que se quedara corta falla en vez de pasar vacía |

---

## 10. Tabla de mutación

| Control | Mutación | Resultado | Test que falló |
|---|---|---|---|
| `emit_text` no escribe cuando no cabe | Escribir los bytes que quepan antes de devolver el error | **Muerto** | `asking_with_no_room_reports_the_length_and_writes_nothing` |
| El suelo del guardián de pánico | Dejarlo en 8 | **Superviviente por diseño** — por eso subió a 19 con el argumento escrito |
| `safeDisplayName` quita el override | Devolver el argumento | **Muerto** | `bidirectional overrides and controls are dropped` |
| Aceptar visible con clave cambiada | Quitar el `if (!alarming)` | **Muerto** | `an offer from a changed key offers no way to accept it` |

**Alcance del barrido automático:** `cargo-mutants` no se corrió en esta fase.
El código nuevo de Rust son nueve funciones `extern "C"`, y `cargo-mutants`
**no puede mutarlas de forma observable**: un cuerpo sustituido por
`Ok(Default::default())` en una `extern "C" fn` que devuelve `i32` produce `0`,
que es `QYRO_OK`, y el contrato de esas funciones se comprueba desde Dart, en
otro proceso, con la biblioteca ya compilada. Las cuatro mutaciones de arriba se
aplicaron **a mano** por eso, y se dice aquí en vez de presentar un barrido que
habría medido menos de lo que aparenta.

---

## 11. Tests antes y después

| Suite | Antes (`3bdf5f6`) | Después (`0aef8ea`) |
|---|---|---|
| Rust, Windows | 617 / 2 ignored | **623 / 2 ignored** |
| Dart, Windows | 79 | **94** |

---

## 12. Delta de dependencias

**Ninguno.** `Cargo.lock` sigue en **64** paquetes; `pubspec.lock` sigue en 45.
No entró ni un paquete de Rust ni uno de Dart en toda la fase.

---

## 13. Archivos tocados

```
git diff --name-only 3bdf5f6..0aef8ea
```

Diecisiete archivos: la ADR y su enmienda, los cuatro de la superficie C, los dos
de la fachada, el módulo de confianza de Dart, los tres de la interfaz, los dos
catálogos y las cuatro suites de prueba.

---

## 14. Runs de CI

Se listan en el informe de cierre de la v1.0, sobre el commit final de la rama,
para que la tabla sea la de un árbol y no la de un tramo.

---

## 15. Qué NO debe leerse como progreso

- **Nadie ha visto esta interfaz.** `flutter build` no corre en esta máquina
  (QYR-0324) y no hay emulador. Todo lo que esta fase prueba es lógica de
  widget en la VM de Dart, no píxeles en una pantalla.
- **No hay descubrimiento automático.** La 04b va después, por ADR-0035
  enmienda 1. El camino que existe es la cadena, escrita o escaneada.
- **El QR no se lee con una cámara.** Existe la cadena que un QR codificaría, y
  el campo donde se pega. Leerla con la cámara es hardware.
- **La confianza no sobrevive al cierre de la aplicación.** El libro vive en
  memoria hasta la fase 06.
- **El historial se muestra vacío**, y es verdad: `qyro_fs::history` lo graba y
  ningún símbolo C lo lee todavía. Una lista vacía es una afirmación cierta sobre
  lo que esta build puede enseñar; una inventada no lo sería.
- **Nada se ha probado en hardware físico**, y **dos procesos en `127.0.0.1` no
  son dos aparatos en una Wi-Fi.**

---

## 16. Ledger y handoff

**Ninguna ficha nueva.** El ledger sigue en **147 fichas, 37 abiertas**.

**Qué necesita saber lo que viene:** la 04b tiene su trampa escrita en ADR-0035
enmienda 1 (`MulticastLock`), la 06 es lo que hace que la confianza y la
identidad sobrevivan a un reinicio, y la 07 es lo único que esta sesión no puede
hacer — necesita dos aparatos y una persona.
