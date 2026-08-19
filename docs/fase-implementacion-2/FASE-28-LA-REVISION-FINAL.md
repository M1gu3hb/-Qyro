# FASE 28 — La revisión final

> **La última fase.** No añade nada. Su único trabajo es **decidir, desde el
> código, si esto va a funcionar en un aparato real** — porque en aparato real
> todavía no se ha probado nunca.
>
> El propietario lo pidió literal: *«revisión completa del código de la aplicación,
> para confirmar que ésta ya funciona, aunque no haya pruebas reales; que él
> confirme en código que va a funcionar.»*

---

## 1. La regla que hace que esta fase valga algo

Una revisión que sólo lee **encuentra lo que sabía buscar**. Esta fase se hace con
**agentes en paralelo, cada uno con un dominio y ninguno con el panorama**, y
después **otro agente intenta refutar cada hallazgo**.

> **Un hallazgo no existe hasta que tiene `archivo:línea`**, y **no sobrevive hasta
> que ha aguantado tres intentos de refutarlo.** El resto es opinión.

Y la regla que este proyecto ya pagó caro: **cuando una guarda te contradice, tiene
razón más veces de las que crees.** Seis pararon al implementador el 2026-08-17 y
las seis acertaron.

---

## 2. El reparto: once agentes, con nombre y con dominio

Cada uno abre **sólo su dominio**, y devuelve una lista de hallazgos con
`archivo:línea`, severidad y **el escenario concreto que falla** (entrada → salida
incorrecta). Nada de «se podría mejorar».

| # | Agente | Su dominio | La pregunta que sólo él hace |
|---|---|---|---|
| 1 | **EL ADUANERO** | la frontera C | cada `unsafe`, cada puntero, cada dueño que cruza, **cada `u32` que puede dar la vuelta**. ¿Quién libera esto? ¿Qué pasa si el otro lado ya lo liberó? |
| 2 | **EL CRIPTÓGRAFO** | handshake, nonces, identidad, persistencia de claves | ¿hay un camino que degrade a menos seguridad? ¿un nonce se repite? ¿una clave llega a un log? |
| 3 | **EL CARTERO** | el protocolo en el cable, los cuatro canales | frame malformado, longitud mentida, lectura parcial, `EOF` a mitad, peer que se calla |
| 4 | **EL NOMBRADOR** | rutas y nombres | **el CVE del sector** (`R11` §4): traversal, symlink, junction, `CON`/`NUL`, `:`, 260 caracteres, NFC/NFD que colisiona |
| 5 | **EL CONTADOR** | recursos | descriptores, **memoria O(1) por frame**, buffers sin techo, temporizadores que nadie cancela, `.qyro-part` huérfanos |
| 6 | **EL FORENSE** | la comprobación 14, sobre **todo** | por cada capacidad declarada, **el llamante de producción con archivo y línea**. Si es una prueba, un arnés o nadie, **la capacidad no existe**. Ya lleva **nueve** cadáveres |
| 7 | **EL RETRATISTA** | la interfaz | contraste real, `Semantics` en cada nodo tocable, objetivo ≥48 dp, **estado que se comunica sólo por color**, coste del blur |
| 8 | **EL EMPAQUETADOR** | lo que de verdad se instala | **alineación de 16 KB en el `.so` del APK**, firma, targets, permisos del manifiesto, **y si el artefacto sale del commit que se publica** |
| 9 | **EL BIBLIOTECARIO** | documentos contra código | cada afirmación de `README`, ADR y `STATUS` **verificada contra el código**; avisos de seguridad; licencias de fuentes y crates |
| 10 | **EL ABOGADO DEL DIABLO** | los hallazgos de los otros | **su trabajo es refutar**, no confirmar. §3 |
| 11 | **EL QUE FALTA** | la propia revisión | *¿qué archivo no abrió nadie? ¿qué afirmación quedó sin comprobar? ¿qué canal no miró ningún agente?* Lo que encuentre **es la siguiente ronda** |

**Los nueve primeros van a la vez y ninguno ve el informe de otro.** Que dos
lleguen al mismo defecto por caminos distintos es la mejor señal que existe; si se
leen entre ellos, esa señal se pierde.

---

## 3. Cómo se verifica un hallazgo

Por cada hallazgo de los agentes 1–9:

1. **Tres refutadores con lentes distintas** —corrección, seguridad, *¿de verdad se
   reproduce?*—, cada uno instruido para **empezar suponiendo que el hallazgo es
   falso**.
2. **Sobrevive si dos de tres no consiguen refutarlo.** Los que caen **se apuntan
   igual**, con el motivo por el que cayeron: es la mitad del valor de la revisión.
3. **Un hallazgo confirmado se convierte en una prueba que falla**, y sólo después
   en un arreglo. **Arreglar sin la prueba que falla primero está prohibido**: sin
   ella nadie sabe si el arreglo arregla.
4. **Se repite hasta que dos rondas seguidas no traen nada nuevo.** Un contador
   («busca diez») encuentra diez y para. La cola es donde viven los caros.

**Y nada de topes callados.** Si por contexto se recorta la revisión —menos
agentes, menos rondas, un directorio sin mirar— **se escribe qué se dejó fuera**.
Un informe que no dice lo que no miró se lee como si lo hubiera mirado todo.

---

## 4. Las diez preguntas que hay que responder por código

Esto es lo que el propietario pidió de verdad: **confirmar sin hardware.** Cada
respuesta lleva `archivo:línea` o una prueba que la demuestra.

1. **Un archivo grande que no cabe en RAM, ¿cruza?** Dónde está el bucle que lo
   garantiza y qué prueba lo mide.
2. **Un lote de 200 archivos, ¿cuántos descriptores abre a la vez?** El número, y de
   dónde sale.
3. **Un contador que cruza el FFI, ¿es de 64 bits en todo el camino?** Uno a uno.
4. **Si el peer se calla a mitad, ¿qué temporizador salta y quién limpia?**
5. **Si el disco se llena, ¿qué ve la persona y qué queda en el destino?**
6. **Si el nombre viene envenenado, ¿dónde exactamente se rechaza?** Y **la
   contraprueba: quitando esa línea, ¿falla la suite?**
7. **Si el puerto 49517 está ocupado, ¿qué pasa?** `errno 10013` es real
   (`R11` §2.2) y el formato `QYRO1|ip:port|huella` **ya soporta el respaldo**.
8. **Si Android deniega cada permiso, ¿qué pantalla sale?** Una por una.
9. **El `.so` que va dentro del APK, ¿está alineado a `0x4000`?** Medido sobre el
   APK.
10. **Cada capacidad de la tabla de paridad, ¿tiene llamante de producción en las
    dos caras?** O su `NO -- <argumento>`.

---

## 5. Los entregables

1. **`docs/reports/revision-final.md`** — los once informes, los hallazgos
   confirmados, **y los refutados con su motivo**.
2. **Una prueba que falla por cada hallazgo confirmado**, en su propio commit,
   **antes** del arreglo.
3. **Los arreglos**, y la puerta en verde **en el commit que el informe nombra**
   (comprobación 16).
4. **`docs/reports/lo-que-no-se-ha-probado.md`** — la lista honesta de lo que sigue
   dependiendo de un aparato físico, **con los huecos en blanco**. Es la única
   página que el propietario necesita leer antes de enchufar dos aparatos.
5. **El veredicto, en una frase, con las tres métricas**: fundamentos técnicos /
   producto utilizable / preparación para publicar. **En rangos y con el método.**

---

## 6. Lo que NO hay que hacer

- **No cierres un hallazgo respondiendo a otra pregunta.** Es cómo se perdió la v1.0.
- **No arregles sin una prueba que falle antes.**
- **No conviertas «compiló» en «funciona».** Compilado, probado en unidad, probado
  en integración, probado en ejecución, probado en emulador y probado en hardware
  son **seis cosas distintas** y esta fase sólo puede llegar a la quinta.
- **No inventes evidencia de hardware.** **Un hueco en blanco es la verdad**, y es
  la última vez que hace falta decirlo.
