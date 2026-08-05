# ADR-0022: cifrado autenticado de frames QYRO/1

Estado: aceptada. Depende de ADR-0016 (framing), ADR-0018 (errores) y ADR-0021
(handshake). Congela el formato **antes** de que exista el código, por el mismo
motivo que ADR-0021: un formato descrito después documenta lo que el código
hace, no lo que el protocolo exige.

## Contexto

El handshake deriva dos secretos de tráfico direccionales y un `SessionId` de
ocho bytes, y nada los usa. `EncryptedEnvelope` define desde el sprint 2 la
forma de un frame cifrado y expone la cabecera completa como datos asociados,
pero ningún AEAD calcula ese tag: su documentación dice explícitamente que los
bytes que llama «tag» no los verifica nadie.

Esta decisión cierra ese hueco. Sigue sin haber transporte.

## Suite

| Elemento | Valor |
|---|---|
| Algoritmo | ChaCha20-Poly1305, RFC 8439 |
| `AEAD_KEY_LEN` | 32 bytes |
| `NONCE_LEN` | 12 bytes |
| `TAG_LEN` | 16 bytes |
| `NONCE_PREFIX_LEN` | 4 bytes |
| `REPLAY_WINDOW` | 1024 |

**No XChaCha20-Poly1305.** Su nonce de 24 bytes existe para poder elegirlo al
azar sin miedo a colisiones. Aquí el nonce es un contador, así que la ventaja
desaparece y lo único que quedaría es un nonce que no cabe en el diseño.

**No nonce aleatorio por frame.** Un nonce aleatorio de 96 bits colisiona con
probabilidad apreciable mucho antes de agotar un contador de 64 bits, y una
colisión de nonce en un cifrador de flujo revela el XOR de los dos textos claros.

## Derivación

Los secretos de tráfico del handshake **no** son claves AEAD. Cada dirección
deriva las suyas con HKDF-SHA256, usando el secreto de tráfico como PRK —ya es
uniforme, porque sale del `HKDF-Expand` del handshake— así que solo hace falta la
fase de expansión.

```
info(label) = label || 0x00 || auth_transcript (32) || session_id (8)

aead_key     = HKDF-Expand( PRK = traffic_secret, info(<dir>/key),          32 )
nonce_prefix = HKDF-Expand( PRK = traffic_secret, info(<dir>/nonce-prefix),  4 )
```

Etiquetas, literales y completas:

```
QYRO-AEAD-V1/i2r/key
QYRO-AEAD-V1/i2r/nonce-prefix
QYRO-AEAD-V1/r2i/key
QYRO-AEAD-V1/r2i/nonce-prefix
```

La dirección va **dentro de la etiqueta**, así que las dos direcciones no pueden
producir la misma clave aunque partieran del mismo secreto. El
`auth_transcript` y el `SessionId` entran en cada `info`: dos sesiones distintas
producen claves distintas aunque un fallo futuro repitiera un secreto de tráfico.

`i2r` es lo que escribe el iniciador y lee el respondedor. `r2i`, al revés.

## Nonce

```
nonce = nonce_prefix (4 bytes) || sequence (u64, big-endian) = 12 bytes
```

Reglas, todas verificadas por tests:

- `sequence` empieza en 0.
- **Solo el sealer asigna `sequence`.** El llamante no elige ni el nonce ni la
  secuencia; si los pone en el frame que entrega, se sobrescriben.
- `sequence` **nunca** da la vuelta. Agotarla es un error terminal
  (`SequenceExhausted`), no un envolvimiento: repetir un nonce en un cifrador de
  flujo significa que dos textos claros se revelan mutuamente por XOR.
- Si `seal` devuelve un frame, la secuencia queda consumida. Descartar el frame
  no la libera.
- Un nonce nunca se repite dentro de una sesión y dirección.

Las dos direcciones comparten el espacio de secuencias —ambas empiezan en 0—
pero no el prefijo ni la clave, así que no comparten nonces.

## Datos asociados

El AAD es **exactamente la cabecera QYRO/1 de 48 bytes**, ya construida, con:

- magic, versión mayor y menor;
- `message_type`;
- flags, incluido `ENCRYPTED`;
- `header_len` = 48, `trailer_len` = 16;
- `payload_len` = longitud del ciphertext, que es la del plaintext porque
  ChaCha20 es un cifrador de flujo;
- `session_id`, `transfer_id`, `stream_id`, `item_id`;
- `sequence`.

Esto solo es correcto porque la reserialización es byte-exacta (ADR-0018): si
un byte de cabecera no se conservara, el tag se calcularía sobre algo distinto
de lo que viaja.

Alterar **cualquiera** de los 48 bytes invalida el tag.

### Quién decide qué

| Campo | Lo elige |
|---|---|
| `message_type` | el llamante |
| `END_OF_ITEM`, `END_OF_TRANSFER` | el llamante |
| `transfer_id`, `stream_id`, `item_id` | el llamante |
| plaintext | el llamante |
| `session_id` | **el sealer** |
| `sequence` | **el sealer** |
| nonce | **el sealer** |
| `ENCRYPTED`, `trailer_len`, tag | **el sealer** |

## Ventana de replay

Ventana fija de 1024, con el mayor visto y un bitmap de 1024 bits
(`[u64; 16]`).

Orden obligatorio al abrir:

1. validar el framing;
2. validar la longitud del tag;
3. validar `ENCRYPTED`;
4. validar el `SessionId`;
5. **precomprobar** el replay sin modificar el estado;
6. verificar y descifrar con el AEAD;
7. **solo si el AEAD pasa**, actualizar la ventana.

El orden es la decisión, no un detalle. Si la ventana se actualizara antes de
verificar, cualquiera sin la clave podría enviar `sequence = u64::MAX - 1` con
un tag basura y dejar la sesión inservible. Un tag inválido **no** consume
secuencia; un `SessionId` ajeno **no** toca la ventana.

Se acepta el desorden dentro de la ventana: las redes reordenan. Se rechaza lo
que caiga por detrás de ella, porque ya no se puede distinguir de un replay.

## Tipos

| Tipo | Garantía |
|---|---|
| `FrameSealer` | posee clave saliente, prefijo y contador. Sin `Clone`, `Debug` redactado, zeroize |
| `FrameOpener` | posee clave entrante, prefijo y ventana. Sin `Clone`, `Debug` redactado, zeroize |
| `SealedFrame` | **solo** lo produce `FrameSealer::seal`. Constructor privado. No expone plaintext |
| `AuthenticatedFrame` | **solo** lo produce `FrameOpener::open`. Constructor privado. Su plaintext está verificado |
| `EncryptedEnvelope` | sigue significando lo mismo: **bytes no confiables del cable** |

Esa última fila es la razón de que existan las otras cuatro. Un `SealedFrame` y
un `AuthenticatedFrame` afirman algo; un `EncryptedEnvelope` no afirma nada, y
su documentación lo dice desde que se llamaba `SealedFrame` y no debía.

## Errores

`WrongSession`, `ReplayDetected`, `SequenceTooOld`, `AuthenticationFailed`,
`InvalidTagLength`, `SequenceExhausted`, `InvalidNonceState`,
`NotEncrypted`, `PayloadTooLarge`, `KeyDerivationFailed`.

`AuthenticationFailed` es una sola variante a propósito: distinguir «tag
incorrecto» de «cabecera alterada» le diría a un atacante qué mitad seguir
cambiando.

## Ciclo de vida

Un `EstablishedInitiator` o `EstablishedResponder` se consume con
`into_frame_crypto()` y produce `(FrameSealer, FrameOpener)`. La sesión
establecida deja de existir: no hay forma de derivar dos sealers de la misma
dirección y arrancar dos contadores desde cero.

## Alternativas descartadas

- **AES-GCM.** Aceptable, pero exige aceleración por hardware para ser rápido y
  seguro frente a canales laterales; ChaCha20-Poly1305 es constante en software
  en cualquier CPU, y Qyro corre en teléfonos de gama baja.
- **XChaCha20-Poly1305.** Descrito arriba: su ventaja es para nonces aleatorios.
- **Nonce derivado del contenido (SIV).** Resiste la reutilización de nonce,
  pero cuesta dos pasadas y no hace falta cuando el nonce es un contador que no
  da la vuelta.
- **Usar el secreto de tráfico como clave AEAD directamente.** Ahorra una
  expansión y pierde la separación entre «secreto de la sesión» y «clave de este
  cifrador», que es lo que permite añadir otro consumidor sin compartir clave.
- **Ventana deslizante sin bitmap.** Solo acepta orden estricto; las redes
  reordenan y una transferencia legítima se rompería.

## No objetivos

Sockets, TCP, TLS, LAN, mDNS, transferencia de archivos, SQLite, persistencia de
identidad, almacenes seguros, FFI criptográfico, interfaz y modo óptico. Nada de
esto existe al cerrar esta decisión, y cifrar frames en memoria no acerca ninguno
por sí solo.

Tampoco hay rotación de claves ni rekey: una sesión usa una clave por dirección
hasta agotar la secuencia.

## Vectores

- `docs/security/test-vectors/rfc8439-chacha20poly1305.json`: los vectores del
  propio RFC 8439.
- `docs/security/test-vectors/aead-v1.json` y su schema estricto: sellado
  completo de Qyro, encadenado al vector de handshake ya comprometido.
