# 04 — El plan final: de aquí al 100 % de código

> Escrito el 2026-08-19 sobre `main` en `2f117b6` — **522 commits, una sola rama,
> todos a nombre del propietario**, ledger **167 fichas / 1 abierta (QYR-0365)**,
> Linux **766 pruebas / 0 fallos**, `v1.0.0` → `98200e13` intacto.
> Sustituye a `03-EL-CAMINO-AL-99.md` §1.

---

## 1. Qué significa «el 100 %», dicho sin trampa

El propietario pidió: *«cien por ciento de código, sin errores, sin bugs,
totalmente funcional, en Windows y Android. La prueba real aún queda pendiente.»*
**Esa última frase es la que hace la definición honesta:**

> **100 % = todo lo que se puede escribir, verificar y demostrar sin enchufar dos
> aparatos, hecho.** La prueba en hardware **no entra en el 100 % de código** — es
> la fase 19, y sólo la puede ejecutar el propietario.

Y hay **seis clases de evidencia**, no una. Esto se puede llevar a la quinta:

`compilado` → `probado en unidad` → `probado en integración` →
`probado en ejecución (dos procesos)` → `probado en emulador` → **`probado en
hardware`** ← ésta no.

**El 100 % de código se alcanza cuando las cinco primeras están llenas para cada
capacidad de la tabla de paridad, y la sexta está en blanco y dice que lo está.**

---

## 2. Qué queda, exactamente

| # | Qué | Fase |
|---|---|---|
| 1 | **QYR-0365**: 200 archivos cruzan sin mentir, y sin subir un timeout | 25 |
| 2 | **Lotes, carpetas y texto** — lo que 13 de 13 proyectos tienen y Qyro no | 25 |
| 3 | **El aislamiento del destino** — el CVE 7.5 del sector, antes de la primera carpeta | 25 |
| 4 | **La interfaz entera**: azul eléctrico, vidrio, Matrix por tipografía, accesible | 26 |
| 5 | **Tile, share sheet y ventanita** — dos de ellas nadie del sector las tiene | 27 |
| 6 | **La alineación de 16 KB** del `.so` dentro del APK | 27 |
| 7 | **La revisión final con once agentes** y el veredicto por código | 28 |
| 8 | **El APK reconstruido y republicado** — desbloqueado: las licencias ya están aceptadas | cierre |
| 9 | **Alguien manda un archivo de verdad** | 19 — **el propietario** |
| 10 | **La v2.0** | 23, y su condición de entrada es (9) |

**El orden es 25 → 26 → 27 → 28 → cierre.** Sin parar entre ellas.

**Por qué ése:** la 25 puede cambiar el protocolo (lotes, texto), y **cambiar el
protocolo después de diseñar las pantallas obliga a rehacerlas**. La 26 crea el
sistema de diseño que la 27 necesita para la ventanita. La 28 no puede revisar lo
que aún no existe.

---

## 3. Los once agentes, y cuándo se disparan

La lista entera, con dominio, está en `FASE-28` §2. Resumen para el arranque:

**En la fase 28, los nueve primeros a la vez y ciegos entre sí:**
**EL ADUANERO** (frontera C) · **EL CRIPTÓGRAFO** (handshake e identidad) ·
**EL CARTERO** (protocolo en el cable) · **EL NOMBRADOR** (rutas y nombres) ·
**EL CONTADOR** (recursos) · **EL FORENSE** (comprobación 14) ·
**EL RETRATISTA** (interfaz) · **EL EMPAQUETADOR** (lo que se instala) ·
**EL BIBLIOTECARIO** (documentos contra código).
Después **EL ABOGADO DEL DIABLO** ×3 por hallazgo, y **EL QUE FALTA** al cerrar
cada ronda.

**Y antes de la 28 también se usan, pero sueltos y con encargo corto:**

- **EL NOMBRADOR**, en la fase 25, **antes** de aceptar la primera carpeta.
- **EL RETRATISTA**, en la fase 26, sobre los nueve primitivos.
- **EL EMPAQUETADOR**, en la fase 27 paso 0, sobre el `.so` del APK.

**La regla que los hace valer:** ninguno confirma su propio hallazgo. **Un hallazgo
sin `archivo:línea` no existe; uno que no aguanta tres intentos de refutación, no
sobrevive.**

---

## 4. Las reglas de trabajo, y las tres que cambian

**Lo que no cambia:** ADR congelada antes del código, en su propio commit · dos
destinos para una ficha, HECHA o cerrada con argumento · la comprobación 14 antes
de escribir el informe · la 16, el gate en el commit que el informe nombra ·
**no se inventa evidencia de hardware**.

**Lo que cambia:**

1. **Todo va a `main`. Nunca una rama.** Y el autor de cada commit es
   `M1gu3hb <118588634+M1gu3hb@users.noreply.github.com>`. **Nunca `Claude`, nunca
   `Co-Authored-By`.** Jamás force-push, jamás reescribir historia.
2. **`ESTADO-ACTUAL.md` se actualiza *dentro* del commit de contenido.** El commit
   `chore(status)` aparte fue idea mía y produjo 28 commits de 78 que no eran
   trabajo, cada uno disparando tres workflows. Se acabó.
3. **El calibre de la verificación se ajusta al riesgo.** Alto (cripto, protocolo,
   frontera C, `unsafe`, identidad): todo el ceremonial. Medio (motor, canales, CI):
   ADR sólo si decide algo. **Bajo (pantallas, textos, Kotlin de interfaz,
   empaquetado, documentos): compila + una prueba que falle sin el cambio + la
   puerta. Sin ADR, sin barrido, sin deliberación.** La fase 26 es casi toda de
   calibre bajo y por eso puede ir rápida.

**Parar:** sólo un P0 o **quedarse sin contexto**. Quedarse sin contexto es
legítimo y se anuncia en **un** mensaje: `ESTADO-ACTUAL.md` dice dónde se corta,
commit, push, una frase. **No se argumenta.** Lo que sigue prohibido es parar
*por ordenado*: cerrar una fase con contexto de sobra y decir «lo siguiente es X».
Ahí se abre X.

---

## 5. La puerta pasa a veinte

A las dieciocho de hoy se añaden dos, las dos de esta tanda:

- **19 — sin literales de color fuera de `lib/design/`.** Un `grep` de `Color(0x`
  rompe la build. Es lo único que impide que el sistema de diseño se pudra.
- **20 — alineación de 16 KB en el `.so` *extraído del APK*.** No sobre el que
  salió de `cargo`: sobre el que se instala. Este proyecto ya publicó una vez un
  artefacto que no era el que decía.

---

## 6. Cómo se cierra

1. La revisión final en verde, **con su lista de lo refutado**.
2. **`docs/reports/lo-que-no-se-ha-probado.md`** — los huecos de hardware, en
   blanco y diciendo que lo están.
3. **El APK reconstruido desde el commit que se publica** (QYR-0359 costó una
   retractación pública aprender esto) y **la Release rehecha**, con la advertencia
   proporcional: lo que no se ha probado va **antes** de lo que sí.
4. **El veredicto en tres métricas, en rangos y con el método**: fundamentos
   técnicos / producto utilizable / preparación para publicar.

Y la que decide de verdad, que no la firma ningún agente:

> **La v2.0 no se etiqueta hasta que una persona haya mandado un archivo con esto,
> en hardware, y esté escrito quién, cuándo y por qué canal.** Es lo único que la
> v1.0 no tenía y por lo que su etiqueta valía menos de lo que parecía.
