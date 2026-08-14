# Deuda de calidad

**Qué es este archivo.** Desde el 2026-08-14 rige la regla del carril: **sólo un
P0 detiene una fase.** Todo lo demás —P1, P2, P3, guardas que prometen de más,
comentarios inexactos, deriva de ADR, supervivientes de mutación— se registra
aquí y **no se arregla en el momento**. Se arregla en la **fase 09**, que existe
exactamente para eso.

**Por qué existe.** Tres sesiones seguidas se consumieron enteras en hallazgos de
calidad reales y bien arreglados, mientras el producto no se movía. Los hallazgos
sobre guardas son infinitos —esa es la naturaleza de una guarda— y cada iteración
compra una mejora marginal sobre un estándar que ya es extraordinario. El
proyecto lleva siete meses y una persona todavía no puede mandar una foto. Eso
también es un defecto, y es el único que no tenía ficha.

**La excepción, además del P0:** si un defecto **impide construir lo siguiente**
—no lo hace más feo, lo impide— entonces no es deuda, es un bloqueo, y se
arregla.

---

## Abierto

| Ficha | Sev | Qué | Se cierra en |
|---|---|---|---|
| QYR-0317 | P2 | El receptor no informa de progreso: `done` se queda en cero | 05 |
| QYR-0318 | P2 | `Progress::item` se documenta uno-based y no se asigna nunca | 05 |
| QYR-0320 | P2 | Los finales que fallan de `qyro_session` no están cubiertos | 09 |
| QYR-0322 | P2 | Un receptor no puede decir su puerto antes de que alguien se conecte | 05 |
| QYR-0323 | P1 | `file_selector_android` copia el archivo a la caché | **03 — es un bloqueo, se arregla** |
| QYR-0324 | P2 | Esta máquina no puede construir Flutter con plugins (Modo Desarrollador) | Depende del propietario |
| QYR-0326 | P3 | Un cliente HTTP viaja en el árbol de dependencias de Dart sin que nadie lo llame | 09 |
| QYR-0330 | P3 | El rebobinado tras el digest es código muerto y su justificación afirma lo contrario | 09 |
| QYR-0004 | P1 | Builds no retenidos con checksum y etiqueta | 08 |
| QYR-0005 | P1 | Auditorías y suites avanzadas no disponibles | 09 |
| QYR-0064 | P1 | El harness de binario empujado no alcanza Android Keystore | 06 |
| QYR-0295 | P1 | Barreras de integridad de la materialización | 09 |

## Registrado en el carril, sin ficha propia

*(Un hallazgo cuya ficha sería ruido, pero que no se pierde.)*

| Qué | Dónde | Se mira en |
|---|---|---|
| La guarda textual de `into_zeroizing_payload` no cubre `deref`, variable intermedia ni conversiones | `qyro_crypto/src/aead/guards.rs` | 09 — la defensa que carga el peso es `VerifiedPayload`, así que es cosmético |
| `cargo doc -D warnings` no está en la puerta, así que un enlace intra-doc roto no falla | `R2` §1 | 09 — decisión del supervisor, no arreglo |
| No hay job de CI que corra `check_docs_consistency.ps1` en `windows-latest` | `.github/workflows/ci.yml` | 09 |
| `assert_analysis_reached_the_end` compara la última línea no vacía, y en un `.rs` esa línea es `}`: sobrevive a cualquier truncamiento, así que la comprobación es cierta y vacua (así se escondió QYR-0328) | `rust/guards/source_guard.rs` | 09 |
| El chequeo de formato de Dart corre con `working-directory: apps/qyro`, así que `tools/branding_generator` no está cubierto y hoy `dart format` lo cambiaría | `.github/workflows/ci.yml` | 09 |
