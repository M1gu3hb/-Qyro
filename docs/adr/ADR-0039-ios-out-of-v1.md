# ADR-0039 — iOS sale de la v1.0

- **Estado:** aceptada
- **Fecha:** 2026-08-14
- **Decide:** el alcance de plataformas de la v1.0
- **No revoca:** ADR-0025 ni ningún trabajo de iOS existente. Se aparca, no se
  borra.

---

## Contexto

**Xcode sólo corre en macOS, y no hay ningún macOS disponible para este
proyecto.** La máquina de desarrollo es Windows 10; CI tiene runners
`macos-latest`, pero un runner de CI no sustituye a una máquina de desarrollo
para trabajo que exige iterar contra un simulador, un depurador y un perfil de
firma.

Eso no es una preferencia ni una estimación de esfuerzo. Es una condición de
hardware, y determina lo que se puede construir:

| Fase | Mitad iOS | Por qué |
|---|---|---|
| 03 selector de archivos | `UIDocumentPickerViewController` | Swift, y hay que verlo funcionar |
| 04 descubrimiento | `NWBrowser` / `NWListener`, `NSBonjourServices` en `Info.plist` | Igual, y el `Info.plist` no se puede validar sin construir |
| 06 identidad persistente | Keychain + Secure Enclave | Igual |
| 07 hardware físico | Un iPhone | Ni aparato ni forma de instalarle nada |
| 08 empaquetado y firma | IPA ad-hoc | Xcode, y una cuenta de desarrollador |

**Cinco de las nueve fases que quedan tienen una mitad que no se puede
construir.** Seguir escribiéndolas como si se fueran a entregar es la misma clase
de defecto que este proyecto lleva encontrando desde la fase 01: una afirmación
que nadie puede ejecutar.

---

## Decisión

**La v1.0 de Qyro es Android + Windows.**

iOS queda **aplazado**, no cancelado. Cuando exista un Mac, es una v1.1.

### Qué se pierde, dicho sin suavizar

- **Un iPhone no podrá mandar ni recibir nada.** Para una aplicación cuyo valor
  es «mandar un archivo del teléfono al ordenador», perder la mitad del mercado
  de teléfonos es una pérdida grande, y conviene que esté escrita como tal en vez
  de como una nota al pie.
- El criterio de aceptación de la fase 10 —dos personas, dos aparatos físicos—
  sigue siendo alcanzable con Android + Windows, así que el listón de la v1.0 no
  baja: cambia de qué aparatos.

### Qué NO se toca

- **`ADR-0025` sigue vigente y sin cambios.** Decidió `jni-sys` para Android; no
  decidía nada de iOS.
- **Nada del trabajo de iOS que exista se borra.** El runner de Flutter para iOS,
  los workflows `ios-runtime.yml`, las decisiones de plataforma ya registradas y
  el `Runner.app` que CI construye siguen donde están, verdes y sin tocar. Se
  aparcan igual que se aparcó el sprint 4D.2.
- **Los workflows de iOS siguen corriendo.** Cuestan poco y son la prueba de que
  la puerta sigue abierta: el día que haya un Mac, lo que hay compila.

### Qué haría falta para reincorporarlo

1. **Un Mac.** Cualquiera capaz de correr una versión de Xcode compatible con la
   versión de Flutter fijada.
2. **99 USD/año de cuenta de Apple Developer**, para firmar más allá del
   simulador y para instalar en un aparato físico.

Sin las dos, iOS no avanza. Con las dos, lo que hay hoy no es un punto de
partida vacío: el núcleo de Rust ya compila para iOS en CI y `qyro_ffi` ya se
carga con `DynamicLibrary.process()`, que es el camino de iOS.

---

## Alternativas descartadas

| Alternativa | Por qué no |
|---|---|
| **Construir iOS sólo en CI, a ciegas** | Un `Info.plist` que CI acepta y un permiso que el sistema pide en tiempo de ejecución son cosas distintas. Sin poder ejecutar en un simulador, cada iteración es un empujón a CI y una espera, y nada de eso demuestra que la aplicación *funcione* — que es exactamente la distinción entre «compiló» y «funciona» que `R1` §4 prohíbe borrar |
| **Alquilar un Mac en la nube** | Cuesta dinero recurrente, que es una de las cuatro cosas que este proyecto no decide solo. Queda como opción a proponer, no como decisión tomada |
| **Dejar iOS «en el alcance» y ver qué pasa** | Es lo que estaba pasando. Trece documentos prometen tres plataformas hoy. Prometer una plataforma que no se puede construir es la misma clase de defecto que las siete que este repositorio ya catalogó |
| **Cancelar iOS del todo** | Innecesario. El coste de aparcarlo es cero y la puerta se cierra sola si alguien borra el trabajo |

---

## Lo que esta decisión NO promete

- **No promete que Android + Windows esté cerca.** Al escribir esto no hay una
  sola prueba en hardware físico, en ninguna plataforma.
- **No promete que iOS vuelva.** Depende de una compra que este proyecto no ha
  hecho.
- **No cambia nada del código.** Es una decisión de alcance; el efecto es sobre
  los documentos que prometen y sobre lo que las fases 03 a 08 declaran como
  hecho o como aplazado.
