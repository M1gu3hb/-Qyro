# ADR-0024 — Persistencia segura de `DeviceIdentity`

Estado: **congelada**. Sprint 4D.1, 2026-08-07.
Ámbito: el contrato de almacén, el formato del blob, y **una** implementación:
Windows DPAPI. Android e iOS llegan en 4D.2 contra este contrato.

Esta ADR se escribe **antes** del código, como ADR-0016, ADR-0017, ADR-0021 y
ADR-0022. El orden de los commits lo demuestra.

## Contexto

Hoy `DeviceIdentity` vive solo en memoria: generar una identidad y cerrar el
proceso la pierde, así que ninguna decisión de confianza sobrevive a un
reinicio. Persistirla obliga a algo que este proyecto ha evitado deliberadamente
durante cuatro sprints: **un camino que entregue la semilla**. `identity.rs:50`
lo dice hoy con estas palabras —«There is **no accessor for the seed or the
private key**»— y este sprint las invalida. Es el cambio más arriesgado desde
que existe `qyro_crypto`, y §4 de esta ADR es el que hay que revisar dos veces.

Windows primero porque `windows-latest` es un host real donde
`crypto-platform.yml` ya compila y ejecuta un harness nativo. Es la única de las
tres plataformas donde la persistencia se prueba **en runtime** sin emulador ni
simulador.

---

## 1. `unsafe` — la decisión estructural

Todos los crates de este repositorio llevan `#![forbid(unsafe_code)]`. DPAPI es
una API de C. Los tres caminos y por qué se elige el tercero:

| Camino | Coste |
|---|---|
| Dependencia (`windows-sys`) | ~11 crates al grafo auditado (`windows-targets` más un crate de enlace por arquitectura), entradas en `LICENSE_AUDIT.md` y `THIRD_PARTY_NOTICES.md`, para dos declaraciones de función |
| `extern "system"` a mano en un crate de producto | Cero dependencias, pero `unsafe` dentro del árbol que hoy lo prohíbe entero |
| **Crate de plataforma aparte, el único que relaja `forbid`** | Cero dependencias, `unsafe` confinado a un crate que no depende nadie más que el almacén |

**Decisión: el tercero, con el `extern` escrito a mano.**

El precedente es ADR-0023: el harness de criptografía se aisló en su propio
crate en lugar de relajar una regla en el árbol del producto. Aquí es lo mismo,
con una diferencia que importa —aquel crate no entra en el producto y este sí—,
así que el aislamiento es de `unsafe`, no de distribución.

Se rechaza `windows-sys` por proporción, no por calidad: este workspace rechazó
`proptest` por arrastrar 39 crates para una herramienta de desarrollo, y traer
once para declarar dos funciones es el mismo intercambio peor. `cargo audit`
vigila hoy 56 crates y esa cifra es un activo.

**La contrapartida, dicha entera:** `windows-sys` se genera de los metadatos
oficiales de Windows y una transcripción a mano puede equivocarse en la ABI o en
el layout de una struct, y equivocarse ahí es comportamiento indefinido, no un
error de compilación. Se acepta porque la superficie es mínima y estable desde
Windows 2000, y se mitiga con una prueba de ida y vuelta contra la API real en
CI: un `DATA_BLOB` mal declarado no sobrevive a un `protect`/`unprotect`.

**La superficie `unsafe`, en una frase:** dos funciones externas
—`CryptProtectData` y `CryptUnprotectData`—, `LocalFree`, y una struct
`#[repr(C)] DATA_BLOB`; nada más, y solo en el crate de plataforma.

**La guarda que impide que crezca**, dos mitades porque ninguna basta sola:

1. Todo crate del workspace salvo el de plataforma debe conservar
   `#![forbid(unsafe_code)]`; una prueba lee los manifiestos y las raíces de
   crate y falla si aparece uno nuevo sin la línea, o si el de plataforma deja
   de ser el único con la excepción.
2. En el crate de plataforma, una prueba enumera los bloques `unsafe` **por
   nombre de función contenedora**. Añadir uno hace fallar la lista. Contar no
   basta: sustituir un bloque por otro mantiene el número.

---

## 2. DPAPI o CNG, y con qué parámetros

**Decisión: DPAPI** (`CryptProtectData` / `CryptUnprotectData`), no CNG DPAPI
(`NCryptProtectSecret`). CNG DPAPI existe para descriptores de protección con
varios principales —grupos, SIDs, certificados—, y aquí el principal es uno: el
usuario que ejecuta Qyro. La API más simple que resuelve el problema.

Fuente primaria: [`CryptProtectData`, Microsoft Learn][cpd], consultada
**2026-08-07** (la página declara `ms.date: 2025-11-13`).

### Parámetros congelados

| Parámetro | Valor | Razón |
|---|---|---|
| `dwFlags` | `CRYPTPROTECT_UI_FORBIDDEN` | Qyro no puede presentar UI desde el almacén |
| Ámbito | **usuario**, sin `CRYPTPROTECT_LOCAL_MACHINE` | Ver abajo |
| `pOptionalEntropy` | constante de aplicación ‖ cabecera de 16 bytes | Ver abajo |
| `pPromptStruct` | `NULL` | Ver la deprecación de febrero de 2027 |
| `szDataDescr` | `NULL` | Viaja con el blob en claro; no hay nada que quiera decir ahí |
| `pvReserved` | `NULL` | Obligatorio |

### Por qué ámbito de usuario y no de máquina

La referencia actual dice de `CRYPTPROTECT_LOCAL_MACHINE`: «associates the data
encrypted with the current computer instead of with an individual user. Any user
on the computer on which **CryptProtectData** is called can use
**CryptUnprotectData** to decrypt the data.»

La documentación archivada es más dura, y es la que decide: «Application
developers should understand that by using this flag no "real" protection is
provided by DPAPI. By "real" we mean that **any** process running on the system
can unprotect any data protected with this flag. We highly recommended that this
flag not be used on workstations to protect user's data.»

Una identidad de dispositivo privada en una estación de trabajo es exactamente
el caso que esa frase desaconseja. **No se usa.**

### Por qué sí hay entropía adicional, y qué compra exactamente

La referencia: «contains a password or other additional entropy used to encrypt
the data. The DATA_BLOB structure used in the encryption phase must also be used
in the decryption phase.»

La archivada explica qué es y qué no: «Technically, this "secret" should be
called secondary entropy. It is secondary because, while it doesn't strengthen
the key used to encrypt the data, it does increase the difficulty of one
application, running under the same user, to compromise another application's
encryption key.» Y advierte: «If it is simply saved to a file unprotected, then
adversaries could access the entropy and use it to unprotect an application's
data.»

Por eso la entropía **no se guarda junto al blob**, que sería no tener ninguna.
Se compone de dos partes, y las dos existen en ambas fases sin almacenarse:

    entropía = QYRO_IDENTITY_ENTROPY_V1 (constante compilada) ‖ cabecera[0..16]

**Qué compra:** separación de dominio. Otra aplicación que corra como el mismo
usuario y encuentre el archivo no puede desprotegerlo llamando a
`CryptUnprotectData` sin más.

**Qué no compra, y hay que decirlo:** la constante está compilada en un binario
que el usuario tiene. **No es un secreto.** Quien lea el binario la obtiene. No
añade fuerza criptográfica —la fuente lo dice: «it doesn't strengthen the key»—
y no defiende contra un atacante que ya ejecuta código como ese usuario y puede
leer el ejecutable de Qyro.

La segunda mitad sí compra algo estructural: **al meter la cabecera en la
entropía, el MAC de DPAPI pasa a cubrir nuestra cabecera** sin que este proyecto
invente ninguna capa propia. Un byte alterado en la cabecera cambia la entropía y
`CryptUnprotectData` falla. Ver §3.

### Integridad: DPAPI ya autentica

La referencia actual: «The function also adds a Message Authentication Code (MAC)
(keyed integrity check) to the encrypted data to guard against data tampering.»
La archivada concreta: «the entire BLOB is hashed with a Hashed Message
Authentication Code (HMAC), in this case SHA-1.»

**Consecuencia de diseño: Qyro no añade su propio MAC.** Sería criptografía
casera sobre una capa que ya autentica, y el sprint prohíbe inventarla. Lo que sí
se hace es meter la cabecera en la entropía para que quede bajo ese MAC.

### La deprecación con fecha

La referencia trae un aviso que ningún resumen de este sprint mencionaba: «The
prompt-based flow controlled by this parameter is deprecated and will be removed
in **February 2027**. Passing NULL or a struct with `dwPromptFlags` set to 0 will
use the non-interactive path for new operations. However, operations on data
originally protected with the PromptStruct flow will fail.»

Qyro pasa `NULL`, que es el camino no interactivo y el que sobrevive. Queda
anotado con su fecha porque es dentro de seis meses y porque un blob de Qyro
nunca se protege con el flujo de prompt, así que la retirada no puede romper
ninguno.

### Cambio de contraseña, reset y reinstalación

Lo investigado, que §13(b) del sprint pedía y no daba por sabido.

**Cambio normal de contraseña: transparente.** Archivada: «DPAPI hooks into the
password-changing module and when a user's password is changed, all MasterKeys
are re-encrypted under the new password.» Y como red de seguridad: «the system
keeps a "Credential History" file in the user's profile directory… DPAPI will use
the current password to decrypt the "Credential History" file and try the old
password to decrypt the MasterKey… This continues until the MasterKey is
successfully decrypted.»

**Reset administrativo o desde otra máquina: recuperable solo con respaldo.** En
dominio existe la copia en el controlador: «When a MasterKey is generated, DPAPI
talks to a Domain Controller… The client encrypts the MasterKey with the Domain
Controller public key.» Fuera de dominio, el Password Reset Disk. Sin ninguno de
los dos, la MasterKey no se recupera y **el blob de Qyro deja de abrirse**.

**Reinstalación del sistema: el blob se pierde.** Aquí hay que separar lo
documentado de lo inferido, porque no es lo mismo. Documentado: las MasterKeys
«are kept forever in the user's profile directory». Inferencia directa: una
reinstalación que no conserve ese perfil se lleva las MasterKeys, y sin ellas el
blob no se abre. **No se ha encontrado una página de referencia que enuncie el
caso de la reinstalación en esos términos**, así que queda como inferencia
etiquetada y como hallazgo abierto, no como hecho citado.

Para Qyro ninguno de los tres es un fallo: **el blob es caché, no archivo.** Si
no se puede abrir, la respuesta correcta es un error tipado y que el usuario
decida generar una identidad nueva —perdiendo la confianza asociada—, nunca
generar una en silencio.

### Perfil móvil: dos máquinas, una identidad

La referencia: «decryption usually can only be done on the computer where the
data was encrypted. However, a user with a roaming profile can decrypt the data
from another computer on the network.»

Es una decisión de producto, no un detalle. Con perfil móvil, «identidad de
dispositivo» pasa a ser en realidad **identidad de usuario**: dos máquinas
presentarían el mismo fingerprint y un par no podría distinguirlas.

**Decisión: el blob se guarda bajo `%LOCALAPPDATA%`, no bajo `%APPDATA%`.**
`LOCALAPPDATA` no viaja con el perfil móvil, así que el archivo se queda en la
máquina donde se creó. Esto **reduce** el problema; no lo elimina, porque la
MasterKey sí viaja y alguien que copie el archivo a mano a la otra máquina
podría abrirlo. Cerrarlo del todo exige atar el blob a un valor propio de la
máquina, que este sprint no hace y queda anotado.

[cpd]: https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata
[wdp]: https://learn.microsoft.com/en-us/previous-versions/ms995355(v=msdn.10)

> **Sobre la fuente archivada.** [Windows Data Protection][wdp] está marcada
> `is_archived: true`, fechada en 2001 y describe Windows XP. Se cita para la
> **arquitectura** —MasterKey, Credential History, respaldo en el controlador,
> naturaleza de la entropía secundaria—, que sigue vigente. **No** se cita para
> los algoritmos: habla de Triple-DES y SHA-1, y Windows moderno no usa eso.
> Presentar sus cifras como actuales sería exactamente la clase de afirmación
> que este proyecto lleva cuatro sprints quitando.

---

## 3. El formato del blob, byte a byte

Big-endian, como QYRO/1. Congelado aquí antes de existir el código.

    offset  bytes  campo         valor
    0       8      magic         "QYRO-IDS" (ASCII, sin NUL)
    8       1      version       0x01
    9       1      wrap          0x01 = DPAPI ámbito de usuario
    10      2      reserved      0x0000, debe ser cero
    12      4      wrapped_len   u32, longitud del envoltorio DPAPI
    16      N      wrapped       salida opaca de CryptProtectData

Total: `16 + N`. La cabecera son los 16 primeros bytes y es lo que entra en la
entropía junto a la constante de aplicación.

**Qué autentica qué.** DPAPI autentica `wrapped` con su propio MAC. La cabecera
queda autenticada **indirectamente**: forma parte de la entropía, así que
alterarla cambia la entropía y `CryptUnprotectData` falla. Qyro no añade MAC
propio (§2).

`reserved` debe ser cero y se rechaza si no lo es: un campo ignorado es un campo
que dos versiones interpretan distinto, que es la lección de ADR-0018.

**Orden de lectura, y es un orden con consecuencias:**

1. ¿Hay blob? Si no: `IdentityAbsent`. **No es un error de E/S ni un `None`
   ambiguo.**
2. ¿Al menos 16 bytes? Si no: `Truncated`.
3. ¿Magia correcta? Si no: `NotAnIdentityBlob`.
4. ¿`version == 1`? Si no: `UnsupportedVersion { found }` —**nombrando la
   versión encontrada**, sin intentar interpretarla—.
5. ¿`wrap` conocido? Si no: `UnsupportedWrap { found }`.
6. ¿`reserved == 0`? Si no: `ReservedNotZero`.
7. ¿`wrapped_len` coincide con los bytes restantes? Si no: `LengthMismatch`.
8. `CryptUnprotectData`. Si falla: `Unwrap { code }`.
9. ¿La semilla mide 32 bytes? Si no: `MalformedSecret`.

Los pasos 2–9 son **todos** «hay una identidad y no se puede leer». El paso 1 es
«no hay identidad». Confundirlos genera una identidad nueva en silencio, que es
el peor resultado posible de este sprint, y por eso son variantes distintas del
enum y no dos usos del mismo error.

---

## 4. El accesor de semilla — la decisión peligrosa

Sin un camino que entregue la semilla no hay persistencia. Hoy no existe:
`from_test_seed` es `cfg(any(test, fuzzing))` y `pub(crate)`.

**Qué tipo lo devuelve.** No `[u8; 32]` desnudo. Un tipo propio,
`IdentitySecret`, que:

- envuelve `Zeroizing<[u8; 32]>` y se borra al soltarse;
- **no** es `Clone` ni `Copy`, para que no haya copias que nadie audite;
- tiene `Debug` redactado, porque el camino más corto entre un secreto y un log
  es un `{:?}` en un mensaje de error;
- no es serializable.

**Quién puede llamarlo.** Aquí está el intercambio real y no tiene salida
gratis:

| Dónde vive el almacén | Accesor | Coste |
|---|---|---|
| En `qyro_crypto` | `pub(crate)`, superficie sin cambios | Mete código de plataforma y `unsafe` en el crate de criptografía |
| **En un crate aparte** | **obligatoriamente `pub`** | La semilla pasa a ser alcanzable por cualquiera que dependa de `qyro_crypto` |

**Decisión: crate aparte, y el accesor es `pub`.** La razón es §1: `unsafe` no
entra en `qyro_crypto`. Se prefiere ampliar una superficie de API —visible,
enumerable y guardable con una prueba— antes que relajar `forbid(unsafe_code)`
en el crate que guarda las claves. Una regla que se relaja una vez deja de ser
una regla; una función pública sigue siendo contable.

**No se disimula lo que cuesta:** después de este sprint, cualquier crate que
dependa de `qyro_crypto` puede pedir la semilla de una identidad que tenga en la
mano. Antes no podía. Lo que lo contiene no es la visibilidad sino que haya que
**poseer** el `DeviceIdentity`, que no es `Clone` y solo sale de `generate` o del
almacén.

Los dos caminos nuevos, y son exactamente dos:

    DeviceIdentity::export_secret(&self) -> IdentitySecret
    DeviceIdentity::from_secret(secret: &IdentitySecret) -> Self

**La guarda.** Una prueba enumera **por nombre** los caminos públicos de
`qyro_crypto` que devuelven material de clave y falla si aparece uno que no está
en la lista. Enumerar y no contar, por lo mismo que en §1: sustituir un camino
por otro deja el número igual. La lista antes de este sprint está vacía; después
tiene dos entradas.

---

## Alternativas descartadas

- **Cifrar la semilla con una clave derivada de una contraseña del usuario.**
  Qyro no pide contraseña y pedirla para arrancar sería un producto distinto.
- **Guardar la identidad en el registro en vez de un archivo.** El registro no
  es más seguro que `%LOCALAPPDATA%` —lo protege la misma ACL de usuario— y es
  más difícil de inspeccionar cuando algo falla.
- **Un MAC propio sobre el blob.** DPAPI ya autentica (§2). Añadir otro sería
  inventar criptografía sobre una capa que no lo necesita.
- **`CRYPTPROTECT_LOCAL_MACHINE`.** Descartado en §2 con la cita que lo
  desaconseja explícitamente.
- **Derivar la entropía del fingerprint de la identidad.** El fingerprint es
  público y va en el cable; usarlo como entropía es usar un valor conocido.

## No objetivos

Android Keystore, iOS Keychain, sockets, transporte, transferencia, SQLite,
emparejamiento, confianza, FFI criptográfica, rotación de claves de sesión,
release y SBOM. `rotate` en esta ADR es de identidad, no de clave de sesión.

## Lo que esta decisión no promete

- **DPAPI no protege contra código que ya corre como ese usuario.** Un atacante
  en esa posición llama a `CryptUnprotectData` con la misma constante compilada y
  obtiene la semilla. Es la limitación real de este diseño y está en
  `THREAT_MODEL.md`, no escondida aquí.
- **La constante de entropía no es un secreto.** Está en el binario.
- **Nada de esto se ha probado en hardware.** `windows-latest` es un runner con
  un perfil de usuario recién creado, sin dominio, sin perfil móvil y sin
  historial de contraseñas. Prueba que la persistencia funciona entre dos
  procesos; **no** prueba nada sobre cambio de contraseña, reset, dominio ni
  reinstalación, que son justo los casos de §2 que no se pueden ejercitar ahí.
