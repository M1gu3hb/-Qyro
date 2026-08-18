# FASE 18 — La verdad: modelo de amenazas, documentos, y la deuda vaciada

> La fase 09 de la tanda anterior hizo esto y lo hizo bien en casi todo — **salvo en
> una ficha, y esa ficha era el producto entero.** Esta fase existe para hacerlo otra
> vez, con la lección aprendida escrita en la puerta.

---

## 1. Lo que la fase 09 hizo mal, y no se repite

QYR-0322 se cerró **respondiendo a una pregunta distinta de la que hacía**. La ficha
decía «lo que no se puede es preguntarla **a tiempo**» y «**sube en cuanto Dart tenga
que recibir**». El cierre dijo «ahora existe un getter».

**Regla, ya escrita en `00-LEEME` §4, y aquí es donde se aplica:**

> Una ficha se cierra respondiendo a la pregunta que hace. Si contiene «sube»,
> «escala», «cuando», o nombra una fase futura como condición, **no se puede cerrar
> sin comprobar si esa condición ocurrió**, y el resultado de esa comprobación se
> escribe en el cierre.

**Entregable concreto:** un barrido sobre `BUGS_PENDING.md` que **liste todas las
fichas cerradas o descartadas cuyo texto contiene una condición de escalada**, y
que las revise una por una contra el estado actual. No una muestra. Todas.

---

## 2. El modelo de amenazas de los canales nuevos

`THREAT_MODEL.md` describe hoy un canal: TCP autenticado sobre una LAN. **Los tres
canales nuevos tienen adversarios distintos y ninguno está descrito.**

Una fila por cada uno, como mínimo:

### Canal óptico (fase 15)

- **No hay handshake bidireccional.** La pantalla no ve a la cámara. Todo lo que el
  handshake de cuatro mensajes garantizaba hay que reconquistarlo o declararlo
  perdido.
- **Adversarios nuevos y físicos:** una segunda cámara en la habitación; una
  grabación de pantalla; un hombro; una foto de la pantalla tomada a distancia con
  teleobjetivo. **Un QR es un canal de difusión, no un canal punto a punto.** Dilo con
  esas palabras.
- **Qué protege el cifrado del payload y qué no**, según lo que decidiera ADR de la
  fase 15.

### Canal serie (fase 16)

- El **modo degradado** —el receptor es un script de 15 líneas de PowerShell— casi
  seguro **no autentica nada**. Eso es una fila entera del modelo, no una nota.
- Un cable físico es más difícil de interceptar que el aire, y eso **también** hay que
  escribirlo, porque es una ventaja real y el documento debe ser honesto en las dos
  direcciones.

### Enlace directo (fase 14)

- El anuncio de descubrimiento **lleva la huella pública** y la lee cualquiera en el
  enlace. Ya está documentado para mDNS (`THREAT_MODEL.md:152`); confirma que sigue
  siendo exacto para el broadcast nuevo.
- **RFC 3927 §5:** *«The ARP protocol is insecure. A malicious host may send
  fraudulent ARP packets…»* ⇒ **la dirección nunca es identidad.** La identidad es la
  clave que el peer prueba poseer. Escríbelo.

---

## 3. Los documentos, contra lo que existe

Barrido completo, con la comprobación 14 como método: **por cada capacidad que un
documento afirma, el llamante de producción con archivo y línea.**

Objetivos concretos, además de lo que aparezca:

- **`STATUS.md`** — el archivo canónico. En la auditoría del 2026-08-17 su línea de
  Milestone contradecía su propio cuerpo setenta y cinco líneas más abajo. Reescríbelo
  contra lo que existe.
- **`docs/release/`** — la página que un usuario lee para decidir si instala. Cada
  viñeta necesita su llamante.
- **`README.md`**, **`ARCHITECTURE.md`**, **`PROTOCOL.md`**, **`SECURITY.md`**,
  **`ROADMAP.md`**, **`NEXT_STEPS.md`**, **`HANDOFF.md`**.
- **Las ADR superadas, marcadas como superadas**, con la que las sustituye.
- **`docs/release/v1.0.md` §7** dice «Dependencias externas de Rust: **una**». Son 66
  paquetes no-`qyro` en `Cargo.lock`. Lo que se quiere decir es «una añadida en esta
  release». Este error lleva semanas en el proyecto y ha llegado a una página pública.

---

## 4. La deuda, vaciada

`docs/reports/deuda-de-calidad.md` se reabrió en la fase 12 y **se vacía aquí.** Dos
destinos y ninguno más: **cerrada con la evidencia ejecutada**, o **descartada con el
argumento de por qué la versión sale sin ello**. Nunca «pendiente».

Lo que ya está en la lista desde la auditoría:

- **El mojibake de `rust/crates/qyro_session/src/session.rs`** — 30 secuencias
  `Ã¢â‚¬â€` y `Ã‚Â§` en doc-comments: UTF-8 leído como Latin-1 y reescrito. Único
  archivo afectado del árbol. **Y una comprobación de puerta nueva que lo cace**, por
  código de salida, porque ninguna de las trece lo miró.
- **Los dos `Estado:` duplicados** de QYR-0088 y QYR-0089: llevan `cerrado` seguido de
  un `abierto` viejo sin borrar. El script del proyecto lee la primera coincidencia y
  por eso dice 0 abiertas — correcto —, pero un lector ingenuo lee 2. **El archivo
  permite dos lecturas y eso es un defecto de archivo.**
- **Las cifras de test que no concuerdan entre artefactos:** la etiqueta dice «Dart
  101», `ESTADO-ACTUAL.md` y `docs/release/v1.0.md` dicen «92 pasadas, 9 saltadas».
  Son el mismo número con y sin la biblioteca nativa. **Elige una forma de decirlo y
  úsala en los tres sitios.**

---

## 5. La prueba que cierra la fase

- `check_docs_consistency` en verde por código de salida, **con las comprobaciones
  nuevas dentro**: mojibake, `Estado:` duplicado, y la tabla de llamantes.
- **Cero fichas abiertas** por el script Python de `R2` §1.10, y **cero fichas con dos
  `Estado:`**.
- El barrido de §1 ejecutado sobre **todas** las fichas con condición de escalada, con
  la lista y el veredicto de cada una en el informe.

---

## 6. Lo que NO hay que hacer

- **No abras trabajo nuevo aquí.** Si el barrido encuentra un P0, párate y arréglalo;
  cualquier otra cosa se cierra o se descarta con argumento.
- **No borres un cierre equivocado.** Se añade el nuevo debajo. Un cierre equivocado
  documentado es más valioso que uno borrado — es cómo se aprendió esta regla.
