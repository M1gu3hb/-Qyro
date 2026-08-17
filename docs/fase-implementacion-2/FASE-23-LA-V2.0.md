# FASE 23 — La v2.0

> La última. Y a diferencia de la v1.0, esta etiqueta se pone sobre algo que **una
> persona ha usado**.

---

## 1. Qué tiene que ser cierto antes de etiquetar

Cuatro cosas, y las cuatro se comprueban por código de salida o no cuentan:

1. **Los cuatro canales existen y tienen llamante de producción en las dos caras**, o
   están declarados fuera con argumento. La tabla de paridad de la fase 21, en verde.
2. **Cero celdas vacías sin decisión escrita**, cero fichas abiertas, cero entradas en
   `deuda-de-calidad.md`.
3. **`docs/testing/hardware-protocol.md` tiene resultados de verdad.** No los
   veintiuno: los que el propietario haya podido ejecutar, y **los huecos que queden
   siguen en blanco**. Un hueco en blanco es la verdad.
4. **Alguien ha mandado un archivo con esto.** Al menos una vez, en hardware, y está
   escrito quién, cuándo y por qué canal. **Es lo único que la v1.0 no tenía y por lo
   que su etiqueta valía menos de lo que parecía.**

Si (4) no se cumple porque el propietario no ha podido, **la v2.0 no se etiqueta y se
dice por qué.** No se etiqueta «casi».

---

## 2. Entregables

1. **La etiqueta `v2.0.0`**, anotada, con el mismo estándar de honestidad que la
   v1.0.0: lo que hace, lo que no, y lo que no se ha probado, **antes** de lo que sí.
2. **Artefactos desde el commit que la etiqueta nombra.** QYR-0359 lo dejó por
   escrito y costó una retractación pública aprenderlo: *un hash correcto prueba que
   te dieron el archivo que nombraron, no que ese archivo haga lo que dijeron.*
   **Se construye desde el commit que se publica. Siempre.**
3. **La Release**, con la advertencia proporcional a lo que se haya probado de verdad
   — si el protocolo de hardware tiene diez escenarios en verde, la advertencia lo
   dice y deja de decir «nada se ha ejecutado nunca».
4. **`docs/release/v2.0.md`**, con la escalera de canales explicada para una persona
   que no ha leído ninguna ADR, y **con los tiempos reales de cada canal** de `R8`
   §4 y §5.1 — para que nadie intente mandar un vídeo por QR.
5. **Un `CHANGELOG` de v1.0 a v2.0** que nombre los defectos encontrados, no sólo las
   funciones añadidas. Los cinco de la fase 12 y la 13 son la mejor documentación que
   este proyecto tiene sobre cómo se rompe el software.

---

## 3. La retrospectiva, y es un entregable

`docs/reports/como-se-rompio-y-como-se-encontro.md`. Una página. No es ceremonia: es
lo que hace que la lección sobreviva a la siguiente sesión.

Los cinco casos, todos de la misma forma —**escrito, probado, y sin llamante**:

| | Encontrado en | Cómo |
|---|---|---|
| `KeystoreWrapper` | fase 11 | «¿quién llama a esto? Nadie» |
| `qyro_session_local_address` | fase 12 | la misma pregunta |
| `Session::finish` | fase 12 | una prueba de dos procesos que dejó un `.qyro-part` |
| `history()` | fase 12 | la comprobación 14, aplicada antes del informe |
| El descubrimiento entero | fases 04b–12 | escrito en Kotlin y en Rust, sin cruzar la frontera C |

Y las dos formas que los cazaron, que son el legado real del proyecto:

- **La comprobación 14:** por cada capacidad declarada, el llamante de producción con
  archivo y línea. Si es una prueba, un arnés o nadie, **la capacidad no existe.**
- **Dejar que una guarda te contradiga.** Seis pararon al implementador el
  2026-08-17 y las seis acertaron — incluida una que él mismo había escrito, y cuyo
  fallo destapó que el binario **no arranca en Windows 7**. Eso no se descubre
  razonando.

---

## 4. Lo que NO hay que hacer

- **No etiquetes sin el punto (4) del §1.** La v1.0 se etiquetó sobre algo que no
  podía completar una transferencia. La lección cuesta poco si se aprende una vez.
- **No borres la retractación de la Release de la v1.0.** Un error público
  corregido en público vale más que un historial limpio.
- **No cierres una ficha respondiendo a otra pregunta.** Es cómo se perdió la v1.0.
- **No inventes evidencia de hardware.** La última vez que se dice en este plan, y la
  única que de verdad importa.
