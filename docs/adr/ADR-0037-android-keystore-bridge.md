# ADR-0037 — La identidad sobrevive al reinicio en Android

- **Estado:** aceptada
- **Fecha:** 2026-08-14
- **Fase:** 06, paso 1
- **Sustituye el plan de la fase 06 en un punto y lo dice**: no se usa `jni-sys`.
- **iOS:** fuera de la v1.0 por ADR-0039. El Keychain queda aplazado.

---

## 1. El problema, y por qué el harness anterior no podía resolverlo

**No hay API de Keystore en el NDK.** La lista oficial de APIs estables del NDK
no la incluye, y por eso QYR-0064 quedó abierta: el harness de la fase 4D.1
empujaba un binario a `/data/local/tmp` y lo ejecutaba, **y un binario suelto no
tiene JVM, ni `Context`, ni proceso de aplicación**. Estructuralmente no podía
alcanzar Keystore, y decirlo fue correcto.

Así que hacen falta dos cosas distintas:

1. **Un camino** que llegue a Keystore desde donde el motor vive.
2. **Una prueba** que corra *dentro de una aplicación*, bajo `am instrument`, no
   un ejecutable en `/data/local/tmp`.

---

## 2. La decisión: el envoltorio cruza la frontera, no la JVM

`qyro_identity_store::SecretWrapper` es un trait con tres métodos: `wrap`,
`unwrap` y `wrap_id`. **Rust no llama a Keystore. Rust recibe dos punteros a
función.**

```
qyro_identity_set_wrapper(wrap_fn, unwrap_fn, context) -> i32
```

Kotlin implementa las dos con `AndroidKeyStore`; Dart las registra al arrancar;
`qyro_session::BridgedWrapper` las adapta al trait.

### Por qué no `jni-sys`, que estaba pre-autorizada

ADR-0025 §1.4 la aceptó, y el argumento era bueno: transcribir tres firmas y
transcribir una tabla de 233 punteros cuyo **orden es la ABI** no son el mismo
riesgo, así que para JNI se toma la caja.

**Pero esa comparación era entre «escribir JNI a mano» y «tomar `jni-sys`». Aquí
hay una tercera opción que ninguna de las dos consideraba: no hacer JNI.**

- **Cero dependencias nuevas de Rust.** El conteo de `Cargo.lock` no se mueve.
- **Cero `unsafe` nuevo fuera de `qyro_ffi`**, que es donde `unsafe` ya vive.
  `qyro_identity_store` y `qyro_session` conservan `#![forbid(unsafe_code)]`.
- **Rust deja de necesitar un `Context`.** El objeto que Keystore exige es del
  lado de Android y se queda ahí; lo que cruza son bytes.
- **Y funciona igual en Windows**, donde el envoltorio es DPAPI y no hay JVM
  ninguna: un puntero a función es la misma cosa en las dos plataformas.

El coste es una indirección más en una ruta que se ejecuta dos veces por arranque.

### Lo que NO cambia

- **Ninguna clave privada llega a Dart.** Lo que cruza es el blob **envuelto** —
  ya cifrado por Keystore— y la semilla **en claro sólo dentro de Rust**. La
  función de envolver recibe bytes y devuelve bytes; Dart nunca los ve porque
  nunca los pide: los punteros los llama Rust, no Dart.
- **Ninguna clave en `SharedPreferences`.** El blob envuelto va al directorio
  privado de la aplicación, y lo que lo protege es Keystore, no el sistema de
  archivos.
- `wrap_id` del puente es **3**, distinto del 1 de DPAPI, para que un blob de
  una plataforma no se abra como el de otra.

---

## 3. La clave de Keystore

- Alias `qyro.identity.v1`, AES-256-GCM, generada al primer uso.
- **`setUserAuthenticationRequired(false)`**: la identidad tiene que estar
  disponible al arrancar la aplicación, y exigir huella dactilar para *existir*
  convertiría «tengo identidad» en «el usuario está mirando». La decisión de
  confianza sigue siendo de la persona; la identidad del aparato no es un
  secreto que la persona custodie.
- **`setRandomizedEncryptionRequired(true)`**, que es el defecto y se escribe
  igualmente: un IV que el llamante eligiera es un IV que el llamante puede
  repetir, y repetir un IV en GCM rompe la confidencialidad y la autenticación a
  la vez.
- El IV de doce bytes se antepone al texto cifrado. El formato del blob envuelto
  es `IV ‖ ciphertext ‖ tag`, y **quien lo escribe es el mismo que lo lee**.
- **`StrongBox` no se exige.** No está en todos los aparatos, y una aplicación
  que se negara a arrancar sin él sería una aplicación que no arranca en la mitad
  del parque.

---

## 4. La prueba, que es la mitad que faltaba

QYR-0064 se cierra con **un test instrumentado**, no con un binario empujado:

```
./gradlew connectedDebugAndroidTest
# o, contra un APK ya instalado:
adb shell am instrument -w com.owner.qyro.test/androidx.test.runner.AndroidJUnitRunner
```

Lo que ejercita, y en este orden, porque el orden es la prueba:

1. Genera una identidad y la guarda.
2. **Mata el proceso.** `Runtime.getRuntime().exit(0)` en el proceso de prueba no
   sirve: mata el runner. Se usa un segundo proceso —`:keystore` en el
   manifiesto— y se le pide morir.
3. Un proceso nuevo carga y **compara la huella**.

**Dos llamadas dentro de un proceso no prueban nada**: el sistema operativo entre
ellas es el sujeto de la prueba. Es la misma forma que la evidencia de DPAPI en
Windows, que ya dice `"process_invocations":2`.

---

## 5. Lo que esta decisión NO promete

- **No promete que la clave sea inextraíble.** Keystore la mantiene fuera del
  espacio de direcciones de la aplicación en los aparatos con TEE; en un
  emulador es software. La diferencia se declara y no se disimula.
- **No promete iOS.** ADR-0039.
- **No promete nada visto en un teléfono.** Hasta la fase 07 esto corre en un
  emulador de CI, y un emulador no es hardware.

---

## Enmienda 1 (2026-08-16) — el mecanismo se retira, la decisión no

**§3 dice «Dart las registra al arrancar». No es implementable.** ADR-0040 §6 da
las cuatro razones; la que basta sola es la cuarta: la callback tendría que
alcanzar Keystore por `MethodChannel`, que completa por el bucle de eventos del
isolate, dentro de una callback síncrona anidada en una llamada FFI bloqueante
que **es** ese isolate. El `Future` no puede completarse nunca. Interbloqueo
garantizado, no carrera.

**Lo que esta ADR decidió bien y se conserva entero:** que Rust no llame a la
JVM, que crucen dos punteros a función, que `SecretWrapper` sea la costura, y que
la prueba corra bajo `am instrument` dentro de un proceso de aplicación. Nada de
eso cambia. Lo que cambia es **quién produce los punteros**: un shim en C
compilado por el NDK, no Dart.

**Y esta ADR nunca se conectó a nada.** `BridgedWrapper` no se construye en
ninguna parte del producto: la única referencia fuera de su módulo y de su
contrato es el re-export de `lib.rs`. El título —«La identidad sobrevive al
reinicio en Android»— es falso de la aplicación y siempre lo fue.

**Un defecto que este mecanismo escondía.** `BRIDGED_WRAP_ID` es 3 y
`blob::KNOWN_WRAPS` es `[1, 2]`, así que un blob sellado por el puente **no se
puede volver a abrir**. El contrato del envoltorio lo ejercitaba contra
`entropy_for` y nunca a través de `seal_identity`/`open_identity`. Se corrige en
ADR-0040 §5.

**La v1.0 sale sin Keystore** (ADR-0040 §7), con el sandbox por UID y dicho en
voz alta en el modelo de amenazas. La etapa B se hace con un teléfono delante,
porque un shim JNI que nadie ha ejecutado nunca no es una función: es una
afirmación.
