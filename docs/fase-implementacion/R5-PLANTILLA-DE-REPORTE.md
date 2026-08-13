# R5 — La plantilla del informe de fase

Un informe por fase, en **`docs/reports/fase-NN-<nombre>.md`**.

**Se escribe durante la fase, no al final.** Cada puerta añade su sección antes de
que empiece el paso siguiente. Un informe redactado de memoria al terminar es
exactamente el que contiene «§4 dice 63 y §12 dice 62».

---

## Las dieciséis secciones

### 1. Objetivo y alcance

El objetivo de la fase **en una frase**, copiado del documento de la fase, y los
no objetivos declarados.

### 2. Qué se hizo

Punto por punto contra los objetivos del documento de fase. Sin adornos.

### 3. Cómo se hizo

Las decisiones tomadas y **las alternativas descartadas, con el motivo**. Si la
fase congeló una ADR, el resumen de lo que decide va aquí y el detalle en la ADR.

### 4. Qué se encontró que no estaba en el plan

Tabla: hallazgo, dónde, gravedad, cómo se descubrió. **Todo**, incluidos los
errores propios y los defectos de este plan.

### 5. Qué se arregló y qué no

Tabla, con la ficha `QYR-00xx` de cada uno. **Para los no arreglados: por qué no,
y qué haría falta.**

### 6. A qué afectaba cada defecto

Por cada hallazgo: **qué se rompía, para quién, en qué escenario**. No «podría
causar problemas»: el escenario concreto.

### 7. Resultado contra el objetivo

Objetivo por objetivo: **cumplido / parcial / no hecho**.

**«Parcial» es una respuesta válida. «Cumplido» sin evidencia no lo es.**

### 8. Clase de evidencia por afirmación

Tabla: afirmación → clase (`R3` §5) → **plataforma** → dónde está la evidencia
(test, run de CI, comando).

**Una afirmación sin clase se audita como no probada.**

### 9. Las puertas

Una subsección por puerta, con fecha y las **doce comprobaciones** de `R2` con su
veredicto. Si una quedó **parcial**, se dice y se explica — como se hizo con la
mutación que colgaba en vez de fallar.

### 10. Tabla de mutación

| Control | Mutación aplicada | Resultado | Test que falló | Commit |

Más **el alcance declarado por crate**: cuántos mutantes de cuántos, caught /
missed / unviable / timeout.

**Los supervivientes van en la tabla, no escondidos.** Y las mutaciones que
resultaron mal apuntadas también, con la nota de que no significaban nada: *una
tabla más limpia y menos cierta no sirve.*

### 11. Tests antes y después

Números **con su plataforma** y el comando. Y **una línea por test nuevo**
diciendo qué prueba.

### 12. Delta de dependencias

Paquetes antes y después, con el comando. Si el número no cambió, **dilo
explícitamente y pega el diff vacío de `Cargo.lock`**.

Si entra algo externo: nombre, versión, licencia, **conteo transitivo medido con
`cargo tree`**, `cargo audit`, y la alternativa descartada.

### 13. Archivos tocados

```
git diff --name-only <base-de-la-fase>..HEAD
```

Salida literal. **La base es el commit con el que empezó la fase**, no
`origin/main` — `main` está anterior al sprint 4A y ese diff devuelve cientos de
archivos que no dicen nada.

### 14. Runs de CI

**Todos los de la rama durante esta fase, sin filtrar**, con workflow, commit, ID
y conclusión. **Los fallidos y los cancelados también.**

*Una lista de la que se pueden caer los fallos no es evidencia, es un resumen
favorable.*

### 15. Qué NO debe leerse como progreso

**La sección más importante del informe.** Lo que la fase **no** hace y que alguien
podría suponer que sí.

Al menos: qué sigue sin existir, qué sigue sin probarse, en qué plataformas no
corre nada, y si los botones siguen apagados.

Y siempre, mientras siga siendo cierto: **nada se ha probado en hardware físico,
y dos procesos en `127.0.0.1` no son dos aparatos en una Wi-Fi.**

### 16. Ledger y handoff

- La tabla de fichas de `R4` §9, con el balance de abiertas antes y después.
- **Qué documentación del repositorio quedó desfasada** por lo que hiciste.
- **Qué necesita saber la fase siguiente**: decisiones que la afectan, APIs
  nuevas, cosas que descubriste y no cabían aquí.

---

## Reglas de escritura

- **Números con su comando.** Siempre.
- **Ninguna sección contradice a otra.** Comprobación 9 de la puerta.
- **Ninguna sección dice «pendiente» de algo ya hecho.**
- **Los errores propios se escriben.** Los dos falsos verdes por leer mal la salida
  de un comando están en un informe de este proyecto, escritos por quien los
  cometió, y eso es lo que hace creíble el resto.
- **Sin adjetivos.** «Robusto», «sólido» y «completo» no son evidencia.
