# FASE 06 — Identidad persistente en Android e iOS

## 1. Objetivo

**Que la identidad de un aparato y su lista de peers conocidos sobrevivan al
reinicio en Android y en iOS**, como ya sobreviven en Windows con DPAPI.

## 2. Por qué esta fase va aquí

**Depende de:** fase 05 (para tener dónde probarlo de verdad).

**Y por qué no fue antes:** se aparcó en el sprint 4D.2a con evidencia, y la
decisión fue correcta. Nada del camino crítico la necesitaba: el motor corre en
memoria, el filesystem no la necesita, la LAN tampoco. Lo único que la necesita de
verdad es **la confianza**, y la confianza sólo significa algo cuando hay un
producto en el que usarla.

**Ahora sí es urgente**, y por una razón concreta: la fase 05 dejó a Qyro
generando una identidad nueva en cada arranque en móvil. **Eso hace que la lista de
peers conocidos no valga nada** — el otro aparato te ve como un desconocido
distinto cada vez, y el aviso de «clave cambiada» que la fase 04 construyó se
dispara constantemente hasta que la gente aprende a ignorarlo. **Un aviso de
seguridad que salta siempre es peor que no tenerlo.**

## 3. Lo que ya está decidido — no lo reabras

**ADR-0025 está congelada desde el sprint 4D.2a**, 369 líneas, con cinco fuentes
primarias. Decide, para Android:

- **`AndroidKeyStore` con TEE, sin StrongBox.**
- **Sin autenticación de usuario** para desenvolver.
- El blob en `getNoBackupFilesDir()`.
- **`wrap = 0x02`** registrado en el formato del blob.
- Descartado explícitamente hablar con `keystore2` por binder a pelo.
- **`jni-sys`, no `jni`** — la justificación está en §1.4 y es sólida: DPAPI son
  tres símbolos planos, JNI es una tabla de ~233 punteros **cuyo orden es la ABI**.

**Léela entera antes de escribir nada.** Y lee `qyro_identity_store/src/lib.rs`:
el contrato `IdentityStore` / `SecretWrapper` ya está probado contra una
plataforma real, que era el objetivo de hacer Windows primero.

## 4. QYR-0064 — el error que hay que respetar

**Está abierta desde hace cuatro meses y es un P1.** El prompt del sprint 4D.2a
mandó usar el patrón de `qyro_store_smoke`: un binario nativo empujado a
`/data/local/tmp` y lanzado con `adb shell`.

**Eso no puede alcanzar Keystore.** Documentación oficial de Android:

> «To use this feature, you use the standard `KeyStore` and `KeyPairGenerator` or
> `KeyGenerator` classes along with the **AndroidKeyStore provider**»

> «Use the Android Keystore provider to let an individual app store its own
> credentials, **which only that app can access**»

`AndroidKeyStore` es un proveedor **JCA de Java**, **no hay API en el NDK**, y las
claves están separadas por aplicación. Un binario que corre como el usuario
`shell` no tiene ni la API ni la identidad.

**Hace falta un test instrumentado bajo `am instrument`, con el andamiaje Gradle
que eso arrastra.** Es materialmente más trabajo de lo que se scopeó entonces, y
por eso esta fase existe como fase entera y no como un paso de otra.

**No improvises un harness que dé verde sin probar nada.** El sprint 4D.2a paró y
lo documentó, y fue lo correcto.

## 5. Lo que hay que construir, paso a paso

### Paso 1 — El andamiaje del test instrumentado

**Va primero, antes que el código de Keystore.** El motivo: si el harness no se
puede construir, todo lo demás es teoría — que es exactamente lo que QYR-0064
descubrió.

- Módulo de test instrumentado en `apps/qyro/android`, corriendo bajo
  `am instrument`.
- **Una prueba trivial que demuestre que el harness funciona**: escribe algo en
  Keystore desde el test, léelo, y comprueba que corre como la app y no como
  `shell`.
- **Y que corra en `android-runtime.yml`**, que ya existe y ya levanta un
  emulador.

**Puerta.** Si el harness no se puede construir, **para y reporta**. No sigas.

### Paso 2 — El crate de Android

- `qyro_android_keystore`, con `jni-sys` según ADR-0025 §1.4.
- **`Cargo.lock` sube: dilo, con el conteo, el diff y `cargo audit`.**
- `SecretWrapper` implementado con `wrap = 0x02`.
- `unsafe` acotado con `SAFETY:` escrito, y **la lista de crates exentos de
  `forbid(unsafe_code)` actualizada y justificada** — ese número es una guarda.
- `guards.rs` con el conjunto mínimo, o excepción con razón escrita.

**Puerta.**

### Paso 3 — La prueba que importa

**Generar una identidad, matar el proceso, reabrir, y comprobar que es la misma
identidad.** En el emulador, bajo `am instrument`.

Y las negativas, que son las que dan valor:

- **Un blob de otra plataforma se rechaza por `wrap`, no por casualidad** — ya
  existe esa prueba en el crate compartido; **hazla correr en Android**.
- Un blob truncado se rechaza sin cargar nada a medias.
- Una versión futura se rechaza **nombrándola**.
- **QYR-0065 y QYR-0066**, las dos abiertas de este bloque: qué pasa cuando la
  clave se invalida, y **qué error exacto da Keystore cuando el alias ya no
  existe**. La segunda dice literalmente «no está medido». **Mídelo.**

**Puerta.**

### Paso 4 — iOS con Keychain

- ADR propia — `docs/adr/ADR-0037-ios-keychain.md`— porque Keychain **no es
  Keystore**: tiene clases de accesibilidad, sincronización con iCloud, y
  comportamiento distinto en backup y restauración.
- **Decide y escribe:** la clase de accesibilidad —`kSecAttrAccessibleWhenUnlocked`
  o `...ThisDeviceOnly`, y **la diferencia importa**: sin `ThisDeviceOnly` el
  secreto puede acabar en un backup y restaurarse en otro aparato—; si se
  sincroniza con iCloud (**no**); y qué pasa al reinstalar la app.
- `wrap = 0x03` registrado en el formato del blob.
- Prueba en simulador, bajo XCTest, con `ios-runtime.yml` que ya existe.

**Puerta.**

### Paso 5 — Los peers conocidos, también

`known_peers` tiene su propio sellado. **Comprueba que usa el mismo
`SecretWrapper`** y por tanto hereda las tres plataformas. Si no lo usa, ésta es
la fase de conectarlo.

**Puerta de fase.**

## 6. Las trampas concretas

1. **El harness que da verde sin probar nada.** Es la trampa de QYR-0064 y ya
   costó un sprint. Si el test no corre como la app, no está probando Keystore.
2. **El simulador de iOS no es un iPhone.** Keychain se comporta distinto: en el
   simulador no hay Secure Enclave y la accesibilidad tras reinicio no se puede
   ejercitar de verdad. **Todo lo que se pruebe ahí tiene clase de evidencia
   "probado en simulador" y no más.** La fase 07 lo cierra.
3. **`wrap` sin comprobar.** Con un solo envoltorio la pregunta no surge; con tres,
   un blob de Windows entregado al envoltorio de Android llegaría hasta `unwrap` y
   volvería como fallo de plataforma, indistinguible de un archivo corrupto. **Ya
   se cerró esa clase en el sprint 4D.2a; no la reabras.**
4. **La accesibilidad de Keychain por defecto.** Si no eliges `ThisDeviceOnly`, la
   identidad puede viajar en un backup. Decide a conciencia.
5. **La reinstalación.** En Android, `getNoBackupFilesDir()` se borra; en iOS, el
   Keychain **sobrevive a la desinstalación** en algunas configuraciones. **Son
   comportamientos opuestos y hay que decidir qué quiere Qyro en cada uno.**
6. **La dependencia nueva.** `jni-sys` es la primera dependencia externa real del
   core en siete sprints. **Justifícala en el informe como si fuera la primera
   vez**, con el árbol medido.

## 7. Pruebas obligatorias

- Android, instrumentado: `the_harness_runs_as_the_app_and_not_as_shell`
- Android: `an_identity_survives_a_process_death_and_reopens_identical`
- Android: `a_blob_from_another_platform_is_refused_by_wrap_and_not_by_luck`
- Android: `a_missing_alias_produces_the_error_this_test_names` — cierra QYR-0066
- Android: `an_invalidated_key_produces_the_error_this_test_names` — QYR-0065
- iOS, simulador: `an_identity_survives_a_relaunch_and_reopens_identical`
- iOS: `the_keychain_item_is_marked_this_device_only` — o el argumento escrito
- Las tres plataformas: `known_peers_survive_the_same_way_the_identity_does`

## 8. Criterios de aceptación

1. **El harness instrumentado existe, corre en CI, y hay una prueba que demuestra
   que corre como la app.**
2. **QYR-0064 cerrada** con esa evidencia.
3. Una identidad sobrevive a la muerte del proceso en **emulador Android** y a un
   relanzamiento en **simulador iOS**.
4. `wrap = 0x02` y `wrap = 0x03` registrados en el formato, y **un blob de otra
   plataforma se rechaza por `wrap`** en las tres.
5. **QYR-0065 y QYR-0066 cerradas con el error medido**, no supuesto.
6. ADR-0037 congelada antes del código de iOS, con la clase de accesibilidad
   decidida y argumentada.
7. La lista de crates exentos de `forbid(unsafe_code)` actualizada **con
   justificación escrita**.
8. **`jni-sys` justificada como si fuera la primera dependencia**: árbol medido con
   `cargo tree` en los tres targets, licencia, `cargo audit`, alternativa
   descartada.
9. Los peers conocidos heredan la misma persistencia.
10. Barrido con `cargo-mutants`. `R2` en todas las puertas. Informe según `R5`.
11. **§15 dice: probado en emulador y simulador. NO en hardware físico. Keychain
    en un simulador no ejercita el Secure Enclave.**

## 9. Cómo tiene que quedar el resultado

Cierras Qyro en el teléfono, lo reinicias, lo vuelves a abrir, **y el otro aparato
te sigue reconociendo**. El aviso de clave cambiada deja de saltar, y por tanto
vuelve a significar algo.

## 10. No objetivos

- Hardware físico — fase 07.
- StrongBox, autenticación de usuario para desenvolver, sincronización con iCloud.
  **ADR-0025 los descartó y no se reabren.**
- Empaquetado, firma.

## 11. Qué desbloquea

La fase 07, donde por primera vez esto se prueba en aparatos de verdad — y donde
Keychain y Keystore enseñan lo que un simulador no puede enseñar.
