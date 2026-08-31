# Qyro

**Manda un archivo de un aparato a otro sin que salga de tu red.** Sin nube, sin
cuentas, sin servidor, sin anuncios, sin telemetría.

**La v1.0 son Android y Windows** (ADR-0039): iOS está **aplazado**, no
cancelado, porque Xcode exige macOS y este proyecto no tiene ninguno. El núcleo
de Rust ya compila para iOS en CI y el trabajo hecho se conserva; reincorporarlo
pide un Mac y una cuenta de desarrollador de Apple.

> **Nada de esto se ha ejecutado nunca en hardware físico.** Está probado en
> unidad, en integración, entre dos procesos reales y en CI sobre Linux y
> Windows. Ningún teléfono ha ejecutado nunca esta aplicación y ninguna
> transferencia ha cruzado una Wi-Fi de verdad. Los **veintiséis** escenarios que
> cerrarían ese hueco están escritos y **sin marcar** en
> [docs/testing/hardware-protocol.md](docs/testing/hardware-protocol.md).
>
> Trátalo como software que funciona en las pruebas y que nadie ha usado.

Si vas a probarlo hoy, la página que necesitas es
[docs/GUIA-DE-PRUEBA.md](docs/GUIA-DE-PRUEBA.md): está escrita para alguien que
no ha leído nada de este repositorio.

## Dos caras, un motor

Qyro se usa de dos maneras, y las dos hablan con **el mismo motor de Rust**:

- **La aplicación**, en Android y en Windows. Flutter, cuatro pantallas, dos
  idiomas.
- **`qyro`, el binario de terminal.** Un ejecutable, sin instalador, que se copia
  en un USB y se ejecuta desde ahí. Existe para la máquina que **no puede
  instalar nada**: sin GPU que Flutter acepte, sin Windows 10, sin permiso de
  administrador. Lo que esa máquina sí tiene es una terminal.

      qyro                                  abre el menú
      qyro send <archivo> --to "<código>"   manda
      qyro recv --out <carpeta>             recibe
      qyro whoami                           el código de este aparato
      qyro find                             quién más hay en esta red
      qyro qr                               dibuja el código como QR

  **El código va entre comillas dobles.** El `|` que lleva dentro es una tubería
  en PowerShell y en `cmd`, así que sin comillas la consola parte la línea y el
  error que sale no menciona a Qyro. `qyro whoami`, `qyro recv` y `qyro find` ya
  lo imprimen entrecomillado: se copia entero, comillas incluidas.

**Una capacidad no está terminada hasta que las dos caras la alcanzan**, o hasta
que está escrito que es de una sola y por qué. La tabla que lo lleva, celda a
celda y con `archivo:línea`, es [docs/PARIDAD-GUI-CLI.md](docs/PARIDAD-GUI-CLI.md).

## Qué hace

- **Dos aparatos se emparejan tecleando un código**: el receptor lo enseña en su
  pantalla y el emisor lo escribe. Funciona en cualquier red, incluida una con
  aislamiento de cliente —que es la mayoría de las Wi-Fi públicas—, y ése es el
  motivo de que este camino se construyera primero.
- **También se encuentran solos**, cuando la red lo permite: `qyro find` en la
  terminal y la lista «cerca de ti» en la aplicación. Un aparato que se anuncia
  **no ha probado nada**: sigue pasando por la misma comprobación de confianza
  que un código tecleado.
- **El teléfono puede leer un QR** que la terminal dibuja. Es el canal que
  funciona **sin red de ninguna clase**, y por eso existe. La dirección está
  fijada: la terminal dibuja y el teléfono mira (ADR-0044 §6); en la máquina que
  necesita los códigos no hay cámara.
- Eliges archivos con el selector del sistema. En Android, sin un solo permiso de
  almacenamiento y **sin copiar el archivo** para leerlo.
- Ves con quién hablas: una huella corta para comparar en voz alta. **Y el
  código que tecleas es una promesa**: lleva dentro la huella que el otro aparato
  tiene que demostrar, y si la que sale del handshake no es ésa, Qyro **se niega
  sin preguntar**. No hay «continuar de todos modos» y no lo va a haber.

  **Lo que todavía no hace, dicho aquí:** Qyro no lleva una libreta de aparatos
  conocidos. Nada llama a `remember_peer` en producción, así que la advertencia
  «este aparato conocido presenta otra clave» **no puede ocurrir**: para Qyro
  todos los aparatos son nuevos cada vez. Lo que sí protege es la comparación de
  arriba, que es la que una persona hace con los ojos.
- El receptor decide, siempre, y ve **qué** le mandan antes de aceptar: los
  nombres y los tamaños. **Nada se acepta solo, nunca.**
- Handshake autenticado, ChaCha20-Poly1305 por frame, SHA-256 por archivo. Un
  archivo que no verifica **no se entrega**.
- Español e inglés.

## Los cuatro canales

| Canal | Para qué | Estado en código |
|---|---|---|
| **Red local (TCP)** | el camino normal, y el que se prueba primero | las dos caras |
| **Cable directo, sin router** | dos máquinas y un cable; espera a que APIPA asigne y lo dice mientras espera | las dos caras |
| **Óptico (QR)** | sin red de ninguna clase: la terminal dibuja, el teléfono lee | terminal dibuja · teléfono lee |
| **Serie (COM)** | la máquina que no tiene red ni cámara | sólo terminal |

**Ninguno de los cuatro se ha probado en hardware.** La tabla de arriba dice
quién llama a qué en el código, que es una cosa distinta.

Qué **no** hace, y por qué: [docs/release/v1.0.md](docs/release/v1.0.md).
Qué defiende y qué no: [THREAT_MODEL.md](THREAT_MODEL.md).

## Estado verificable

La fuente canónica es [STATUS.md](STATUS.md), que distingue entre compilar,
ejecutar, empaquetar y probar, con runs y bloqueos reales.
[ESTADO-ACTUAL.md](ESTADO-ACTUAL.md) dice dónde se cortó la última sesión. El
registro de hallazgos es [BUGS_PENDING.md](BUGS_PENDING.md); las decisiones, con
lo que cada una descartó y por qué, están en [docs/adr/](docs/adr).

## Instalar

Los artefactos y sus SHA-256 están en
[docs/release/v1.0.md](docs/release/v1.0.md). **Compara el hash antes de
instalar nada.**

El APK declara **tres permisos**, y cada uno con su motivo escrito al lado en el
manifiesto: `INTERNET` (todo canal de red es un socket TCP; se concede al
instalar y no pide ningún diálogo), `CHANGE_WIFI_MULTICAST_STATE` (sin él el
descubrimiento recibe cero paquetes y no da ningún error) y `CAMERA` — **el único
que se pide en tiempo de ejecución**, y sólo para leer QR. No hay ningún permiso
de almacenamiento: el selector del sistema no lo necesita.

## Desarrollo

    bash scripts/doctor.sh
    bash scripts/bootstrap.sh
    bash scripts/test_all.sh

Equivalentes PowerShell:

    pwsh -NoProfile -File scripts/doctor.ps1
    pwsh -NoProfile -File scripts/bootstrap.ps1
    pwsh -NoProfile -File scripts/test_all.ps1

La puerta, que corre **los mismos comandos que CI** porque los lee de
`.github/workflows/ci.yml` en vez de llevar su propia lista:

    pwsh -File scripts/gate.ps1

Para ejecutar la aplicación:

    cd apps/qyro
    flutter run

Para construir el binario de terminal:

    cargo build --release -p qyro_cli --target x86_64-pc-windows-msvc

En Windows, `flutter build` y `flutter run` con plugins necesitan el Modo
Desarrollador activado, porque Flutter usa enlaces simbólicos para el registrante
de plugins (`start ms-settings:developers`).

Lee [AGENTS.md](AGENTS.md), [STATUS.md](STATUS.md), [HANDOFF.md](HANDOFF.md) y
[NEXT_STEPS.md](NEXT_STEPS.md) antes de modificar código.
