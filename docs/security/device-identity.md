# Identidad de dispositivo

Implementación: `rust/crates/qyro_crypto`. Decisión: ADR-0020.
Vectores: `docs/security/test-vectors/identity-v1.json`.

## Qué es

Un par de claves Ed25519 que identifica **un dispositivo**.

No es una persona. No es una cuenta. No contiene nombre, correo ni teléfono, y
nada de lo previsto lo añadirá. Tener identidad **no implica confianza**: el
emparejamiento y la decisión de confiar son pasos posteriores y explícitos que
todavía no existen. Dos dispositivos del mismo dueño tienen identidades
distintas y no vinculadas.

## Formatos congelados

| Elemento | Formato |
|---|---|
| Clave pública | 32 bytes, codificación canónica Ed25519 |
| Identidad pública en el cable | 33 bytes: byte 0 versión, bytes 1..33 clave |
| Firma | 64 bytes, `R \|\| s` |
| Fingerprint | `SHA-256("QYRO-DEVICE-IDENTITY-V1" \|\| 0x00 \|\| version \|\| public_key)` |
| Entrada de firma | `"QYRO-SIGN-V1" \|\| 0x00 \|\| domain \|\| len(msg) u64 BE \|\| msg` |

`IDENTITY_VERSION = 1` entra en el fingerprint, así que cambiar el formato
cambia todos los fingerprints y no puede pasar inadvertido.

`PublicIdentity::encode`/`decode` fijan la forma de 33 bytes
(`PUBLIC_IDENTITY_WIRE_LEN`). La versión viaja **con** la clave en lugar de
acordarse fuera de banda: entregar 32 bytes sueltos obliga al receptor a suponer
en qué formato están, y una suposición equivocada sobre el formato de una clave
es una identidad equivocada.

## Claves rechazadas

`PublicIdentity::from_bytes` y `decode` rechazan los ocho puntos de orden bajo
con `WeakPublicKey`. Los ocho son codificaciones Ed25519 **válidas**, así que no
hay nada en los bytes que los delate: `[0u8; 32]` es uno de ellos y antes se
aceptaba como una identidad perfectamente normal. Una firma bajo una de esas
claves verifica para casi cualquier mensaje, de modo que aceptarlas equivale a
aceptar una identidad que no autentica nada mientras aparenta lo contrario.

La verificación usa `verify_strict`, no el `verify` permisivo: rechaza valores
`R` no canónicos y componentes de torsión pequeña. Es defensa en profundidad —
con las claves débiles ya rechazadas en construcción, no hay entrada alcanzable
por esta API que distinga ambas funciones, y no se ha demostrado un caso de
maleabilidad concreto contra ella.

## Formas canónicas del fingerprint

Exactamente dos escrituras se aceptan al parsear: 64 caracteres hex minúsculos
sin separadores, u ocho grupos de ocho unidos por exactamente siete `-`.
Cualquier otra cosa se rechaza, incluidos guiones en otras posiciones, guiones
dobles, guion inicial o final, mayúsculas y espacios.

La implementación anterior eliminaba todos los `-` antes de mirar la entrada, de
modo que `-9fd69388…`, `9f-d6-93-88…` y un guion final nombraban la misma
identidad. Un fingerprint que la gente lee en voz alta para decidir si confía en
un dispositivo debe tener una escritura, no una familia: si dos cadenas distintas
nombran la misma identidad, comparar cadenas deja de ser una forma sólida de
comparar identidades.

## Separación de dominios

| ID | Dominio | Disponible |
|---|---|---|
| 1 | `TestVector` | sí |
| 2 | `DeviceClaim` | sí |
| 3 | `HandshakeTranscript` | **no**, reservado |

Una firma hecha en un dominio no verifica en otro. El separador `0x00` y la
longitud explícita del mensaje impiden que dos pares (dominio, mensaje)
distintos produzcan los mismos bytes firmados; sin ellos, un mensaje elegido por
un atacante podría reproducir el prefijo de otro dominio y una firma podría
reutilizarse en un contexto para el que nunca se emitió.

`HandshakeTranscript` se rechaza hasta que el handshake congele su formato de
transcript. Permitir firmas ahí ahora sería comprometerse con un formato que
nada ha validado.

## Manejo de secretos

- `DeviceIdentity` no es `Clone`, `Copy` ni serializable.
- No existe API que devuelva la semilla ni la clave privada.
- La semilla se envuelve en `Zeroizing` durante la generación; la clave de firma
  se zeroiza al liberarse.
- `Debug` imprime el fingerprint público y `<redacted>`. Una prueba comprueba que
  ninguna ventana de 16 caracteres hex de la semilla aparece en la salida.
- La entropía de producción viene de `getrandom` y de ningún otro sitio. Si el
  sistema no puede darla, la generación falla con `EntropyUnavailable`; no hay
  fallback más débil.
- El constructor de semilla fija es `cfg(test)` y privado del crate: no existe
  fuera del build de pruebas. Vivía tras la feature `test-vectors`, que seguía
  siendo API pública: las features son aditivas, así que cualquier crate del
  grafo podía activarla para todos los demás y ningún build de release podía
  demostrar que estaba apagada.
- Firmar es solo falible (`try_sign`). Había también un `sign` infalible que
  hacía `expect` sobre el anterior; convertir un dominio disponible en reservado
  habría transformado en pánico cada llamada, en silencio.

## Fallo de verificación

`verify` devuelve una sola variante para cualquier fallo criptográfico. Es
deliberado: distinguir «clave equivocada» de «mensaje alterado» le diría a un
atacante qué mitad seguir cambiando.

## Handshake

Desde el sprint 4B existe un handshake autenticado de cuatro mensajes, **solo en
memoria**: `rust/crates/qyro_crypto/src/handshake`, especificado por ADR-0021.
X25519 para el acuerdo de clave, Ed25519 en el dominio `HandshakeTranscript`
para autenticar, HKDF-SHA256 para derivar y HMAC-SHA256 para confirmar.

Al terminar, cada lado tiene el mismo identificador de sesión, un par de claves
direccionales y la `PublicIdentity` del otro. Esa identidad es **la que firmó**,
atada al transcript, no la que se esperaba. Tenerla **no es confiar en ella**.

## Todavía no existe

AEAD, replay protection, rotación, revocación, dispositivos de confianza,
almacenamiento seguro (Keystore, Keychain, DPAPI/CNG), FFI criptográfico e
interfaz. Tampoco hay transporte: el handshake no corre sobre ningún socket, y
las claves que deriva no cifran nada. La identidad vive **solo en memoria**:
generar una y perderla al cerrar el proceso es el comportamiento actual.
