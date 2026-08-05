# Análisis de amenazas del handshake

Alcance: `rust/crates/qyro_crypto/src/handshake`, tal como está hoy. Este
documento no describe el transporte, que no existe.

## Suposiciones

- El atacante controla la red por completo: lee, altera, reordena, repite y
  suprime cualquier mensaje.
- El atacante puede iniciar handshakes con cualquiera de los dos lados.
- El atacante **no** conoce las claves privadas Ed25519 de los peers legítimos.
- El CSPRNG del sistema es correcto cuando responde. Si falla, el handshake
  falla.
- El proceso local no está comprometido. Un proceso que sí lo está lee las
  claves de la memoria y ninguna propiedad de aquí lo impide.

## Amenazas y controles

| Amenaza | Control | Prueba |
|---|---|---|
| Suplantación de identidad | firma Ed25519 sobre el transcript, que contiene ambas identidades | `a_tampered_responder_hello_is_refused` |
| Alteración de cualquier byte de un mensaje | todo entra en el transcript firmado | mismo test, bit a bit |
| Reutilización de una firma en otra sesión | el transcript incluye ambos nonces y ambas efímeras | `a_responder_signature_cannot_be_replayed_into_another_session` |
| Degradación de suite | versión y suite se rechazan, no se negocian | `a_foreign_version_or_suite_is_refused_not_downgraded` |
| Confusión de rol | el tipo de mensaje va dentro del transcript | `a_message_of_the_wrong_kind_is_refused` |
| Firma de un rol presentada como la del otro | las entradas de firma miden 32 y 96 bytes, y la entrada de firma incluye la longitud | `the_initiator_signs_over_the_responder_signature` |
| Clave de identidad de orden bajo | `WeakPublicIdentity`; los ocho puntos se rechazan | `a_low_order_identity_key_is_refused` |
| Clave efímera de orden bajo | `NonContributorySharedSecret` | `a_low_order_ephemeral_key_is_refused` |
| Fuga por temporización al comparar MAC | comparación en tiempo constante (`subtle`), nunca `==` | revisión de `verify_finished_mac` |
| Desacuerdo silencioso de claves | MAC de confirmación en ambos sentidos | `a_tampered_responder_finish_is_refused` |
| Reflexión de los mensajes de un peer hacia él | claves direccionales separadas | `the_two_directions_never_share_a_key` |
| Entropía fabricada | el secreto se construye desde bytes obtenidos de forma falible; no hay adaptador que pueda sustituirlos | `no_code_path_can_substitute_bytes_for_entropy` |
| Sesión usada antes de que el peer la conozca | `ResponderFinishPending` | `the_responder_is_not_established_until_it_confirms_delivery` |
| Truncamiento del identificador de sesión | derivado a ocho bytes, no recortado | `the_handshake_derives_the_wire_session_identifier` |
| Longitud declarada por el peer | no existe: todos los mensajes son de tamaño fijo | `a_message_of_the_wrong_length_is_refused` |

## Lo que este handshake **no** protege

- **No hay protección de replay de sesión completa.** Nada impide que un
  atacante reproduzca un `InitiatorHello` capturado ante el mismo respondedor.
  El respondedor generará entropía nueva, así que la sesión resultante será
  distinta y el atacante no obtendrá claves; pero sí consume recursos, y el
  control contra eso —rate limiting, timeouts— pertenece al transporte.
- **No hay confidencialidad de identidad.** Ambas identidades públicas viajan en
  claro dentro de los hellos. Un observador de la red aprende quién habla con
  quién. Ocultar la identidad del iniciador exigiría un patrón distinto y es una
  decisión de diseño que no se ha tomado.
- **No hay secreto hacia adelante frente a un compromiso de la clave de
  identidad *durante* la sesión.** Las claves efímeras dan secreto hacia
  adelante frente a un compromiso *posterior* de la clave de identidad; un
  atacante que ya tiene la clave Ed25519 puede hacerse pasar por el peer en
  handshakes nuevos.
- **No hay verificación fuera de banda.** El handshake demuestra que la
  identidad al otro lado es la que firmó, no que sea la que se esperaba. Comparar
  fingerprints, un SAS o un QR es el paso que falta, y no existe.
- **Nada está cifrado.** No hay AEAD.

## Riesgo residual conocido

Un handshake completo con un peer desconocido **tiene éxito**. Es deliberado: el
handshake autentica, no autoriza. Quien decide si esa identidad merece confianza
es el usuario, en un paso explícito posterior que todavía no existe. Hasta que
exista, ninguna interfaz debe presentar un handshake correcto como un peer
confiable.
