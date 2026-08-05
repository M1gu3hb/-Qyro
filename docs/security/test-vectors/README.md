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

## Regenerar

    cargo run -p qyro_crypto --features test-vectors --example emit_vectors

La feature `test-vectors` no está en `default`, así que un build de release no
puede alcanzar el constructor de semilla fija.

## Verificar

`rust/crates/qyro_crypto/tests/identity_vectors.rs` carga este JSON y comprueba
cada valor. No duplica los vectores en código: si el archivo y el código
discrepan, el test falla.

## Todavía no existe

Handshake, X25519, HKDF, AEAD, replay protection y almacenamiento seguro. Cuando
existan tendrán sus propios archivos de vectores.
