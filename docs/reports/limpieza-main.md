# Limpieza — el árbol entero en `main`, compilando en las dos plataformas

**Rama:** ninguna. **`main`** · **2026-08-18**

> Esta sesión **no abrió ninguna fase**. Dejó lo que ya existía limpio, sin
> errores y en `main`.

---

## 1. Qué compila en qué plataforma, con el comando y el código de salida

| Comprobación | Comando | Salida |
|---|---|---|
| **Linux, todo el árbol** | `cargo check --workspace --all-targets --target x86_64-unknown-linux-gnu` | **0** |
| Windows, pruebas | `cargo test --workspace` | **0** — 739 pruebas |
| Windows, lints | `cargo clippy --workspace --all-targets -- -D warnings` | **0** |
| Dependencias | `cargo audit --deny warnings` | **0** — 122 paquetes |
| Dart | `flutter test` | **0** — 122 pruebas, 10 saltadas |
| Documentación | `scripts/check_docs_consistency.ps1` | OK |
| Paridad GUI/CLI | `scripts/check_parity.ps1` | OK — 13 capacidades |

**La primera fila es nueva y es la que faltaba.** Se cumple en esta máquina con
`rustup target add x86_64-unknown-linux-gnu`; `check` no enlaza, así que no hace
falta un enlazador cruzado. **Es la comprobación 17.**

### El P0: por qué no compilaba

`dab9fa3` insertó los `pub use` del beacon **entre un `#[cfg(windows)]` y el
elemento que ese atributo guardaba**. El atributo se pegó al bloque nuevo, y en
un solo commit pasaron dos cosas opuestas: **el beacon desapareció fuera de
Windows** —su único caso de uso— y **`MdnsDiscovery` se exportó en todas partes
sin existir**. `qyro_session::discovery` llama a `BeaconSwarm::bind_all()` sin
`cfg`, correctamente, así que el árbol dejó de compilar. **193 ejecuciones de CI
en rojo.**

Es la misma forma que separó `#[cfg(test)]` de `mod tests` el mismo día: **un
atributo externo se pega al siguiente elemento, y lo que se inserte en medio se
lo queda.**

**El defecto de raíz no fue el atributo: fue compilar sólo en Windows.** Un error
que sólo existe en la otra plataforma era invisible por construcción. Congelado
en **ADR-0043 enmienda 2**, y el informe de la fase 14 lleva su corrección: dijo
«puerta en verde» habiendo compilado en una sola plataforma.

---

## 2. Qué ramas quedan

**Una: `main`.**

Las 19 ramas de trabajo se fusionaron con `git merge --no-ff` —sin rebase, sin
force, sin reescribir un solo hash— y **se comprobó una a una que estuvieran
contenidas en `main` antes de borrarlas**: `git merge-base --is-ancestor` sobre
las 19. Los 461 commits se conservan uno a uno y la etiqueta `v1.0.0` sigue
apuntando a lo que apuntaba.

`main` pasó de `a8bafcf` a la punta del trabajo. También se borraron **17
documentos duplicados** de `docs/` raíz: no eran sólo copias, eran **copias
caducadas** — la de `R1-REGLAS-NO-NEGOCIABLES.md` seguía diciendo *«Cargo.lock
tiene 63 paquetes y todos son de primera parte»*, que QYR-0312 desmintió el
2026-08-13 en la versión de `docs/fase-implementacion/`. Dos copias de una regla,
una con un dato ya sabido falso, es peor que no tener la regla escrita.

---

## 3. El diluvio de correos: las tres causas, las tres cerradas

| Causa | Qué era | Arreglo |
|---|---|---|
| **QYR-0367** | `ci.yml` **sin ningún filtro de rutas**: un commit de una línea compilaba Rust en Linux, Rust en Windows, Flutter, guardas y scripts | El trabajo `documentation` sale a `documentation.yml` con sus rutas, y `ci.yml` recibe `paths` de código. **Cada mitad con su disparador** |
| **La regla que los generaba** | 28 de 78 commits fueron `chore(status)` — el 36 % | **`ESTADO-ACTUAL.md` va dentro del commit de contenido.** La comprobación 16 se cumple corriendo la puerta antes de empujar |
| **QYR-0366** | Tres trabajos de iOS en cada `push` sobre runners de macOS | A `workflow_dispatch`. **No se borran**: ADR-0039 dice *aplazado, no cancelado* |

Un `paths-ignore` en `ci.yml` habría sido el arreglo fácil y el equivocado: el
trabajo de documentación **tiene que correr justo cuando cambia la
documentación**, y lo habría apagado para los commits que lo necesitan.

Y `branches: [main, 'claude/**']` pasa a `[main]` en seis flujos: un patrón que
ya no puede coincidir con nada es una promesa de cobertura que no cubre.

---

## 4. Fichas

**165 fichas. 3 abiertas.** Vocabulario comprobado sobre todas: ninguna fuera de
`cerrado`, `descartado` o `abierto`.

- **QYR-0362 → cerrada.** Estaba en «arreglado, evidencia parcial», que **no es
  un estado de este registro**: era un pendiente disfrazado. La evidencia que le
  faltaba ya existe — la matriz entrega desde la GUI en las dos direcciones con
  el archivo comparado byte a byte.
- **QYR-0365 → sigue abierta, y más acotada.** Leer `pump` **descarta** que
  serialice por elemento: su bucle pasa al siguiente elemento en cuanto drena el
  anterior, y `WINDOW_CHUNKS` es 16, no 1. Veinte archivos de 200 bytes salen en
  **un solo** `pump`. Queda una medida concreta: cuántos `step` hace el emisor
  entre el último `DataChunk` y el `Complete`.
  **No se descarta**, y ésa es la decisión: a los 50 archivos el reloj de 60 s
  convierte una transferencia completa en un fallo. Descartarla sería cerrarla
  con un argumento falso.
- Las otras dos abiertas venían de antes y siguen con su ficha.

**La Release** decía *«probado en CI sobre Linux y Windows»*, que era falso el día
que se publicó y siguió siéndolo. Lleva su corrección encabezando las notas, sin
borrar la frase.

---

## 5. Comprobación 14 sobre el árbol

Barrido de los símbolos exportados por las doce cajas del motor contra todos los
llamantes de producción: **97 símbolos sin mención fuera de su caja**, que
filtrados a *capacidades* —fuera constantes y tipos de error— quedan en 45.

**Y ese número está inflado, dicho antes de usarlo.** El barrido busca el nombre
literal, así que no ve a `FoundPeer`, `Channel` ni `Tally`, que viajan como
valores de retorno y nunca se escriben en quien los usa. Un barrido textual
encuentra el patrón; no sustituye a la tabla por consumidor.

**Lo que sí es una familia entera sin consumidor —y está declarada—** es el
historial de `qyro_fs`: `TransferHistory`, `HistoryRecord`, `HistoryStatus` y
compañía. **Cero llamantes fuera de su caja**, y es exactamente lo que QYR-0358
retiró de la interfaz y D3 lleva fichado: el motor sigue grabando y ningún
símbolo de la frontera C lo lee. **Declarada, no olvidada.**

---

## 6. Qué sigue roto

1. **QYR-0365** — ~1,2 s por archivo pequeño; a los 50 el reloj de 60 s convierte
   una transferencia completa en un fallo. Diagnosticada hasta el emisor,
   sin arreglo.
2. **El APK de la Release** sigue sin el arreglo de QYR-0362. **Bloqueado con
   nombre:** el SDK de Android está en esta máquina pero **sus licencias no están
   aceptadas**, y aceptar un acuerdo legal en nombre del propietario no lo hace
   el implementador.
3. **Ningún hardware.** Ninguna cámara ha leído un QR de Qyro, ningún cable serie
   ha llevado un byte, y dos máquinas no se han encontrado nunca por un cable.
   Fase 19, y el hueco sigue en blanco.
4. **La fase 22 a medias**, con ADR-0047 congelada y tres de sus entregables
   hechos. **No se abrió aquí y no se abre.**
