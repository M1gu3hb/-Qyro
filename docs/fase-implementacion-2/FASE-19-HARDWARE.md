# FASE 19 — Hardware: los cuatro canales, en aparatos de verdad

> **Esta fase no la puedes ejecutar tú.** Necesita dos aparatos y una persona. Lo que
> sí puedes —y debes— es dejarla al punto de que el propietario conecte algo y corra
> un comando.

---

## 1. El estado, y no se maquilla

`docs/testing/hardware-protocol.md` tiene **21 huecos `Resultado: [ ]` y los 21 están
en blanco.** Verificado en la auditoría del 2026-08-17. **Nadie ha ejecutado nunca
Qyro en un teléfono, y ninguna transferencia ha cruzado una Wi-Fi de verdad.**

Eso es correcto y es honesto. **Un hueco en blanco es la verdad hasta que alguien lo
llena. No se inventa evidencia de hardware — es lo único que arruinaría el proyecto.**

---

## 2. Lo que hay que añadir al protocolo

Los 21 escenarios existentes cubren la GUI y el canal TCP. Faltan los tres canales
nuevos y el CLI. Cada escenario nuevo, con **el mismo formato**: comando literal, qué
tiene que pasar, y un hueco.

### F — El binario de terminal (fase 13)

- **F1.** El `.exe` arranca en un Windows 10/11 recién instalado, **sin Visual C++
  redistributable**. Esperado: el menú. Ni un diálogo de DLL faltante.
- **F2.** El binario musl arranca en un Linux viejo con glibc anterior a 2.17.
- **F3.** El menú se dibuja legible en `cmd.exe` con fuente **raster** y code page
  **437**. Esperado: sin caracteres de escape sueltos, sin basura.
- **F4.** Copiar el `.exe` a `%USERPROFILE%\Downloads` y ejecutarlo **sin
  administrador**. Anotar si aparece SmartScreen y si hay «Run anyway».
- **F5.** Lo mismo en un Windows 11 con **Smart App Control** activo. **Esperado:
  probablemente bloqueado sin bypass** (`R8` §12). **Anota lo que pase de verdad** —
  este escenario existe para medir el problema de distribución, no para pasarlo.

### G — Sin router (fase 14)

- **G1.** Cable Ethernet directo entre los dos, sin router, sin DHCP. Cronometrar
  **cuánto tarda cada lado en tener dirección** — `R8` §8 dice decenas de segundos y
  no hay fuente vigente de Microsoft para Windows 10/11. **Este número hay que
  medirlo.**
- **G2.** Con una NIC de 10/100 en la máquina vieja: ¿enlaza con cable recto, o hace
  falta cruzado? (Auto-MDI-X está en la cláusula de 1000BASE-T.)
- **G3.** Se encuentran solos por el descubrimiento propio, en una red sin router.
- **G4.** El diálogo del firewall de Windows: anotar **si aparece**, en qué perfil, y
  qué pasa si el usuario **no es administrador**.
- **G5.** Aislamiento de cliente activado: el descubrimiento **no** encuentra y el
  código tecleado **sí** funciona.

### H — Canal óptico (fase 15)

- **H1.** Un archivo de texto de 100 KB, de la pantalla del PC a la cámara del móvil.
  **Cronometrar y anotar el throughput real en KB/s.** Comparar con los 6–10 KB/s
  de `R8` §4. **Es la medición que valida o refuta todo el diseño.**
- **H2.** Lo mismo a un metro de distancia, y a dos. Anotar a qué distancia deja de
  decodificar.
- **H3.** Lo mismo con el móvil **a pulso**, sin soporte. Anotar la degradación.
- **H4.** Lo mismo en la terminal de Windows 7 con cp437 y fuente raster: **¿se
  decodifica el QR de half-block?** Es la única forma de saber si esa técnica sirve
  en la máquina de la escena.
- **H5.** Con luz directa sobre la pantalla (glare). Anotar si sobrevive.
- **H6.** Interrumpir a mitad —tapar la cámara diez segundos— y comprobar que
  **reanuda**, no que reinicia.

### I — Canal serie (fase 16)

- **I1.** Cable **null-modem** de verdad entre el PC moderno (adaptador USB-serie) y
  el viejo (DB9). 1 MB. **Cronometrar.** Esperado por `R8` §5.1: ~1,6 minutos.
- **I2.** El script de bootstrap que Qyro imprime, **pegado en una PowerShell de
  Windows 7 real**, recibe el archivo. Anotar cuántos minutos cuesta el pegado.
- **I3.** En Windows XP, con **HyperTerminal**, siguiendo las instrucciones que Qyro
  imprime. Anotar si las instrucciones bastaron sin buscar nada más.
- **I4.** Desconectar el cable a mitad y volver a conectarlo: ¿reanuda?

### J — Lo aburrido que gana (fase 16 §2)

- **J1.** Antes de nada: **¿la máquina vieja tiene lector de CD, disquetera, PCMCIA o
  NIC?** Anotarlo. Si tiene NIC, **G1 es la respuesta y el resto es curiosidad
  técnica** — y eso también es un resultado del protocolo.

---

## 3. Lo que hay que dejar preparado

- **Cada escenario con su comando literal rellenado.** Sin `<placeholder>`.
- **Una lista de compra**, porque hay cosas que hay que tener: cable Ethernet, cable
  cruzado por si acaso, adaptador USB-serie, cable null-modem DB9, y un soporte o
  trípode para el móvil.
- **Un solo comando que prepare todo**: construir, firmar, instalar en el teléfono
  por `adb`, y dejar el `.exe` donde se espera.

---

## 4. La regla, otra vez y en mayúsculas

**NO INVENTES EVIDENCIA DE HARDWARE.** Un hueco en blanco es la verdad. Un hueco
relleno con lo que crees que pasaría es lo único que destruiría siete meses de
trabajo, porque haría que ninguna otra afirmación del proyecto valiera nada.

Si el propietario ejecuta cinco escenarios y deja dieciséis en blanco, **el documento
dice cinco ejecutados y dieciséis sin ejecutar**, y eso es un éxito, no un fracaso.
