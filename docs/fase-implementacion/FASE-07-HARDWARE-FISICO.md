# FASE 07 — Hardware físico

## 1. Objetivo

**La primera evidencia real de este proyecto.** Dos aparatos de verdad, en una
Wi-Fi de verdad, pasándose archivos de verdad.

## 2. Por qué esta fase existe y por qué es una fase entera

**Este proyecto lleva siete meses y no tiene ni una sola prueba en hardware
físico.** Está dicho así en su documentación, y decirlo ha sido correcto. Pero hay
un límite a cuánto se puede construir sin ello.

**Lo que un emulador y un runner de CI no pueden enseñar:**

| Cosa | Por qué el emulador no la ve |
|---|---|
| Pérdida de paquetes y latencia variable | `127.0.0.1` no pierde nada, nunca |
| MTU y fragmentación | Loopback usa 65 536; una Wi-Fi, 1 500 |
| **Aislamiento de cliente** | Muchos routers domésticos impiden que dos clientes se vean. **Esto puede romper el producto entero y sólo se ve en una red real** |
| Suspensión de la radio Wi-Fi | Android e iOS apagan la radio con la pantalla apagada |
| La app en segundo plano | iOS suspende procesos; Android los mata |
| El permiso de red local de iOS | Se comporta distinto en un aparato real, y **su denegación es silenciosa** |
| El Secure Enclave | El simulador no lo tiene |
| Térmica y batería | Cifrar y hashear un gigabyte en un teléfono |
| Almacenamiento lleno de verdad | El emulador tiene el disco del host |

**Depende de:** fases 01 a 06. Es la primera vez que todo se ejercita junto.

## 3. Lo que hace falta antes de empezar

**Esta fase necesita aparatos, y eso es del usuario, no del agente.** Como mínimo:

- **Un teléfono Android** con depuración USB.
- **Una máquina Windows.**
- **Un iPhone**, si iOS sigue en el plan — y con él llega el coste real del
  proyecto: **99 USD/año de cuenta de desarrollador de Apple** para instalar
  ad-hoc en hasta 100 aparatos, con builds válidas un año.
- **Una red Wi-Fi doméstica**, y si se puede, **una segunda red con aislamiento de
  cliente activado**, que es el escenario que rompe.

**Si no hay aparatos, esta fase no se puede hacer y hay que decirlo.** No la
simules. Un emulador declarado como hardware físico es exactamente la mentira que
este proyecto ha evitado.

## 4. El protocolo de prueba, escrito antes de tocar un aparato

`docs/testing/hardware-protocol.md`. **Escrito antes**, porque una sesión de
pruebas manuales sin guion produce anécdotas, no evidencia.

Por cada escenario: **qué se hace, qué se espera, qué se registra, y qué se hace
si falla.** Y **cómo se captura la evidencia** — logs, capturas, `adb logcat`, el
Console de macOS.

## 5. Los escenarios, en orden

### Bloque A — Que funcione una vez

1. **Android ↔ Windows, misma Wi-Fi, archivo pequeño (1 MiB).** El caso base. Si
   esto no funciona, para y arregla antes de seguir.
2. **El mismo, archivo grande (≥1 GiB).** Aquí aparecen la térmica, la batería y
   los tiempos de verdad.
3. **Android ↔ Android.**
4. **iOS ↔ Windows** y **iOS ↔ Android**, si hay iPhone.

**Puerta.**

### Bloque B — Que funcione la red real

5. **Aislamiento de cliente activado.** Se espera que el descubrimiento falle.
   **Comprueba que el fallback manual y el QR funcionan** — es exactamente para
   esto que se construyeron primero.
6. **Wi-Fi de 2,4 GHz saturada**, o a distancia con mala señal. Se espera lentitud,
   **no corrupción y no cuelgue**.
7. **Los dos aparatos en redes distintas.** Se espera un fallo **limpio y
   explicado**, no un cuelgue de treinta segundos.
8. **Un router que reasigna la IP a mitad** de una transferencia larga.

**Puerta.**

### Bloque C — Que sobreviva a la vida real

9. **Pantalla apagada a mitad**, en los dos aparatos, por separado.
10. **App a segundo plano a mitad**, en los dos.
11. **Llamada entrante**, notificación, cambio de app.
12. **Wi-Fi apagada a mitad** y vuelta a encender.
13. **Batería baja y modo de ahorro de energía.**
14. **La app matada por el sistema a mitad**, y reabierta: **¿se reanuda?** Toda la
    maquinaria de `.qyro-resume` existe desde el sprint 5B.1 y **nunca se ha
    ejercitado de verdad**.

**Puerta.**

### Bloque D — Que no mienta

15. **Reinicio completo de los dos aparatos**, y comprobar que **la identidad y los
    peers conocidos siguen ahí** — lo que la fase 06 construyó, ahora sobre Secure
    Enclave y TEE reales.
16. **Comparar la huella en voz alta** entre dos personas, con los dos aparatos
    delante. **Es la prueba de usabilidad de la seguridad y no se puede automatizar.**
17. **Borrar la app y reinstalarla.** ¿Qué sobrevive? En Android
    `getNoBackupFilesDir()` se borra; en iOS el Keychain a veces no. **Comprueba
    cuál de los dos comportamientos ocurre de verdad.**
18. **Cambiar la clave de un peer a propósito** —reinstalando el otro aparato— y
    comprobar que **el aviso salta y se ve alarmante**.

**Puerta.**

### Bloque E — Que sea usable

19. **Que alguien que no seas tú lo use, sin explicarle nada.** Una persona, un
    teléfono, la tarea «mándame esta foto». **Anota dónde se atasca.**
20. Repetir con una segunda persona.

**Puerta de fase.**

## 6. Cómo se registra

**Cada escenario produce una fila**, y las filas van en
`docs/testing/hardware-results-<fecha>.md`:

| # | Escenario | Aparatos y versiones | Resultado | Evidencia | Ficha |
|---|---|---|---|---|---|

- **Aparatos con modelo y versión de sistema.** «Android» no es un dato; «Pixel 6a,
  Android 15» sí.
- **Resultado: pasó / falló / pasó con salvedad.** Sin adjetivos.
- **Evidencia:** captura, log, o el fragmento de `logcat`.
- **Toda falla es una ficha** en el ledger, con severidad juzgada.

**Y el informe dice cuántos escenarios se ejecutaron de cuántos.** Un bloque que no
se pudo correr por falta de aparato **se declara**, no se omite.

## 7. Las trampas concretas

1. **Declarar un emulador como hardware físico.** Es la única forma de arruinar
   esta fase entera.
2. **Probar sólo el camino feliz.** Los bloques B, C y D son donde está el valor.
   El bloque A sólo dice que compilaste bien.
3. **Probar en una sola red.** El aislamiento de cliente es el escenario que rompe
   productos de este tipo, y no aparece en casa de todo el mundo.
4. **Arreglar sobre la marcha sin registrar.** Cada fallo es una ficha **antes** de
   arreglarlo, para que quede la cuenta de cuántos hubo.
5. **La prueba de usabilidad hecha por quien escribió el código.** No vale. Tiene
   que ser alguien que no sepa cómo funciona.
6. **El log que filtra.** Antes de empezar, **comprueba que ningún log imprime
   material de clave**. La guarda estructural existe para el código; el `logcat` de
   un aparato real es donde se ve de verdad.

## 8. Criterios de aceptación

1. `docs/testing/hardware-protocol.md` escrito **antes** de tocar un aparato.
2. **Los veinte escenarios ejecutados, o declarados como no ejecutables con su
   motivo.** Con el conteo: cuántos de cuántos.
3. Bloque A completo en **Android ↔ Windows** como mínimo.
4. **El fallback manual probado bajo aislamiento de cliente**, con evidencia.
5. **La reanudación tras la muerte de la app, ejercitada de verdad** — es la
   primera vez.
6. Identidad y peers conocidos sobreviven a un reinicio real, sobre TEE y Secure
   Enclave.
7. **La comparación de huella hecha por dos personas**, con lo que costó anotado.
8. **Dos personas ajenas usaron la app**, y sus atascos están escritos.
9. **Ningún log de ningún aparato contiene material de clave**, comprobado.
10. Toda falla tiene ficha. `R2` en todas las puertas. Informe según `R5`.
11. **Y el cambio de lenguaje que esta fase permite:** a partir de aquí, y sólo
    para lo ejecutado, la clase de evidencia sube a **«probado en hardware
    físico»**. Todo lo demás sigue donde estaba.

## 9. Cómo tiene que quedar el resultado

Un documento con veinte filas, la mayoría en verde, algunas en rojo con su ficha,
y ninguna en blanco sin explicación.

**Y por primera vez en la vida del proyecto, `STATUS.md` puede decir que algo se
probó en un aparato de verdad.**

## 10. No objetivos

- Empaquetado y firma — fase 08. Aquí se instala con `flutter run` o un APK de
  depuración.
- Optimizar el rendimiento. **Mídelo y regístralo**; optimizar es fase 09 si los
  números lo piden.
- Añadir funcionalidad. Si en el bloque E alguien pide algo, **es una ficha, no un
  cambio**.

## 11. Qué desbloquea

La fase 08. Y algo que no es técnico: **la primera vez que se puede decir que Qyro
funciona sin que sea una extrapolación.**
