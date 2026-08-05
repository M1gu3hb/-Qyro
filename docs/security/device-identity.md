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
| Firma | 64 bytes, `R \|\| s` |
| Fingerprint | `SHA-256("QYRO-DEVICE-IDENTITY-V1" \|\| 0x00 \|\| version \|\| public_key)` |
| Entrada de firma | `"QYRO-SIGN-V1" \|\| 0x00 \|\| domain \|\| len(msg) u64 BE \|\| msg` |

`IDENTITY_VERSION = 1` entra en el fingerprint, así que cambiar el formato
cambia todos los fingerprints y no puede pasar inadvertido.

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
- El constructor de semilla fija vive tras la feature `test-vectors`, que no está
  en `default`, y los targets que la usan la declaran en `required-features`.

## Fallo de verificación

`verify` devuelve una sola variante para cualquier fallo criptográfico. Es
deliberado: distinguir «clave equivocada» de «mensaje alterado» le diría a un
atacante qué mitad seguir cambiando.

## Todavía no existe

Handshake, X25519, HKDF, AEAD, replay protection, rotación, revocación,
dispositivos de confianza, almacenamiento seguro (Keystore, Keychain,
DPAPI/CNG), FFI criptográfico e interfaz. La identidad vive **solo en memoria**:
generar una y perderla al cerrar el proceso es el comportamiento actual.
