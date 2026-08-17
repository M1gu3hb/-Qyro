# ADR-0006: SQLite local

- Estado: **superada** por los formatos propios de `qyro_fs` — ver la enmienda al final
- Fecha: 2026-08-04

## Contexto

Resume, confianza e historial requieren persistencia crash-safe.

## Decisión

SQLite desde Rust con migraciones y campos sensibles cifrados.

## Alternativas

Archivos JSON; DB por plataforma.

## Consecuencias

Semántica uniforme y transacciones.

## Riesgos

Bindings/compilación móvil y gestión de clave.

---

## Enmienda de la fase 10 — 2026-08-16

**Esta decisión no describe la v1.0.** Se deja entera y sin reescribir: una ADR
que se corrige en silencio deja de ser un registro de decisiones y pasa a ser
una descripción del presente, que es lo que ya hace `docs/release/v1.0.md`.

**Qué decía:** «SQLite desde Rust con migraciones y campos sensibles
cifrados».

**Qué existe:** **no hay SQLite** — no aparece en `Cargo.lock`. Las tres cosas
que la ADR quería persistir van cada una en un formato propio, pequeño y
verificable:

| Qué | Dónde | Cómo |
|---|---|---|
| Historial | `qyro_fs::history` | Archivo append-only `QYRO-HST`, registros de tamaño fijo, recuperable tras un corte |
| Peers conocidos | `qyro_identity_store::known_peers` | Archivo propio; la clave pública de cada peer, que no es un secreto |
| Reanudación | `qyro_fs::resume` | Estado parcial junto al `.qyro-part` |

**Por qué:** SQLite es una dependencia C que hay que compilar para cuatro
objetivos —incluidos dos de Android— y el proyecto tiene **cero** dependencias
externas de Rust en Android. Lo que se guarda son tres estructuras planas sin
consultas, sin joins y sin migraciones previsibles; una base de datos para eso es
una dependencia grande a cambio de nada.

**Lo que se pierde y se dice:** transacciones. Cada formato resuelve su
atomicidad a mano, y esa es la clase de código donde aparecen los errores de
corte de corriente. Por eso el historial es append-only con registros de tamaño
fijo: la peor pérdida posible es el último registro, no el archivo.
