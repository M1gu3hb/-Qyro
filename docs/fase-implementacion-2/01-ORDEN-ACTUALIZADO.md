# Orden actualizado — dónde está esto el 2026-08-17

> Lee esto después de `00-LEEME-PRIMERO.md`. Sustituye a su §3.

---

## 1. Lo que está hecho, medido por mí sobre `5459a64`

| Fase | Estado | Evidencia verificada |
|---|---|---|
| **12** — cerrar la cadena | **HECHA** | Tres P0 arreglados. Un archivo cruza entre dos procesos con la clase de producción de receptora, byte a byte, con control de falsabilidad |
| Los cinco puntos de la auditoría | **HECHOS** | La Release republicada desde `2c01de0` con retractación pública; `deuda-de-calidad.md` reabierta; historial retirado (QYR-0358); cuatro pruebas de contrato para `qyro_session_finish`; `STATUS.md` corregido |
| **13** — el binario de terminal | **HECHA** | `rust/crates/qyro_cli`, 1 077 líneas, 653 KB, estático, `send`/`recv`/`whoami`/`find`/menú, pipeline de cuatro targets, evidencia de contenedor con su clase escrita |
| **14** — sin router | **~40 %** | ADR-0043 congelada. `qyro_session::browse` existe y `qyro find` es **el primer llamante de producción que `MdnsDiscovery` ha tenido nunca**. Falta: la cuenta atrás de APIPA, multicast por interfaz con `socket2`, y el lado Dart de `dev.qyro/discovery` |
| **15** — canal óptico | **~15 %** | ADR-0044 congelada con las cifras de `R8`. Cero código |
| **16** — canal serie | **0 %** | Sin abrir |
| **17–20** | **0 %** | Sin abrir |

**Rust 664 pruebas / 0 fallos** (eran 637 antes de la fase 13), ledger **160 fichas /
0 abiertas**, `main` intacto en `a8bafcf`.

**Un defecto que hay que arreglar el primer minuto:** `check_docs_consistency`
**está en rojo en `5459a64`** — *«Stale verified commit: HEAD is 11 commits ahead of
the verified commit (limit 10)»*. El informe final de la sesión dijo «gate green», y
lo era dos commits antes. La regla de `ESTADO-ACTUAL.md` tras cada paso existe
exactamente para que esto no pase.

---

## 2. El orden nuevo, y por qué cambia

Se añaden **tres fases** y el orden pasa a ser:

```
14 → 15 → 16 → 21 → 22 → 17 → 18 → 19 → 20 → 23
```

**Por qué la 21 y la 22 van antes que la 17 y la 18**, y no al final:

- La **21** prueba que **las dos caras del motor se hablan** — el teléfono manda y el
  PC viejo recibe, que es literalmente la escena de `R7` §2. Hoy nadie ha puesto la
  GUI contra el CLI ni una sola vez. Es el mismo tipo de hueco que la fase 12
  encontró: **dos mitades probadas y la costura nunca ejercitada.** Descubrirlo
  después de empaquetar y firmar sería repetir la historia.
- La **22** es lo que la gente hace de verdad: carpetas, muchos archivos, archivos
  enormes, disco lleno, cancelar y volver. Cualquiera de esas cosas puede cambiar el
  protocolo, y **cambiar el protocolo después de la 18 obliga a reescribir el modelo
  de amenazas dos veces.**
- La **18** —la verdad— tiene que ir después de todo lo que crea afirmaciones nuevas,
  o barre un árbol que va a cambiar.
- La **23** es la etiqueta y sustituye a la parte de release de la 20, que se queda
  con la distribución y la firma.

| Fase | Qué es |
|---|---|
| 14 | Que se encuentren sin router — **la mitad que falta** |
| 15 | El canal óptico: QR animado |
| 16 | El canal serie |
| **21** | **Las dos caras se hablan** — GUI ↔ CLI, y el consejero de canal |
| **22** | **Lo que la gente hace de verdad** — carpetas, tamaño, interrupción |
| 17 | Windows 7 y 32 bits |
| 18 | La verdad: modelo de amenazas y documentos |
| 19 | Hardware: los escenarios de los cuatro canales |
| 20 | Distribución y firma |
| **23** | **La v2.0** |

---

## 3. Dos reglas que cambian

### 3.1 — Quedarse sin contexto es una parada legítima

`R9` §1 lo explica entero. El resumen: la regla anterior decía «sólo para un P0» y
produjo un bucle en el que se gastó el contexto que quedaba discutiendo si se podía
seguir. **Se acabó el contexto es motivo de parada, igual que un P0.** La forma de
parar es **un solo mensaje**: `ESTADO-ACTUAL.md` diciendo dónde se corta, commit,
push, y una frase. No se repite, no se argumenta.

Lo que sigue prohibido es parar **por ordenado**: cerrar una fase con contexto de
sobra y decir «lo siguiente es X». Ahí se abre X.

### 3.2 — La comprobación 16: el gate se corre en el commit que se nombra

> Ninguna afirmación de «gate en verde» vale si no se ejecutó **en el commit que el
> informe nombra**. Si commiteas después de correr la puerta, **la vuelves a correr**.

Sale de que el informe de la fase 13 dijo «gate green» y en `5459a64` está en rojo
por dos commits de documentación posteriores. La afirmación era cierta cuando se
hizo y falsa donde apunta.

---

## 4. Lo que no cambia

La regla del carril · las quince —ahora dieciséis— comprobaciones · ADR congelada
antes del código en su propio commit · dos destinos para una ficha · `main` jamás ·
**no se inventa evidencia de hardware.**

Y lo que se demostró en la sesión del 17: **cuando una guarda te dice que estás
equivocado, tiene razón más veces de las que crees.** Seis guardas pararon al
implementador ese día y las seis acertaron — incluida una que él mismo escribió, y
cuyo fallo destapó que el binario no arranca en Windows 7. Eso no se descubre
razonando; se descubre dejando que una comprobación te contradiga.
