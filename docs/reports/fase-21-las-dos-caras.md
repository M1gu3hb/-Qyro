# Fase 21 — las dos caras se hablan

**Rama** `claude/qyro-cerrar-cadena-12` · **Commit del informe** `52fa4d5` ·
**2026-08-18**

**Puerta ejecutada en `52fa4d5`, el commit que este informe nombra**
(comprobación 16).

---

## 1. Lo que encontró, que es más de lo que iba a construir

**Ninguna de las dos caras había enviado nunca un archivo.** Las dos estaban
rotas, cada una a su manera, y las dos se encontraron la misma tarde por la misma
razón: esta fase pone una cara contra la otra.

| Ficha | Qué | Desde |
|---|---|---|
| **QYR-0361** (P0) | `qyro send` pasaba a `open_sender` el nombre pelado con `root` = el padre; `strip_prefix` falla siempre y **todo envío devolvía `BadArgument`** | fase 13 |
| **QYR-0362** (P0) | `NativeTransferService.send` capturaba un `ReceivePort` en el closure de `Isolate.run`; **todo envío moría con «object is unsendable»** antes de mover un byte | desde que se escribió |
| **QYR-0363** | `_commonRoot` partía sólo por `\`, y en Windows `C:\salida/p.bin` es válido: el archivo **aterrizaba un directorio más abajo del que nadie nombró**, diciendo que había ido bien | desde que se escribió |

Los tres son la misma forma: **dos mitades probadas y el medio jamás recorrido.**
Es la sexta, séptima y octava vez en este proyecto. `open_sender` tiene sus
pruebas con argumentos correctos; las del CLI nunca llegaban a un socket; las
pantallas se prueban contra un servicio falso; y la prueba de dos procesos
ejercita **recibir**. Cada pieza, verde. La cadena, rota.

**Y hay una Release publicada con los dos P0.** Se retracta y se republica, como
con `2c01de0`. Es lo primero de la siguiente sesión y está en `ESTADO-ACTUAL.md`:
tocar una Release publicada no se hace con el contexto justo.

---

## 2. La prueba que cierra la fase — las cuatro casillas, ejecutadas

`apps/qyro/test/transfer/gui_cli_matrix_test.dart`, con
`QYRO_FFI_LIBRARY_PATH` y `QYRO_CLI_PATH`.

| | recibe GUI | recibe CLI |
|---|---|---|
| **manda GUI** | **PASA** — dos sesiones, un motor, un socket real | **PASA** — la escena de `R7` §2, entera |
| **manda CLI** | **PASA** | **PASA** — dos copias del binario, huellas distintas |

Cada casilla: un archivo cruza y se compara **byte a byte** en destino. Un
archivo del tamaño correcto no es el mismo archivo, y este taller tiene QYR-0359
escrito sobre exactamente esa distinción.

**El otro extremo es el binario `qyro` de verdad, nunca un arnés.** El arnés es
lo que escondió el defecto de la identidad cinco fases seguidas; una prueba cuyo
otro extremo es un fixture prueba el fixture.

**Y son dos aparatos, no uno.** La identidad vive junto al ejecutable (ADR-0042),
así que las casillas de CLI copian el binario a dos directorios y **aseveran que
las huellas difieren** antes de nada. Sin eso sería un aparato hablando consigo
mismo, que pasa y no prueba nada.

**Los dos controles:**

1. **Nadie escuchando** → falla, con un mensaje que nombra la conexión, y **no se
   cuelga**.
2. **Huella que no coincide** → `REFUSED`, y **ningún byte aterriza**. Es la
   garantía de seguridad del producto y no vale sólo en una cara.

**Y una tercera cosa que enseñó la suite entera y el archivo solo no:** las
cuatro casillas comparten el puerto fijo de ADR-0041, así que no pueden
solaparse. En verde por separado y en rojo al lado de otra prueba es la forma
exacta de un test que se rompe el día que alguien añade el siguiente.

---

## 3. Comprobación 14, aplicada **por cara**

La tabla completa está en `docs/PARIDAD-GUI-CLI.md` y **la comprueba
`scripts/check_parity.ps1` por código de salida**. Doce capacidades; cada celda
es una referencia `ruta:línea` o un `NO -- <argumento>`.

**Vista fallar tres veces antes de creerla:** celda vacía, referencia a un archivo
borrado, y fila desaparecida. **La tercera no fallaba** — el piso era 10 sobre una
tabla de 12, así que borrar una fila pasaba en verde. Un piso deja desaparecer
capacidades de una en una, que es justo cómo se pierde una. Ahora es el número
exacto, y añadir una obliga a tocarlo a propósito.

**Cinco celdas dicen `NO` con argumento** — cancelar a mitad, peers recordados,
óptico y serie en la GUI. Son decisiones de producto.
**Ninguna dice «todavía»**: la única que lo decía era el consejero de canal, y se
cerró llenándola.

---

## 4. Comprobación 15 — del gesto al byte, y las dos caras dicen lo mismo

1. 👤 La persona tiene un archivo y otra máquina.
2. Pregunta cómo mandarlo: `qyro how` en la terminal, o la GUI llamando a
   `qyro_advice`. **Las dos llegan al mismo módulo** —
   `qyro_session::advisor` — porque es lo único que las dos alcanzan.
3. El motor ordena: red → cable directo → serie → óptico, estima con las cifras
   de `R8`, y **antes de ofrecer cualquiera de los dos lentos pregunta lo
   aburrido**: ¿CD, disquetera, PCMCIA, red? Cualquiera es entre 10 y 10 000
   veces más rápida.
4. **La frase la escribe el motor**, no cada cara. Un error que en la GUI dice
   «la clave de este aparato ha cambiado» y en el CLI dice `code -7` son dos
   productos. Es el precedente de `HumanFingerprint::to_grouped_hex`.
5. La persona elige, y a partir de ahí la cadena de la fase 12 sin cambios —
   handshake, manifiesto, `DataChunk` sellados, `Session::finish`.

Y ese paso 2 **existe en las dos caras desde esta fase**: `qyro_advice` cruza la
frontera C, que pasa de 24 a 25 símbolos con su enmienda en ADR-0032. Cuatro
hechos entran, una frase sale a un buffer prestado; no cruza ningún tipo y no
abre ninguna arista nueva.

---

## 5. Lo que las guardas hicieron esta fase

Tres pararon el trabajo y **las tres tenían razón**:

- `the_c_surface_is_exactly_the_symbols_that_are_written_down` **no dejó añadir
  `qyro_advice`** hasta que existió la enmienda a ADR-0032. Exactamente su
  trabajo: un símbolo en esa frontera es superficie de seguridad.
- `check_docs_consistency` rechazó la ficha del primer P0 por **reusar un
  identificador ya ocupado** — «un hallazgo tiene un estado, no dos».
- Y `check_parity.ps1`, que yo mismo acababa de escribir, **pasó cuando debía
  fallar**. Que una comprobación nueva se equivoque a favor es el peor caso, y
  sólo se ve intentando romperla a propósito.

---

## 6. La puerta, en `52fa4d5`

| Comprobación | Resultado |
|---|---|
| `cargo test --workspace` | **739 pruebas, 0 fallos** |
| `flutter test` | **120 pruebas, 0 fallos** (eran 105) |
| `cargo clippy --workspace --all-targets -D warnings` | 0 errores |
| `cargo audit --deny warnings` | 0 avisos (2 ignorados, con argumento y guarda) |
| `dart analyze` | sin incidencias |
| `check_docs_consistency` | OK |
| `check_parity` | OK, 12 capacidades |

---

## 7. Lo que esta fase NO promete

- **Dos máquinas.** Todo es loopback en un host: una NIC, un switch, un
  cortafuegos y un cable quedan fuera. **Fase 19.**
- **Que QYR-0363 tenga control.** Está arreglado y verificado a mano; la prueba
  que lo guardaría se quedó colgada y **se retiró en vez de dejarla en rojo**. La
  ficha lo dice: un arreglo cuya prueba nunca ha fallado es una conjetura.
- **Que la Release publicada esté arreglada.** Los dos P0 están corregidos en la
  rama; la Release sigue rota y su retractación es lo primero de la siguiente
  sesión.
- **Cancelar a mitad en el CLI, ni óptico ni serie en la GUI.** Cinco celdas de
  la tabla dicen `NO` con su argumento, y eso es una respuesta completa.
