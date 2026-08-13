# FASE 09 — Cierre de deuda y endurecimiento

## 1. Objetivo

**Que no quede nada abierto que impida llamar a esto v1.0 sin mentir.**

## 2. Por qué esta fase va aquí

**Depende de:** fases 01 a 08.

Va al final por un motivo concreto: **la mitad de las fichas abiertas hoy sólo se
pueden cerrar cuando existe producto.** «No hay evidencia en hardware físico» no
se cierra construyendo; se cierra probando. «`TransferReject` no lo emite nadie» se
cerró en la fase 05 porque la UI lo necesitaba.

Y porque las fases 01 a 08 **van a abrir fichas nuevas**. Esta fase se enfrenta a
la lista real, no a la de hoy.

## 3. Estado de partida

**Al empezar esta fase, cuenta:**

```
python3 - <<'PY'
import re
t=open('BUGS_PENDING.md').read()
b=[x for x in re.split(r'\n(?=## QYR-)',t) if re.match(r'## QYR-',x)]
op=[x for x in b if re.search(r'- Estado: *abierto',x)]
print('total',len(b),'abiertas',len(op))
from collections import Counter
print(Counter((re.search(r'- Severidad: \*{0,2}(P\d)',x) or [None,'?'])[1] for x in op))
PY
```

**Al empezar el plan había 24 abiertas: 5 P1, 13 P2, 6 P3.** Anota cuántas hay
ahora y de dónde salieron las nuevas.

### Las que vienen de antes del plan y siguen aquí

| ID | Sev | Qué | Dónde debería haberse cerrado |
|---|---|---|---|
| QYR-0004 | P1 | Builds no retenidos | Fase 08 |
| QYR-0005 | P1 | Auditorías y suites avanzadas no disponibles | **Aquí** |
| QYR-0064 | P1 | Harness de Android Keystore | Fase 06 |
| QYR-0078 | P1 | `qyro_net` en Windows | Fase 01 |
| QYR-0295 | P1 | La materialización no prueba todas sus barreras de integridad | **Aquí** |
| QYR-0088 / 0089 | P2 | `FileSink::abandon` / `TransferReject` | Fase 05 |
| QYR-0290, 0292, 0294, 0296 | P2 | Contratos de frontera del decoder, del manifest, de E/S y del sealer | **Aquí** |
| QYR-0052, 0053, 0054, 0056 | P2 | Guardas de material de clave y de `forbid(unsafe_code)` | **Aquí** |
| QYR-0065, 0066 | P2 | Errores de Keystore sin medir | Fase 06 |
| QYR-0001 | P2 | Referencia visual de scramble | **Aquí o descartada** |
| QYR-0059 | P3 | DPAPI no autentica todos los bytes de su blob | **Aquí o descartada** |
| QYR-0069 | P3 | Un crate externo no puede construir un handshake determinista | **Descartar con argumento** |
| QYR-0090 | P3 | Una prueba se cuelga bajo mutación en vez de fallar | **Aquí** |
| QYR-0003, 0057, 0092 | P3 | Higiene | **Aquí** |

**Si alguna de las que debían cerrarse en fases anteriores sigue abierta, eso es
lo primero: significa que una fase se declaró cerrada sin estarlo.**

## 4. Lo que hay que hacer, paso a paso

### Paso 1 — El triaje honesto

**Cada ficha abierta acaba en uno de tres sitios, y ninguno es «sigue abierta».**

- **Se cierra** — con prueba, y con la mutación que la demuestra.
- **Se descarta** — con el motivo escrito. Descartar es legítimo: QYR-0001 es una
  referencia visual y QYR-0069 es un constructor determinista público que **no
  debería existir en producción**. Descartar bien es mejor que arrastrar.
- **Se declara deuda conocida de la v1.0** — con su motivo, y **entra en las notas
  de la release**. Una deuda que el usuario puede encontrarse y que no está escrita
  es una garantía falsa.

**Escribe la tabla de triaje antes de arreglar nada.**

**Puerta.**

### Paso 2 — Los P1

**Ningún P1 llega a la v1.0 abierto sin que las notas de la release lo digan.**

- **QYR-0005** —«auditorías y suites avanzadas no disponibles»— es de
  infraestructura y probablemente se cierra habilitando lo que falte en CI, o se
  descarta con el motivo.
- **QYR-0295** —«la materialización no prueba directamente todas sus barreras de
  integridad»— es sustancial: hay barreras en `qyro_fs` cuya prueba es indirecta.
  **Escribe las directas.**

**Puerta.**

### Paso 3 — Los contratos de frontera

QYR-0290, 0292, 0294 y 0296 son la misma familia: **los límites internos existen y
no tienen contratos directos.** Un límite que sólo se ejercita de refilón es un
límite que se puede mover sin que nada lo diga.

Por cada uno: **una prueba que ataque el límite exacto** —justo debajo, justo
encima, y el desbordamiento— y la mutación que la demuestra.

**Puerta.**

### Paso 4 — Las guardas de material de clave

QYR-0052, 0053, 0054 y 0056 son de la misma familia y llevan abiertas desde el
sprint 4D.1: **la guarda que impide que material de clave salga por un camino
público no ve `Vec<u8>` ni `String`**, y hay tres más de la misma clase.

**Es la propiedad más antigua de este proyecto y su guarda tiene agujeros
conocidos.** Ciérralos, o escribe por qué no se puede y qué queda descubierto.

**Puerta.**

### Paso 5 — El barrido final, completo

- **`cargo-mutants` sobre todo el workspace**, con `--timeout 90`.
- Clasificación **por familia** (`R3` §3), no una por mutante.
- **La salida a `docs/reports/mutation-sweep-final.md`**, con alcance declarado por
  crate. **Nunca al ledger** (`R4` §1).
- Las tres familias de riesgo —validadores, material de clave, ramas de error—
  cerradas o con ficha juzgada.
- **Compara con `mutation-sweep-2026-08-11.md`**: ¿bajaron los supervivientes, o
  subieron porque hay más código? **Di el número de las dos veces.**

**Puerta.**

### Paso 6 — La auditoría de seguridad de extremo a extremo

**No es un barrido: es leer el sistema completo por primera vez desde que existe
entero.**

Ocho preguntas, cada una respondida por escrito con la evidencia:

1. **¿Puede algo que llegue por el socket hacer que el proceso reserve memoria sin
   límite, o entre en un bucle?** Antes y después del handshake.
2. **¿Puede un peer escribir fuera del directorio destino?** Por ruta, por
   symlink, por junction, por el `.qyro-part`, por el `.qyro-resume`.
3. **¿Puede material de clave salir por un camino público, un log, un mensaje de
   error, o un panic?** Incluido el `logcat` de un aparato real.
4. **¿Puede un peer hacerse pasar por otro?** Handshake, confianza, huella, y el
   caso de clave cambiada.
5. **¿Se repite algún nonce, en algún camino, incluida la reanudación?**
6. **¿Qué pasa si el disco se llena a mitad?** Y si el archivo es más grande que
   el espacio libre — **¿se comprueba antes de empezar?**
7. **¿Qué queda en disco tras cada uno de los cinco finales?** En las tres
   plataformas.
8. **¿Qué se ve en la red?** Lo que anuncia mDNS, el tamaño de los frames, los
   tiempos. **Un observador pasivo no debería aprender qué archivos se mandan.**

**Cada respuesta es una sección del informe con su evidencia. Una respuesta que sea
«creo que no» es una ficha.**

**Puerta.**

### Paso 7 — El rendimiento, medido

**Este proyecto nunca ha medido velocidad**, a propósito: «un cronómetro en un
runner compartido mide el runner». **En hardware físico sí se puede.**

- MB/s en Wi-Fi real, Android ↔ Windows, con un archivo de ≥1 GiB.
- Memoria sostenida en el teléfono — el motor promete que no crece con el archivo;
  **compruébalo en un aparato**.
- Batería y temperatura durante la transferencia grande.
- Tiempo hasta que aparece un peer.

**Sin objetivos inventados.** Se mide, se escribe, y si algo es inaceptable se
convierte en ficha.

**Puerta de fase.**

## 5. Las trampas concretas

1. **Cerrar fichas sin prueba, para bajar el número.** El número no importa; la
   lista importa.
2. **Descartar sin motivo.** Un descarte sin argumento es una ficha borrada.
3. **Volcar el barrido en el ledger.** Ya costó un P1. `R4` §1.
4. **Declarar la auditoría de §4.6 hecha sin responder las ocho preguntas por
   escrito.** «Revisé la seguridad» no es una respuesta.
5. **Medir el rendimiento en un emulador y llamarlo rendimiento.**
6. **Dejar un P1 abierto sin que las notas de la release lo digan.**

## 6. Criterios de aceptación

1. **Tabla de triaje completa**: cada ficha abierta en cerrada, descartada o deuda
   conocida declarada. **Ninguna se queda como «abierta» sin categoría.**
2. **Ningún P1 abierto sin estar en las notas de la release.**
3. Los contratos de frontera de QYR-0290, 0292, 0294 y 0296 escritos, con su
   mutación.
4. Las guardas de material de clave cerradas, o el hueco declarado.
5. **Barrido final de todo el workspace**, con alcance declarado, salida en un
   informe, y **comparación con el barrido anterior**.
6. **Las ocho preguntas de §4.6 respondidas por escrito con evidencia.**
7. **Rendimiento medido en hardware físico**, con los números escritos y sin
   objetivos inventados.
8. `cargo audit --deny warnings` en verde, con la fecha de la base de advisories.
9. `R2` en todas las puertas. Informe según `R5`.
10. **El ledger legible**: menos de 40 abiertas, todas con título que una persona
    entiende.

## 7. Cómo tiene que quedar el resultado

Una lista de deuda que cabe en una pantalla, donde **cada línea está ahí a
propósito** y alguien puede explicar por qué.

Y un documento de auditoría de seguridad con ocho respuestas, cada una con la
evidencia detrás.

## 8. No objetivos

- Funcionalidad nueva. **Si en la fase 07 alguien pidió algo, es una ficha para
  después de la v1.0.**
- Refactorizar por gusto.
- Optimizar sin un número que lo pida.

## 9. Qué desbloquea

La fase 10. Y la posibilidad de decir «v1.0» sin que sea una extrapolación.
