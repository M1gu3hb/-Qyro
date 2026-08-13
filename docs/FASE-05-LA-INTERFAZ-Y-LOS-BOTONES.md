# FASE 05 — La interfaz de transferencia, y los botones

## 1. Objetivo

**Que una persona use Qyro.** Ver a quién tiene delante, comprobar su huella,
mandarle archivos, aceptarlos o rechazarlos, ver el progreso, y encontrar lo que
recibió.

**Y encender los botones.** Es la fase en que la línea que este proyecto ha
respetado durante siete meses deja de aplicarse — y sólo si las fases 01 a 04
cumplieron.

## 2. Por qué esta fase va aquí

**Depende de:** 01 (FFI), 02 (Dart conduce), 03 (selector), 04 (descubrimiento y
confianza). **Las cuatro. Sin excepción.**

Si alguna quedó parcial, **esta fase no empieza**. Una UI encima de una pieza a
medias es la forma más cara de descubrir que estaba a medias.

## 3. La condición para encender los botones

El código dice hoy, en `apps/qyro/lib/home/home_screen.dart:70`, `onPressed: null`,
con un texto en pantalla que explica que nada se habilita hasta que exista **una
transferencia real, cifrada y comprobada de extremo a extremo**.

**Los botones se encienden cuando las cinco cosas son ciertas y están probadas:**

1. Dart conduce una transferencia entre dos procesos y la verifica byte a byte
   (fase 02).
2. El usuario elige el archivo con el selector de su sistema (fase 03).
3. Dos aparatos se encuentran, o al menos hay un camino manual que funciona
   (fase 04).
4. La huella del otro se puede ver y comparar, y una clave cambiada se rechaza por
   nombre (fase 04).
5. **El receptor puede rechazar, y el emisor se entera** — ver §5, QYR-0089.

**Escribe en el informe las cinco, con la evidencia de cada una, antes de tocar la
línea 70.** Si una no se puede afirmar, los botones no se encienden y se dice por
qué.

## 4. Estado de partida

Reproduce lo de la fase 04, y lee:

- `apps/qyro/lib/` entero.
- El `i18n` que ya existe: **hay dos idiomas y hay que respetarlos.** Cada cadena
  nueva va en los dos.
- El branding y la pantalla de arranque.

## 5. La deuda que hay que cerrar aquí

**QYR-0089 — `TransferReject` existe en el protocolo y nadie lo emite ni lo
entiende.** Sin eso, «el receptor acepta o rechaza» no se puede implementar: el
mensaje está en el formato pero no hay ni productor ni consumidor.

**QYR-0088 — `FileSink` no tiene forma de abandonar una transferencia.** Un
rechazo a mitad tiene que dejar el destino limpio, y hoy no hay API para eso.

**Las dos se cierran en esta fase, antes de la UI**, porque la UI las necesita.

## 6. Lo que hay que construir, paso a paso

### Paso 1 — Cerrar QYR-0089 y QYR-0088

- Emitir y entender `TransferReject`, con motivo tipado: el usuario dijo que no,
  no hay espacio, el peer no es de confianza, el manifest es inaceptable.
- `FileSink::abandon()`: borra los `.qyro-part` y el `.qyro-resume` de esa
  transferencia y **no toca nada más del destino**.
- **Prueba de las dos:** un receptor rechaza a mitad; el emisor recibe el motivo
  exacto y para; el destino queda **sin un solo archivo nuevo**, comprobado
  listando el directorio.

**Puerta.**

### Paso 2 — El diseño de la interfaz, escrito antes de dibujar

`docs/adr/ADR-0036-transfer-ui.md`. No es una ADR de tecnología, es de producto, y
por eso hace falta:

- **Las pantallas y sus estados**, incluidos los feos: sin red, sin peers, peer
  con clave cambiada, transferencia fallida, destino lleno, permiso denegado.
- **Cómo se muestra la huella** y qué se le pide a la persona. `to_grouped_hex()`
  ya da el formato; **no lo cambies en la UI**.
- **Qué pasa cuando llega una transferencia de un desconocido.** Ésta es la
  decisión de producto que más importa: si se acepta por defecto, Qyro es un
  buzón abierto para cualquiera en la Wi-Fi.
- **Qué ve el receptor antes de decidir**: cuántos archivos, cuánto pesan, cómo se
  llaman. **Y qué NO se muestra sin sanear** — un nombre de archivo es texto que
  eligió el peer, y la validación de `qyro_manifest` protege el disco, no la
  pantalla.
- **Los dos idiomas.**

**Puerta.**

### Paso 3 — La pantalla de peers

- Lista de encontrados, con nombre y huella.
- Estado de confianza por peer: **conocido / nuevo / clave cambiada**, y el
  tercero **tiene que verse distinto y alarmante**, no como un aviso más.
- Entrada manual de `ip:puerto` y lector de QR — **siempre visibles**, no
  escondidos tras «avanzado». Son el camino que funciona cuando el descubrimiento
  falla.

**Puerta.**

### Paso 4 — Enviar

- El botón, ya vivo, abre el selector de la fase 03.
- Confirmación: a quién, qué, cuánto pesa.
- Progreso con el callback de la fase 02: bytes, porcentaje, archivo actual.
- **Pausar, reanudar, cancelar** — el motor ya los tiene desde el sprint 5A.

**Puerta.**

### Paso 5 — Recibir

- El botón, ya vivo, pone a Qyro a escuchar.
- Cuando llega una petición: **quién** (con huella y estado de confianza), **qué**,
  **cuánto**. Aceptar o rechazar.
- Progreso, y al terminar: **dónde quedaron los archivos**, con una forma de
  abrirlos desde el sistema.
- Un rechazo usa lo del paso 1 y deja el destino intacto.

**Puerta.**

### Paso 6 — El historial

`qyro_fs::history` ya existe: `latest`, `for_peer`, `with_status`, y recuperación
de un registro truncado. **Expónlo y muéstralo.** No lo reescribas.

**Puerta.**

### Paso 7 — Los botones, y la línea 70

- Encender **sólo si** las cinco condiciones de §3 están escritas con evidencia.
- **Quitar el texto que explica por qué están apagados**, que deja de ser cierto.
- Y **añadir en el informe qué sigue sin ser cierto**: en este punto todavía no
  hay identidad persistente en móvil (fase 06), ni una sola prueba en hardware
  físico (fase 07), ni nada empaquetado (fase 08).

**Puerta de fase.**

## 7. Las trampas concretas

1. **Encender los botones sin las cinco condiciones.** Es la única forma de
   mentirle al usuario que este proyecto ha evitado durante siete meses.
2. **La UI que inventa su propio formato de huella.** Si la pantalla muestra una
   huella distinta de la que muestra el otro aparato, la comparación en voz alta
   no vale nada. **Un solo formato, el del core.**
3. **El nombre de archivo del peer, pintado sin sanear.** `qyro_manifest` rechaza
   rutas peligrosas para el disco. La pantalla es otro problema: un nombre con
   caracteres de dirección puede leerse al revés. **Ya se cerró esa clase una vez
   para el filesystem** — no la reabras en la UI.
4. **El desconocido aceptado por defecto.** Decide y escribe qué pasa.
5. **El estado que sólo vive en Dart.** Si la app se va a segundo plano y el
   sistema la mata, ¿qué pasa con la transferencia? Decide, y si la respuesta es
   «se pierde», dilo en la UI.
6. **Los textos en un solo idioma.** El proyecto tiene dos.
7. **La pantalla feliz.** Los estados que hay que dibujar con más cuidado son sin
   red, sin peers, y clave cambiada.

## 8. Pruebas obligatorias

Widget tests y de integración en Dart:

- `a_rejected_transfer_leaves_the_destination_untouched` — listando el directorio
- `the_sender_learns_the_exact_reason_it_was_rejected`
- `a_peer_whose_key_changed_is_shown_as_changed_and_not_as_known`
- `the_fingerprint_on_screen_matches_the_one_the_core_computed` — **por dos
  caminos distintos**
- `a_manual_endpoint_can_start_a_transfer_without_any_discovery`
- `progress_reaches_one_hundred_percent_and_the_file_is_verified`
- `cancelling_from_the_ui_leaves_no_part_file`
- `every_new_string_exists_in_both_locales` — mecanizable, hazla
- `a_hostile_file_name_is_rendered_without_reordering_the_line`

## 9. Criterios de aceptación

1. QYR-0089 y QYR-0088 cerradas, con prueba de mutación.
2. ADR-0036 congelada antes de la UI, con los estados feos incluidos.
3. Las pantallas de peers, enviar, recibir e historial existen y funcionan.
4. **La entrada manual y el QR están siempre visibles.**
5. Un peer con clave cambiada se ve **distinto y alarmante**.
6. **Los botones encendidos, y las cinco condiciones de §3 escritas con su
   evidencia.** O apagados, y escrito por qué.
7. El texto que explicaba los botones apagados, retirado.
8. Todas las cadenas en los dos idiomas, con prueba mecanizada.
9. Un nombre de archivo hostil no reordena la línea en pantalla.
10. **Cero dependencias externas de Rust.** En Dart, sólo lo estrictamente
    necesario —selector y QR—, cada una justificada con publisher y licencia.
11. Barrido con `cargo-mutants` sobre lo nuevo de Rust. `R2` en todas las puertas.
    Informe según `R5`.
12. **§15 del informe dice, sin adornos: no hay identidad persistente en Android ni
    iOS, no hay ni una prueba en hardware físico, y no hay nada empaquetado.**

## 10. Cómo tiene que quedar el resultado

Dos ventanas —o dos emuladores— abiertas. Una ve a la otra. Comparas la huella.
Pulsas Enviar, eliges un archivo, la otra pregunta si lo acepta, dices que sí, ves
la barra, y el archivo aparece verificado del otro lado. Y queda en el historial de
los dos.

**Eso es el producto.** Lo que queda después no es funcionalidad: es plataforma,
evidencia real y empaquetado.

## 11. No objetivos

- Keystore y Keychain — fase 06.
- **Hardware físico** — fase 07. Aquí todo es emulador, simulador y escritorio.
- Empaquetado y firma — fase 08.
- Temas, animaciones, pulido visual más allá de lo que hace falta para usarlo.
- Ajustes, perfiles, avatares, renombrar aparatos. **No metas producto que nadie
  pidió.**

## 12. Qué desbloquea

La fase 07, que es probar todo esto en aparatos de verdad. Y por primera vez el
proyecto tiene algo que enseñar.
