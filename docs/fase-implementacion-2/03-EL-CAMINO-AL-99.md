# El camino al 99 % — qué queda, y qué significa «terminado»

> Medido sobre `c4252a5`, en `main`, 2026-08-19. Linux **766 pruebas / 0 fallos**,
> ledger 167 con **1 abierta**, 513 commits, una sola rama, todos a nombre del
> propietario.

---

## 1. Qué es el 99 %, y por qué no es el 100 %

**99 % = todo lo que un implementador puede hacer, hecho y verificado.**
**El 1 % que falta es la evidencia de hardware, y sólo la puede producir el
propietario.** No es una reserva de humildad: es la fase 19, con nombre y
escenarios escritos.

Traducido a una frase por bloque, esto es todo lo que queda:

| # | Qué falta | Quién |
|---|---|---|
| 1 | **El teléfono lee un QR.** Hoy `qyro beam` dibuja y **nadie mira** | implementador |
| 2 | **Doscientos archivos cruzan sin mentir** — QYR-0365 | implementador |
| 3 | **El binario arranca en la máquina vieja** — fase 17 | implementador |
| 4 | **Lo que se publica es verdad** — fases 18 y 20, y el APK republicado | implementador |
| 5 | **Alguien mandó un archivo de verdad** — fase 19 | **el propietario** |
| 6 | **La v2.0** — fase 23, y su condición de entrada es (5) | los dos |

**Nada más.** El motor, los cuatro canales en código, las dos caras, la identidad
persistente y la paridad GUI/CLI están hechos y probados.

### Y una cosa que el propietario puede desbloquear en un minuto

El APK lleva **tres sesiones** sin poder reconstruirse, y no por falta de tiempo:
`flutter doctor` encuentra el SDK de Android pero **las licencias no están
aceptadas**, y aceptarlas es firmar un acuerdo legal — que no lo hace el
implementador en nombre de nadie.

```
flutter doctor --android-licenses
```

Sin eso, el APK publicado **sigue sin poder enviar** y la fase 20 no puede cerrar.

---

## 2. Lo que la última sesión hizo bien, y lo que hay que corregir

**Bien, y verificado por mí:** los cinco arreglos dentro · `scripts/gate.ps1`, que
**lee `ci.yml` en vez de llevar su propia lista** —la comprobación 18 hecha
ejecutable, y es la mejor idea de la sesión— · QYR-0089 estaba **duplicada entera**
y lo cazó una guarda · los ignores de `audit.toml` **no se borraron los dos**
porque la condición se cumplió a medias, y eso es exactamente lo que hay que hacer
cuando una instrucción mía es medio falsa.

**Y lo que hay que corregir: la fase 24 no entregó lo que existía para entregar.**

Se construyó `qyro_eye` —570 líneas de Rust que decodifican, con llamante de
producción real en el preflight de `qyro beam`— y se **aplazó el aparato entero**.
Verificado: **cero archivos Kotlin de cámara, cero `androidx.camera` en Gradle,
cero permiso `CAMERA` en el manifest, cero pantalla de escaneo.** El teléfono
sigue sin poder mirar, que es lo único que la fase 24 tenía que arreglar.

El argumento del aplazamiento —que el cruce JNI sería la segunda excepción a
`forbid(unsafe_code)` de todo el taller, y que un slot equivocado no da error de
compilación sino un salto a una función arbitraria— **es correcto y sigue en pie
para el cruce de copia cero.** Pero cubre una de las tres rutas, no las tres:
`R10` §3 ofrecía explícitamente un camino con **cero `unsafe` nuevo**, y el Kotlin,
el manifest y la pantalla de Flutter no tienen nada que ver con `unsafe`.

**El aplazamiento fue más ancho que su argumento.** La fase 24B lo corrige, y la
decisión ya está tomada para que nadie vuelva a deliberarla.

---

## 3. La regla nueva, y es sobre por qué esto va despacio

> **El calibre de la verificación se ajusta al riesgo de lo que se verifica.**

Este taller tiene una cultura de verificación extraordinaria, y **la construí yo**:
la comprobación 14 lleva nueve capacidades muertas encontradas, dejar que una
guarda te contradiga ha acertado once veces de once, y ejecutar lo generado
destapó un Base64 inválido que habría entregado un archivo vacío diciendo «éxito».
**Nada de eso se toca.**

Pero esa misma cultura aplica el mismo ceremonial a un archivo de interfaz que a
la frontera criptográfica, y el resultado se mide: la última sesión produjo
**siete commits** y aplazó la capacidad que era su motivo de existir.

**Tres calibres, y elige tú cuál toca:**

| Calibre | Qué es | Qué exige |
|---|---|---|
| **Alto** | cripto, protocolo de cable, frontera C, `unsafe`, identidad, cualquier cosa que persista o viaje | todo lo de siempre: ADR congelada, control de falsabilidad visto fallar, mutación, las dieciocho comprobaciones |
| **Medio** | motor, canales, sistema de archivos, CI | ADR si cambia una decisión; prueba con control; puerta completa |
| **Bajo** | **pantallas, textos, Kotlin de UI, empaquetado, documentos** | que compile, que haya una prueba que falle sin el cambio, y la puerta. **Sin ADR, sin barrido de mutación, sin tres agentes refutando** |

**Una pantalla de cámara es calibre bajo.** Si está mal, se ve enseguida y no
corrompe nada. Un `#[cfg]` mal puesto en la frontera C es calibre alto. Tratarlos
igual es lo que hace que queden cuatro bloques después de siete meses.

**Y la regla que no cambia con el calibre:** la comprobación 14 se aplica siempre.
Lo que se ajusta es cuánto se delibera antes de escribir, no si se comprueba
después.

---

## 4. El orden

```
24B → QYR-0365 → 17 → 18 → 20 → (19 del propietario) → 23
```

**24B primero**, porque es la última capacidad y porque cierra el nivel 3 de la
escalera de `R7`. **QYR-0365 después**, porque puede tocar el bucle de sesión y eso
cambia lo que la 18 tiene que describir. **17, 18 y 20** son pipeline, verdad y
empaquetado, y ninguna crea capacidades nuevas.

**La 19 no la abre el implementador.** La deja al punto de que el propietario
conecte algo y corra un comando, con los escenarios de los cuatro canales escritos
y en blanco.

---

## 5. Lo que no cambia

Una sola rama, todo a `main`, commits a nombre del propietario, nunca
`Co-Authored-By` · sólo un P0 detiene · `ESTADO-ACTUAL.md` **dentro** del commit de
contenido · quedarse sin contexto es una parada legítima y se dice **una vez** ·
las dieciocho comprobaciones · y **no se inventa evidencia de hardware, que es lo
único que arruinaría esto.**
