# Formato del blob de identidad

Especificación: `docs/adr/ADR-0024-secure-identity-storage.md`, donde está
congelado y donde vive el razonamiento. Este documento es la referencia byte a
byte. Donde los dos discrepen manda la ADR.

**Estado: formato congelado, implementación no escrita todavía.** Este archivo
describe lo que la ADR fija; cuando exista el código, los vectores de
`storage-v1.json` serán lo que decida si coinciden.

## La forma

Big-endian, como QYRO/1.

    offset  bytes  campo         valor
    0       8      magic         "QYRO-IDS" (ASCII, sin NUL)
    8       1      version       0x01
    9       1      wrap          0x01 = DPAPI ámbito de usuario
    10      2      reserved      0x0000, debe ser cero
    12      4      wrapped_len   u32, longitud de `wrapped`
    16      N      wrapped       salida opaca de CryptProtectData

Longitud total: `16 + N`. `N` lo elige DPAPI y no es constante: el blob que
devuelve incluye su propia cabecera, el GUID de la MasterKey, sal y un MAC. No se
parsea nunca. La [documentación][wdp] es explícita: «Being opaque, application
developers do not need to parse or understand the format at all.»

Los 16 primeros bytes son **la cabecera**, y aparecen dos veces: en el archivo y
dentro de la entropía adicional que se pasa a DPAPI.

## Qué autentica qué

DPAPI autentica `wrapped` por su cuenta: «The function also adds a Message
Authentication Code (MAC) (keyed integrity check) to the encrypted data to guard
against data tampering» ([CryptProtectData][cpd], consultada 2026-08-07).

La cabecera queda autenticada **indirectamente**, y esa indirección es el diseño:

    entropía = QYRO_IDENTITY_ENTROPY_V1 ‖ cabecera[0..16]

Como la entropía tiene que ser idéntica al proteger y al desproteger, alterar un
byte de la cabecera cambia la entropía y `CryptUnprotectData` falla. Así la
cabecera cae bajo el MAC de DPAPI **sin que Qyro añada un MAC propio**, que sería
criptografía casera sobre una capa que ya autentica.

Consecuencia práctica: voltear un bit en **cualquier** posición del archivo
—cabecera o envoltorio— produce un error tipado. Las dos mitades llegan ahí por
caminos distintos, y por eso la prueba recorre todas las posiciones en vez de
comprobar una de cada.

## La constante de entropía no es un secreto

`QYRO_IDENTITY_ENTROPY_V1` está compilada en un binario que el usuario tiene.
Quien lo lea la obtiene.

Lo que compra es separación de dominio: otra aplicación que corra como el mismo
usuario y encuentre el archivo no lo abre llamando a `CryptUnprotectData` a
secas. Lo que **no** compra es fuerza criptográfica —la [documentación
archivada][wdp] lo dice de la entropía secundaria: «it doesn't strengthen the key
used to encrypt the data»— ni defensa contra quien ya ejecuta código como ese
usuario.

Guardarla junto al blob sería no tener ninguna, y la misma fuente lo advierte:
«If it is simply saved to a file unprotected, then adversaries could access the
entropy and use it to unprotect an application's data.»

## Dónde vive el archivo

    %LOCALAPPDATA%\Qyro\identity.bin

`LOCALAPPDATA` y no `APPDATA`, deliberadamente. Un perfil móvil puede
desproteger datos DPAPI desde otra máquina —«a user with a roaming profile can
decrypt the data from another computer on the network»—, y si el archivo viajara
con el perfil, dos máquinas presentarían la misma identidad de dispositivo.
`LOCALAPPDATA` no viaja.

Esto **reduce** el problema y no lo cierra: la MasterKey sí viaja, así que quien
copie el archivo a mano a la otra máquina puede abrirlo. Cerrarlo exige atar el
blob a un valor propio de la máquina, que el sprint 4D.1 no hace.

## Orden de lectura

Es un orden, no una lista, y cada paso decide una variante de error distinta:

1. ¿Existe el archivo? Si no: **`IdentityAbsent`**.
2. ¿Al menos 16 bytes? Si no: `Truncated`.
3. ¿`magic == "QYRO-IDS"`? Si no: `NotAnIdentityBlob`.
4. ¿`version == 1`? Si no: `UnsupportedVersion { found }`.
5. ¿`wrap` conocido? Si no: `UnsupportedWrap { found }`.
6. ¿`reserved == 0`? Si no: `ReservedNotZero`.
7. ¿`wrapped_len` == bytes restantes? Si no: `LengthMismatch`.
8. `CryptUnprotectData`. Si falla: `Unwrap { code }`.
9. ¿La semilla mide 32 bytes? Si no: `MalformedSecret`.

**El paso 1 y los pasos 2–9 son cosas distintas.** El paso 1 es «no hay
identidad»; los demás son «hay una y no se puede leer». Confundirlos lleva a
generar una identidad nueva en silencio cuando en realidad había una ilegible, y
perder en silencio la identidad de un dispositivo es el peor resultado que este
formato puede producir. Por eso son variantes separadas del enum y no dos usos
del mismo error.

La versión futura se rechaza **nombrando la versión encontrada** y sin intentar
interpretar nada. Un formato que adivina qué quiso decir una versión que no
conoce es un formato con dos lecturas.

`reserved` se rechaza si no es cero, por lo mismo que ADR-0018: un campo que se
ignora es un campo que dos versiones leen distinto.

## Lo que este formato no promete

- **No protege contra código que ya corre como ese usuario.** Ese atacante llama
  a `CryptUnprotectData` con la misma constante y obtiene la semilla. Es la
  limitación real y está también en `THREAT_MODEL.md`.
- **No sobrevive a perder la MasterKey.** Un reset administrativo de contraseña
  sin respaldo de dominio, o una reinstalación que no conserve el perfil, dejan
  el blob ilegible. La respuesta correcta es un error tipado y que el usuario
  decida, no una identidad nueva en silencio: el blob es caché, no archivo.
- **No está probado en hardware.** Cuando exista la implementación, correrá en
  `windows-latest`, que es un runner con un perfil recién creado, sin dominio,
  sin perfil móvil y sin historial de contraseñas.

[cpd]: https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata
[wdp]: https://learn.microsoft.com/en-us/previous-versions/ms995355(v=msdn.10)
