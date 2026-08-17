# FASE 21 — Las dos caras se hablan

> Es **la escena de `R7` §2, entera y por primera vez**: el teléfono manda, el PC
> viejo recibe en una terminal. Hoy nadie ha puesto la GUI contra el CLI ni una sola
> vez.

---

## 1. Por qué existe, y por qué no puede esperar

Desde la fase 13 el motor tiene **dos consumidores**: la GUI de Flutter, que cruza el
FFI, y el CLI de Rust, que llama a `qyro_session` directamente. ADR-0042 lo dijo y lo
aceptó: *«habrá dos consumidores del motor, y una capacidad no está hecha hasta que
los dos la alcanzan».*

**Y la costura entre ellos no se ha ejercitado nunca.**

Ése es exactamente el hueco que produjo los cuatro defectos de este proyecto —
`KeystoreWrapper`, `qyro_session_local_address`, `Session::finish`, `history()` —
todos con las dos mitades probadas y el medio jamás recorrido. `Session::finish` se
encontró porque alguien puso por primera vez un receptor de Dart contra un emisor
real. **Aquí falta el equivalente en las cuatro combinaciones.**

La matriz que nadie ha ejecutado:

| | recibe GUI | recibe CLI |
|---|---|---|
| **manda GUI** | ¿? | **la escena de R7 §2** |
| **manda CLI** | ¿? | ¿? |

---

## 2. La decisión que hay que congelar

`docs/adr/ADR-00XX-dos-consumidores.md`. Decide:

1. **Qué significa «una capacidad existe».** Propuesta, y si la cambias escribe por
   qué: **una capacidad existe cuando los dos consumidores la alcanzan, o cuando está
   escrito en un documento de producto que es de uno solo y por qué.** Nada queda en
   el medio.
2. **Dónde vive la tabla de paridad**, y que la puerta la lea por código de salida.
   Una tabla en prosa se desincroniza; una tabla que un script comprueba, no.
3. **El consejero de canal.** Las fases 14, 15 y 16 le dicen cada una al usuario algo
   distinto sobre qué camino usar, y eso son tres interfaces contradictorias esperando
   a existir. **Un solo lugar decide** el orden —red > cable directo > serie >
   óptico—, estima el tiempo con las cifras de `R8` §4 y §5.1, y dice lo aburrido de
   `FASE-16` §2 antes de proponer nada lento. Decide dónde vive: probablemente en
   `qyro_session`, para que las dos caras lo compartan y no diverjan.

---

## 3. Entregables

1. **La ADR de §2, congelada en su propio commit.**
2. **La tabla de paridad GUI/CLI**, comprobada por script:

   | Capacidad | GUI | CLI |
   |---|---|---|
   | Mandar por código tecleado | | |
   | Recibir y enseñar su código | | |
   | Ver la huella antes de aceptar | | |
   | Rechazar con motivo | | |
   | Peer con clave cambiada, rechazado por nombre | | |
   | Cancelar a mitad | | |
   | Descubrimiento (fase 14) | | |
   | Canal óptico (fase 15) | | |
   | Canal serie (fase 16) | | |

   Una celda vacía es una decisión, no un olvido: **o se llena o se escribe por qué
   esa cara no la tiene.**
3. **El consejero de canal**, un solo módulo, con las dos caras llamándolo.
4. **Los mismos textos.** Un error que en la GUI dice «la clave de este aparato ha
   cambiado» y en el CLI dice `code -7` son dos productos. Decide si comparten tabla
   —y cómo, porque la GUI vive en `.arb`— o si el core devuelve el texto ya formado,
   que es el precedente de `HumanFingerprint::to_grouped_hex`.

---

## 4. La prueba que cierra la fase

> **Las cuatro casillas de la matriz, ejecutadas.** Cada una: un archivo cruza,
> verificado byte a byte en destino, usando **sólo** el código que el receptor
> publicó.

En CI eso son dos procesos por casilla: el binario `qyro` de un lado y, del otro, la
misma clase de producción que la GUI usa —`NativeTransferService` bajo `flutter test`,
como ya hace `two_process_pairing_test.dart`—. **No un arnés.** El arnés es lo que
escondió el defecto de la identidad cinco fases seguidas.

**Controles, los tres:**
1. Cada casilla, con el receptor **no escuchando**, falla **por nombre** y con un
   final distinto.
2. Un peer **cuya clave cambió** es rechazado **en las cuatro casillas**, por nombre.
   Es la garantía de seguridad del producto y no puede valer sólo en una cara.
3. **La tabla de paridad, con una fila borrada a propósito, hace fallar el script.**
   Una tabla que nadie ha visto fallar no es una comprobación.

---

## 5. La puerta

Dieciséis comprobaciones. Y la 14 —el llamante de producción— se aplica **por cara**:
una capacidad con llamante en la GUI y ninguno en el CLI **es una celda vacía de la
tabla**, no una capacidad hecha.

---

## 6. Lo que NO hay que hacer

- **No hagas que el CLI cruce el FFI.** ADR-0042 decidió lo contrario con argumento;
  esta fase mide la consecuencia, no la revierte.
- **No unifiques las interfaces.** Una terminal y una pantalla táctil no se parecen y
  no deben. Lo que se unifica son **las decisiones y los textos**, no la forma.
- **No añadas capacidades para llenar la tabla.** Una celda que dice «la GUI no
  necesita esto, y por qué» es una respuesta completa.
