<!-- Las notas de la Release v1.0.1, revisables en git antes de publicarse.
     `release.yml` las publica tal cual y les pega debajo los SHA-256 que mide
     sobre los archivos que sube. La prosa se revisa aqui; los numeros no
     los escribe una persona. -->

## Lo primero, porque es lo que decide si esto sirve para algo

**Nadie ha mandado nunca un archivo con Qyro entre dos aparatos de verdad.**

Ni un telefono, ni dos PCs, ni un cable. Todo lo que este proyecto sabe lo sabe
de dos procesos hablando dentro de la misma maquina y de los servidores de
GitHub. Los treinta escenarios de `docs/testing/hardware-protocol.md` estan **en
blanco**, y en blanco se quedan hasta que alguien los ejecute con dos aparatos
delante. Un hueco vacio es la verdad; un hueco marcado sin haberlo hecho es lo
unico que este documento no puede permitirse.

**El APK va firmado con la clave de depuracion.** Se llama
`app-release-debugkey.apk` para que no haya forma de confundirlo. Esa clave es
publica y universal: la tiene todo el que instale el SDK de Android, asi que la
firma **no dice quien construyo este archivo**. Sirve para instalar y probar; no
sirve para creerse nada sobre su origen. La clave de firma real
(`key.properties`) no esta en este repositorio y no va a estar: este proyecto no
mete una clave privada en un secreto de CI. Firmar es un paso local y
deliberado, y esta escrito en `docs/release/DECISION-DE-FIRMA.md`.

Android avisara de que viene de fuera de la tienda. Windows SmartScreen avisara
de que el `.exe` no esta firmado con certificado — tambien cuesta dinero, tambien
esta decidido y escrito. **Los dos avisos tienen razon.**

## Que es esto

Los **tres** artefactos de Qyro, construidos por GitHub Actions en la corrida que
se nombra al final, con sus SHA-256 medidos por la misma maquina que los
construyo.

| Archivo | Que es |
|---|---|
| `qyro.exe` | **El de terminal, y el que usa la guia de prueba.** Un solo archivo: se copia donde sea y funciona. |
| `app-release-debugkey.apk` | La aplicacion de Android. Tres ABIs (`arm64-v8a`, `armeabi-v7a`, `x86_64`), alineada a 16 KB y medido sobre el APK, no sobre lo que salio del enlazador. |
| `qyro-windows-x64.zip` | La aplicacion de escritorio de Windows x64, portable. **Hace falta la carpeta entera**: el `.exe` de dentro, solo, no arranca. |
| `SHA256SUMS.txt` | Los tres hashes. |

**Ojo con los dos `qyro.exe`, que es el aviso que mas tiempo hace perder.** El
`qyro.exe` suelto de esta pagina es el de **terminal**. El `qyro.exe` que hay
dentro de `qyro-windows-x64.zip` es la aplicacion **con ventanas** y necesita la
carpeta entera. Se llaman igual y no son intercambiables.

Para instalar y probar, `docs/GUIA-DE-PRUEBA.md` esta escrita para alguien que no
ha leido nada de este repositorio.

## Que sustituye, y que no borra

Sustituye a **`v1.0.0`, que sigue publicada y sigue retractada**. Sus notas dicen
«RETRACTADO: estos binarios no pueden enviar» y sus tres archivos siguen ahi. No
se borra: dos personas ya se los habian descargado, y quien tenga uno en el disco
merece encontrar la pagina que le explica que tiene en las manos.

Lo que aquella publicacion no podia hacer —enviar— se arreglo en QYR-0361 y
QYR-0362. Estos binarios llevan esos arreglos.

## Lo que no funciona, dicho antes de que se pierda tiempo en ello

| Que | Por que |
|---|---|
| **Windows 7 y Windows 8** | Los dos `.exe` de esta pagina necesitan Windows 10 u 11. Hay un binario aparte para Windows 7 —lo construye `win7-builds.yml`— y **no va en esta Release**: nadie lo ha arrancado nunca en un Windows 7 de verdad, solo se ha medido que su tabla de imports esta limpia. |
| **iPhone** | No hay version de iOS. Construirla necesita un Mac, y este proyecto no tiene ninguno. |
| **Mandar por QR desde el telefono** | La direccion esta fijada: **el PC dibuja los codigos y el telefono los lee** (ADR-0044 §6). Al reves no existe, porque el PC no tiene camara. |
| **El canal por cable serie** | Solo esta en el binario de terminal, y no tiene interfaz. Telefono ↔ PC por serie no es una prueba posible. |
| **Que los dos aparatos se encuentren solos** | Depende del router, no de Qyro. Cuando no ocurre, **el codigo tecleado siempre funciona**, y es el camino principal de la guia. |

## Lo que si esta comprobado, y por quien

Todo lo de esta lista lo comprueba GitHub Actions en el commit que se nombra
abajo, no una persona escribiendo que lo hizo:

- La suite completa de Rust en Linux y en Windows. En macOS corre `qyro_fs`,
  que es el crate donde la plataforma cambia la respuesta: enlaces, mayusculas
  y normalizacion de nombres.
- Que el manifiesto **de release** del APK declara `INTERNET` y ninguna de
  almacenamiento — la permisologia se mide sobre el manifiesto fusionado, que es
  el unico que describe lo que se instala.
- Que el APK lleva las tres ABIs y ninguna herramienta de pruebas de cifrado.
- Que las bibliotecas nativas estan alineadas a 16 KB, medido **dentro del
  APK**: en Android 15 una alineada a 4 KB no carga y la aplicacion muere al
  abrirse.
- Que el paquete de Windows lleva todo lo que necesita para arrancar.

Nada de eso es «funciona entre dos aparatos». Es «no esta roto de las maneras que
una maquina puede comprobar sola».
