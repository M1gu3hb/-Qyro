# R3 — Cómo auditar

Este documento dice **cómo se comprueba que algo está hecho**. Se aplica a tu
propio trabajo antes de cerrar cada puerta.

---

## 1. El principio

> **Una prueba que pasa no es evidencia. Una prueba que pasa y que falla cuando
> borras el control, sí.**

Todo lo demás en este documento es una forma de aplicar eso.

---

## 2. El barrido de mutación

### Cómo se corre

`cargo-mutants 27.1.0` está adoptado y hay un job de CI que ya exige evidencia
para `qyro_fs`. Úsalo:

```
cargo mutants --package <crate> --timeout 90 --output target/mutants
```

**Siempre con `--timeout`.** Sin él, un mutante que entra en bucle infinito
consume el barrido entero. Ya pasó.

### Cómo se lee la salida

Cuatro veredictos, y **sólo uno es un hallazgo**:

| Veredicto | Significa | Es hallazgo |
|---|---|---|
| `caught` | Un test falló | No |
| `missed` | La suite quedó verde | **Sí, si el control importa** (ver §3) |
| `unviable` | No compila | No. Dilo en el alcance y sigue |
| `timeout` | La suite no terminó | **No directamente.** Ver §4 |

### El alcance se declara siempre

**Un barrido que no dice cuántos mutantes cubrió de cuántos se lee como exhaustivo
sin serlo.** Escribe siempre, por crate:

> `qyro_protocol`: 281 mutantes — 176 caught, 54 missed, 39 unviable, 12 timeout.

Y si limitaste el barrido —por tiempo, por módulo, por muestra— **dilo, con lo que
quedó fuera**.

---

## 3. Clasificar los `missed`: por familia, no uno a uno

**No mires 150 mutantes de uno en uno.** `cargo-mutants` genera un número pequeño
de formas y las nombra literalmente en la salida. Ordena por esa forma y decide
**por familia**:

| Familia | Ejemplo de la salida | Veredicto |
|---|---|---|
| `Display`/`Debug`/formateo | `replace <impl fmt::Display>::fmt -> Ok(Default::default())` | **Ruido.** Nadie prueba el texto de un error, ni debe |
| Accesor trivial | `replace X::field -> T with Default::default()` sobre un getter | Ruido, **salvo** que el campo gobierne una decisión |
| Aritmética saturante | `replace saturating_add with +` | Una ficha para toda la familia |
| **Retorno de un validador** | `replace ... -> bool with true` | **Ficha propia cada uno.** Aquí vive el riesgo |
| **Material de clave, tag, digest** | `replace ...tag -> &[u8] with Vec::leak(...)` | **Ficha propia cada uno** |
| **Rama de error suprimida** | `delete match arm`, `replace Err(..) with Ok(..)` | **Ficha propia** si decide un rechazo |
| Equivalente | el cambio no altera la semántica | Ruido. Dilo y sigue |

**Las tres familias en negrita son las que importan.** Las demás se resumen en una
línea del informe con el criterio de exclusión escrito **una vez**.

---

## 4. Los `timeout`: la pregunta correcta

Un timeout significa que el mutante **cambió el comportamiento tanto que el
programa dejó de terminar**. La propiedad *está* cubierta; lo que falla es la
forma de fallar.

**La única pregunta que importa: ¿puede un peer producir esa condición desde el
cable?**

- **Si no** — y casi siempre la respuesta es no, porque el mutante sustituye el
  *cuerpo de una función* y ningún atacante puede hacer eso — entonces es una
  **guarda de progreso** de cinco líneas: «si no avancé, error tipado». P2.
- **Si sí** — es un posible bucle infinito alcanzable, y eso es **denegación de
  servicio remota**. Va primero, antes que todo lo demás.

**Y escribe el argumento estructural en los dos casos.** El modelo correcto es
cómo se demostró que `ItemVerdict::Incomplete` era inalcanzable: no «no lo vemos»,
sino «no existe camino, y aquí está el porqué».

Ejemplo real, para calibrar: `FrameHeader::total_len -> 0` colgaba la suite.
Parecía P0. No lo era:

```rust
pub const fn total_len(&self) -> u64 {
    HEADER_LEN as u64 + self.payload_len as u64 + self.trailer_len as u64
}
```

`HEADER_LEN` es la constante 48 y los otros dos son sin signo. **No puede devolver
menos de 48 con ninguna entrada.** El mutante sustituyó el cuerpo, que no es algo
que un peer pueda hacer. Hueco de cobertura, no agujero.

---

## 5. Las clases de evidencia

Cada afirmación del informe lleva la suya. **Una afirmación sin clase se audita
como no probada.**

| Clase | Qué significa |
|---|---|
| **Compilado** | `cargo build` pasó. No dice nada sobre comportamiento |
| **Probado en unidad** | Un test en el mismo proceso, con dobles si los hay |
| **Probado en integración** | Varias piezas reales juntas, sin dobles |
| **Probado entre procesos** | Dos procesos del sistema operativo, comunicándose de verdad |
| **Probado en emulador** | Android emulator |
| **Probado en simulador** | iOS Simulator. **No es un iPhone** |
| **Probado en hardware físico** | Un aparato real |
| **Probado por un usuario** | Una persona lo usó |
| **Probado en release** | Build firmada e instalada |

**Y la plataforma es parte de la clase.** «Probado en unidad» sin decir dónde no
vale: este proyecto corre 527 tests en Linux y un subconjunto distinto en Windows,
por selección `cfg`.

**Nunca** conviertas una clase en otra. «Compiló en Linux» no es «funciona».
«Pasa en el simulador» no es «funciona en un iPhone».

---

## 6. Auditar tu propio informe

Antes de cerrar cualquier puerta, hazte estas preguntas y responde por escrito:

1. **¿Qué afirmación de este informe no podría defender ahora mismo con un comando
   o un archivo?** Ésa es la que hay que quitar o rebajar.
2. **¿Qué sección escribí hace tres pasos y no he vuelto a mirar?** Ésa es la que
   está mal.
3. **¿Qué número dije de memoria?** Vuelve a obtenerlo.
4. **¿Qué prueba nueva pasaría igual si borrara el código que dice probar?**
5. **¿Qué se rompería si esto corriera en Windows, o en un teléfono?** Si no lo
   sabes, la clase de evidencia es «Linux» y hay que decirlo.
6. **¿Qué he dejado fuera del alcance sin decirlo?** Un tope silencioso —top-N,
   sin reintentos, muestreo— se lee como cobertura completa.

---

## 7. Auditar una fase al terminarla

Además de la puerta (`R2`), al cerrar una fase escribe explícitamente:

- **Objetivo por objetivo: cumplido / parcial / no hecho.** «Parcial» es válido.
- **Qué encontraste que no estaba en el plan**, y de eso, **qué arreglaste y qué
  no**, con ficha y motivo.
- **Qué NO debe leerse como progreso.** La sección más importante del informe. Si
  la fase 02 pasa, eso **no** significa que haya producto: significa que Dart
  puede llamar a Rust.
- **Qué documentación del repositorio quedó desfasada** por lo que hiciste.
- **Qué necesita saber la fase siguiente.**

---

## 8. Cuando el plan está mal

**Los documentos de este directorio no son sagrados. La evidencia sí.**

Si un documento de fase te pide algo que:

- se contradice con otro documento,
- se contradice con el código,
- es imposible por una razón estructural,
- o es un error técnico,

**para, escríbelo con la evidencia, y propone la corrección.** Ya ha pasado tres
veces en este proyecto y las tres veces el agente tuvo razón:

- un harness de Android que estructuralmente no podía alcanzar Keystore, porque no
  hay API de Keystore en el NDK (QYR-0064);
- tres reglas del propio prompt que no se podían cumplir a la vez;
- un criterio de `git diff` contra una base que nunca se fusionó.

**No arregles algo que no está roto sólo porque el plan lo dice, y no te
inventes** un arreglo para un plan imposible.
