# Auditoría del ciclo de vida de cada secreto

Ámbito: todo valor secreto o derivado de un secreto que existe en `qyro_crypto`,
desde que aparece hasta que se libera. Sprint 4C.1.

Este documento **no afirma garantías que no se hayan comprobado leyendo la
dependencia**. Donde una biblioteca no promete borrar algo, aquí lo dice y
explica qué se hizo en su lugar.

## Método

Para cada valor: dueño, duración, si es secreto, cómo se borra, cuántas copias
existen, si puede salir de la biblioteca y qué hace su `Drop`.

Las garantías de terceros se verificaron abriendo el código de la versión fijada
en `Cargo.lock`, no la documentación de la última versión ni el nombre de la
feature. Dos de ellas resultaron estar apagadas.

## Lo que se corrigió al escribir esto

**`sha2` y `hmac` no borraban su estado.** Ambas exponen una feature `zeroize`
que reenvía a `digest/zeroize`, y ninguna de las dos estaba activada. El estado
de compresión de SHA-256 detrás de cada transcript, y el estado con clave de
HMAC detrás de cada MAC de confirmación y de cada expansión HKDF, quedaba en
memoria liberada. Ahora están activadas.

**`hkdf` no tiene feature de zeroize y no la necesita.** `Hkdf<Sha256>` guarda un
`Hmac<Sha256>` —comprobado leyendo `GenericHkdf` en hkdf 0.13, no deducido del
nombre—, así que el PRK, que es un secreto de tráfico, lo borra el `Drop` de ese
valor en cuanto `hmac/zeroize` está activo.

## Inventario

### Identidad

| Valor | Dueño | Duración | Secreto | Borrado | Copias | Salida pública |
|---|---|---|---|---|---|---|
| `SigningKey` Ed25519 | `DeviceIdentity` | vida del proceso | sí | `ZeroizeOnDrop` de ed25519-dalek, feature `zeroize` activa | ninguna: no es `Clone` ni serializable | ninguna; no hay accesor a la semilla ni a la clave privada |
| semilla de 32 bytes | temporal en `generate` | una llamada | sí | `Zeroizing` | una | ninguna |
| `PublicIdentity` | `DeviceIdentity` | vida del proceso | **no** | n/a | libre | sí, es lo que va al cable |

### Handshake

| Valor | Dueño | Duración | Secreto | Borrado | Copias | Salida pública |
|---|---|---|---|---|---|---|
| entropía de 64 bytes | temporal en `send_hello` | una llamada | sí | `Zeroizing`, y `from_secret_bytes` borra su copia | una | ninguna |
| `EphemeralKeyPair` (`StaticSecret`) | el estado del handshake | hasta el intercambio | sí | `Drop` de x25519-dalek con `zeroize` activo | ninguna: el envoltorio no es `Clone` y `diffie_hellman` consume `self` | ninguna |
| secreto compartido X25519 | `ResponderAwaitInitiatorFinish` | hasta derivar | sí | `Zeroizing<[u8;32]>`; el `SharedSecret` de la biblioteca también borra el suyo | dos: la de la biblioteca y la copia envuelta | ninguna |
| `Hkdf<Sha256>` (PRK = secreto compartido) | temporal en `Schedule::derive` | una llamada | sí | `Drop` del `Hmac` interno, ahora con `hmac/zeroize` | una | ninguna |
| claves `*_finished` | `Schedule` | hasta verificar los MAC | sí | `Zeroizing<[u8;32]>` | una | ninguna |
| `SessionKey` (secretos de tráfico) | `Session`, luego `PendingSessionSecrets` | hasta `into_frame_crypto` | sí | `Drop` propio que hace `zeroize` | una; el tipo no es `Clone` | ninguna: `SessionKey` no se exporta y no hay accesor público |
| `auth_transcript` | `Session` | hasta derivar el AEAD | **no** | n/a | varias | ninguna hoy, aunque no sería un fallo |
| `SessionId` | `Session` y la cabecera | vida de la sesión | **no** | n/a | libre | sí, va en cada frame |

### AEAD de frames

| Valor | Dueño | Duración | Secreto | Borrado | Copias | Salida pública |
|---|---|---|---|---|---|---|
| clave AEAD de 32 bytes | `DirectionalKeys` | vida del sealer/opener | sí | `Zeroizing<[u8;32]>` | una | ninguna |
| prefijo de nonce | `DirectionalKeys` | igual | **no**, pero se borra igual | `Drop` explícito | una | solo bajo `cfg(test)` |
| `ChaCha20Poly1305` | temporal por frame | una operación | sí, contiene la clave | `Drop` de la biblioteca con `zeroize` activo, más `ZeroizeOnDrop` | una por operación | ninguna |
| clave one-time de Poly1305 | interna de chacha20poly1305 | una operación | sí | `mac_key.zeroize()` dentro de la biblioteca | una | ninguna |
| búfer de `seal` | temporal | una llamada | **sí mientras es texto claro** | `Zeroizing<Vec<u8>>` | una; el ciphertext se copia fuera de forma explícita | no |
| búfer de `open` | temporal | una llamada | **sí en cuanto el tag verifica** | `Zeroizing<Vec<u8>>` | una, que se mueve al frame | no |
| `AuthenticatedFrame::payload` | quien abrió el frame | hasta que lo suelta | sí | `Zeroizing<Vec<u8>>` | una; el tipo no es `Clone` | sí, como `Zeroizing<Vec<u8>>` |
| contador de secuencia | `FrameSealer` | vida del sealer | **no** | n/a | una | no |
| ventana de replay | `FrameOpener` | vida del opener | **no** | n/a | una | solo bajo `--cfg fuzzing` |

## Límites que no se pueden cerrar aquí

Se enumeran porque un documento que solo lista lo que sí se hace es un documento
que engaña por omisión.

**Reasignación de un `Vec`.** La documentación de `zeroize` lo dice de su propia
implementación para `Vec`: borra los elementos inicializados y la capacidad
sobrante, y «no puede garantizar que reasignaciones anteriores no hayan dejado
valores en el heap». Un `Vec` que crece copia sus bytes a otra dirección y libera
la anterior sin tocarla. Los búferes de `seal` y `open` se construyen con
`to_vec()` a su tamaño final y nunca crecen, así que hoy no hay reasignación; eso
es una propiedad del código actual, no de `Zeroizing`.

**El sistema operativo puede haber copiado la página.** Swap, hibernación,
`fork`, un core dump o el compactador de un GC ajeno están fuera del alcance de
cualquier `Drop`. Nada en este repositorio bloquea páginas en memoria
(`mlock`/`VirtualLock`), y decir que borra el secreto «de la máquina» sería
falso.

**El compilador puede eliminar una escritura.** `zeroize` usa escrituras
volátiles y una barrera de compilador precisamente para impedirlo; esa es la
razón de usar el crate en lugar de asignar ceros a mano. La garantía es la que
`zeroize` da, no una que este proyecto pueda comprobar por su cuenta.

**Registros y pilas.** Un valor de 32 bytes vive en registros durante una parte
de su uso, y nada los borra. Es un límite de todas las bibliotecas de este tipo.

**No hay medición.** Ninguna de estas garantías se ha observado ocurriendo. Leer
memoria liberada es comportamiento indefinido y el asignador puede haber
reutilizado o desmapeado la página, así que una prueba que afirmara verlo estaría
mintiendo. Lo que las pruebas comprueban es el **tipo**, que es donde vive la
garantía.

## Lo que se decidió no hacer

- **No hay `mlock`.** Requiere permisos que una app móvil no tiene siempre, falla
  distinto en cada plataforma, y su ausencia es más honesta que un intento que
  falla en silencio.
- **No se borra el `auth_transcript`.** No es secreto: los dos pares lo calculan
  a partir de mensajes que cruzaron el cable.
- **No se borra el `SessionId`.** Va en claro en cada cabecera.
- **No se envuelve el prefijo de nonce en `Zeroizing`.** Ya se borra en `Drop`, y
  un nonce no es secreto; se borra porque es barato y porque un prefijo suelto
  junto a una clave invita a confusión sobre cuál es cuál.
