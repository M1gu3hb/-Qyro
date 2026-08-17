# Fase 13 — `qyro` en la terminal

**Base:** `2c01de0`. **Rama:** `claude/qyro-cerrar-cadena-12`.

---

## 1. Objetivo

> **La segunda cara del motor.** R7 §2: *«en su terminal pongo el comando, listo,
> sale el logo, le das en recibir o enviar»*. La máquina de esa escena no puede
> instalar nada y puede no tener Windows 10.

---

## 2. Qué existe, medido

```
  THIS DEVICE
  fingerprint  d4c0d5f2-1c6a62eb-7dd2ac84-453e5c2f
  pairing code -- read this to the other device:
    QYRO1|192.168.100.136:49517|d4c0d5f21c6a62eb7dd2ac84453e5c2f
```

| | |
|---|---|
| Binario | `qyro`, **653 KB** en `x86_64-pc-windows-msvc` |
| Comandos | `send`, `recv`, `whoami`, `help`, y el menú sin argumentos |
| Sin TTY | Se niega y **nombra la bandera** que hacía falta |
| Idiomas | Inglés, y la pantalla **lo dice** (ADR-0042 §6) |
| Pruebas | 21 en el crate |

**653 KB y no 750–950.** La cifra de `R8` §6 es para un binario que lleva QR y
serie; éste no lleva ninguno todavía. Un número menor es correcto aquí, y el
workflow falla por encima de 1500 KB — que es donde un número grande deja de ser
«todavía no» y pasa a ser «entró algo que nadie eligió».

---

## 3. Comprobación 14 — el llamante de producción

**Desde esta fase hay dos consumidores del motor**, y la tabla lleva columna,
porque una capacidad puede estar viva en uno y muerta en el otro — que es
exactamente cómo se rompió la v1.0.

| Capacidad | Símbolo de `qyro_session` | Consumidor | Llamante | Archivo:línea |
|---|---|---|---|---|
| Abrir identidad | `open` | **CLI** | `flows::ensure_identity` | `flows.rs:59` |
| Huella propia | `fingerprint` | **CLI** | `flows::whoami` | `flows.rs:103` |
| Parsear código | `parse_pairing` | **CLI** | `flows::address_of` | `flows.rs:~310` |
| Abrir emisor | `Session::open_sender` | **CLI** | `flows::send` | `flows.rs:169` |
| Abrir receptor | `Session::open_receiver` | **CLI** | `flows::receive` | `flows.rs:~245` |
| Huella del peer | `Session::peer_fingerprint` | **CLI** | `flows::send`, `flows::receive` | `flows.rs:177`, `~252` |
| Avanzar | `Session::step` | **CLI** | `flows::drive` | `flows.rs:~293` |
| Progreso | `Session::progress` | **CLI** | `flows::drive` | `flows.rs:~300` |
| **Materializar** | **`Session::finish`** | **CLI** | `flows::receive` | `flows.rs:~272` |

`Session::finish` **se llama desde el primer día en este consumidor**, y es
deliberado: fue la capacidad que la GUI anunciaba y no alcanzaba (QYR-0357).

**Filas con «ninguno», dichas y no escondidas:**

| Capacidad | Consumidor | Llamante |
|---|---|---|
| `TrustBook` / veredictos | CLI | **ninguno.** El CLI enseña la huella y pregunta; no recuerda peers entre ejecuciones |
| Descubrimiento | ambos | **ninguno.** Fase 14 |
| Historial | ambos | **ninguno.** Retirado de la GUI (QYR-0358) |

El CLI sin `TrustBook` es una decisión, no un olvido: un binario que se copia y
se ejecuta no tiene dónde guardar un libro de peers sin escribir en la máquina, y
`R7` §3 dice que no instala nada. La huella se enseña y la persona compara.

---

## 4. Comprobación 15 — la cadena desde el gesto

**Recibir, desde el comando hasta el byte:**

1. Una persona teclea `qyro` → `parse` devuelve `Command::Menu` → hay TTY → menú.
2. Pulsa `2` → `flows::receive(None, None, vt)`.
3. `ensure_identity()` → `qyro_session::open` con el blob **junto al ejecutable**
   —no en `%APPDATA%`: algo que escribe ahí se ha instalado, se llame como se
   llame.
4. `whoami(vt)` imprime la huella y el código **antes de ligar** (ADR-0041).
5. `Session::open_receiver` en `0.0.0.0:49517`, liga y espera.
6. Alguien conecta → `peer_fingerprint()` → se imprime → **se pregunta**.
   Sin `--expect`, nadie acepta por la persona (ADR-0036 §1).
7. `drive()` → `step()` en bucle, barra con `\r`.
8. **`session.finish()`** verifica el digest y renombra el `.qyro-part`.
9. Se dice **cuántos archivos y dónde quedaron**.

**Enviar:** `qyro send a.txt --to QYRO1|…` → `address_of` → `parse_pairing` →
`open_sender` → se imprime la huella del otro → `--expect` la compara o la
persona la lee → `drive()` → `sent.`

Sin saltos.

---

## 5. Lo que el control destapó

`+crt-static` **no es observable** en el conjunto de imports de este toolchain:
con el flag y sin él, imports idénticos y hashes distintos. Es R8 §6 reproducido,
y una comprobación que pasa igual con y sin la cosa que dice comprobar no prueba
nada — así que el script dejó de afirmarlo (QYR-0360).

Lo que sí destapó: **el binario de hoy no arranca en Windows 7.** Importa
`api-ms-win-core-synch-l1-2-0.dll`, que es Windows 8 mínimo, y al ser import
estático el cargador falla antes de `main`. `verify_static.ps1 -ExpectWindows7`
**falla hoy a propósito**: es lo que la fase 17 tiene que poner en verde, y ahora
hay una comprobación esperando en vez de una afirmación por escribir.

---

## 6. La evidencia, con su clase exacta

| Afirmación | Clase |
|---|---|
| `qyro whoami` imprime una huella y un código reales | **Ejecutado en esta máquina**, salida arriba |
| Sin TTY se niega | **Ejecutado**, `echo "" \| qyro` |
| No emite escapes sin VT | **Probado en unidad**, en las dos direcciones |
| No importa runtime de C | **Medido** con `dumpbin /imports` |
| **No arranca en Windows 7** | **Medido.** El import está ahí |
| Corre sin libc | **Contenedor `FROM scratch` en CI.** No es una máquina física y no se cuenta como tal |
| Funciona en un PC viejo de verdad | **Ninguna.** Es el protocolo de hardware, y sigue en blanco |

---

## 7. La puerta

Quince, por exit code. Rust workspace, clippy, `flutter analyze`/`test`,
`dart format`, los dos checkers de documentación: todos 0. Y las dos guardas del
propio proyecto que pararon esta fase —`no_cargo_profile_sets_panic_abort` y la
meta-guarda de crates— tenían razón las dos, y están en el ledger.

---

## 8. Ledger

**160 fichas, 0 abiertas.** Nuevas: QYR-0360.
Siguiente: **fase 14**, que se encuentren sin router.
