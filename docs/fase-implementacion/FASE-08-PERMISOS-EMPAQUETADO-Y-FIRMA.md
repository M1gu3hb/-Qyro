# FASE 08 — Permisos, empaquetado y firma

> **iOS está fuera de la v1.0 (ADR-0039, 2026-08-14).** Xcode exige macOS y este
> proyecto no tiene ninguno, así que la mitad iOS de esta fase queda **aplazada,
> no cancelada**: lo escrito abajo sobre iOS sigue siendo el plan del día que
> exista un Mac y una cuenta de desarrollador. **Lo que esta fase entrega para la
> v1.0 es Android y Windows.**


## 1. Objetivo

**Que Qyro se pueda instalar en un aparato que no sea el de desarrollo.** Un APK,
un `.exe`, y un IPA ad-hoc si iOS sigue en el plan.

## 2. Por qué esta fase va aquí

**Depende de:** fase 07. Empaquetar algo que nunca corrió en un aparato real es
empaquetar una suposición.

## 3. La decisión de distribución, ya tomada

Qyro es para el dueño y sus amigos. **No va a tiendas.**

| Plataforma | Camino | Coste |
|---|---|---|
| **Android** | APK directo. **Cuenta gratuita de distribución limitada** si algún día se quiere Play: sólo email, hasta 20 aparatos | **0** |
| **Windows** | `.exe` sin firmar. Un aviso de SmartScreen la primera vez y ya | **0** |
| **iOS** | **Ad-hoc**: cuenta de desarrollador, hasta 100 aparatos registrados, **builds válidas un año** | **99 USD/año** |

**iOS es el único con coste real.** Con un Apple ID gratuito las builds caducan a
los 7 días, que no sirve para nadie.

## 4. Las dos decisiones que hay que tomar YA y que no se pueden deshacer

**QYR-0050 lleva abierta desde el sprint 4D.1 y esta fase la cierra.**

### 4.1 — El nombre del paquete

Hoy es **`com.owner.qyro`**, provisional, y `check_docs_consistency` tiene una
regla que lo vigila.

**Cambiar el nombre de paquete después de distribuir obliga a desinstalar y
reinstalar desde cero en todos los aparatos, perdiendo la identidad y el
historial.** Decídelo ahora.

### 4.2 — La clave de firma

**Cambiar la clave de firma de un APK después obliga a lo mismo**: Android se
niega a actualizar una app firmada con otra clave.

Decide, y escribe en `docs/release/signing.md`:

- **Dónde vive la clave** y quién la tiene. **Nunca en el repositorio.** Nunca.
- Cómo se hace una build reproducible sin ella —CI firma con una de depuración—.
- **Qué pasa si se pierde.** La respuesta honesta es «todo el mundo reinstala», y
  hay que escribirla para que se entienda por qué la copia de seguridad importa.
- `.gitignore` con el keystore, y **una guarda que falle si un `.jks`, un `.p12` o
  un `.keystore` entra en el repositorio.** Este proyecto tiene guardas
  estructurales para todo; ésta también.

## 5. Los permisos, plataforma por plataforma

**Antes de empaquetar, audita lo que pides.** Un permiso que no se usa es
superficie y es una pregunta que el usuario tiene que responder sin motivo.

### Android — el manifiesto

Lo que **debería** estar, y nada más:

- `INTERNET` — necesario para sockets, incluso locales.
- **Nada de almacenamiento.** SAF no lo necesita (fase 03).
- **`ACCESS_LOCAL_NETWORK` sólo si `NsdManager` con picker no bastó** (fase 04). Si
  está, **justifícalo por escrito**.
- `CHANGE_WIFI_MULTICAST_STATE` sólo si acabaste usando `MulticastLock`.

**Y una prueba mecanizada que falle si aparece un permiso no listado.** No una
revisión manual: una prueba.

### iOS — el `Info.plist`

- `NSLocalNetworkUsageDescription` y `NSBonjourServices` (fase 04).
- `UIFileSharingEnabled` si se quiere que la carpeta de recibidos se vea.
- **Y comprobar que NO está el entitlement de multicast**, que exige revisión
  humana de Apple.
- **Los textos de uso los lee una persona en un diálogo.** Escríbelos en los dos
  idiomas y que digan la verdad: por qué Qyro necesita ver la red local.

### Windows

Nada que declarar. **Pero comprueba qué mete el bundle**: `verify_windows_package.ps1`
ya existe y ya corre en CI.

## 6. Lo que hay que construir, paso a paso

### Paso 1 — Decidir §4 y cerrar QYR-0050

Nombre de paquete y política de firma, escritos. **Y la guarda que impide que una
clave entre al repositorio, vista fallar.**

**Puerta.**

### Paso 2 — La auditoría de permisos

- Lista real por plataforma, con justificación de cada uno.
- **Prueba mecanizada** que falle ante un permiso no listado, en Android e iOS.
- Los textos de uso, en dos idiomas.

**Puerta.**

### Paso 3 — Las builds de release

- **Android:** APK de release, firmado, con `minSdkVersion` y `targetSdkVersion`
  decididos y escritos. **Ojo con `targetSdk` 37 y las Local Network Protections
  de la fase 04.**
- **Windows:** `.exe` empaquetado con sus DLLs, verificado con el script que ya
  existe.
- **iOS:** IPA ad-hoc, si hay cuenta.
- **Y cierra QYR-0004** —«builds no retenidos»—: los artefactos se suben como
  artifacts del workflow, con retención declarada.

**Puerta.**

### Paso 4 — Instalar de verdad

**En un aparato distinto del de desarrollo.** Es el punto entero de la fase.

- APK instalado en un teléfono que no sea el de las pruebas.
- `.exe` en una máquina Windows limpia — **y anota exactamente qué dice
  SmartScreen**, porque es lo primero que va a ver un amigo.
- IPA en un iPhone registrado.
- **Y una transferencia completa entre dos aparatos con las builds de release**,
  no de depuración. Las builds de release optimizan distinto y quitan símbolos.

**Puerta.**

### Paso 5 — Las instrucciones para una persona

`docs/release/install.md`, escrito para alguien que no sabe qué es un APK:

- Cómo instalar en cada plataforma, con lo que va a ver.
- **Qué avisos van a salir y por qué** — «orígenes desconocidos» en Android,
  SmartScreen en Windows, el perfil de confianza en iOS.
- Qué permisos va a pedir y por qué.
- Cómo desinstalar y qué se borra.

**Puerta de fase.**

## 7. Las trampas concretas

1. **La clave de firma en el repositorio.** Es irreversible: una vez en el
   historial de git, está comprometida. **Guarda estructural, no buena voluntad.**
2. **La build de release que se comporta distinto.** Optimizaciones, símbolos,
   `assert!` que desaparecen. **Prueba la de release, no la de depuración.**
3. **El `targetSdkVersion` que activa restricciones nuevas.** Subirlo a 37 activa
   las Local Network Protections de Android 17. Decide y prueba.
4. **El permiso que se coló.** Un plugin de Flutter puede añadir permisos al
   manifiesto sin que lo escribas. **Por eso la comprobación es mecanizada y sobre
   el manifiesto fusionado**, no sobre el que escribiste.
5. **La build de iOS que caduca.** Un año, y hay que reconstruir. **Escríbelo en
   `install.md`** para que nadie se sorprenda.
6. **El bundle de Windows al que le falta una DLL.** Funciona en la máquina de
   desarrollo y no en otra. Por eso el paso 4 exige una máquina limpia.

## 8. Criterios de aceptación

1. **Nombre de paquete y clave de firma decididos**, escritos, y **QYR-0050
   cerrada**.
2. **Una guarda que falla si una clave de firma entra al repositorio, vista
   fallar.**
3. Lista de permisos por plataforma, cada uno justificado, **con prueba mecanizada
   sobre el manifiesto fusionado**.
4. **Android no declara ningún permiso de almacenamiento.**
5. **iOS no declara el entitlement de multicast.**
6. Los textos de uso en los dos idiomas.
7. APK firmado, `.exe` empaquetado, IPA ad-hoc si hay cuenta.
8. **QYR-0004 cerrada**: artefactos retenidos con política escrita.
9. **Una transferencia completa entre dos aparatos con builds de release**, no de
   depuración.
10. Instalado en al menos un aparato distinto del de desarrollo por plataforma, con
    lo que dijo cada aviso del sistema anotado literalmente.
11. `docs/release/install.md` escrito para una persona, no para un desarrollador.
12. `R2` en todas las puertas. Informe según `R5`.

## 9. Cómo tiene que quedar el resultado

Le mandas un APK a un amigo por WhatsApp, se lo instala, y **funciona**. Sin que
tú toques su teléfono.

## 10. No objetivos

- Play Store, App Store, certificado de firma de Windows.
- Actualizaciones automáticas.
- Analítica de instalaciones. **Este proyecto no tiene telemetría y no la va a
  tener.**

## 11. Qué desbloquea

La fase 09, que es el cierre de deuda con la vista puesta en poder llamar a esto
v1.0.
