# ADR-0036 — La interfaz de transferencia, y los botones

- **Estado:** aceptada
- **Fecha:** 2026-08-14
- **Fase:** 05, paso 1
- **No es una ADR de tecnología. Es de producto.** Lo que decide es qué ve una
  persona y qué puede hacer, no con qué widget.

---

## 1. La decisión que más importa: qué pasa cuando llega un desconocido

**Nada se acepta solo. Nunca.**

Una transferencia entrante de un peer con veredicto `New` **espera** a que una
persona la acepte, mostrando la huella. Si nadie está mirando, no pasa nada: no
hay temporizador que acepte por cansancio, ni «recordar esta decisión» que
convierta un sí en una regla.

**Por qué no la alternativa.** Aceptar por defecto convierte a Qyro en un buzón
abierto para cualquiera en la misma Wi-Fi. En una cafetería eso es que un extraño
te escribe en el disco. El coste de la decisión correcta es un toque de más; el
coste de la otra es que el producto entero deje de ser lo que dice ser.

**Y `KnownAndChanged` no pregunta: rechaza.** No hay «continuar de todos modos».
Para volver a confiar en esa clave hay que **olvidar** el peer, que es una acción
distinta, en otra pantalla, con su propia confirmación. En SSH un cambio de clave
de host es un aviso a gritos; aquí es un rechazo.

---

## 2. Qué ve el receptor antes de decidir

**Cuatro cosas, y las cuatro antes del primer byte:**

1. **Quién** — la huella agrupada, tal como la formatea el núcleo, y su estado de
   confianza en palabras: *conocido*, *clave cambiada*, *nuevo*.
2. **Cuántos** archivos.
3. **Cuánto** pesan en total.
4. **Cómo se llaman.**

Una pantalla que sólo dijera «¿aceptar transferencia?» está pidiendo permiso para
algo que no ha descrito.

**Y el nombre se pinta como dato, no como texto.** Un nombre hostil no reordena
la línea: se fuerza el sentido de escritura, se recortan los caracteres de control
y se marca visualmente que es un nombre que viene de fuera. La validación del
manifiesto protege **el disco**; la pantalla es otro problema y ya se cerró una
vez esta clase de defecto para el sistema de archivos.

---

## 3. Las cuatro pantallas y sus estados feos

Los estados feos son la parte que decide si esto es un producto o una demo, así
que se enumeran aquí y **cada uno tiene su texto en los dos idiomas**.

| Pantalla | Estados |
|---|---|
| **Peers** | sin peers · sólo manual · un peer **con clave cambiada** · nombre inválido al guardar |
| **Enviar** | sin destino elegido · selector cancelado · destino inalcanzable · **rechazado por el receptor, con el motivo** · fallo de integridad · cancelado |
| **Recibir** | esperando · **oferta de un desconocido** · aceptada y en progreso · rechazada por mí · sin espacio · fallo · terminada, con dónde quedaron los archivos |
| **Historial** | vacío · con entradas · una entrada fallida |

**La entrada manual y el lector de QR están siempre visibles**, no detrás de
«avanzado». Son el único camino que funciona en el 100 % de las redes, y
esconderlos sería esconder la función.

---

## 4. Un peer con clave cambiada se ve distinto

No es un aviso más en la lista. Es:

- **color de error**, no de advertencia;
- **icono propio**, no el mismo con otro tinte;
- **texto que dice qué significa** — «la clave de este aparato ha cambiado» —, no
  un código;
- y **sin acción de enviar**. El botón no está atenuado: no está.

*Un peligro que se parece a los demás elementos de la lista es un peligro que
nadie va a leer.*

---

## 5. Los cinco requisitos de los botones

Los botones `Enviar` y `Recibir` se encienden **sólo** con estas cinco, cada una
con su evidencia escrita en el informe de fase:

1. Dart conduce una transferencia verificada.
2. La persona elige el archivo con el selector de su sistema.
3. Hay un camino para encontrar al otro extremo.
4. La huella se ve y una clave cambiada se rechaza.
5. El receptor puede rechazar.

**Y cuando se encienden, el texto que explicaba por qué estaban apagados se
borra.** Dejarlo sería mentir en la otra dirección.

---

## 6. Los dos idiomas

Español e inglés, **con prueba mecanizada**: toda clave del catálogo existe en los
dos y ninguno tiene claves que el otro no tenga. Una cadena que sólo existe en uno
es una pantalla en blanco para la mitad de los usuarios.

---

## 7. Lo que esta decisión NO incluye

- **Ni temas, ni avatares, ni ajustes.** Lo mínimo para que una persona lo use.
- **Ni notificaciones**, ni ejecución en segundo plano. Una transferencia ocurre
  mientras la aplicación está delante.
- **Ni cola de transferencias.** Una cada vez.
- **Ni «recordar esta decisión»** para aceptar entrantes. §1.
