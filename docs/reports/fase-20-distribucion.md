# Fase 20 — distribución y firma

**Rama:** `main` · **2026-08-19**

---

## 1. Qué hay

| Lo que la fase pedía | Estado |
|---|---|
| `BUILD-INFO.txt` por artefacto, con SHA-256 y la firma **en mayúsculas** | **HECHO**, en `cli-builds.yml` |
| `qyro send --self` | **HECHO**, con su control |
| README de instalación de cinco líneas | **HECHO**, `docs/release/INSTALAR.md` |
| La decisión de firma, escrita **para el propietario** | **HECHA, y no tomada** |
| La página de la Release con la advertencia arriba | **NO.** §4 |

---

## 2. `qyro send --self`, y por qué es la respuesta correcta

**La ironía útil:** Qyro existe para meter archivos en máquinas difíciles. Una vez
que hay un Qyro corriendo en una, **puede llevarse a sí mismo a la siguiente** —
unos 800 KB, que por serie son ochenta segundos.

Se resuelve en el análisis de argumentos, no en el flujo de envío, para que ese
flujo siga recibiendo **una ruta** y no tenga dos caminos.

**Con su control**, que es el que impide el desastre silencioso: sin `--self` una
ruta sigue haciendo falta. Un `--self` que se aplicara siempre convertiría
`qyro send informe.pdf` en `qyro send qyro.exe` **sin decir nada**.

---

## 3. La decisión de firma: escrita, no tomada

`docs/release/DECISION-DE-FIRMA.md`. **Cuesta dinero, y el dinero es una de las
cuatro excepciones que el implementador no cruza.**

Lo que el documento aporta: las tres salidas con su coste, qué resuelve un
certificado (**Smart App Control sí; SmartScreen parcialmente y con el tiempo**) y
qué **no** (**AppLocker corporativo, nada**).

**Y una advertencia sobre sí mismo:** el precio que cita es un orden de magnitud,
no una cotización. Un precio de hace meses no es un precio, y decidir con él sería
decidir con un dato inventado.

### Lo que el implementador sí dice, con su argumento

El caso de uso empuja hacia **no firmar**. La máquina que este producto existe
para servir es la que **no puede instalar nada**, y a esa el archivo le llega por
USB —que borra el Mark of the Web, porque FAT32 no tiene dónde guardarlo— o por
el propio Qyro. **En las dos rutas el certificado no cambia nada.**

Firmar compra sobre todo la primera impresión de quien descarga desde GitHub en
una máquina normal, que es un público distinto del que este producto persigue.

**Eso es una opinión sobre un producto, y quien decide es su propietario.**

---

## 4. Lo que NO se hizo

- **La página de la Release no se tocó.** La advertencia de no-firmado tiene
  redacción propuesta en `DECISION-DE-FIRMA.md` §6, lista para copiar. **No se
  publicó porque publicar es una acción hacia fuera** y la Release ya lleva dos
  correcciones de esta semana; una tercera edición sin que nadie haya leído las
  anteriores es ruido.
- **`BUILD-INFO.txt` sólo en el artefacto de Windows.** El job de musl tiene su
  propio `upload-artifact` y no se tocó. **Dicho, no olvidado.**
- **Nada se compró.** Ningún certificado, ninguna cuenta.
