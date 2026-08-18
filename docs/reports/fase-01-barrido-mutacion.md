# Barrido de mutación — fase 01

`cargo-mutants 27.1.0`, `--timeout 90`, sobre los dos crates que la fase añade o
reescribe: `qyro_ffi` y `qyro_session`. Linux x86_64, `cargo test` como oráculo.

**Este archivo es el informe. Al ledger no va nada de aquí en bruto.** La regla
está escrita porque ya costó un P1: volcar 173 clasificaciones mecánicas en
`BUGS_PENDING.md` dejó el ledger en 262 fichas y sin usar (QYR-0289). Las
conclusiones humanas de este barrido son **dos** fichas, escritas a mano, y están
al final.

---

## 1. Los números, con su orden

```
$ cargo mutants --package qyro_ffi --package qyro_session --timeout 90
124 mutants tested in 3m: 35 missed, 75 caught, 14 unviable
```

Tras añadir cobertura a lo barato y real que el barrido destapó, re-barrido de
`qyro_ffi` solo:

```
$ cargo mutants --package qyro_ffi --timeout 90
93 mutants tested in 2m: 9 missed, 81 caught, 3 unviable
```

`qyro_ffi` pasa de **15 supervivientes a 9**. `qyro_session` se queda en **20**, y
el motivo es uno solo, dicho sin adorno en §3.

---

## 2. `qyro_ffi` — los nueve que quedan, y por qué

### 2.1 Un mutante equivalente, demostrado y no supuesto

| Mutante | Veredicto |
|---|---|
| `handle.rs:100` `replace \| with ^` en `compose` | **Equivalente. No es un hueco.** |

La generación ocupa los 32 bits altos y la ranura los 32 bajos, así que los dos
operandos **no comparten ningún bit encendido** y `|` y `^` dan lo mismo para toda
entrada posible. Ningún test puede matarlo porque no hay nada que matar.

No se deja como afirmación: `the_two_halves_of_a_handle_do_not_overlap` comprueba
que `(u64::from(generation) << 32) & u64::from(slot) == 0`, que `split` y `compose`
van y vuelven, y que la composición cubre el ancho entero. Si alguien cambiara el
reparto de bits, ese test se pone rojo **antes** de que el mutante deje de ser
equivalente.

### 2.2 Ocho que exigen una sesión viva, o sea un peer

| Mutante | Qué haría falta |
|---|---|
| `session_abi.rs:75` `table` → tabla nueva en cada llamada | Un handle que resuelva entre dos llamadas |
| `session_abi.rs:94` `state_code` → `0`, `1`, `-1` (×3) | Un `step` que devuelva un estado |
| `session_abi.rs:149` `with_session` → `-1` | Ver §2.3 |
| `session_abi.rs:169` `insert` → `0`, `1`, `-1` (×3) | Un `open_*` que llegue a insertar |

Los ocho comparten causa: **las rutas de éxito de la superficie C no se ejercen**.
Abrir una sesión dial­a y completa un handshake, y la superficie C no tiene
accesor para el puerto que `open_receiver` liga, así que desde un test de este
crate no se puede montar el peer. Los tests que hay recorren las rutas de rechazo,
que son las que un handle inválido o un puntero nulo alcanzan sin red.

### 2.3 Uno que merece leerse aparte

`with_session -> -1` sobrevive por una razón que no es «falta un peer», y conviene
no esconderla: **`-1` es `QYRO_ERR_INVALID_HANDLE`**, que es justo lo que los tests
de handle muerto esperan. Un `with_session` que devolviera `-1` pase lo que pase
pasaría esos tests por coincidencia numérica entre el centinela y la constante del
mutante. Es la misma familia de error que QYR-0086 en 6A —una prueba que no
distingue una medida de una constante— y aquí no está cerrada.

---

## 3. `qyro_session` — veinte supervivientes, una causa

| Zona | Mutantes |
|---|---|
| `session.rs` `advance` (`==`→`!=`, ×2) | 2 |
| `session.rs` `finished` (retorno ×2, `==`→`!=` ×2) | 4 |
| `session.rs` `verdict` (`==`→`!=` ×2, `&&`→`\|\|`, borrar `!`) | 4 |
| `session.rs` `finish` (retorno ×2, `==`→`!=` ×2) | 4 |
| `session.rs` `cancel` → `()`, `is_cancelled` → `true`/`false` | 3 |
| `session.rs` `Debug::fmt`, `RefusingSink::write_at` | 2 |
| `error.rs` `Display::fmt` | 1 |

**La causa es una y hay que decirla entera: `qyro_session` no tiene un solo test de
comportamiento.** Sus seis tests son guardas estructurales —qué archivos hay, que
ninguna ruta pueda entrar en pánico, que cada variante de error tenga sitio de
construcción—. Ninguno abre una sesión, así que **ninguna de las decisiones de
`advance`, `verdict`, `finished` o `finish` está defendida por nada.**

Que el workspace esté en 563 verdes no dice nada sobre si una sesión transfiere un
archivo. El informe de fase ya lo decía en §15 antes de este barrido; el barrido lo
convierte en un número: **veinte**.

Tres de los veinte son de presentación —`Debug`, `Display`, el sumidero que
registra en vez de tragar— y su coste es un log peor, no una transferencia mal
hecha. **Diecisiete no.** `verdict` decide si un archivo se acepta o se rechaza, y
sus cuatro mutantes cambian esa decisión sin que nada proteste.

---

## 4. Lo que este barrido **no** midió

- **Nada de `qyro_net`, `qyro_transfer`, `qyro_fs` ni `qyro_crypto`.** El barrido
  se acotó a lo que la fase añade, que es lo que `FASE-01` §6 paso 5 pide. Sus
  supervivientes históricos siguen donde estaban, en las familias QYR-0290 y
  siguientes.
- **Ninguna plataforma que no sea Linux x86_64.** Un mutante que sólo muera en
  Windows aquí figura como muerto o como vivo por razones que este equipo no puede
  ver.
- **Ningún mutante de `lib.rs`, `abi.rs` ni `guards.rs`** más allá de los ya
  contados: son superficie declarativa y guardas, y `cargo-mutants` genera poco
  sobre ellos.

---

## 5. Las dos conclusiones que sí van al ledger

Escritas a mano, no volcadas:

- **QYR-0309** — `qyro_session` no tiene cobertura de comportamiento, y veinte
  mutantes lo demuestran. P1.
- **QYR-0310** — las rutas de éxito de la superficie C no se ejercen, y ocho
  mutantes lo demuestran; uno de ellos sobrevive por colisión entre un centinela de
  test y una constante. P2.
