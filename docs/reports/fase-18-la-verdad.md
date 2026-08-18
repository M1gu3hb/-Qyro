# Fase 18 — la verdad

**Rama:** `main` · **2026-08-19**

**Puerta corrida con `scripts/gate.ps1`**, que ahora ejecuta **seis** comandos de
`ci.yml` en vez de cinco — y no se tocó el script para eso.

---

## 1. Lo que este árbol afirmaba y no era cierto

### La Release prometía cifrado por cuatro canales y lo tiene por dos

`docs/release/v1.0.md` decía, sin acotar:

> **Los archivos viajan cifrados y verificados.**

Es cierto **por la red**. No lo es por el QR ni por el modo degradado del serie:
el fountain **codifica, que no es cifrar** —XOR de bloques con una semilla que
viaja en la propia cabecera— y el receptor de PowerShell escribe lo que llegue.

Corregido a «**Por la red**, los archivos viajan cifrados y verificados», con las
dos excepciones nombradas y un puntero al modelo de amenazas. **Dos palabras que
la frase no tenía y que la hacían falsa para la mitad de los canales.**

### El modelo de amenazas describía un canal de cuatro

`THREAT_MODEL.md` §4.bis, nueva, con los tres que faltaban:

- **El óptico es difusión, no punto a punto.** No hay handshake y **no puede
  haberlo**: la pantalla no ve a la cámara. Una segunda cámara en la habitación
  recibe lo mismo y el emisor **no se entera**; una grabación se decodifica
  después, con calma. Y lo que va por ahí **va en claro**.
- **El serie**: el modo degradado **no autentica nada**, y eso es una fila
  entera. Y la ventaja, porque este documento tiene que ser honesto en las dos
  direcciones: **un cable físico es el canal más privado de los cuatro** en un
  cuarto cerrado.
- **El enlace directo**: la huella viaja también por broadcast ahora, y RFC 3927
  §5 — *«The ARP protocol is insecure»* — significa que **una dirección nunca es
  una identidad**.
- **La cámara como superficie de entrada**, añadida al pedirla una guarda.

---

## 2. La deuda, vaciada donde se podía

| | Estado |
|---|---|
| **D1 — mojibake** | **CERRADA.** 46 secuencias reparadas y 13 a mano |
| **D6 — `cargo doc` sin puerta** | **CERRADA**, y cazó tres enlaces rotos al primer intento |
| D3, D4, D5 | Abiertas, con dueño y motivo |
| D9 (`mdns-sd`), D11 (reanudación óptica) | Con fecha en la fase 19 y la 22 |
| D12, D13 | Cerradas en su sesión |

### D1 costó dos intentos, y los dos enseñan algo

Parecía Latin-1 y era **Windows-1252**: las secuencias traen U+201A y U+2013, que
en Latin-1 **no existen**. Y encima la codificación era **mixta** — aparece
U+009D, un byte que cp1252 **no define** y que se había decodificado como
Latin-1, así que un `encode('cp1252')` a secas revienta justo ahí.

Los 13 últimos no tenían vuelta limpia: los bytes eran `E2 80 E2 80 9D`, un guion
largo estropeado **dos veces**. Se sustituyeron a mano, y `tools/fix_mojibake.py`
**dice en voz alta lo que no puede reparar** en vez de salir en verde.

### D6 se ganó el sueldo en su primera ejecución

Tres enlaces rotos, y el tercero es el que importa: `qyro_identity_store`
documentaba una función **pública** enlazando a un item **privado** — un enlace
muerto para exactamente quien lee los docs generados y no puede ver ese item.

**Y entró sola en la puerta.** `scripts/gate.ps1` lee `ci.yml` en vez de llevar
su propia lista, así que pasó de 5 comandos a 6 sin que nadie tocara el script.
Ésa es la razón por la que se escribió así.

---

## 3. Comprobación 14

| Capacidad | Llamante de producción |
|---|---|
| `tools/fix_mojibake.py` | Herramienta de un solo uso, ejecutada; **queda para el regreso**, y lo dice su cabecera |
| `cargo doc --workspace --no-deps` | `.github/workflows/ci.yml`, job `rust` |

---

## 4. Lo que esta fase NO hizo

- **No barrió los documentos entero contra la comprobación 14.** Se corrigió lo
  que se encontró midiendo, no se recorrió el árbol de documentos afirmación por
  afirmación. Queda dicho como lo que es: incompleto, no hecho.
- **No vació D3, D4 ni D5.** Las tres tienen dueño y motivo escrito; ninguna es
  una afirmación falsa, que es lo que esta fase existía para cazar.
