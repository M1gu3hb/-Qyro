# Índice de decisiones

- ADR-0001: Flutter + Rust para multiplataforma.
- ADR-0002: Rust como núcleo compartido.
- ADR-0003: protocolo QYRO/1 versionado.
- ADR-0004: TLS 1.3 más cifrado de contenido.
- ADR-0005: RaptorQ para modo óptico, pendiente de benchmark.
- ADR-0006: SQLite local desde Rust.
- ADR-0007: política sin nube.
- ADR-0008: launch estático y boot Flutter.
- ADR-0009: Bluetooth limitado a control/experimental.
- ADR-0010: paquetes por plataforma y releases reproducibles.
- ADR-0012: branding generado en tiempo de build.
- ADR-0013: StartupCoordinator y tareas obligatorias de arranque.
- ADR-0014: ruta canónica del logo de Qyro.
- ADR-0015: reconciliación de ramas divergentes.
- ADR-0016: framing binario de QYRO/1.
- ADR-0017: codificación canónica del manifest.
- ADR-0018: errores estructurales frente a eventos semánticos.
- ADR-0019: nombre visible derivado de la ruta.
- ADR-0020: fundación de identidad de dispositivo.
- ADR-0021: handshake autenticado de cuatro mensajes.
- ADR-0022: cifrado autenticado de frames QYRO/1.
- ADR-0023: harness aislado de pruebas criptográficas por plataforma.
- ADR-0024: persistencia segura de `DeviceIdentity`, formato del blob y Windows
  DPAPI. Congela dos decisiones que cuestan algo y lo dicen: `unsafe` vive en un
  crate de plataforma aparte para no relajar `forbid(unsafe_code)` en el crate
  que guarda las claves, y a cambio el accesor de semilla tiene que ser público.
  Se prefiere una superficie de API contable a una regla relajada.

El sprint 4B.1 cerró el handshake sin cambiar ninguna decisión: unificó el
`SessionId` en ocho bytes, añadió `ResponderFinishPending`, sacó las claves de
la API pública y comprometió vectores. Está registrado como enmienda dentro de
ADR-0021.

El sprint 4C.1 no cambió ningún formato. Añadió ADR-0023 —evidencia real de que
`qyro_crypto` funciona en Android, iOS y Windows, que hasta entonces solo se
había compilado y ejecutado en x86_64 Linux— y enmendó ADR-0016, que llevaba
cuatro sprints afirmando dos reglas que ADR-0018 y ADR-0022 ya habían revertido.

No existe ADR-0011.

Consulta docs/adr/.

El sprint 4D.2a añade ADR-0025: persistencia de identidad en Android. No cambia
`IdentityStore` ni `SecretWrapper` —que es la prueba de que la costura de
ADR-0024 estaba bien puesta— y añade un solo valor al byte `wrap`, `0x02`, que
es un cambio de formato y está registrado como tal.

Su decisión estructural es la contraria a la de ADR-0024 §1 y por la misma
razón. Allí se escribieron dos declaraciones de función a mano para no traer
once crates; aquí JNI no se alcanza por símbolos con nombre sino por una tabla
de unos 233 punteros en orden fijo, donde un índice equivocado llama a otra
función en silencio. Se trae `jni-sys`, dos entradas nuevas en el grafo, y con
ello **termina la racha de cero dependencias externas** que 4D.1 mantuvo.

ADR-0025 §1.2 registra QYR-0064: el harness de binario empujado por `adb` que
4D.1 usó en Windows **no puede alcanzar Android Keystore**, porque no hay API
nativa y el proceso del shell no tiene ni runtime de framework ni UID de
aplicación.

El sprint 5A añade ADR-0026: `TransferSession`, el primer sprint que conecta el
framing, el manifest, el handshake y el AEAD entre sí. No añade criptografía: fija
quién habla cuándo, con qué cuerpo, y qué ocurre cuando no.

Tres decisiones que conviene no volver a discutir sin datos nuevos: ACK
**acumulativo** y no selectivo, porque dos conjuntos que pueden divergir exigen un
protocolo de reconciliación propio; chunk de 64 KiB y ventana de 16, elegidos
**desde la cota de memoria** —1 MiB en vuelo por dirección— y no al revés; y dos
numeraciones separadas, la secuencia del frame que asigna el sealer y el
`chunk_index` que elige el motor, porque unificarlas exigiría que el motor
eligiera la secuencia y eso es justo lo que ADR-0022 prohíbe para que un nonce no
se repita.

Su corolario práctico: una retransmisión es un frame **nuevo**, sellado de nuevo.
Reenviar los mismos bytes sellados lo rechazaría la ventana de replay, y tendría
razón.

El sprint 5A añade ADR-0026. No cambia ningún formato existente y no añade
criptografía; conecta por primera vez el framing, el manifest, el handshake y el
AEAD, y define quién habla cuándo.

Al conectarlos aparecieron dos desajustes que cinco sprints de pruebas por
separado no podían encontrar, los dos registrados y **no** arreglados: QYR-0068,
la cabecera de 48 bytes reserva `transfer_id`, `stream_id` e `item_id` dentro de
los datos asociados autenticados y no hay forma pública de rellenarlos; y
QYR-0069, los constructores deterministas del handshake son `pub(crate)`, así que
un crate dependiente no puede reproducir una sesión byte a byte.

El sprint 5B.1 añade ADR-0027: leer y escribir archivos de verdad, sin selector y
sin FFI. No cambia `ContentSource` ni `ContentSink` —que era la comprobación de
que la costura de ADR-0026 estaba bien puesta— y no toca el motor.

Cuatro decisiones que conviene no rediscutir sin datos nuevos: ningún componente
de la ruta materializada puede ser un enlace simbólico, con `O_NOFOLLOW` cerrando
la carrera del último componente y QYR-0072 registrando que la de los
intermedios sigue abierta; una colisión en el destino **se rechaza**, porque
sobrescribir es pérdida de datos ajenos y renombrar inventa nombres que el
emisor no mandó; el `.qyro-part` vive **junto al destino** porque `rename` no
funciona entre volúmenes; y los metadatos de reanudación **no guardan el estado
del hasher** —`sha2` no lo expone— sino que releen el prefijo, que cuesta E/S y
no cuesta correcciones cuando esa biblioteca cambie por dentro.

El sprint 5B.1 añade ADR-0027: leer y escribir archivos de verdad, sin selector y
sin FFI. `ContentSource` y `ContentSink` **no cambiaron** —segunda vez que una
costura de este proyecto aguanta su segunda implementación sin ensancharse, tras
`SecretWrapper` en 4D.2a—, y el motor no se tocó.

Su hallazgo más serio no es de filesystem: QYR-0071. El análisis compartido de
guardas leía 13 401 bytes de un archivo de 30 861 porque `item_end` no sabía
terminar un item en la coma de un campo. Desde 5A, la guarda anti-pánico cubría
el 43 % de `session.rs` mientras decía cubrirlo entero. Cuarta guarda que dejó de
guardar en este proyecto, y la primera cuyo fallo estaba en el análisis que todas
comparten.

El sprint 5C enmienda ADR-0027 con fecha 2026-08-11. Implementa la política de
reanudación de §5: sólo metadata del mismo `transfer_id` y con entrada para el
item describe un `.qyro-part`; se trunca al prefijo confirmado y cualquier otro
parcial es huérfano. Metadata malformada sigue siendo error, no ausencia.

La misma enmienda resuelve QYR-0072 con la opción (c), mitigación parcial sin
dependencia ni `unsafe`: después de abrir el componente final se canonicaliza
su padre y se comprueba contención antes de escribir. Detecta el cambio que
persiste, pero no cierra un doble swap ni las operaciones posteriores por
nombre. El cierre completo sigue requiriendo resolución por descriptor/handle
en Unix y Windows.

El sprint 5C añade ADR-0029: los identificadores de la cabecera QYRO/1 usan la
API pública preexistente `Frame::with_identifiers`/`FrameHeader::with_identifiers`.
Cero es válido y significa «sin ámbito asignado» en framing; los 48 bytes y sus
offsets no cambian. AEAD autentica los IDs que puso el emisor, pero no demuestra
que un transfer/item exista: routing debe comprobarlo después de abrir el
frame. Por esa separación se eliminan `FrameError::InvalidIdentifier` e
`IdentifierField`, variantes inalcanzables que framing no debía prometer.
