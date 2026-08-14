# Fase 04 — El descubrimiento y el emparejamiento

> **ESTA FASE ESTÁ ABIERTA.** Este informe se escribe durante la fase, como pide
> `R5`. Al cerrar la sesión del 2026-08-14 están hechos el paso 1 y la mitad de
> Rust del paso 2. Los pasos 3, 4, 5 y 6 **no están hechos** y §7 dice qué falta
> exactamente.

**Base de la fase:** `62658d7` (el commit que cerró la fase 03).
**Último commit de esta fase:** `67dd8da`.

---

## 1. Objetivo y alcance

> **Que dos aparatos en la misma red se encuentren solos, y que una persona pueda
> confirmar que el que ve es el que quiere.**

**No objetivos:** UI (fase 05), NAT, internet, reconexión automática, TLS,
Keystore, empaquetado, y **hardware físico** (fase 07). iOS está fuera de la v1.0
por ADR-0039, así que el paso 6 queda aplazado, no cancelado.

---

## 2. Qué se hizo

| Paso | Estado | Commit |
|---|---|---|
| 1 — ADR-0035 congelada | **hecho** | `39f645c` |
| 2 — el endpoint manual y el QR | **mitad de Rust hecha**; falta la prueba entre dos procesos | `67dd8da` |
| 3 — la confianza por el FFI | **no hecho** | — |
| 4 — Windows con `mdns-sd` | **no hecho** | — |
| 5 — Android con `NsdManager` | **no hecho** | — |
| 6 — iOS con `NWBrowser` | **no hecho** — ADR-0039 | — |

---

## 3. Cómo se hizo

### El orden, que es la decisión de la fase

**El camino manual va primero.** `FASE-04` §3.4 lo pide y ADR-0035 §1 lo congela:
una cadena `ip:puerto` funciona con aislamiento de cliente, con redes que filtran
multicast, con un permiso denegado y en un emulador. El descubrimiento automático
falla en los cuatro. Construirlo primero también significa que **la fase 05 puede
empezar sin esperar a tres integraciones nativas**.

### La cadena

```
QYRO1|<socket-addr>|<32 hex en minúscula>
```

Tres campos y un separador que no aparece en ninguna de las dos mitades, así que
dividir es exacto y **nada necesita escaparse** — que es la razón de que esto no
sea una URL. La dirección la escribe y la lee `SocketAddr`, que ya pone los
corchetes del IPv6 y ya rechaza lo que no es una dirección.

**Alternativas descartadas, con el motivo:**

| Alternativa | Por qué no |
|---|---|
| Una URL `qyro://host:puerto/huella` | Parsear una URI a mano es más código que esto y el IPv6 mete corchetes que hay que tratar aparte |
| Separar por `:` | Un IPv6 son colons. Sería ambiguo justo donde más importa |
| Base32 o base58 de un blob binario | Más denso en el QR y **ilegible en voz alta**, que es lo único que hace útil una huella |
| Aceptar hex en mayúscula también | Dos ortografías de la misma huella es la ambigüedad que ADR-0031 quitó de la huella humana. El coste de ser estricto lo paga un escáner, que no cambia de caja |

### La huella de la cadena no es una credencial

ADR-0035 §2.1. Escanear **no establece confianza**: fija qué huella tiene que
salir del handshake. Si no coincide, se corta **sin preguntar** — quien escaneó
un código ya respondió la pregunta, y volvérsela a hacer es enseñarle a decir que
sí.

---

## 4. Qué se encontró que no estaba en el plan

| # | Hallazgo | Dónde | Cómo se descubrió |
|---|---|---|---|
| 1 | **`qyro_net::Session` no publica la identidad del peer.** `qyro_crypto` sí la tiene; `qyro_net` la envuelve y no la republica. **Sin ensanchar ahí no hay huella que enseñar** | `qyro_net/src/handshake.rs` | Buscando de dónde saldría la huella para el paso 3 |
| 2 | **`qyro_session` no puede reexportar `TrustVerdict` ni `HumanFingerprint`.** La guarda `qyro_session_re_exports_nothing_it_does_not_own` sólo admite `pub use` de `crate`, `self`, `super`, `error` y `session` | `qyro_ffi/tests/c_abi_contract.rs` | Leyendo la guarda antes de escribir código, no después |
| 3 | Una meta-guarda exige que **todo enum de error de un crate tenga su comprobación de sitio de construcción**. Añadir `PairingError` puso en rojo `qyro_identity_store::guards` | `rust/crates/qyro_identity_store/src/guards.rs:211` | `cargo test --workspace` falló al añadir el enum |

**Los dos primeros no son obstáculos: son ADR-0032 funcionando.** La fachada
existe para acotar lo que `qyro_ffi` puede nombrar, y que una guarda impida
republicar un tipo ajeno es exactamente su trabajo. La consecuencia —que
`qyro_session` **posea su propio vocabulario de confianza** y convierta por
dentro— está decidida en ADR-0035 §5 y cuesta un `match` de tres brazos.

**El tercero es una guarda cazando lo que existe para cazar**, y la respuesta fue
escribir la exención con su argumento: `PairingError` se declara y se construye en
el mismo archivo porque lo único que puede rechazar una cadena de emparejamiento
es el parser de cadenas de emparejamiento.

---

## 5. Qué se arregló y qué no

Ninguna ficha nueva. Ningún defecto encontrado en código existente.

---

## 6. A qué afectaba cada defecto

No aplica: los tres hallazgos de §4 son restricciones estructurales encontradas
antes de escribir código, no defectos de comportamiento.

---

## 7. Resultado contra el objetivo — **PARCIAL, y muy parcial**

| # | Criterio de `FASE-04` §8 | Veredicto |
|---|---|---|
| 1 | ADR-0035 congelada antes del código | **Cumplido** — `39f645c` precede a `67dd8da` |
| 2 | El fallback manual + QR funciona y está probado **entre dos procesos** | **Parcial** — el tipo, el formato y siete refusales están probados en unidad; **la prueba entre dos procesos no está escrita** |
| 3 | La confianza se consulta por el FFI, clave cambiada refutada por nombre | **No hecho** |
| 4 | La huella la formatea el core | **Decidido en ADR-0035 §4, no implementado** |
| 5 | `mdns-sd` sólo bajo `cfg(windows)` | **No hecho.** `Cargo.lock` sigue en **64** paquetes, sin cambios |
| 6 | Cero dependencias externas en Android e iOS | **Se mantiene** por no haber añadido ninguna |
| 7 | Android con `NsdManager` y sin `ACCESS_LOCAL_NETWORK` | **No hecho** |
| 8 | iOS con `NWBrowser` sin entitlement de multicast | **No hecho** — ADR-0039 |
| 9 | Lo que no se pudo probar, escrito con su motivo | **Cumplido** — esta sección y §9 |
| 10 | Barrido de mutación, `R2` en las puertas, informe `R5` | **Parcial** — sin barrido todavía |
| 11 | Los botones siguen `onPressed: null` | **Cumplido** — sin tocar |

**Qué falta exactamente, en orden, para el paso 2:**

`two_processes_connected_by_a_manual_endpoint_transfer_a_file`. El arnés existe:
`qyro_net_smoke serve` ya imprime `LISTENING <puerto>` y vacía el búfer antes de
aceptar. Falta que imprima además la cadena de emparejamiento, que `send` la
acepte, y que **rechace por tipo cuando la huella autenticada no es la que la
cadena prometía** — que es la propiedad de ADR-0035 §2.1 y la única que hace que
escanear valga la pena. Eso necesita el hallazgo 1 de §4 resuelto:
`Session::peer_identity()` en `qyro_net`.

---

## 8. Clase de evidencia por afirmación

| Afirmación | Clase | Plataforma | Dónde |
|---|---|---|---|
| Una cadena de emparejamiento sobrevive al viaje de ida y vuelta, en IPv4 y en IPv6 | **Probado en unidad** | Windows 10 | `a_manual_endpoint_string_round_trips_through_a_qr_payload` |
| Un bit cambiado de huella produce una cadena distinta de la misma longitud | **Probado en unidad** | Windows 10 | `a_changed_fingerprint_would_be_visible_to_that_round_trip` |
| Hay exactamente dos separadores, también con IPv6 | **Probado en unidad** | Windows 10 | `the_string_has_the_shape_the_adr_froze` |
| Cada forma de estar mal tiene su propio rechazo | **Probado en unidad** | Windows 10 | `every_way_a_pairing_string_can_be_wrong_is_its_own_refusal` |
| Una dirección que nadie puede marcar no llega a ser un endpoint | **Probado en unidad** | Windows 10 | `an_address_nothing_can_dial_never_becomes_an_endpoint` |
| El código compila para Android | **Compilado** | `aarch64-linux-android` | `cargo clippy -p qyro_net --all-targets --target aarch64-linux-android -- -D warnings`, exit 0 |
| Dos procesos se conectan con una cadena manual y transfieren | **Ninguna** | — | No escrita |
| La confianza se consulta por el FFI | **Ninguna** | — | No implementada |
| Dos aparatos se encuentran solos | **Ninguna** | — | No implementado en ninguna plataforma |

---

## 9. Las puertas

### Puerta del paso 1 (ADR) — 2026-08-14, sobre `39f645c`

Es un commit sólo documental: las comprobaciones 1–8 y 10 no cambian respecto de
la puerta de la fase 03, y **la 11 se corrió sobre este commit** —
`check_docs_consistency` en Bash y PowerShell, **exit 0** los dos. CI sobre
`39f645c`: **success** (run 31844758207).

### Puerta del paso 2 — 2026-08-14, sobre `67dd8da`

| # | Comprobación | Veredicto |
|---|---|---|
| 1 | `cargo fmt --all --check` | **exit 0** |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** |
| 3 | `cargo test --workspace` | **exit 0** — 611 passed, 0 failed, 2 ignored, 51 suites |
| 4 | Barrido de mutación | **NO CORRIDO.** Es lo primero que le toca a la sesión siguiente |
| 5 | Lectura de aserciones | **Cumplido** — los dos lados difieren en las 7 pruebas nuevas |
| 6 | Lectura de contadores | **No aplica** — sin contadores nuevos |
| 7 | La medida se ve fallar | **Cumplido** — `a_changed_fingerprint_would_be_visible_to_that_round_trip`, y el control positivo dentro de `every_way_a_pairing_string_can_be_wrong_is_its_own_refusal` |
| 8 | Lectura de nombres | **Cumplido** |
| 9 | Coherencia del informe | **Cumplido** |
| 10 | El ledger sigue legible | **147 fichas, 39 abiertas** — sin cambios: esta fase no ha añadido ninguna |
| 11 | `check_docs_consistency` en los dos shells | **exit 0** |
| 12 | Escribir el resultado | este documento |
| 13 | `cargo clippy -p qyro_net --all-targets --target aarch64-linux-android` | **exit 0** |

**La comprobación 4 quedó sin correr y se dice en vez de omitirse.** El paso no
está cerrado hasta que corra.

---

## 10. Tabla de mutación

**Vacía. El barrido de esta fase no se ha ejecutado.** Alcance previsto cuando se
haga: `rust/crates/qyro_net/src/pairing.rs`, con `--test-workspace true`, por el
hallazgo de la fase 03 — un barrido `--package` subestima la cobertura de toda
función cuyas pruebas viven aguas abajo, y `pairing_contract.rs` es un test de
integración del propio crate, así que aquí no debería morder; se declara igual.

---

## 11. Tests antes y después

| Suite | Antes (`62658d7`) | Después (`67dd8da`) |
|---|---|---|
| Rust, Windows 10 | 603 passed / 2 ignored | **611 passed / 2 ignored** |

Ocho más: las siete de `pairing_contract.rs` y `every_pairing_error_has_a_construction_site`.

---

## 12. Delta de dependencias

**Ninguno.** `Cargo.lock` sigue en **64** paquetes, 50 de crates.io.

```
grep -c '^\[\[package\]\]' Cargo.lock   # 64
grep -c '^source = ' Cargo.lock         # 50
```

`mdns-sd` **no ha entrado**, porque el paso 4 no se ha hecho. Cuando entre, será
la primera dependencia externa desde el sprint 4A y va con su conteo, su
`cargo tree` y su `cargo audit` en el informe.

---

## 13. Archivos tocados

```
git diff --name-only 62658d7..HEAD
```

```
docs/adr/ADR-0035-discovery-and-pairing.md
docs/reports/ESTADO-ACTUAL.md
docs/reports/fase-04-descubrimiento-y-emparejamiento.md
rust/crates/qyro_net/src/guards.rs
rust/crates/qyro_net/src/lib.rs
rust/crates/qyro_net/src/pairing.rs
rust/crates/qyro_net/tests/pairing_contract.rs
```

---

## 14. Runs de CI

| Commit | Workflow | Run | Conclusión |
|---|---|---|---|
| `f153d61` | CI | 31844609972 | **success** |
| `39f645c` | CI | 31844758207 | **success** |
| `67dd8da` | CI | 31845134334 | lanzado; sin conclusión al escribir esto |
| `67dd8da` | Platform builds | 31845134142 | lanzado; sin conclusión al escribir esto |

**Los dos últimos quedan sin verificar en este informe**, y eso es una obligación
de la sesión siguiente, no una nota al pie.

---

## 15. Qué NO debe leerse como progreso

- **No hay descubrimiento.** Ni en Windows, ni en Android, ni en iOS. Nada busca
  ni anuncia nada; lo único que existe es una cadena que una persona copia.
- **No hay QR.** Existe la **cadena** que un QR codificaría, congelada y probada.
  Dibujarla y leerla con una cámara es interfaz, y la interfaz es la fase 05.
- **Nada del emparejamiento cruza el FFI todavía.** Dart no puede consultar un
  veredicto de confianza ni ver una huella. El paso 3 no está hecho.
- **La confianza sigue sin llamarse desde ninguna parte**, igual que antes de
  esta fase. `decide_trust` existe, está probada, y nada la invoca.
- **La cadena no se ha usado nunca para conectar dos procesos.** Está probada
  como texto, no como camino.
- **Los botones Enviar y Recibir siguen `onPressed: null`.**
- **Nada se ha probado en hardware físico**, y **dos procesos en `127.0.0.1` no
  son dos aparatos en una Wi-Fi** — que en esta fase, precisamente, es la
  diferencia entera.

---

## 16. Ledger y handoff

**Ninguna ficha nueva.** El ledger sigue en **147 fichas, 39 abiertas**.

### Qué documentación quedó desfasada

Ninguna. ADR-0031 no se contradice: ADR-0035 §3 **responde** la pregunta que
aquélla dejó abierta, y lo dice en su cabecera.

### Qué necesita saber quien siga

1. **Lee ADR-0035 antes de tocar nada.** Decide el orden, el formato, el momento
   de la confianza y las dos fronteras que hay que mover.
2. **El paso 3 empieza por `qyro_net`**, no por el FFI: sin
   `Session::peer_identity()` no hay huella que enseñar, y todo lo demás depende
   de eso.
3. **`qyro_session` gana `qyro_identity_store` como dependencia de primera
   parte**, y hay que actualizar `CLOSURE` en `qyro_ffi/tests/c_abi_contract.rs`,
   que es un registro de cambios y no una prohibición.
4. **La fachada no puede reexportar `TrustVerdict`.** Tiene que poseer su propio
   enum y convertir. La guarda que lo exige es
   `qyro_session_re_exports_nothing_it_does_not_own`.
5. **El barrido de mutación del paso 2 está pendiente**, y la comprobación 4 de
   su puerta quedó abierta.
