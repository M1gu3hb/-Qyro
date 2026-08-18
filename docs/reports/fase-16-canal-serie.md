# Fase 16 — el canal serie

**Rama** `claude/qyro-cerrar-cadena-12` · **Commit del informe** `5699fcd` ·
**2026-08-18**

**Puerta ejecutada en `5699fcd`, el commit que este informe nombra**
(comprobación 16).

---

## 1. Qué hay

Un archivo entra en una máquina **que no puede leer un QR porque no tiene
cámara**. Es la respuesta literal a la escena de `R7` §2, y el canal óptico no
sirve en esa dirección.

| Lo que ADR-0045 decidió | Estado | Dónde |
|---|---|---|
| §2 — preguntar lo aburrido antes de ofrecer el canal lento | **HECHO**, en la interfaz | `qyro_cli/src/serial.rs:30` |
| §3 — `serialport` sin características por defecto | **HECHO** | `qyro_cli/Cargo.toml` |
| §4 — ARQ con CRC32, y **no** el fountain | **HECHO** | `qyro_serial/src/arq.rs` |
| §5 — receptor tonto en Base64, con `certutil -decode` | **HECHO y ejecutado** | §4 |
| §5.1 — decir lo que se pierde **antes** de mandar | **HECHO** | `serial.rs:78`, `serial.rs:150` |
| §6 — 115 200 8N1, RTS/CTS | **HECHO** | `serial.rs:22` |
| §7 — el teclado PS/2 escrito como nota, no como código | **HECHO** | ADR-0045 §7 |

Tres comandos: `qyro serial [--port]`, `qyro send <archivo> --serial <puerto>`,
`qyro recv --serial <puerto>`.

---

## 2. Comprobación 14 — llamante de producción, con archivo y línea

| Capacidad | Llamante de producción | Consumidor |
|---|---|---|
| `qyro_serial::send_all` | `qyro_cli/src/serial.rs:165` | **CLI** |
| `qyro_serial::receive_all` | `qyro_cli/src/serial.rs:240` | **CLI** |
| `qyro_serial::receiver_for` | `serial.rs:88`, `:100`, `:107` (los tres objetivos) | **CLI** |
| `qyro_serial::DEGRADED_WARNING` | `serial.rs:80` y `serial.rs:150` | **CLI** |
| `qyro_serial::Reply::of` | `serial.rs:174` | **CLI** |
| `qyro_serial::SerialError::Wire` | `serial.rs:166`, `:167` | **CLI** |
| `serialport::available_ports` | `serial.rs:49` | **CLI** |

**Declarado, no olvidado:** el canal serie **no llega a la GUI**. No hay símbolo
en la frontera C y no se anuncia en ninguna pantalla. Es un canal de terminal
para una máquina de terminal, y la GUI no miente diciendo que lo tiene.

---

## 3. Comprobación 15 — del gesto al byte, **con el paso humano marcado**

1. 👤 **La persona conecta el cable** entre las dos máquinas.
2. Escribe `qyro serial`. Lo primero que sale **no es el canal**: es la pregunta
   de ADR-0045 §2 — *¿esa máquina tiene CD, disquetera, PCMCIA o tarjeta de red?*
   Cualquiera es entre 10 y 10 000 veces más rápida.
3. Qyro enumera los puertos con su nombre (`COM3`, `/dev/ttyS0`), porque la
   persona tiene que saber cuál eligió. **Ejecutado en esta máquina:** salen
   `COM3` y `COM4`.
4. `qyro serial --port COM3` imprime **primero la advertencia** — no cifrado, no
   autenticado, y qué sí queda — y después el receptor, **con los valores reales
   puestos**, para PowerShell, para HyperTerminal de XP y para Linux.
5. 👤 **La persona pega el script** en la máquina vieja. Ahí PowerShell abre el
   puerto y se pone a leer líneas.
6. `qyro send archivo --serial COM3` parte el archivo en bloques de **510 bytes**
   —múltiplo de tres, §4.1— y manda una línea por bloque:
   `QS1 <índice> <total> <crc32> <base64>`.
7. El receptor comprueba el prefijo, acumula el campo Base64 y contesta `OK
   <índice>`. Si el CRC no cuadra contesta `NAK` y el emisor **lo vuelve a
   mandar**, hasta cinco veces; a la sexta se rinde **con el número de bloque en
   el mensaje**, que es una frase con la que alguien puede hacer algo.
8. Al terminar, `certutil -decode` reensambla el binario y `certutil -hashfile`
   imprime el SHA-256.
9. 👤 **La persona compara el hash** con el que imprimió Qyro. Es la única
   comprobación de integridad sobre el archivo entero que existe en este modo — y
   **detecta corrupción, no sustitución**, que es exactamente QYR-0359.

---

## 4. El defecto que encontró ejecutar el script generado

**Antes de que se enviara nada, y habría sido invisible.**

`BLOCK_BYTES` era 512. 512 no es múltiplo de tres, así que **cada bloque
codificaba con relleno `=`** — y el receptor concatena los campos Base64 de todos
los bloques en un solo archivo para dar una sola llamada a `certutil`. Al
concatenar bloques rellenados, el `=` queda **en medio del flujo**, que no es
Base64 válido.

Medido, con el `certutil` de verdad:

```
DecodeFile devolvió Datos no válidos. 0x8007000d (WIN32: 13 ERROR_INVALID_DATA)
```

**La transferencia habría informado de éxito y la máquina de enfrente se habría
quedado con nada.** Ninguna prueba interna lo habría visto: el decodificador
Base64 de Qyro trabaja línea a línea y estaba perfectamente de acuerdo consigo
mismo.

510 codifica a 680 caracteres exactos sin relleno. La invariante es ahora
`const _: () = assert!(BLOCK_BYTES % 3 == 0, ...)`, que no se puede optimizar:
no compila.

**Ésta es la lección de la fase, y es la que el documento de fase ya avisaba:**
*un script generado que nadie ha ejecutado es un script que no funciona.*

---

## 5. La puerta se puso en rojo, y no por el código

`cargo audit` pasó a 1: `rqrr` —el decodificador de QR de la fase 15— arrastra
`lru` 0.12.5, con **dos avisos de unsoundness** (RUSTSEC-2026-0002 y -0253).
`cargo update -p lru` no lo mueve: `rqrr` 0.9.3 fija esa minor.

Se ignoran los dos en `.cargo/audit.toml`, con tres cosas escritas: **qué son**
(unsoundness, no vulnerabilidad conocida), **por qué no llegan al producto**
(`rqrr` es `dev-dependency`, verificado con `cargo tree -i lru -e normal,dev`, y
añadirlo movió el binario cero bytes), y **qué los borra** (que `rqrr` acepte
`lru` 0.13, o que alguien lo mueva a dependencia normal).

Y porque **un argumento en un comentario es una promesa que nadie comprueba**,
hay una guarda —`the_qr_decoder_never_becomes_a_shipped_dependency`— que falla si
`rqrr` pasa a `[dependencies]`. Esa guarda **falló sobre sí misma la primera
vez**, porque leía el comentario que explica la excepción como si fuera una
dependencia; tenía razón, y la corrección fue quitar los comentarios antes de
mirar, como hace `production_source` con el Rust.

---

## 6. Clase de evidencia, escrita con precisión

| Qué | Cómo se probó |
|---|---|
| El protocolo entero, ida y vuelta | Dos mitades hablando por una cola en proceso |
| Recuperación con **5 %** de líneas dañadas | Completa, y **los reintentos se aseveran**, no se suponen |
| Línea sin remedio (100 % dañado) | Falla **por su nombre** en `MAX_ATTEMPTS`, sin colgarse |
| `certutil -decode` acepta lo que Qyro escribe | **El `certutil` real, bytes reales, en Windows** |
| Enumeración de puertos | Ejecutada: `COM3`, `COM4` |

**Lo que NO se probó, y no se sube de categoría:** un UART físico y un cable
null-modem. Los dos puertos de esta máquina son endpoints Bluetooth, **no un par
enlazado**, así que no hay transferencia real entre ellos y no se finge que la
haya. Timing, errores de framing, una FIFO desbordándose y un cable con dos hilos
cruzados quedan fuera — y ahí es donde un enlace serie falla de verdad. **Fase 19.**

---

## 7. La puerta, en `5699fcd`

| Comprobación | Resultado |
|---|---|
| `cargo test --workspace` | **739 pruebas, 0 fallos** |
| `cargo clippy --workspace --all-targets -D warnings` | 0 errores |
| `cargo audit --deny warnings` | 122 dependencias, 0 avisos (2 ignorados con argumento y guarda) |
| `flutter test` | 105 pruebas, 0 fallos (sin tocar) |

Rust pasó de **711 a 739**.

---

## 8. Lo que esta fase NO promete

- **Un cable.** Ni un UART. Fase 19.
- **Que el modo degradado esté autenticado.** No lo está, va en claro, y **se
  dice en pantalla antes de mandar** — no en una nota al pie.
- **El canal serie en la GUI.** No hay símbolo en la frontera C y ninguna
  pantalla lo menciona.
- **El emulador de teclado PS/2.** ADR-0045 §7 tiene el cálculo —37–375 B/s, y
  como bootstrap para teclear 2–10 KB son 5–27 segundos— y **necesita un RP2040**,
  así que es de la fase 19. Aquí queda escrito para que no se pierda.
