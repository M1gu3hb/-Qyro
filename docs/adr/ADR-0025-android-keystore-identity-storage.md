# ADR-0025 — Persistencia de `DeviceIdentity` en Android

- Estado: **congelada** antes de escribir código de almacenamiento en Android.
- Fecha: 2026-08-07
- Sprint: 4D.2a
- Continúa: ADR-0024 (Windows/DPAPI), ADR-0023 (harness por plataforma).
- Reemplaza: nada. **No modifica** `IdentityStore` ni `SecretWrapper`.

Todas las fuentes se consultaron el **2026-08-07**. Donde no hay fuente
primaria, esta ADR lo dice y lo deja abierto; no lo supone. Es la regla que
ADR-0024 usó con la reinstalación de Windows y sigue vigente.

## Contexto

El sprint 4D.1 dejó una identidad que sobrevive al cierre del proceso **en
Windows**. `SecretWrapper` es la costura —`wrap`, `unwrap`, `wrap_id`— y el
formato del blob está congelado: cabecera de dieciséis bytes, entropía =
constante ‖ `cabecera[0..12]`, nueve pasos de lectura, doce errores tipados.

Android tiene que entrar por esa costura sin ensancharla. Lo que sigue decide
cómo, y una de las decisiones **contradice una suposición del prompt del
sprint**; está en §1 y es el hallazgo principal de esta ADR.

## 1. Cómo se alcanza Keystore, y qué obliga eso

### 1.1 No existe API nativa

Android Keystore **no tiene API en el NDK**. La página de APIs nativas estables
enumera C library, C++ library, logging, trace, zlib, OpenGL ES, EGL, Vulkan,
bitmaps, sync, cámara, media, asset, choreographer, configuration, input,
looper, native activity, hardware buffers, native window, memory, networking,
sensor, storage, SurfaceTexture, **binder**, AAudio, OpenSL ES y NNAPI. No hay
keystore, ni keychain, ni gestión de claves ([NDK stable APIs][ndk]).

Esto es una **ausencia comprobada en la lista**, no una cita. Se registra como
tal porque una ausencia se argumenta distinto que una afirmación.

La ruta soportada es Java, desde el proceso de la aplicación:

> «`AndroidKeyStore` is an implementation of the standard Java Cryptography
> Architecture APIs, but also adds Android-specific extensions and consists of
> Java code that runs in the app's own process space.» ([AOSP Keystore][aosp])

> «`AndroidKeyStore` fulfills app requests for Keystore behavior by forwarding
> them to the keystore daemon.» ([AOSP Keystore][aosp])

Y las claves se separan por UID del llamante:

> «the UID of the caller is also included to disambiguate keys from different
> apps, preventing one app from accessing another's keys» ([AOSP Keystore][aosp])

### 1.2 Consecuencia: el harness de 4D.1 no sirve aquí (QYR-0064)

`android_crypto_smoke.sh` empuja un **ejecutable nativo** a
`/data/local/tmp` y lo lanza con `adb shell`. Ese proceso:

- no tiene runtime ART con las clases del framework, así que no puede instanciar
  `java.security.KeyStore.getInstance("AndroidKeyStore")`;
- corre como el UID del shell, no como el UID de una aplicación, así que aunque
  pudiera, las claves no serían las de Qyro.

De 1.1 se sigue que **la persistencia en Android no se puede demostrar con la
forma de harness que 4D.1 usó**, y el prompt de 4D.2a §8.4 la da por buena
(«empujado por `adb` como hace `android_crypto_smoke.sh`»). Es un hallazgo de
especificación, del mismo tipo que QYR-0048 en 4D.1: la suposición es razonable
y no se sostiene contra la plataforma.

Lo que sí puede demostrarlo, y es lo que esta ADR fija: **un test instrumentado
de Android**, ejecutado con `am instrument` dentro de un proceso de aplicación,
con un UID de aplicación. Dos invocaciones separadas de `am instrument` son dos
procesos, que es lo que la propiedad exige. Sigue siendo `adb`, sigue siendo el
emulador, y sigue siendo un harness aislado del producto según ADR-0023 — lo que
cambia es que el proceso es una app y no un binario de shell.

**Esta ADR no construye ese harness; lo especifica.** El coste real de la
decisión es andamiaje Gradle nuevo (módulo, manifiesto, runner de
instrumentación, empaquetado de la `.so` en `jniLibs`), y por eso queda escrito
antes de empezarlo en vez de descubrirse a mitad.

### 1.3 La alternativa descartada: binder a pelo contra `keystore2`

`binder` **sí** está en la lista de APIs nativas del NDK, y `keystore2` expone
su interfaz por AIDL. Se podría escribir un cliente AIDL a mano desde Rust
nativo y saltarse la JVM.

Se descarta. Sería transcribir a mano una interfaz de sistema versionada, con
sus parcelaciones, para evitar una dependencia — y el resultado sería mucha más
superficie `unsafe` que la de todo el sprint 4D.1 junto, contra una API cuya
estabilidad no está prometida a las aplicaciones. Es exactamente lo que
«no inventes criptografía» y ADR-0024 §1 argumentan en contra, sólo que aquí el
invento sería un protocolo IPC en vez de un cifrado.

### 1.4 La dependencia: `jni-sys`, y por qué no se escribe a mano

ADR-0024 §1 escribió a mano el `extern` de DPAPI en vez de traer `windows-sys`,
porque eran **dos declaraciones de función** y traerlas costaba once crates. El
mismo razonamiento, aplicado aquí, apunta al lado contrario.

JNI no se alcanza por símbolos con nombre: se alcanza por una **tabla de ~233
punteros a función en orden fijo**, `JNINativeInterface`. El orden *es* la ABI.
Transcribirla a mano significa que un índice equivocado llama a otra función con
otra firma, en silencio, y ninguna prueba de round trip lo detecta como tal — a
diferencia de `DATA_BLOB`, donde un campo mal declarado rompe el protect/unprotect
y la prueba `a_data_blob_that_lies_does_not_round_trip` lo dice.

Se decide **`jni-sys`**, que es la definición `#[repr(C)]` de esa tabla y nada
más. Coste medido en este árbol, no estimado:

| Candidato | Entradas nuevas en el grafo |
|---|---|
| `jni-sys` 0.4.1 | **2** — `jni-sys` y `jni-sys-macros` |
| `jni` 0.22.4 | ~11 — añade `combine`, `bytes`, `memchr`, `cfg-if`, `simd_cesu8`, `simdutf8`, `rustc_version`, `semver` y los macros |

`proc-macro2`, `quote`, `syn` y `unicode-ident` ya están en `Cargo.lock`, así
que no cuentan como entradas nuevas. Se elige `jni-sys` y **no** `jni`: la capa
segura de `jni` es cómoda y trae ocho crates más para envolver llamadas que Qyro
hace una vez cada una.

**Esto rompe la racha de cero dependencias externas del sprint 4D.1**, y se dice
en vez de disimularlo. Las dos entradas se justifican aquí, se auditan con
`cargo audit` como todo lo demás, y su licencia entra en `docs/LICENSE_AUDIT.md`
antes de que el crate compile.

### 1.5 `unsafe`

El crate de Android necesitará `unsafe`: llamar por puntero a función es
`unsafe` por construcción. Entra como **cuarta entrada argumentada** en
`CRATES_THAT_MAY_RELAX_FORBID_UNSAFE`, y su guarda
`the_unsafe_blocks_are_the_ones_we_listed` se escribe **con la lista vacía antes
del primer bloque**, como se hizo en `qyro_win_dpapi`. Escribirla después es
escribirla contra el resultado.

## 2. Envolver, no guardar

Keystore no guarda una semilla ajena:

> «Key material never enters the application process. When an app performs
> cryptographic operations using an Android Keystore key, behind the scenes
> plaintext, ciphertext, and messages to be signed or verified are fed to a
> system process that carries out the cryptographic operations.»
> ([Android Keystore system][ks])

> «Once keys are in the keystore, you can use them for cryptographic operations,
> with the key material remaining non-exportable.» ([Keystore, training][kst])

Así que Keystore **no puede ser el almacén de la semilla**; puede ser el
envoltorio. Se decide:

1. Generar en Keystore una clave **AES-256-GCM no exportable**, con alias
   `qyro.identity.wrap.v1`.
2. Usarla para envolver la semilla Ed25519 de 32 bytes.
3. Guardar el blob envuelto en el almacenamiento privado de la aplicación.

Esto hace que Keystore ocupe **exactamente el sitio de DPAPI**: `wrap` y
`unwrap` de `SecretWrapper` sin tocar el trait. Que la costura aguante una
segunda plataforma sin ensancharse es la prueba de que estaba bien puesta; si
hubiera que ensancharla, eso sería el hallazgo y no un cambio.

Lo que esto **sí** cambia respecto de DPAPI: el secreto ya no está protegido por
una credencial de usuario derivada, sino por una clave que vive en el TEE. La
propiedad que se gana es que la semilla no se puede extraer copiando el archivo;
la que **no** se gana es protección contra código que ya corre como esta
aplicación, igual que en Windows.

## 3. Las cuatro sub-decisiones

### 3.1 StrongBox o TEE: **TEE, sin StrongBox**

> «StrongBox is appropriate for applications requiring the highest level of
> security, particularly those at risk of physical tampering or side-channel
> attacks. However, it is slower, more resource-constrained, and supports fewer
> concurrent operations. **For most apps, StrongBox is not necessary.**»
> ([Android Keystore system][ks])

Qyro envuelve una semilla al arrancar y la desenvuelve al arrancar: una
operación, no concurrente, no caliente. El argumento de rendimiento no decide
nada aquí. Lo que decide es la disponibilidad:

> «Devices running Android 9 (API level 28) or higher **can** include a StrongBox
> KeyMint» ([Android Keystore system][ks])

«Can», no «do». Y el fallo es explícito:

> «If the StrongBox KeyMint does not support the specified algorithm or key size,
> the framework will throw a `StrongBoxUnavailableException`. If this occurs,
> generate or import the key without calling `setIsStrongBoxBacked(true)`.»
> ([Android Keystore system][ks])

**Decisión: no se pide StrongBox.** El emulador de CI casi con seguridad no lo
tiene, así que pedirlo obligaría a una ruta de degradación que solo se
ejercitaría en el camino de fallo — y una degradación silenciosa de StrongBox a
TEE es peor que no pedirlo: deja al lector creyendo que hay StrongBox.

**Qué pasa en un dispositivo sin StrongBox: nada, porque no se pide.** Si en el
futuro se pide, la respuesta correcta no es degradar en silencio sino registrar
en qué respaldo quedó la clave y decírselo al usuario. Queda fuera de 4D.2a.

### 3.2 Autenticación de usuario: **no se exige**

Qyro no pide contraseña para arrancar. Si la identidad exigiera autenticación,
la app no podría leerla en frío y el arranque cambiaría de forma — eso es una
decisión de producto que este sprint no tiene mandato para tomar.

**Decisión: `setUserAuthenticationRequired(false)`**, es decir, no llamarlo.

Las consecuencias, con el nivel de confianza de cada fuente marcado:

- **Fuente primaria, citada:** ninguna de las páginas que se pudieron obtener
  documenta qué le ocurre a una clave **sin** requisito de autenticación cuando
  se quita el bloqueo de pantalla.
- **Fuente secundaria, sin verbatim:** las páginas de referencia de
  `KeyGenParameterSpec.Builder`, `KeyPermanentlyInvalidatedException` y
  `KeyProtection.Builder` se renderizan con JavaScript y **no se pudieron
  obtener** en esta sesión; lo que se tiene de `setInvalidatedByBiometricEnrollment`
  —que por omisión es `true`, y que sólo afecta a claves que admiten credenciales
  biométricas— viene de resúmenes de búsqueda sobre esas páginas, no del texto.
  **No se cita como verbatim y no se apoya ninguna decisión en ello.**

Como no se exige autenticación, `setInvalidatedByBiometricEnrollment` **no
aplica**: gobierna claves que admiten credenciales biométricas, y ésta no admite
ninguna. La decisión no depende de la fuente que falta, y eso es deliberado: se
eligió el camino que no necesita el dato que no se pudo verificar.

**Queda abierto (QYR-0065):** confirmar contra la página de referencia, cuando
se pueda obtener, que una clave sin `setUserAuthenticationRequired` sobrevive a
quitar y volver a poner el bloqueo de pantalla. Si no sobrevive, la identidad de
Qyro se pierde en un cambio de PIN y eso cambia esta ADR.

### 3.3 Backup, restore y migración de dispositivo: **abierto, no supuesto**

La página de Keystore **no cubre** backup, restore ni migración. La de Auto
Backup **no menciona** Keystore. Lo comprobé en las dos y lo digo así.

Lo que sí está documentado y es suficiente para decidir el diseño:

> «Once keys are in the keystore, you can use them for cryptographic operations,
> with the key material remaining non-exportable.» ([Keystore, training][kst])

> «If the app's process is compromised, the attacker might be able to use the
> app's keys but can't extract their key material (for example, to be used
> outside of the Android device).» ([Keystore, training][kst])

Una clave no exportable y ligada al dispositivo **no puede viajar en un
respaldo**. De ahí se sigue que si el blob envuelto viaja y la clave no, el blob
llega ilegible al dispositivo nuevo.

**Eso es exactamente el comportamiento correcto para una identidad de
dispositivo**, y es la misma conclusión que ADR-0024 sacó para `LOCALAPPDATA`: si
la identidad viaja, dos aparatos presentan el mismo dispositivo. Aquí la
plataforma lo impone sin que Qyro tenga que hacer nada.

**Lo que queda abierto, registrado y no supuesto (QYR-0066):** cuál es
exactamente el error que devuelve `Cipher.doFinal` cuando la clave del alias no
existe tras un restore —¿`KeyPermanentlyInvalidatedException`, `AEADBadTagException`,
o ausencia del alias?—, porque de eso depende que Qyro distinga «no hay
identidad» de «hay una y no se puede leer», que es la distinción que el paso 1
del orden de lectura existe para hacer. **No lo supongo.** Se mide contra el
emulador cuando el harness exista, o se marca como no medido.

### 3.4 Dónde vive el blob, y `android:allowBackup`

**Decisión: `getNoBackupFilesDir()`.**

> «Apps that target Android 6.0 (API level 23) or higher automatically
> participate in Auto Backup. In your app manifest file, set the boolean value
> `android:allowBackup` to enable or disable backup. **The default value is
> `true`**» ([Auto Backup][ab])

> «Auto Backup excludes files in directories returned by `getCacheDir()`,
> `getCodeCacheDir()`, and `getNoBackupFilesDir()`.» ([Auto Backup][ab])

`getFilesDir()` **sí** entra en el respaldo por defecto. Poner ahí el blob haría
que viajara a otro dispositivo un archivo que allí no se puede abrir: no es una
fuga —está envuelto por una clave que no viaja— pero es basura que llega
disfrazada de identidad, y el modo de fallo es «la app cree que tiene identidad
y no la tiene».

`getNoBackupFilesDir()` lo resuelve **sin depender de `android:allowBackup`**,
que es lo importante: `allowBackup` es una decisión de la aplicación entera y
esta ADR no manda sobre ella. Elegir el directorio hace la propiedad local al
almacén.

## 4. El IV de GCM

Es lo único de este sprint que es criptografía propia y se trata como tal.

**AES-GCM con un IV repetido bajo la misma clave pierde la confidencialidad del
texto claro y permite falsificar tags.** No es una degradación gradual: es la
propiedad rota.

Decisión, en tres partes:

1. **El IV lo genera Keystore, no Qyro.** El proveedor `AndroidKeyStore` genera
   un IV aleatorio por operación cuando se cifra, y rechaza que el llamante
   imponga uno si la clave se creó sin `setRandomizedEncryptionRequired(false)`.
   Qyro **no** lo desactiva. Un contador propio sobre una clave que sobrevive
   reinstalaciones es la forma exacta de repetir un IV sin darse cuenta.
2. **El IV vive dentro del envoltorio**, no en la cabecera de Qyro: `iv_len`
   (u8) ‖ `iv` ‖ ciphertext‖tag. Va dentro de `wrapped`, que el formato del blob
   trata como opaco, así que **el formato congelado no cambia**.
3. **El IV no se deriva de nada.** Ni del alias, ni de la semilla, ni de un
   contador, ni del tiempo. Cualquier derivación con un valor que se repita
   —reinstalar, restaurar, rotar dos veces— repite el IV.

`iv_len` va explícito y se comprueba al leer, en vez de asumir doce bytes: un
campo de longitud que nadie valida es la forma de que un envoltorio de otra
versión se lea como éste.

## 5. El byte `wrap` nuevo: `0x02`

| `wrap` | Envoltorio |
|---|---|
| `0x01` | DPAPI, ámbito de usuario (ADR-0024) |
| `0x02` | **Android Keystore, AES-256-GCM, alias `qyro.identity.wrap.v1`** |

Añadir un valor **es un cambio de formato** y se registra como tal aquí. Lo que
no cambia: la longitud de la cabecera, el orden de lectura, la construcción de
la entropía, ni ninguna otra cosa.

La entropía sigue siendo `QYRO_IDENTITY_ENTROPY_V1 ‖ cabecera[0..12]`, y
`cabecera[0..12]` incluye el byte `wrap`. Es decir: **el byte que dice qué
envoltorio es está dentro de la entropía**, así que un blob de Windows
presentado como Android no sólo falla por `wrap` desconocido — si alguien
cambiara el byte para disimularlo, cambiaría la entropía y el desenvoltorio
fallaría igual. La propiedad ya estaba; aquí sólo se hace explícita.

En Android, `entropy_for` sigue existiendo y se pasa como **AAD** de AES-GCM,
que es el sitio que le corresponde: datos asociados autenticados y no cifrados.

## Alternativas descartadas

- **Guardar la semilla *en* Keystore.** No se puede: Keystore guarda claves que
  genera él, y la semilla Ed25519 viene de `DeviceIdentity`. Importar una clave
  simétrica arbitraria es posible, pero entonces no sería una semilla Ed25519
  sino bytes que Keystore trataría como clave AES, y sacarla otra vez es
  precisamente lo que la no exportabilidad impide.
- **Cambiar la identidad a una clave que Keystore pueda generar.** Rompería
  ADR-0021: el handshake está congelado sobre Ed25519. Es además la pregunta que
  4D.2b tiene que responder para iOS, y responderla aquí a la ligera la
  prejuzgaría.
- **`jni` en vez de `jni-sys`.** Ocho crates más para envolver llamadas que se
  hacen una vez cada una.
- **Escribir la tabla JNI a mano.** §1.4.
- **Binder a pelo contra `keystore2`.** §1.3.
- **Reutilizar el harness de binario empujado.** §1.2. No puede alcanzar
  Keystore.

## No objetivos

iOS Keychain (es 4D.2b), transporte, transferencia, FFI criptográfica,
emparejamiento, atar la identidad a un valor de hardware más allá de lo que la
clave del TEE ya da, y `android:allowBackup` como decisión de la aplicación.

## Lo que esta decisión no promete

- **No protege contra código que ya corre como esta aplicación.** Ese atacante
  llama a `Cipher.doFinal` con el mismo alias y obtiene la semilla. Es la misma
  limitación que DPAPI y está en `THREAT_MODEL.md`.
- **No se ha probado en hardware físico.** Un emulador no es un teléfono, y el
  respaldo de claves de un emulador no es un TEE real. Ninguna afirmación de
  esta ADR sobre StrongBox o TEE queda demostrada por correr en CI.
- **No cubre backup, restore ni migración**, más allá de lo que §3.3 deriva.
  Los dos huecos están en QYR-0065 y QYR-0066, abiertos.

[ks]: https://developer.android.com/privacy-and-security/keystore
[kst]: https://developer.android.com/training/articles/keystore
[aosp]: https://source.android.com/docs/security/features/keystore
[ab]: https://developer.android.com/guide/topics/data/autobackup
[ndk]: https://developer.android.com/ndk/guides/stable_apis
