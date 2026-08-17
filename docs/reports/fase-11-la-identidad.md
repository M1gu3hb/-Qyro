# Fase 11 — La identidad, y lo que costó no tenerla

**Base:** `0c6de85`. **Rama:** `claude/qyro-net-6a`.

---

## 1. Objetivo y alcance

> **Que la aplicación tenga una identidad que sobreviva al proceso.**

Esta fase no estaba en el plan. Apareció tirando de un hilo suelto en la fase 10
—«¿quién llama a `KeystoreWrapper`?»— y la respuesta fue: nadie.

---

## 2. Qué se encontró

`qyro_session::session::new_identity` llamaba a `DeviceIdentity::generate()`, y
los tres constructores de `Session` lo llamaban **sin condición y sin `cfg`**.

**Cada transferencia estrenaba un par de claves, en las dos plataformas.**

| Lo que el producto promete | Lo que hacía |
|---|---|
| «Compara la huella en voz alta una vez» | Cambiaba en cada transferencia |
| «Si la clave cambia, Qyro se niega» | `TrustBook` arrancaba vacío en cada proceso |
| «Teclea el código de emparejamiento» | `ownPairingString()` devolvía `null` **siempre** |
| «La identidad sobrevive al reinicio» (fase 06) | Cierto del mecanismo, falso de la aplicación |

El motor, el vocabulario de confianza, el formato del blob y el backend DPAPI
eran reales y estaban probados. **Nada los unía.**

### La evidencia que lo ocultó durante cinco fases

`STATUS.md` citaba «Persist an identity across two separate process
invocations». Ese paso ejecuta `qyro_store_smoke`, cuya cabecera dice **«Never
shipped»**, y construye `qyro_win_dpapi::WindowsIdentityStore` directamente, sin
pasar por `qyro_session` ni por `qyro_ffi`.

**Probaba que DPAPI hace ida y vuelta. No probaba que el producto tuviera
identidad.** Los runs estaban en verde y la propiedad no existía.

---

## 3. Cómo se verificó antes de tocar nada

Cuatro agentes independientes intentaron **refutar** el hallazgo desde cuatro
ángulos: la fuente de Rust, la suite de pruebas, el lado de Dart y la severidad.
**Cero refutaron.** Los cuatro lo confirmaron y encontraron más:

- el libro de confianza es igual de efímero, y es un `static` de proceso;
- `qyro_session_local_address` no tiene ni un llamante de producción;
- un comentario en `trust.rs` decía «exactamente como la identidad de Android
  hoy», dando a entender que la de **Windows** sí sobrevivía. No sobrevivía;
- `session.rs` documentaba su propio hueco en presente: «Persistent identity
  arrives in phase 06 through the platform stores», justo encima de la función
  que generaba una clave de usar y tirar.

Ese último detalle es el que mejor describe esta fase: **la respuesta estaba
escrita en el archivo, en inglés, sin que nadie la leyera contra lo que STATUS
afirmaba.**

---

## 4. Qué se hizo

1. **ADR-0040 congelada** antes de una línea de código, más las enmiendas que
   retiran el mecanismo de ADR-0037 y corrigen el conteo de ADR-0032.
2. **`new_identity` borrado.** Los tres constructores piden
   `identity::current()?`, que **se niega** si nadie abrió una identidad.
3. **`qyro_session::identity`**: carga o crea, escritura atómica, y **nunca
   regenera** un blob ilegible.
4. **Tres símbolos**, veintitrés en total.
5. **Dart**: abre la identidad antes de cualquier sesión, enseña su huella, y
   pregunta al libro en vez de escribir `newPeer` a mano.
6. **El defecto del byte 3**: un blob sellado por el puente no se podía volver a
   abrir, y nada lo cazaba.
7. **La prueba entre procesos** que falla en cualquier commit anterior.

### La regla que no se dobla

**Nada genera si el almacén no está vacío.** Un blob que existe y no abre es
`IdentityUnreadable`, nunca una razón para acuñar un reemplazo. Un aparato que se
vuelve un desconocido en silencio para cada peer que confiaba en él es peor que
un aparato que se niega a empezar una transferencia.

Y por eso la escritura es atómica: sin temp-file-y-rename, un corte durante la
creación inicial dejaría un blob corto que esa misma regla rechaza **para
siempre**. La regla que protege la identidad la ladrillaría.

---

## 5. Qué se descartó, y lo que cuesta

**Keystore no llega a la v1.0** (QYR-0354, ADR-0040 §7).

El mecanismo que ADR-0037 especificó —«Dart registra los punteros al arrancar»—
no es implementable, y la razón que basta sola es un interbloqueo garantizado: la
callback tendría que alcanzar Keystore por `MethodChannel`, que completa por el
bucle de eventos del isolate, **dentro de la llamada FFI bloqueante que es ese
isolate**.

Lo que sí funciona es un shim en C compilado por el NDK. Ese archivo hace
`AttachCurrentThread` sobre un hilo que la JVM no ha visto nunca, y cada llamada
JNI necesita su `ExceptionCheck` o la siguiente es comportamiento indefinido. Es
el archivo con más probabilidad de fallar **sólo en un aparato**, y este proyecto
no puede ejecutar nada en un aparato. Un emulador tiene Keystore por software.

**Enviar un shim que nadie ha validado es peor que enviar el sandbox y decirlo.**

Lo que cuesta, en la tabla del modelo de amenazas y no en una nota al pie:

> Con Keystore, un atacante con root necesitaría además el TEE. **Con el sandbox,
> root basta.**

---

## 6. A qué afectaba cada defecto

| Ficha | A qué afectaba | Quién lo habría notado |
|---|---|---|
| QYR-0353 | Las dos promesas del producto | Las dos primeras personas que lo usaran juntas |
| QYR-0352 | El conteo de la frontera de seguridad | Nadie, y ése es el problema |
| Byte 3 | El puente de Android, latente | Sólo cuando alguien construyera la etapa B |

---

## 7. Resultado contra el objetivo — **CUMPLIDO**

```
qyro_store_smoke session-open <ruta>      -> a5f9038e-ccd9525a-d473f52e-b802c3fe
qyro_store_smoke session-open <ruta> <fp> -> a5f9038e-ccd9525a-d473f52e-b802c3fe, exit 0
```

Dos procesos, misma ruta, misma huella, **a través de `qyro_session`**. Y el
control: dos rutas distintas dan identidades distintas y la comparación sale con
código 4.

---

## 8. Clase de evidencia por afirmación

| Afirmación | Clase |
|---|---|
| La identidad sobrevive al proceso | **Entre procesos**, medido en Windows local y en CI |
| Un blob ilegible no se regenera | **Probado en unidad** |
| La escritura es atómica | **Compilado y razonado.** El corte no se provoca; no hay prueba de un corte de corriente |
| El byte 3 hace ida y vuelta | **Probado en unidad**, con el fake que lleva el identificador del puente |
| Android guarda la semilla sin envolver | **Probado**, y el test lo dice en su nombre |
| La identidad sobrevive a un **reinicio** | **Ninguna.** Es el escenario B2 del protocolo de hardware, en blanco |
| Algo de esto funciona en un teléfono | **Ninguna** |

---

## 9. La puerta — 2026-08-16

| # | Comprobación | Exit |
|---|---|---|
| 1 | `cargo test --workspace` | 0 |
| 2 | `cargo fmt --all --check` | 0 |
| 3 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| 4 | `cargo clippy … --target aarch64-linux-android` | 0 |
| 5–8 | `flutter analyze`, `flutter test`, `dart format` | 0 — 92 pasadas, 9 saltadas |
| 9–12 | `check_docs_consistency` en Bash y PowerShell | 0 y 0 |
| 13 | CI, incluido el paso entre procesos nuevo | ver §14 |

---

## 10. Tabla de mutación

No se ejecutó un barrido nuevo, y decirlo es más honesto que ejecutar uno para
poder poner una tabla. Lo que sostiene esta fase es distinto y más fuerte para lo
que cambió: **una prueba entre procesos que falla en cualquier commit anterior**,
con su control de falsabilidad en los dos sentidos.

Un barrido de mutación sobre `identity.rs` sigue siendo trabajo que vale la pena
y es lo primero que pediría una fase 12.

---

## 11. Tests antes y después

| | Antes | Después |
|---|---|---|
| Rust | 633 pasadas, 2 ignoradas | 639 pasadas, 2 ignoradas |
| Dart | 91 pasadas, 10 saltadas | 92 pasadas, 9 saltadas |

**Dos de las pruebas verdes de antes afirmaban el defecto.** Fabricaban una clave
cambiada con «un segundo receptor, luego una segunda identidad», que sólo era
cierto porque cada sesión acuñaba la suya. Las dos se invirtieron a propósito,
cada una con su párrafo dentro del test, porque **es exactamente aquí donde una
reparación borra su propia evidencia**.

---

## 12. Delta de dependencias

Ninguna externa. `qyro_session` gana una arista `cfg(windows)` a
`qyro_win_dpapi` —cuyas dependencias ya estaban todas en el grafo— y
`qyro_crypto` como dev-dependency. `Cargo.lock` no se mueve; el `CLOSURE` de
`c_abi_contract.rs` pasa de 68 a 69, que es la actualización deliberada para la
que ese changelog existe.

---

## 13. Archivos tocados

Veintitrés, de los cuales cinco son guardas estructurales que **exigieron su
línea en cuanto el archivo existió**, y las cinco tenían razón.

---

## 14. Runs de CI

En `STATUS.md`. El paso nuevo, «An identity survives a process, through the
engine», es el que **falla en cualquier commit anterior a esta fase**.

---

## 15. Qué NO debe leerse como progreso

**Esto no es una función nueva. Es una función que se creía terminada desde la
fase 06.** El informe de aquella fase decía que la identidad sobrevive al
reinicio, y era cierto de un mecanismo que nada llamaba.

**Cinco fases pasaron con los runs en verde sobre una propiedad ausente.** No
falló ninguna prueba porque ninguna prueba cruzaba esa costura. Es la misma forma
que QYR-0328, QYR-0348, QYR-0349 y el byte 3: dos piezas correctas y una junta
que nadie recorría.

**La identidad de Android está sin envolver.** Root basta. Está escrito donde
debe estar y la aplicación **no lo dice en pantalla**, que es en sí una
limitación.

**Nadie ha reiniciado un teléfono con esto instalado.** Lo más fuerte que enseña
CI es un segundo *proceso*. El escenario B2 del protocolo de hardware sigue en
blanco, y un hueco en blanco es la verdad hasta que alguien lo llene.

---

## 16. Ledger y handoff

- `BUGS_PENDING.md`: **154 fichas, 0 abiertas.** Tres nuevas: QYR-0352 y
  QYR-0353 cerradas, QYR-0354 descartada con argumento.
- IDs siguientes desde **QYR-0355 en adelante**.
- Siguiente: la **fase 07**, y ahora con más razón — el escenario B2 comprueba
  justo lo que esta fase construyó y no puede probar.
