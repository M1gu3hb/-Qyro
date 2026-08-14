# FASE 10 — v1.0

> **iOS está fuera de la v1.0 (ADR-0039, 2026-08-14).** Xcode exige macOS y este
> proyecto no tiene ninguno, así que la mitad iOS de esta fase queda **aplazada,
> no cancelada**: lo escrito abajo sobre iOS sigue siendo el plan del día que
> exista un Mac y una cuenta de desarrollador. **Lo que esta fase entrega para la
> v1.0 es Android y Windows.**


## 1. Objetivo

**Cerrar el proyecto.** Una versión etiquetada, instalable, documentada, con sus
límites escritos, y con la evidencia de todo lo que afirma.

## 2. Por qué esta fase existe

Porque **«ya está»** no es un estado verificable, y este proyecto no ha aceptado un
estado no verificable en siete meses. Una v1.0 es una afirmación fuerte: dice que
alguien puede usar esto y que sabes qué le va a pasar.

**Depende de:** las nueve anteriores. **Todas cerradas, con sus puertas pasadas.**

## 3. La prueba de aceptación de la v1.0

**Antes de etiquetar nada**, esto tiene que ocurrir y quedar registrado:

> **Dos personas distintas, con dos aparatos físicos distintos, instalando desde
> los artefactos de release —no desde el entorno de desarrollo—, se pasan un
> archivo de más de un gigabyte por una Wi-Fi doméstica, comparando la huella en
> voz alta, sin que ninguna de las dos haya escrito una línea del código.**

Si eso no ocurre, **no es una v1.0**. Es una beta y se llama así.

Y su versión negativa, que importa igual:

> **Una de las dos rechaza una transferencia, y el destino queda intacto.**
> **Una de las dos reinstala su app, la otra ve el aviso de clave cambiada, y lo
> entiende sin que se lo expliquen.**

## 4. Lo que hay que hacer, paso a paso

### Paso 1 — El repaso de coherencia total

**El repositorio entero tiene que decir la verdad al mismo tiempo.** Es la última
oportunidad y este proyecto ya cerró dos veces la clase de defecto «la
documentación dice algo que el código contradice» (QYR-0031, QYR-0067).

- `STATUS.md`, `HANDOFF.md`, `NEXT_STEPS.md`, `CHANGELOG.md`, `README.md`,
  `ARCHITECTURE.md`, `PROTOCOL.md`, `SECURITY.md`, `THREAT_MODEL.md`,
  `TESTING.md`, `ROADMAP.md`, `RELEASES.md`, `DATABASE.md`, `FILE_MAP.md`.
- **Cada uno leído entero, contra el código.** No revisado por encima.
- **`docs/adr/`**: las ADR que fueron superadas se marcan como superadas, con la
  que las sustituye. Ninguna se borra.
- `check_docs_consistency` en Bash y PowerShell.

**Puerta.**

### Paso 2 — El documento que define la v1.0

`docs/release/v1.0.md`. **Es el documento más importante de esta fase** porque es
el que una persona lee para saber qué le estás dando.

Cinco secciones, y la tercera es la que le da valor:

**1. Qué hace.** En lenguaje de persona, no de ingeniero.

**2. Qué garantiza, y con qué evidencia.** Cada garantía con su clase (`R3` §5) y
su plataforma. «Los archivos van cifrados» tiene que poder rastrearse hasta una
prueba.

**3. Qué NO hace, y qué puede salir mal.** Sin adornos:

- No funciona entre redes distintas, ni por internet.
- **No funciona si el router tiene aislamiento de cliente** — y qué hacer entonces
  (el código manual o el QR).
- No hay actualizaciones automáticas.
- **La build de iOS caduca en un año.**
- Qué pasa si pierdes el aparato, si reinstalas, si cambias de teléfono.
- **La deuda conocida de la fase 09, entera.**

**4. Qué permisos pide y por qué.**

**5. Qué datos guarda y dónde.** Identidad, peers conocidos, historial. **Y qué NO
guarda: nada sale del aparato, no hay servidor, no hay telemetría.**

**Puerta.**

### Paso 3 — El modelo de amenazas, actualizado

`THREAT_MODEL.md` existe desde hace meses y describía un sistema que no estaba
construido. **Ahora está construido.**

Reescríbelo contra lo que existe:

- **Contra quién protege:** alguien en la misma Wi-Fi que quiere leer, alterar,
  suplantar o repetir.
- **Contra quién NO protege**, dicho claro: alguien con acceso físico al aparato
  desbloqueado; alguien que controla el sistema operativo; **alguien que consigue
  que compares mal la huella**; y **un atacante con escritura en el directorio
  destino, que ya puede escribir ahí lo que quiera** — lo que Qyro impide es que
  use Qyro para escribir **fuera**.
- **Lo que un observador pasivo aprende:** que hay una transferencia, su tamaño
  aproximado, su duración, y lo que el registro mDNS anuncie. **No** los nombres ni
  el contenido. Confírmalo con la respuesta de la fase 09 §4.6.8.
- **Las ventanas conocidas**, con su ficha: la carrera de los componentes
  intermedios de la ruta (QYR-0072), y lo que la fase 09 haya dejado como deuda.

**Puerta.**

### Paso 4 — La etiqueta y los artefactos

- **Un commit de release**, con todo en verde: los seis workflows, los cuatro
  `check_*`, el barrido.
- **Una etiqueta `v1.0.0`.**
- Los artefactos —APK, `.exe`, IPA— construidos **desde esa etiqueta**, no desde
  una rama, y **con su SHA-256 publicado** para que alguien pueda comprobar que el
  APK que recibió es el que saliste tú.
- `RELEASES.md` con la entrada, y `install.md` de la fase 08 enlazado.

**Puerta.**

### Paso 5 — La instalación limpia, por última vez

**Desde los artefactos etiquetados, en aparatos que nunca tuvieron Qyro.**

- Instalar, arrancar, emparejar, transferir. Los cuatro pasos.
- **Y comprobar el SHA-256 del artefacto antes de instalar**, que es la única
  forma de que ese hash sirva de algo.

**Y ejecutar la prueba de aceptación de §3.**

**Puerta de fase — la última.**

### Paso 6 — El cierre

- `NEXT_STEPS.md` deja de ser una lista de tareas y pasa a ser **qué vendría en una
  v1.1**, con lo que la fase 07 recogió de las dos personas ajenas.
- **Un informe final** que responda a tres cosas: qué se construyó, qué costó, y
  qué se aprendió. Incluidos los errores. **Especialmente los errores** — este
  proyecto encontró un P0 de suplantación por Unicode, una guarda que leía el 43 %
  de un archivo, cinco aserciones que no podían fallar y un ledger que se volvió
  ilegible, y **los encontró porque los buscaba**.

## 5. Las trampas concretas

1. **Etiquetar antes de instalar desde la etiqueta.** El artefacto que probaste
   tiene que ser el artefacto que publicaste.
2. **Un `docs/release/v1.0.md` que sólo dice lo bueno.** La sección 3 es la que le
   da valor al resto. Si sólo dice ventajas, nadie va a creerse las ventajas.
3. **Un `THREAT_MODEL.md` que describe el diseño en vez del sistema.** Ya pasó una
   vez; el punto de esta fase es que deje de pasar.
4. **La prueba de aceptación hecha por quien escribió el código.** No vale.
5. **Llamar v1.0 a algo con un P1 abierto que las notas no mencionan.**
6. **El hash publicado que nadie comprueba.** Compruébalo tú, en el paso 5, antes
   de instalar.

## 6. Criterios de aceptación

1. **La prueba de aceptación de §3 ejecutada y registrada**, con quién, qué
   aparatos, qué red, cuánto tardó, y qué falló.
2. Las dos pruebas negativas de §3 también.
3. Todos los documentos raíz y `docs/adr/` leídos enteros contra el código, con
   las divergencias corregidas.
4. `docs/release/v1.0.md` con sus cinco secciones, **y la tercera tan larga como
   haga falta**.
5. `THREAT_MODEL.md` reescrito contra el sistema que existe.
6. Etiqueta `v1.0.0` sobre un commit con todo en verde.
7. **Artefactos construidos desde la etiqueta, con SHA-256 publicado y
   comprobado.**
8. **Instalación limpia en aparatos que nunca tuvieron Qyro**, en las plataformas
   que estén en el plan.
9. **Ningún P1 abierto que las notas no mencionen.**
10. `RELEASES.md`, `install.md`, `NEXT_STEPS.md` al día.
11. El informe final, con los errores dentro.

## 7. Cómo tiene que quedar el resultado

Una etiqueta en el repositorio, tres archivos con su hash, un documento que dice
la verdad sobre lo que hacen y lo que no, y **dos personas que ya se pasaron un
archivo con ellos**.

## 8. Lo que la v1.0 no es

- No es un producto que compita con nadie.
- No está en ninguna tienda.
- No tiene actualizaciones automáticas.
- **No está auditada por un tercero.** Todo lo que se afirma de su seguridad viene
  de sus propias pruebas, y aunque son inusualmente buenas, **son suyas**. Dilo en
  `v1.0.md`.

## 9. Después

`NEXT_STEPS.md` para la v1.1. Y nada más: **una v1.0 que no se cierra no es una
v1.0.**
