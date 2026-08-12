# ADR-0031 — Confianza y emparejamiento

- Estado: **congelada antes del código**
- Fecha: 2026-08-11
- Alcance: decisión de confianza, huella humana y almacén de peers conocidos

## Contexto

El handshake de ADR-0021 demuestra que el otro extremo posee la clave Ed25519
que firmó el transcript. No demuestra que esa clave pertenezca al dispositivo
con el que la persona quería hablar. Un atacante puede completar un handshake
criptográficamente válido con su propia identidad.

Esta ADR separa tres hechos que no son equivalentes:

1. **autenticado**: el peer demostró posesión de la clave presentada;
2. **conocido**: esa clave coincide con un registro local aceptado antes;
3. **verificado fuera de banda**: una persona comparó la huella completa que se
   le mostró en ambos dispositivos.

## Decisión

### 1. TOFU explícito más comparación de huella

Qyro usa TOFU y ofrece comparación de huella, pero el primer uso **no confía
automáticamente**. Una clave que no tiene registro produce el veredicto tipado
`New`; la política interactiva futura podrá mostrar la huella y pedir una
aceptación explícita. Si existe un canal fuera de banda, la persona debe
comparar todos los grupos antes de aceptar. Sin ese canal, aceptar sigue siendo
TOFU: fija la primera clave observada, pero no prueba quién estaba al otro lado.

Los tres veredictos del mecanismo son:

- `KnownAndMatches`: existe el registro local seleccionado y la clave completa
  coincide;
- `KnownAndChanged`: existe ese registro y la clave completa no coincide;
- `New`: no existe el registro local seleccionado.

`KnownAndChanged` es un rechazo terminal para esa sesión. No actualiza la clave,
no cambia fechas y no ofrece un camino de «continuar de todos modos» dentro del
mecanismo. Una rotación legítima exige una acción separada: olvidar o reemplazar
deliberadamente el registro, volver a comparar y aceptar la nueva identidad.
Nunca se conserva silenciosamente el nombre anterior con la clave nueva.

La selección del registro conocido usa un nombre **local elegido por el
usuario**, no un nombre declarado por el peer ni recibido de la red. La entrada
pura recibe un candidato formado por `(nombre local esperado, clave pública
presentada)` y el almacén inmutable. Sin ese selector local no hay información
suficiente para distinguir «el peer conocido cambió de clave» de «apareció otro
peer nuevo»: buscar sólo por la clave convertiría todo cambio en `New`.

Los nombres son UTF-8 no vacío, de 1 a 255 bytes, sin caracteres de control, y
son únicos por comparación exacta de bytes. El mecanismo no hace normalización
Unicode, plegado de mayúsculas ni búsqueda aproximada. La UI futura debe elegir
un registro existente por identidad interna, no reconstruir esa elección desde
texto escrito por el peer.

### 2. Huella que compara una persona

La identidad conserva como valor canónico los 256 bits completos de
`IdentityFingerprint`, calculados por ADR-0020. Toda decisión del mecanismo
compara la clave pública canónica de 33 bytes y, cuando muestra evidencia, puede
conservar la huella completa; **nunca decide confianza con un prefijo**.

La forma humana de emparejamiento muestra los primeros **128 bits** de esa
huella como 32 dígitos hexadecimales minúsculos, en cuatro grupos de ocho:

```text
0123abcd-4567ef89-89abcdef-01234567
```

Se deben comparar los cuatro grupos. Hexadecimal se elige porque tiene una sola
escritura canónica ya comprendida por el proyecto, no necesita diccionario,
locale ni dependencia, y un error de transcripción no se acepta mediante
normalización permisiva.

Para que una clave elegida por el atacante coincida con los 128 bits mostrados,
el trabajo esperado es **2^128 generaciones de clave**, aproximadamente
`3.40 × 10^38`. Una colisión cualquiera entre claves generadas al azar aparece
por cumpleaños alrededor de **2^64**, aproximadamente `1.84 × 10^19`, pero ese
no es el ataque pertinente contra una huella concreta ya mostrada. Como
contraste, 32 bits exigirían sólo `2^32 = 4,294,967,296` intentos dirigidos y
tendrían un umbral de cumpleaños de `2^16 = 65,536`; por eso se rechazan.

La protección de 128 bits sólo existe si se comparan todos los caracteres. Una
persona que mira únicamente el primer o el último grupo reduce por su propia
decisión el coste a 32 bits. La interfaz futura debe presentar los cuatro grupos
con el mismo peso y no marcar éxito por una comparación parcial.

### 3. Formato del almacén de peers conocidos

El archivo es un contenedor envuelto y autenticado por la plataforma. Todos los
enteros son big-endian. Su cabecera exterior tiene exactamente 16 bytes:

| Offset | Bytes | Campo | Regla |
|---:|---:|---|---|
| 0 | 8 | magic | ASCII `QYRO-KPS` exacto |
| 8 | 1 | versión | `1`; cualquier otra es `UnsupportedKnownPeerVersion { found }` |
| 9 | 1 | wrapper | identificador conocido; otro es `UnsupportedKnownPeerWrap { found }` |
| 10 | 2 | reservado | ambos bytes deben ser cero |
| 12 | 4 | `wrapped_len` | debe coincidir exactamente con los bytes restantes y no superar 2 MiB |

Magic, versión, wrapper y reservado se entregan como entropía adicional al
wrapper bajo el dominio `qyro.known-peers.store.v1`. `wrapped_len` no participa:
como en la corrección de ADR-0024, todavía no existe cuando el wrapper corre y
su igualdad con el cuerpo se valida antes de desenvolverlo.

El cuerpo autenticado comienza con `record_count: u32`, con máximo **4096**.
Después contiene exactamente `record_count` registros. No se permiten bytes
sobrantes. Cada registro empieza con `record_len: u32`, seguido por:

| Bytes | Campo | Regla |
|---:|---|---|
| 33 | identidad pública | forma canónica versionada de `PublicIdentity`; debe decodificar |
| 2 | `name_len` | `u16`, entre 1 y 255 |
| 8 | `first_seen` | segundos Unix UTC, `i64`; no negativo |
| 8 | `last_seen` | segundos Unix UTC, `i64`; no menor que `first_seen` |
| `name_len` | nombre local | UTF-8, sin caracteres de control |

Por tanto, `record_len` está entre 52 y 306 bytes y debe ser exactamente
`51 + name_len`. Se rechazan registros truncados, longitudes fuera del límite,
identidades inválidas, nombres duplicados, claves públicas duplicadas, tiempos
inválidos, un conteo que no consume exactamente el cuerpo y cualquier byte
sobrante. El parseo es todo-o-nada: ningún prefijo parcialmente válido se
devuelve si un registro posterior falla.

Sólo se persiste lo necesario: clave pública, nombre local, primer contacto y
último contacto. No se guardan IP, puertos, nombres de red, manifests,
estadísticas, decisiones negativas, claves de sesión ni un booleano de
«verificado». El hecho persistido es sólo que una persona aceptó esa clave bajo
ese nombre. `last_seen` se actualiza únicamente después de `KnownAndMatches` y
debe escribirse mediante reemplazo atómico del archivo completo; un mismatch no
lo toca.

No hay CRC dentro del cuerpo. El wrapper elegido debe cifrar y autenticar el
archivo entero; añadir un CRC no aportaría autenticidad y crearía dos fuentes de
verdad ante corrupción. Tampoco hay modo en claro ni fallback si el wrapper no
está disponible.

### 4. Ubicación y protección por plataforma

Los tres destinos de producto siguen el alcance de ADR-0024:

| Plataforma | Ruta | Protección | Estado en este sprint |
|---|---|---|---|
| Windows | `%LOCALAPPDATA%\Qyro\known-peers.qyro` | DPAPI `CurrentUser`, con entropía de dominio propia | el wrapper ya existe; esta fase construye el formato y mecanismo, no su integración de aplicación |
| Android | `Context.filesDir/qyro/known-peers.qyro` | AES-GCM bajo clave no exportable de Android Keystore | backend fuera de alcance; no se persiste hasta que exista |
| iOS | `Library/Application Support/Qyro/known-peers.qyro` dentro del sandbox | cifrado autenticado bajo clave no exportable conservada en Keychain | backend fuera de alcance; no se persiste hasta que exista |

Aunque cada clave pública sea pública, la colección revela relaciones,
frecuencia aproximada mediante `last_seen` y nombres elegidos por el usuario.
Eso justifica confidencialidad además de integridad. En Android e iOS, «backend
ausente» es un error de capacidad; nunca autoriza a escribir el cuerpo en claro.
Los permisos del directorio y el sandbox son defensa adicional, no sustituyen
al wrapper.

### 5. Orden respecto del handshake

Se completa el handshake autenticado antes de decidir confianza. Sólo entonces
Qyro sabe qué identidad firmó el transcript y puede mostrar una huella ligada a
esa sesión. Esto implica que ya se ejecutó X25519 y se derivaron claves de
sesión con un desconocido; derivar esas claves no concede acceso ni revela
contenido por sí mismo.

El estado establecido queda en cuarentena detrás de la decisión de confianza:
antes de `KnownAndMatches` o de una aceptación explícita de `New` no se crea una
transferencia, no se envían manifests, nombres de archivo, historial ni datos de
aplicación. `KnownAndChanged` o rechazo de `New` destruyen el estado y sus
secretos. Esta fase no modifica handshake, red ni transferencia; congela el
contrato que esa integración futura deberá imponer.

Cortar el handshake antes no es una alternativa válida con el protocolo actual:
la clave aparece antes, pero sólo el handshake completo demuestra posesión y
liga la clave al transcript. Preguntar sobre una clave todavía no autenticada
permitiría que un atacante enseñara la huella de un tercero sin poseerla.

## Alternativas descartadas

- **TOFU automático.** Convierte «primera clave que alcanzó el dispositivo» en
  confianza sin una decisión humana y hace especialmente valioso ganar una sola
  carrera de red.
- **Sólo comparación manual.** Es más fuerte cuando existe un canal fuera de
  banda, pero deja sin mecanismo persistente a los usos donde no existe. Qyro
  distingue aceptación TOFU de verificación fuera de banda en la política, no
  inventa que son equivalentes.
- **Huella de 32 o 64 bits.** Sus costes dirigidos, `2^32` y `2^64`, dejan muy
  poco margen para una identidad de larga vida. Los 128 bits mantienen un coste
  dirigido de `2^128` sin obligar a leer los 256 bits completos.
- **Buscar el peer sólo por la clave presentada.** No puede producir
  `KnownAndChanged`: toda clave distinta parece nueva.
- **Aceptar y sobrescribir una clave cambiada.** Borra la única evidencia de un
  posible ataque de intermediario o de una rotación no autorizada.
- **Archivo en claro porque las claves son públicas.** Ignora que la lista, los
  nombres y los tiempos son metadata privada.
- **CRC además del wrapper.** Detecta errores accidentales que el autenticador
  ya detecta y no protege contra un escritor hostil.

## Lo que esta decisión no promete

- No hay UI para preguntar, comparar, aceptar, olvidar, renombrar ni rotar. Este
  sprint construye el mecanismo, no la política interactiva.
- No implementa sockets, descubrimiento, FFI, routing ni transferencia.
- No implementa Android Keystore ni iOS Keychain.
- No certifica que un nombre local corresponda a una persona o cuenta.
- TOFU sin comparación fuera de banda no impide un intermediario presente en el
  primer contacto.
- No recupera un almacén perdido ni sincroniza confianza entre dispositivos.
- No define revocación remota, transparencia de claves ni identidad de cuenta.
- No protege metadata que una plataforma ya haya expuesto antes de escribir el
  archivo.

## Evidencia exigida a la implementación

- `a_known_peer_whose_key_changed_is_refused_by_name` debe producir
  `KnownAndChanged`; borrar la comparación de claves debe hacerla fallar.
- `a_new_peer_is_reported_as_new_and_not_as_trusted` fija que `New` no es éxito.
- Una clave conocida idéntica produce `KnownAndMatches`.
- Una versión futura se rechaza como `UnsupportedKnownPeerVersion { found }`.
- Un store truncado falla entero y no devuelve registros parciales.
- La forma humana contiene exactamente 128 bits y una sola codificación.
- El módulo nuevo pasa un barrido focal de `cargo-mutants` con límite por
  mutante; su inventario va al informe de sprint, nunca al ledger.
