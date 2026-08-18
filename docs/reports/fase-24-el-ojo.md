# Fase 24 — el ojo

**Rama:** `main` · **2026-08-19**

**Puerta corrida con `scripts/gate.ps1`**, que lee `ci.yml` y ejecuta sus
comandos más el objetivo de Linux — las comprobaciones 16, 17 y 18 a la vez.

---

## 1. Qué prometía y qué hay

ADR-0048, congelada en `349a63f` antes de una línea de código, más su **enmienda
1** en `61853af`.

| Lo prometido | Estado |
|---|---|
| Medir px/módulo en el aparato real (`R10` §8 T1) | **NO HECHO — no hay aparato.** §4 |
| Decidir la versión del emisor con esa medida | **HECHO con aritmética, no con medida**, y dicho así |
| `rqrr` a 0.10.1 y a dependencia normal | **HECHO** |
| El ojo: píxeles → archivo | **HECHO**, 15 pruebas |
| Llamante de producción | **HECHO** — `qyro beam` y `qyro qr` |
| Cruce JNI y `Analyzer` de Kotlin | **Cerrado con argumento a la fase 19.** §5 |

---

## 2. Comprobación 14 — llamante de producción, con archivo y línea

| Capacidad | Llamante de producción | Consumidor |
|---|---|---|
| `qyro_eye::Eye::new` | `qyro_cli/src/flows.rs:577` | **CLI** |
| `Eye::look` | `qyro_cli/src/flows.rs:578` | **CLI** |
| `qyro_eye::Look` | `qyro_cli/src/flows.rs:578` | **CLI** |
| `optical::rasterise` | `qyro_cli/src/flows.rs:576` | **CLI** |
| `qyro_eye::pixels_per_module` | `qyro_cli/src/flows.rs`, en `camera_advice` | **CLI** |
| `qyro_eye::is_above_floor` | `qyro_cli/src/flows.rs`, en `camera_advice` | **CLI** |

**Las dos últimas no tenían llamante y la comprobación 14 las cazó antes de este
informe**, que es exactamente cuándo tiene que cazarlas. Se les dio el que de
verdad sirve: `qyro qr` ahora dice **qué cámara hace falta** — «a 1280×720 este
código da 14,9 px/módulo» — porque quien apunta el teléfono no sabe que la
resolución de captura decide si esto funciona, y nadie se lo iba a decir.

**Sin llamante, y declarado:** `Eye::finish` y `Eye::shape` sólo los usan las
pruebas. Es correcto y tiene dueño: los usará el lado de Android, y **hasta que
exista no se anuncian como capacidad del producto de escritorio.**

---

## 3. Comprobación 15 — del gesto al byte

1. La persona escribe `qyro beam clave.pem`.
2. **Antes del primer frame**, `preflight` dibuja uno, lo rasteriza a 4
   px/módulo y se lo da al ojo. Si esta terminal produce algo que un lector no
   reconoce, se dice **aquí** y se para. Una fuente sin `U+2584`, o que separe
   las celdas, dibuja un código perfecto a la vista e ilegible para cualquier
   lector.
3. El archivo se parte con `qyro_fountain::split`, y cada ronda combina bloques
   por una semilla y los serializa con 17 B de cabecera.
4. `optical::draw` codifica a nivel L y dibuja con medios bloques, **invertido a
   propósito**: un QR necesita oscuro sobre claro y una terminal es clara sobre
   oscuro.
5. `qyro qr` y `qyro beam` dicen cuántas columnas hacen falta —**medidas del
   dibujo, no estimadas**— y ahora también qué cámara.
6. Al otro lado: el plano de luma entra por `Eye::look`, `rqrr` encuentra el
   código, `decode_frame` comprueba que es de Qyro —un QR ajeno **no entra**, y
   aun así se cuenta como leído, porque «no veo nada» y «veo códigos que no son
   de Qyro» piden acciones distintas— y el fountain acumula.
7. `Eye::finish` devuelve el archivo **o nada**. Nunca uno a medias.

**Verificado ejecutando** en esta máquina: la cadena de los puntos 3 a 7 con
**uno de cada cuatro frames tirado**, y `qyro beam` y `qyro qr` corridos de
verdad.

**No verificado:** el punto en el que los píxeles vienen de una cámara.

---

## 4. El hueco, y por qué se queda en blanco

> **`R10` §8 T1 manda medir píxeles por módulo en el aparato real antes de
> escribir nada más. NO HAY APARATO.**

Lo que sí se hizo: **reproducir su aritmética de forma independiente**, y dejarla
en código con prueba en vez de en prosa.

| Versión | Módulos + quiet | 640×480 | 1280×720 |
|---|---|---|---|
| v20 | 105 | 3,89 | 5,83 |
| v22 | 113 | 3,61 | 5,42 |
| **v27** | **133** | **3,07** | **4,60** |

Los dos números que `R10` cita salen **idénticos**, así que el modelo no es una
suposición heredada. La decisión —pedir ≥720p y quedarse en v27— es de ADR-0048
§3, con la palanca escrita como constante y sus números en una prueba.

**Aritmética no es medida, y no se presenta como otra cosa.**

---

## 5. El cruce JNI, cerrado con argumento

**No se escribe sin aparato**, y no es un aplazamiento vago: ADR-0048 enmienda 1
lo congela con dos razones y una fase.

1. **Serían la segunda excepción a `forbid(unsafe_code)`** de todo el taller. La
   primera, `qyro_win_dpapi`, costó una ADR entera. Y un slot equivocado en la
   vtable de JNI **no da error de compilación**: da un salto a una función
   arbitraria, y el síntoma es un proceso muerto sin traza en el aparato de otra
   persona.
2. **No hay forma de ejercitarlas aquí.** Ninguna prueba de este repositorio
   puede tocar una `JNIEnv`.

Escribirlas ahora sería añadir la clase de código que este proyecto trata con más
cuidado, con la clase de evidencia que prohíbe: ninguna.

**Lo que falta es exactamente un transporte de píxeles**, y su forma ya está
fijada por `Eye::look(&[u8], usize, usize)` — un plano de luma, que es el plano 0
de un `ImageProxy` en `YUV_420_888`. **`R7` sigue prometiendo cuatro canales y hay
tres y medio**, y el medio que falta no es medio ojo: es el cable entre el ojo y
la cámara.

`jni-sys` se descartó **y no por el crate**: tampoco se puede ejercitar sin
aparato, así que cambia una incertidumbre por otra y encima añade una
dependencia.

---

## 6. Lo que costó, medido

| | `qyro.exe` para `x86_64-pc-windows-msvc` |
|---|---|
| Antes del ojo | 1 373 696 B |
| Con `rqrr` en el producto | **1 647 104 B** |

**+273 408 B — 267 KB** por un decodificador que hasta ayer era sólo de prueba.

### El aviso que **no** se borró

`.cargo/audit.toml` llevaba dos excepciones con su condición escrita: bórralas
cuando algo haga `rqrr` dependencia normal. Esta fase lo hace. Y **medido, la
condición se cumplió a medias**:

- **RUSTSEC-2026-0002** — arreglada por `lru` 0.16.4. **Borrada.**
- **RUSTSEC-2026-0253** — **viva**. `cargo audit --json` da `patched: >=0.18.2`,
  y `rqrr` 0.10.1 fija `lru ^0.16`: `cargo update -p lru --precise 0.18.2`
  **falla**.

Se conserva con un argumento nuevo, específico, y su condición de caducidad.
**Borrarla porque se esperaba que caducara habría sido exactamente la alarma
silenciada que ese archivo prohíbe.** Y es la primera vez que este proyecto
acepta un aviso en código que **sí** viaja en el producto — dicho, no escondido.

---

## 7. La puerta

Corrida con `scripts/gate.ps1`, que **lee `ci.yml`** en vez de llevar su propia
lista, porque una lista a mano se separa del flujo el día que alguien toca uno de
los dos.

| Comprobación | Resultado |
|---|---|
| `cargo fmt --all --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | **769 pasadas** |
| `cargo test --workspace --all-features` | 0 |
| `cargo test --doc --workspace` | 0 |
| `clippy` **contra Linux** | 0 |
| `check --all-targets` **contra Linux** | 0 |
| `cargo audit --deny warnings` | 0, con **una** excepción argumentada |

**La puerta cazó dos cosas hoy antes de que llegaran a CI:** un `#![cfg(test)]`
duplicado —el mismo error de forma que en la fase 15— y el `ptr_arg` que la
comprobación 17 no veía porque usaba `check` donde CI usa `clippy`.

---

## 8. Lo que esta fase NO promete

- **Que un teléfono lea un QR de Qyro.** Ninguno lo ha hecho.
- **Que el ojo funcione a 640×480.** Se pide 720p; la palanca está escrita y
  **sin medir**.
- **Enfoque a 30 cm sobre pantalla plana** (`R10` §8 T3): en una lente fija puede
  no ocurrir nunca, y eso no lo arregla el software.
- **iOS.** ADR-0039 lo aparcó y esto no lo reabre.
