# Vectores criptográficos

**TEST ONLY — NEVER PRODUCTION.**

Las semillas de estos archivos son públicas y conocidas. Cualquier identidad
derivada de ellas está comprometida por definición. No deben usarse jamás para
generar una identidad real.

## Propósito

Estos archivos son la fuente **interoperable** del formato. Cuando exista una
implementación en Swift, Kotlin o Dart, debe reproducir exactamente estos bytes.
Por eso los vectores viven en JSON y no solo dentro de tests de Rust: un test en
Rust prueba que Rust es consistente consigo mismo, no que el formato esté
definido sin ambigüedad.

## `identity-v1.json`

Identidad Ed25519, fingerprint y firmas con separación de dominios.

La semilla es la de **RFC 8032, sección 7.1, TEST 1**. La clave pública
resultante (`d75a9801…511a`) coincide byte a byte con la del RFC, lo que
comprueba que la implementación de Ed25519 es la correcta antes de comprobar
nada específico de Qyro.

Construcciones que fija el archivo:

- Fingerprint:
  `SHA-256( "QYRO-DEVICE-IDENTITY-V1" || 0x00 || version || public_key )`
- Entrada de firma:
  `"QYRO-SIGN-V1" || 0x00 || domain || len(message) as u64 BE || message`

El separador `0x00` y la longitud explícita impiden que dos pares
(dominio, mensaje) distintos produzcan los mismos bytes firmados.

## `rfc8032-ed25519.json`

Las cinco pruebas de la **sección 7.1 del RFC 8032** (TEST 1, TEST 2, TEST 3,
TEST 1024 y TEST SHA(abc)), extraídas del texto del RFC en
`https://www.rfc-editor.org/rfc/rfc8032.txt`.

Estas firman el mensaje **directamente**, sin separación de dominios de Qyro:
comprueban que la implementación de Ed25519 de la que depende Qyro cumple el
estándar. La construcción propia de Qyro la cubre `identity-v1.json`.

Cada entrada conserva `message_len` tal como lo declara el RFC, y el test
comprueba que coincide con los bytes del propio archivo. Es lo que hace visible
un recorte del mensaje de 1023 bytes en lugar de dejarlo pasar.

La semilla de TEST 1 es la misma que usa `identity-v1.json`, y su clave pública
coincide byte a byte en ambos archivos.

## Regenerar

No hay ejecutable de regeneración, y es deliberado: exigiría un constructor
determinista en la API pública de `qyro_crypto`, que es exactamente lo que la
biblioteca no debe exportar. El constructor de semilla fija es `cfg(test)` y
privado del crate.

Para cambiar un valor a propósito, edita el JSON y ejecuta las pruebas: fallarán
señalando cada discrepancia con la implementación.

## Verificar

`rust/crates/qyro_crypto/src/vectors.rs` carga ambos JSON y comprueba cada valor.
Vive dentro del crate porque un test de integración es otro crate y solo podría
alcanzar el constructor de semilla fija a través de API pública. No duplica los
vectores en código: si un archivo y el código discrepan, el test falla.

Los archivos se parsean como JSON, no raspando subcadenas `"clave": "valor"`. La
búsqueda de texto anterior leía el primer campo que apareciera, así que renombrar
o reordenar una clave habría cambiado en silencio qué valor se comprobaba
mientras el test seguía pasando.

## `rfc7748-x25519.json`

Los vectores de **RFC 7748**: las dos multiplicaciones escalares de la sección 5,
que fijan el clamping que este crate deliberadamente no implementa, y el
intercambio completo de la sección 6.1.

## `handshake-v1.json` y `handshake-v1.schema.json`

Una ejecución completa del handshake autenticado: identidades, entropía, claves
efímeras, secreto compartido, los cuatro mensajes, ambos transcripts, ambas
entradas de firma, ambas firmas, cada `info` de HKDF, cada clave derivada, el
`session_id` de ocho bytes y ambos MAC de confirmación.

El schema es estricto: `additionalProperties: false`, todos los campos
obligatorios, cada campo hex fijado a su longitud exacta, versiones como
constantes. Se genera a partir de los anchos del propio documento, así que no
puede discrepar en silencio de ADR-0021.

El validador de las pruebas implementa solo el subconjunto de JSON Schema que
este schema usa y **falla ante cualquier palabra clave que no entienda**. Un
validador que ignora lo desconocido informa de éxito sobre restricciones que
nunca comprobó, que es peor que no validar.

Regenerar:

    cargo test -p qyro_crypto generate_handshake_vector -- --ignored --nocapture

## Todavía no existe

AEAD, sellado de frames, replay protection y almacenamiento seguro. Cuando
existan tendrán sus propios archivos de vectores.
