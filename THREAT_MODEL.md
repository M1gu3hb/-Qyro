# Modelo de amenazas

## Activos

Contenido, nombres/rutas, claves, identidad local, confianza, historial, temporales y disponibilidad.

## Adversarios

- atacante en la misma LAN;
- MITM;
- peer malicioso;
- cámara u observador óptico;
- dispositivo confiable comprometido;
- proceso local con acceso indebido;
- inputs corruptos o gigantes.

## Amenazas y controles previstos

| Amenaza | Control |
|---|---|
| MITM en el emparejamiento | handshake autenticado: firma Ed25519 sobre un transcript que incluye ambas identidades, ambos nonces y ambas efímeras (ADR-0021) |
| Replay de una firma en otra sesión | el transcript incluye los nonces y las efímeras de esa sesión |
| Degradación de suite | versión y suite se rechazan, no se negocian: no hay nada que negociar |
| Desacuerdo silencioso de claves | MAC de confirmación en ambos sentidos, comparado en tiempo constante |
| Reflexión de mensajes hacia su emisor | claves de sesión separadas por dirección |
| MITM/replay en transporte | TLS 1.3, huella/SAS, sesión, nonce, expiración — **no implementado** |
| Manifest/path traversal | rutas relativas **verbatim** (nunca reescritas), límites, y `ItemKind` con solo `File` y `Directory`, de modo que un symlink o junction es inexpresable — no hay rechazo porque no hay forma de pedirlo |
| Nombre visible engañoso | el nombre se deriva de la ruta, no viaja aparte (ADR-0019), y toda la categoría Unicode `Cf` se rechaza, así que un `U+202E` no puede mostrar `invoice<RLO>fdp.exe` como `invoiceexe.pdf` (QYR-0021) |
| Colisión al materializar | `PortableCollisionKey` rechaza pares que el FS plegaría por mayúsculas o composición Unicode, y desde 4C.2 también un archivo que es además el directorio padre de otro elemento (QYR-0028) |
| Nombre no portable | caracteres ilegales en Windows rechazados en todas las plataformas |
| Frame que miente sobre su protección | `ENCRYPTED` solo lo activa el sellado, con tag |
| Desincronización por mensaje nuevo | tipo desconocido se consume delimitado, no envenena |
| Coste cuadrático con tráfico válido | El decoder drena con un cursor y compacta de forma amortizada: un byte se copia un número acotado de veces entre entrar al búfer y salir de él (ADR-0016 enmendado, QYR-0024) | `draining_a_full_buffer_copies_a_bounded_number_of_bytes`, `a_socket_loop_with_a_backlog_stays_bounded` |
| Reserva que supera su propio techo | `push` conserva el doblado y lo recorta a `MAX_BUFFER_LEN`, con una prueba que llena el búfer de verdad (QYR-0027) | `the_buffer_never_reserves_more_than_its_limit` |
| Pánico provocado por un peer | **ningún** archivo de producción de `qyro_crypto`, `qyro_protocol` ni `qyro_manifest` tiene `panic!`, `unreachable!`, `expect`, `assert!` ni indexado sin comprobar; un lint `deny` lo mantiene así por módulo y una guarda estructural compartida lee los veintiocho archivos de los tres crates, incluido uno que nadie recordara anotar (QYR-0033, QYR-0036) |
| Texto claro que sobrevive al frame | el búfer de `open` y `AuthenticatedFrame::payload` son `Zeroizing<Vec<u8>>`; no hay accesor que entregue un `Vec<u8>` desnudo |
| Sealer que continúa tras un fallo interno | cualquier error lo envenena de forma permanente, así que no reintenta con una secuencia ya usada |
| Robo del blob de identidad | El archivo por sí solo no sirve en otra máquina: DPAPI ata el descifrado a la credencial del usuario y, salvo perfil móvil, a ese equipo (ADR-0024) |
| Otro usuario del mismo equipo | Ámbito de usuario, **sin** `CRYPTPROTECT_LOCAL_MACHINE`; con ese flag «any process running on the system can unprotect any data protected with this flag», que es justo lo que no se quiere |
| Otra aplicación del mismo usuario | Entropía adicional que separa dominio, compuesta por una constante de aplicación y la cabecera del blob. **No es un secreto**: está compilada en el binario |
| Manipulación del blob | El MAC propio de DPAPI cubre el envoltorio; la cabecera entra en la entropía, así que alterarla también hace fallar el desprotegido. Qyro no añade MAC propio |
| Identidad nueva en silencio | «No hay identidad» y «hay una y no se puede leer» son variantes distintas del enum, con una prueba cada una |
| Perfil móvil que duplica la identidad | El blob vive en `%LOCALAPPDATA%`, que no viaja con el perfil. **Mitigación parcial**: la MasterKey sí viaja, así que copiar el archivo a mano lo abre en la otra máquina |
| Corrupción | AEAD por chunk y SHA-256 final |
| Memoria/disco agotados | límites previos, streaming, cuota y preflight |
| Decompression bomb | ratio/tamaño máximo y streaming |
| QR de otra sesión | IDs, versión y autenticación |
| Clave óptica visible | peer confiable o emparejamiento bidireccional; advertencia en una cámara |
| Logs/backups | redacción, retención y cifrado local |
| Cámara activa | indicador y permiso just-in-time |
| Discovery rastreable | alias y session ID rotatorios, metadata mínima |
| DoS | timeouts, rate limits, presupuestos y cancelación |

El detalle del handshake, con la prueba que cubre cada fila, está en
`docs/security/handshake-threat-analysis.md`.

## Lo que DPAPI no protege

Se dice aquí y no en una nota al pie, porque es la limitación real del diseño de
almacenamiento y omitirla sería la clase de garantía falsa que este proyecto
lleva cuatro sprints quitando.

**Un atacante que ya ejecuta código como ese usuario descifra el blob.** Llama a
`CryptUnprotectData` con la misma constante de entropía —que está compilada en un
binario que tiene— y obtiene la semilla. No hay contraseña que pedirle, porque
Qyro no pide ninguna, y DPAPI protege contra *otros usuarios* y *otras máquinas*,
no contra el propio usuario comprometido.

Lo que esto significa en la práctica: el almacén seguro sube el listón de «copiar
un archivo» a «ejecutar código como esa persona». Es una mejora real y no es
inviolabilidad, y ninguna interfaz debe presentarla como tal.

Tres cosas más que el almacén no promete:

- Un reset administrativo de contraseña sin respaldo de dominio, o una
  reinstalación que no conserve el perfil de usuario, dejan el blob ilegible. La
  respuesta es un error tipado: el blob es caché, no archivo.
- La constante de entropía no es un secreto y no añade fuerza criptográfica;
  separa dominio entre aplicaciones del mismo usuario y nada más.
- Nada de esto se ha probado en hardware. Un runner de CI tiene un perfil recién
  creado, sin dominio, sin perfil móvil y sin historial de contraseñas, que son
  justo los casos interesantes.

## Riesgo residual

Un handshake correcto con un peer desconocido **tiene éxito**: autentica, no
autoriza. Quien decide si esa identidad merece confianza es el usuario, en un
paso explícito que todavía no existe; hasta entonces ninguna interfaz debe
presentar un handshake correcto como un peer confiable. Las identidades públicas
viajan en claro, así que un observador aprende quién habla con quién.

Un observador que ve contenido y clave en un flujo óptico unidireccional puede romper la confidencialidad. iOS puede suspender tareas. Un dispositivo comprometido puede leer datos ya descifrados. Estas limitaciones deben mostrarse sin lenguaje engañoso.

## Correcciones registradas

Tres filas de esta tabla afirmaban controles que el código no tenía. Se corrigen
en el sprint 4C.2 y se dejan anotadas en vez de reescribirse en silencio
(QYR-0031):

- «rutas relativas normalizadas» — nunca hubo normalización. `RelativePath`
  guarda la ruta tal como llegó; el campo se llamaba `normalized` y ahora se
  llama `verbatim`. Rechazar en lugar de sanear es la política declarada del
  crate, y reescribir una ruta hostil suele producir otra ruta hostil.
- «rechazo de symlink/junction» — no hay rechazo. `ItemKind` solo tiene `File` y
  `Directory`, así que un symlink no se puede expresar en un manifest. El
  resultado es el mismo; la razón no, y la diferencia importa el día que alguien
  añada un tercer `ItemKind`.
- «la ruta AEAD de producción no tiene pánicos» — era cierto solo de
  `src/aead/`. `handshake/transcript.rs` tenía un `expect` y
  `handshake/schedule.rs` un `unreachable!`, ambos alcanzables desde bytes de un
  peer.
