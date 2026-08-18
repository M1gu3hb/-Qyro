# Modelo de amenazas — v1.0

**Reescrito en la fase 10 contra el código que existe, no contra el que se
planeó.** La versión anterior mezclaba controles implementados con controles
previstos, y una de sus filas decía «TLS 1.3» de un producto que no tiene TLS.
Una tabla que no distingue lo hecho de lo pensado es peor que no tenerla: se lee
como una lista de garantías.

Cada fila de la tabla de §4 nombra **dónde está el control**. Si una fila no
puede nombrarlo, no está en §4: está en §5, que es la lista de lo que este
producto **no** defiende.

---

## 0. La afirmación que gobierna todo lo demás

> **Nada de esto se ha ejecutado nunca en hardware físico.**

Todo lo que sigue está probado en unidad, en integración, entre dos procesos
reales del mismo sistema operativo, y en CI sobre Linux y Windows. Ningún
teléfono ha ejecutado nunca esta aplicación, y ninguna transferencia ha cruzado
una Wi-Fi de verdad. Un control probado en `127.0.0.1` es un control probado
contra una red que no pierde paquetes, no reordena y no tiene a nadie más.

Los veinte escenarios que cierran ese hueco están en
`docs/testing/hardware-protocol.md`, con su comando literal y su resultado en
blanco.

---

## 1. Alcance

**Dentro:** un archivo que va de un aparato a otro por una red local, y la
identidad que cada extremo usa para probar quién es.

**Fuera:** todo lo que pasa después de que el archivo llegue. Qyro escribe un
archivo en un disco y ahí termina su responsabilidad: no lo abre, no lo ejecuta,
no lo indexa y no lo analiza. Un archivo recibido es tan peligroso como el
aparato que lo mandó, y ninguna interfaz debe sugerir lo contrario.

---

## 2. Activos

| Activo | Por qué importa |
|---|---|
| El contenido de los archivos | Es el producto |
| Los nombres y rutas relativas | Se muestran a una persona y se convierten en rutas de disco |
| La semilla de identidad privada | Quien la tiene **es** ese aparato |
| El libro de peers conocidos | Lo que hace que «la clave cambió» signifique algo |
| El historial | Quién mandó qué a quién, y cuándo |
| Los `.qyro-part` | Contenido a medias, en disco, con nombre distinto |
| La disponibilidad | Una aplicación colgada por un desconocido no transfiere nada |

---

## 3. Adversarios, y qué puede cada uno

| Adversario | Qué puede de verdad |
|---|---|
| **Alguien en la misma LAN** | Ver el anuncio mDNS con la huella pública, abrir conexiones, mandar bytes arbitrarios |
| **MITM activo** | Interceptar, reordenar, reinyectar y suplantar direcciones |
| **Peer malicioso ya conectado** | Todo lo que el protocolo permite decir: manifiestos hostiles, nombres hostiles, tamaños mentidos, frames corruptos |
| **Otro usuario del mismo equipo** | Leer archivos que no estén protegidos por permisos del sistema |
| **Otra aplicación del mismo usuario** | Leer el blob de identidad **si consigue descifrarlo** |
| **Código ejecutando como ese usuario** | Todo. Este adversario gana; §6 lo dice sin adornos |
| **Entradas corruptas o gigantes** | Intentar agotar memoria, disco o tiempo |

**No está en el modelo:** un atacante con root o con el aparato desbloqueado en
la mano, y un observador global de la red. El primero gana por definición; el
segundo no existe en una LAN doméstica.

---

## 4. Amenazas con un control que existe

### 4.1 Identidad y handshake

| Amenaza | Control | Dónde |
|---|---|---|
| MITM en el emparejamiento | Firma Ed25519 sobre un transcript con ambas identidades, ambos nonces y ambas efímeras | ADR-0021; `qyro_crypto/handshake/` |
| Replay de una firma en otra sesión | El transcript incluye los nonces y las efímeras de **esa** sesión | `handshake/transcript.rs` |
| Degradación de suite | Versión y suite se **rechazan**, no se negocian | ADR-0021; no hay código de negociación que atacar |
| Desacuerdo silencioso de claves | MAC de confirmación en ambos sentidos, comparado en tiempo constante | `handshake/schedule.rs` |
| Reflexión de mensajes hacia su emisor | Claves de sesión separadas por dirección | ADR-0022 |
| Un aparato conocido presenta otra clave | `KnownAndChanged` es **rechazo terminal**: no actualiza, no pregunta, y en la interfaz el botón de enviar **no existe** — no está deshabilitado, no está | ADR-0031, ADR-0036 §4; `qyro_session/trust.rs`, `PeerTile` |
| Huella del código de emparejamiento falsificada | La huella del código es **una expectativa, no una credencial**: la confianza se decide después del handshake, contra la clave que el peer probó poseer | ADR-0035 |
| Identidad nueva creada en silencio | «No hay identidad» y «hay una y no se puede leer» son variantes distintas del enum, con una prueba cada una | `qyro_identity_store` |

### 4.2 Los bytes en el cable

| Amenaza | Control | Dónde |
|---|---|---|
| Lectura del contenido en tránsito | ChaCha20-Poly1305 por frame | ADR-0022 |
| Modificación en tránsito | El tag AEAD por frame, y SHA-256 por archivo antes de entregar | ADR-0022, ADR-0026 |
| Un frame que miente sobre su protección | El bit `ENCRYPTED` sólo lo activa el sellado, con tag | `qyro_protocol` |
| Desincronización por un mensaje nuevo | Un tipo desconocido se consume delimitado; no envenena el flujo | ADR-0018 |
| Coste cuadrático con tráfico válido | El decoder drena con un cursor y compacta de forma amortizada | ADR-0016 enmendado, QYR-0024 |
| Reserva que supera su propio techo | `push` recorta a `MAX_BUFFER_LEN`, con una prueba que llena el búfer de verdad | QYR-0027 |
| Texto claro que sobrevive al frame | `Zeroizing<Vec<u8>>` en `open` y en `AuthenticatedFrame::payload`; no hay accesor que entregue un `Vec<u8>` desnudo | `qyro_crypto/aead/` |
| Un sealer que continúa tras un fallo interno | Cualquier error lo envenena de forma permanente | `qyro_crypto/aead/` |

### 4.3 Lo que llega al disco

| Amenaza | Control | Dónde |
|---|---|---|
| Path traversal | Rutas relativas **verbatim**, nunca reescritas, y la ruta resuelta tiene que caer dentro de la raíz | ADR-0027; `qyro_fs/safe_path.rs` |
| Symlink en la ruta de destino | Se rechaza si **cualquier** componente existente es un enlace simbólico | `safe_path.rs` |
| Symlink dentro del manifiesto | `ItemKind` sólo tiene `File` y `Directory`: un symlink es **inexpresable**, no rechazado | ADR-0017 |
| Nombre visible engañoso | El nombre se deriva de la ruta y no viaja aparte; toda la categoría Unicode `Cf` se rechaza, así que `U+202E` no puede mostrar `factura<RLO>exe.pdf` | ADR-0019, QYR-0021; y `safeDisplayName()` lo repite del lado de la interfaz |
| Colisión al materializar | `PortableCollisionKey` rechaza pares que el sistema de archivos plegaría por mayúsculas o composición Unicode, y un archivo que además es el directorio padre de otro | QYR-0028 |
| Nombre no portable | Los caracteres ilegales en Windows se rechazan en **todas** las plataformas | `qyro_manifest` |
| Un archivo a medias que parece completo | El contenido va a un `.qyro-part` y el nombre final aparece con un `rename` atómico **después** de verificar el SHA-256 | ADR-0027; `qyro_fs/io.rs` |
| Un archivo recibido que se ejecuta | **Nada se abre y nada se ejecuta.** No hay `Process.start`, no hay `ACTION_VIEW`, no hay «abrir al terminar» | verificado en `apps/qyro/lib` y en el código Kotlin |

### 4.4 Denegación de servicio

Los números están en `qyro_net/src/limits.rs`, cada uno con su argumento al lado
y una prueba que lo observa.

| Amenaza | Control |
|---|---|
| Un desconocido que manda para siempre sin autenticarse | `MAX_PREAUTH_BYTES` = 4096, aplicado **antes** de leer: ninguna lectura pide más de lo que queda del cupo |
| Slowloris en el handshake | `HANDSHAKE_DEADLINE` = 10 s **total**, no por mensaje |
| Muchos desconocidos a la vez | `MAX_PENDING_HANDSHAKES` = 8, separado a propósito de `MAX_ESTABLISHED_SESSIONS` = 4 |
| Una dirección agujero negro | `CONNECT_TIMEOUT` = 10 s, en vez de los ~2 min del sistema |
| Un peer que se queda mudo | `IDLE_TIMEOUT` = 60 s sin **un solo byte**. No hay límite de duración total: lento no es muerto |
| Memoria por un manifiesto o un frame gigante | Cotas en el decoder y en el manifiesto, con seis targets de fuzzing encima |
| Pánico provocado por un peer | **Ningún** archivo de producción de `qyro_crypto`, `qyro_protocol` ni `qyro_manifest` tiene `panic!`, `unreachable!`, `expect`, `assert!` ni indexado sin comprobar; un lint `deny` por módulo y una guarda estructural compartida lo mantienen así | QYR-0033, QYR-0036 |

### 4.5 La aplicación en el aparato

| Amenaza | Control |
|---|---|
| El blob de identidad copiado a otra máquina (Windows) | DPAPI ata el descifrado a la credencial del usuario y, salvo perfil móvil, a ese equipo (ADR-0024) |
| Otro usuario del mismo equipo (Windows) | Ámbito de usuario, **sin** `CRYPTPROTECT_LOCAL_MACHINE` |
| Otra aplicación del mismo usuario | Entropía adicional que separa dominio. **No es un secreto**: está compilada en el binario |
| Manipulación del blob | El MAC de DPAPI cubre el envoltorio, y la cabecera entra en la entropía |
| El blob de identidad en Android | **Nada lo envuelve.** El sandbox por UID de Linux, en `getNoBackupFilesDir()`, y eso es todo (ADR-0040 §7, etapa A) |
| Copia del blob a la nube por Auto Backup | `allowBackup=false`, `fullBackupContent=false` y `dataExtractionRules` con secciones vacías. **Esto faltaba hasta la fase 10** (QYR-0349) |
| Permisos de más | El manifiesto declara **una** permission, `CHANGE_WIFI_MULTICAST_STATE`, y hay una prueba que asserta el conjunto **exacto** |
| Almacenamiento sin permisos | Storage Access Framework en Android; el archivo elegido **no se copia** a la caché (QYR-0323) |

---

---

## 4.bis. Los tres canales que este documento no describía (fase 18)

Hasta aquí, **todo lo de arriba describe un canal**: TCP autenticado sobre una
LAN, con un handshake de cuatro mensajes. Los otros tres tienen adversarios
distintos, y no tenerlos escritos era la mentira por omisión más grande que
quedaba en este archivo.

### El canal óptico — **es difusión, no punto a punto**

> **Una pantalla que enseña un QR se lo enseña a la habitación entera.**

Eso no es una metáfora. Es la propiedad que lo separa de todo lo demás en este
documento, y de ella salen las cuatro filas siguientes.

| Amenaza | Qué pasa de verdad |
|---|---|
| **El handshake de cuatro mensajes no llega hasta aquí, y no puede** | El producto **sí tiene** handshake autenticado —`qyro_crypto/src/handshake`, ADR-0028— y es lo que protege el canal de red. **Por este canal no pasa**: necesita ida y vuelta, la pantalla no ve a la cámara, y aquí sólo hay ida. **Todo lo que ese handshake garantiza —que el otro lado prueba poseer una clave— no llega a este canal.** Se declara perdido aquí, no reconquistado |
| **Una segunda cámara en la habitación** | Recibe exactamente lo mismo que la primera, y el emisor **no se entera**. No hay nada que detectar: un observador pasivo de un canal de difusión no deja rastro |
| **Una grabación, una foto con teleobjetivo, un hombro** | Igual. Y peor: una grabación se puede decodificar **después**, con calma, tantas veces como haga falta. Los frames que este canal emite en bucle durante minutos son un regalo para eso |
| **Lo que `qyro beam` mueve va en claro** | El fountain **codifica, no cifra**: `qyro_fountain` es XOR de bloques con una semilla que viaja en la cabecera. Cualquiera que lea los QR reconstruye el archivo con el mismo código que el destinatario. **Este canal no tiene confidencialidad ninguna** |

**La consecuencia, dicha como regla de producto y no como advertencia:** por el
canal óptico se manda lo que se mandaría por un cartel en la pared. Una clave, un
certificado o un `.env` **no**, salvo que ya estén cifrados por su cuenta.

### El canal serie — el cable es una ventaja, y el modo degradado no autentica

| Amenaza | Qué pasa de verdad |
|---|---|
| **El modo degradado no autentica nada** | El receptor de 15 líneas de PowerShell que `qyro serial` imprime **no hace handshake, no comprueba huellas y no cifra**. Escribe lo que llegue por el puerto. Es lo que permite recibir en una máquina que no puede instalar nada, y es exactamente por eso que no puede verificar nada |
| **Quien tenga acceso físico al puerto** | Está dentro. No hay capa que lo impida |
| **Y la ventaja, porque este documento tiene que ser honesto en las dos direcciones** | **Un cable físico es mucho más difícil de interceptar que el aire.** Para leer un RS-232 hay que estar en el cable; para leer una LAN basta con estar en la LAN, y para leer un QR basta con ver la pantalla. En un cuarto cerrado con dos máquinas y un cable, el serie es el canal **más** privado de los cuatro |

### El enlace directo — la dirección nunca es identidad

| Amenaza | Qué pasa de verdad |
|---|---|
| **El anuncio lleva la huella pública, y ahora también por broadcast** | Ya estaba dicho para mDNS. **Sigue siendo exacto y es peor en alcance**: el beacon de la fase 14 emite también a `255.255.255.255`, que por definición llega a todos los de la red física. La huella es estable, así que cualquiera sabe que este aparato volvió |
| **ARP es inseguro, y por tanto una IP no dice quién es nadie** | RFC 3927 §5: *«The ARP protocol is insecure. A malicious host may send fraudulent ARP packets…»*. **La dirección de un peer no es su identidad.** Lo único que ata una conversación a alguien es la clave que prueba poseer en el handshake — y en un enlace directo, la huella leída en voz alta. Un código de emparejamiento con la IP correcta y una huella distinta es un aparato distinto, y por eso la comparación de huellas no es opcional |
| **APIPA no cambia nada de lo anterior** | Un enlace sin router no tiene menos adversarios: tiene los mismos, en un cable |

## 5. Amenazas reconocidas SIN control

Esta sección es el punto del documento. Lo que está aquí **no está defendido**.

| Amenaza | Qué pasa de verdad |
|---|---|
| **Correlación por la huella anunciada** | El descubrimiento anuncia la huella pública en el TXT del servicio, y es **estable**: cualquiera en esa red puede saber que este aparato ha vuelto. No hay alias rotatorio y no hay ID de sesión rotatorio. La versión anterior de este documento decía que sí, y era falso |
| **Quién habla con quién** | Las identidades públicas viajan en claro en el handshake. Un observador de la LAN aprende el grafo, aunque no el contenido |
| **Android: root basta para leer la identidad** | Y ésa es la frase que importa. **Con Keystore, un atacante con root necesitaría además el TEE; con el sandbox, root basta.** ADR-0040 §7 explica por qué la v1.0 sale así: el mecanismo que ADR-0037 especificó no es implementable, y el que sí funciona es un shim JNI en C que este proyecto no puede ejecutar ni una vez. Enviar un shim que nadie ha validado es peor que enviar esto y decirlo. La aplicación **no lo dice en pantalla**, y eso también es una limitación |
| **Un peer nuevo que miente sobre quién es** | El handshake **autentica, no autoriza**. Un desconocido con una clave válida completa el handshake correctamente; lo que impide que reciba algo es que una persona tiene que decidir. Comparar la huella en voz alta es lo único que ata esa clave a una persona, y eso Qyro no puede hacerlo por nadie |
| **Disco lleno en el receptor** | **No hay preflight ni cuota.** Se descubre al escribir, se convierte en un error tipado y el `.qyro-part` se recoge, pero un peer puede llenar el disco de destino hasta donde el sistema le deje. Es el escenario E3 del protocolo de hardware |
| **Contenido a medias legible en disco** | Un `.qyro-part` interrumpido se queda ahí con lo que había llegado, en claro. No se entrega y no lleva el nombre final, pero existe |
| **Historial en claro** | `QYRO-HST` no está cifrado. Quien lea el almacenamiento de la aplicación ve con quién y cuándo |
| **La cámara como superficie de entrada** (fase 24B) | Un QR es **datos que entran del mundo**, y desde la fase 24B entran por la cámara sin que nadie los teclee. Lo que lo acota: el ojo no ejecuta nada — decodifica a bytes y exige la cabecera `QF` de `qyro_fountain`, así que un QR ajeno se descarta sin llegar al motor. Lo que **no** está acotado: quien apunte la cámara a un código hostil recibe un archivo, igual que quien teclea un código hostil; **Qyro sigue sin analizar lo que llega** (fila de arriba). Y el permiso `CAMERA` es de tiempo de ejecución, así que **la aplicación puede ver por la cámara mientras esa pantalla está abierta** — no hay indicador propio más allá del del sistema |
| **Un archivo recibido que es malicioso** | Qyro no analiza nada. Verifica que llegó **exactamente** lo que el emisor mandó, que es lo contrario de verificar que sea seguro |
| **La aplicación en primer plano** | No hay transferencia en segundo plano. Bloquear el teléfono corta la transferencia; no es una fuga, es una interrupción |
| **Un paquete `http` en el grafo** | `file_selector_platform_interface` arrastra `http`. Qyro **no hace ninguna petición HTTP** y nadie lo llama, pero el paquete viaja en el binario de Windows. Evitarlo exigía escribir `IFileOpenDialog` a mano, que ADR-0034 §4.2 rechaza (QYR-0326) |

---

## 6. Lo que el almacén de identidad no protege

Se dice aquí y no en una nota al pie, porque es la limitación real del diseño.

**Un atacante que ya ejecuta código como ese usuario descifra el blob.** En
Windows llama a `CryptUnprotectData` con la misma constante de entropía —que está
compilada en un binario que tiene— y obtiene la semilla. **En Android ni siquiera
hace falta eso: lee el archivo.** No hay contraseña que pedir, porque Qyro no
pide ninguna.

**Corregido en la fase 11.** Este párrafo decía que en Android el atacante
«llama al mismo alias de Keystore». Ni había alias en uso ni había identidad
persistente: el motor generaba un par de claves por sesión y nada llamaba al
almacén. ADR-0040 arregló la persistencia y aplazó Keystore, así que la frase
verdadera hoy es la de arriba.

Lo que esto significa: el almacén sube el listón de «copiar un archivo» a
«ejecutar código como esa persona, en ese aparato». Es una mejora real y no es
inviolabilidad, y ninguna interfaz debe presentarla como tal.

Cuatro cosas más que no promete:

- **Windows:** un reset administrativo de contraseña sin respaldo de dominio, o
  una reinstalación que no conserve el perfil, dejan el blob ilegible. La
  respuesta es un error tipado: el blob es caché, no archivo.
- **Windows:** el blob vive en `%LOCALAPPDATA%`, que no viaja con el perfil
  móvil — pero la MasterKey sí, así que copiar el archivo a mano lo abre en la
  otra máquina. Mitigación **parcial**, dicho así.
- **Android, etapa A:** la semilla está en el directorio privado de la
  aplicación, sin envolver. Desinstalar borra el directorio, así que **una
  reinstalación es una identidad nueva** — correcto y deliberado, y el otro
  extremo dirá «la clave cambió», en rojo, porque cambió. Lo que **no** protege
  es contra root ni contra otro proceso del mismo UID.
- **Android, etapa B:** cuando exista el shim JNI, el mismo archivo lo envolverá
  una clave de Keystore no exportable y el byte 4 del blob es lo que permitirá a
  ese build saber que el archivo que encuentra vivió sin envolver.
- **La entropía adicional no es un secreto** y no añade fuerza criptográfica;
  separa dominio entre aplicaciones del mismo usuario y nada más.

---

## 7. Riesgo residual, en una frase

**Qyro prueba que el aparato al otro lado posee una clave. Quién sea el dueño de
esa clave lo decide una persona comparando una huella en voz alta, y si nadie la
compara, la primera conexión es confianza al primer uso y nada más.**

---

## 8. Correcciones registradas

Este documento ha afirmado controles que el código no tenía. Se corrigen y se
dejan anotadas en vez de reescribirse en silencio.

**Sprint 4C.2 (QYR-0031):**

- «rutas relativas normalizadas» — nunca hubo normalización. `RelativePath`
  guarda la ruta tal como llegó; el campo se llamaba `normalized` y ahora se
  llama `verbatim`. Rechazar en lugar de sanear es la política declarada, y
  reescribir una ruta hostil suele producir otra ruta hostil.
- «rechazo de symlink/junction» — no hay rechazo en el manifiesto. `ItemKind`
  sólo tiene `File` y `Directory`, así que un symlink no se puede expresar. El
  resultado es el mismo; la razón no, y la diferencia importa el día que alguien
  añada un tercer `ItemKind`.
- «la ruta AEAD de producción no tiene pánicos» — era cierto sólo de `src/aead/`.
  `handshake/transcript.rs` tenía un `expect` y `handshake/schedule.rs` un
  `unreachable!`, ambos alcanzables desde bytes de un peer.

**Fase 10, 2026-08-16:**

- **«MITM/replay en transporte — TLS 1.3»**: no hay TLS y no lo habrá en la v1.0.
  La fila llevaba «no implementado» al lado, que es mejor que mentir y peor que
  quitarla: dejaba en la tabla de controles algo que no era un control. Lo que
  cubre esa amenaza es ADR-0021 más ADR-0022. Ver la enmienda de ADR-0004.
- **«Discovery rastreable — alias y session ID rotatorios, metadata mínima»**:
  **falso**. Se anuncia la huella, y es estable. Movido a §5.
- **«Decompression bomb — ratio/tamaño máximo y streaming»**: no hay compresión
  en ninguna parte de QYRO/1, así que no había amenaza que controlar. Quitada.
- **«QR de otra sesión», «clave óptica visible», «cámara activa»**: no hay canal
  óptico y no hay cámara (ADR-0005 no se implementó, QYR-0348). Quitadas.
- **«Logs/backups — redacción, retención y cifrado local»**: la aplicación no
  escribe logs. Lo que sí había era **backup**, activado por defecto y sin que
  nadie lo decidiera; es QYR-0349 y ahora está apagado con una prueba encima.
- **«Memoria/disco agotados — límites previos, streaming, cuota y preflight»**:
  hay límites y hay streaming; **no hay cuota ni preflight**. La mitad cierta se
  queda en §4.4 y la mitad falsa está ahora en §5.
- **«un paso explícito que todavía no existe»** (riesgo residual): ya existe.
  Es la pantalla de peers, y ADR-0031 lleva la enmienda que dice dónde aterrizó
  cada línea de su política.
