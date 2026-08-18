# ADR-0020: Fundación de identidad de dispositivo

- Estado: aceptada
- Fecha: 2026-08-05
- Implementa: `rust/crates/qyro_crypto`
- Alcance: **solo identidad**. Handshake, X25519, HKDF y AEAD llegan después.

## Qué es y qué no es una identidad de dispositivo

Es un par de claves Ed25519 que identifica **un dispositivo**, no a una persona.

No contiene ni contendrá nombre, correo, teléfono ni ningún dato personal. No es
una cuenta. Tener una identidad no implica confianza: el emparejamiento y la
decisión de confiar son pasos posteriores y explícitos. Dos dispositivos del
mismo dueño tienen identidades distintas y no relacionadas.

## Decisiones

### Algoritmo

**Ed25519** (RFC 8032) mediante `ed25519-dalek`. Firmas de 64 bytes, claves
públicas de 32, determinista, sin dependencia de un RNG en el momento de firmar,
y con vectores oficiales contra los que verificar.

No se implementa criptografía propia. No se usa OpenSSL.

### Formatos

- Clave pública: 32 bytes, la codificación canónica de Ed25519.
- Firma: 64 bytes, `R || s`.
- Clave privada: la semilla de 32 bytes, en memoria, envuelta en un tipo que se
  zeroiza. **No existe API pública para exportarla.**

### Generación

CSPRNG del sistema vía `getrandom`. Los errores se propagan como
`IdentityError::EntropyUnavailable`; no hay fallback más débil. Una semilla fija
solo existe bajo `cfg(test)` y es privada del crate (véase la enmienda del
sprint 4B al final).

### Separación de dominios

Nunca se firma un mensaje desnudo. Lo que se firma es:

```
"QYRO-SIGN-V1" || 0x00 || domain_id (u8) || len(message) (u64 BE) || message
```

El separador `0x00` y la longitud explícita impiden que dos pares
(dominio, mensaje) distintos produzcan los mismos bytes firmados. Sin ellos, un
mensaje elegido por un atacante podría reproducir el prefijo de otro dominio.

Dominios de esta versión:

| ID | Dominio | Uso |
|---|---|---|
| 1 | `TestVector` | solo pruebas y vectores |
| 2 | `DeviceClaim` | afirmación sobre el propio dispositivo |
| 3 | `HandshakeTranscript` | **reservado**, rechazado hasta el siguiente sprint |

Una firma válida en un dominio no verifica en otro; hay un test que lo fija.

### Fingerprint

```
SHA-256( "QYRO-DEVICE-IDENTITY-V1" || 0x00 || version (u8) || public_key (32) )
```

32 bytes completos, nunca truncados. La representación textual es hex en
minúsculas con guiones cada 8 caracteres, para que una persona pueda leerla en
voz alta al comparar dos dispositivos.

Una representación corta para la interfaz podrá añadirse más adelante como
*display*, pero no sustituirá al valor canónico en ninguna comparación.

### Versionado

`IDENTITY_VERSION = 1` entra tanto en el fingerprint como en la codificación
canónica de la clave pública, así que cambiar el formato cambia el fingerprint y
no puede pasar inadvertido.

### Manejo de secretos

- El tipo secreto **no** implementa `Clone`, `Copy`, `Serialize` ni `Deref`.
- Su `Debug` imprime un marcador fijo, nunca bytes.
- Se zeroiza al liberarse.
- No hay getter de la semilla ni de la clave privada.

## Consecuencias

- 27 paquetes nuevos en `Cargo.lock`, todos con licencia permisiva
  (BSD-3-Clause, MIT o Apache-2.0). Registrados en `docs/LICENSE_AUDIT.md`.
- El workspace deja de tener cero dependencias externas. Es un cambio
  deliberado: implementar Ed25519 a mano sería mucho peor que auditar
  `curve25519-dalek`.
- Los vectores en `docs/security/test-vectors/identity-v1.json` fijan el formato
  para futuras implementaciones en Swift, Kotlin o Dart.

## Enmienda (sprint 4B)

Cuatro decisiones de este ADR se endurecieron sin cambiar ningún formato
congelado. El wire de la clave pública se fija además explícitamente.

1. **El constructor determinista deja de ser público.** Estaba tras la feature
   `test-vectors`, fuera de `default`. Las features son aditivas: cualquier
   crate del grafo puede activarla para todos los demás, así que un build de
   release no podía demostrar que estaba apagada. Ahora es `cfg(test)` y
   privado del crate, y no existe fuera del build de pruebas. Los vectores se
   verifican dentro del crate, no desde `tests/`, porque un test de integración
   es otro crate y solo podría alcanzarlo mediante API pública.
2. **Firmar es solo falible.** Se elimina el `sign` infalible que hacía `expect`
   sobre `try_sign`. Su justificación era que quien pasa un dominio literal sabe
   que está disponible; eso no sobrevive a una versión posterior que reserve un
   dominio antes disponible, porque convertiría cada llamada en un pánico.
3. **Las claves de orden bajo se rechazan** con `WeakPublicKey`, y la
   verificación usa `verify_strict`.
4. **El fingerprint tiene exactamente dos escrituras canónicas.** El parser
   anterior eliminaba todos los `-` antes de mirar, dando a cada fingerprint una
   familia de escrituras equivalentes.
5. **`PUBLIC_IDENTITY_WIRE_LEN = 33`**: byte 0 la versión, bytes 1..33 la clave.
   La versión viaja con la clave en lugar de acordarse fuera de banda.

Los formatos de fingerprint y de entrada de firma **no** cambian, así que
`identity-v1.json` sigue siendo válido byte a byte. Se añade
`docs/security/test-vectors/rfc8032-ed25519.json` con las cinco pruebas de la
sección 7.1 del RFC 8032, tomadas del texto del RFC.
