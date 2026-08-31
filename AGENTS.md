# Instrucciones canónicas para agentes

## Fuente de verdad

Leer [STATUS.md](STATUS.md) antes de trabajar. STATUS.md es la única fuente de
verdad para funciones implementadas, plataformas ejecutadas, tests, artefactos y
bloqueos. [ESTADO-ACTUAL.md](ESTADO-ACTUAL.md) dice dónde se cortó la última
sesión; [HANDOFF.md](HANDOFF.md) enlaza ese estado y [NEXT_STEPS.md](NEXT_STEPS.md)
mantiene el backlog. **Este archivo describe cómo se trabaja; no describe el
estado.**

## Objetivo

Construir Qyro como transferencia privada de archivos entre Android y Windows,
en monorepo Flutter/Rust, **sin nube, sin cuentas, sin servidor y sin relay**.

**Qyro transfiere archivos.** Un archivo se elige con el selector del sistema,
cruza un socket TCP con handshake autenticado y ChaCha20-Poly1305 por frame, se
verifica con SHA-256 y se entrega. El transporte vive en
[`qyro_net`](rust/crates/qyro_net/src/lib.rs), la sesión en
[`qyro_transfer`](rust/crates/qyro_transfer/src/session.rs) y la materialización
en disco en [`qyro_fs`](rust/crates/qyro_fs/src/lib.rs).

> **La frase que estuvo aquí hasta 2026-08-31 —«el alcance no incluye
> transferencia, transporte, LAN» y «Qyro sigue sin transferir archivos»— era
> falsa desde la fase 12.** Un agente que la creyera trabajaba sobre un proyecto
> que no existe. Se deja escrito para que no vuelva: **esta sección se verifica
> contra el código, no contra su propia versión anterior.**

**Lo que sigue sin existir es la evidencia de hardware.** Ningún teléfono ha
ejecutado nunca esta aplicación y ninguna transferencia ha cruzado una Wi-Fi de
verdad. Compilado, probado en unidad, probado en integración, probado entre dos
procesos y probado en CI son **cinco cosas**, y probado en hardware es una sexta
que aquí está en blanco. Los escenarios que la cerrarían están, sin marcar, en
[docs/testing/hardware-protocol.md](docs/testing/hardware-protocol.md).

**iOS está aplazado, no cancelado** (ADR-0039): Xcode exige macOS y este proyecto
no tiene ninguno. El núcleo de Rust sigue compilando para iOS en CI.

## Las dos caras, y la regla que de ahí sale

El motor de Rust tiene **dos consumidores**, y los dos son producto:

| Cara | Dónde | Cómo llega al motor |
|---|---|---|
| La aplicación Flutter | [`apps/qyro/lib/`](apps/qyro/lib) | por la frontera C, [`qyro_ffi`](rust/crates/qyro_ffi/src/lib.rs) |
| El binario de terminal `qyro` | [`rust/crates/qyro_cli/`](rust/crates/qyro_cli/src/main.rs) | Rust a Rust, directo a `qyro_session` (ADR-0042 §2) |

> **Una capacidad no está terminada hasta que las dos caras la alcanzan**, o
> hasta que se declara para una sola con su argumento escrito. Así se rompió la
> v1.0: `Session::finish` estaba viva para un consumidor y muerta para el otro.
> La tabla que lo lleva es [docs/PARIDAD-GUI-CLI.md](docs/PARIDAD-GUI-CLI.md), y
> sus citas `archivo:línea` se verifican, no se copian.

## Los cuatro canales

1. **La red local (TCP).** El canal normal. `qyro_net` + `qyro_session`.
   Emparejamiento **tecleando un código** `QYRO1|ip:puerto|huella`, que funciona
   incluso con aislamiento de cliente en el router.
2. **El cable directo, sin router.** Dos máquinas y un cable; APIPA/link-local.
   `qyro_session::link`.
3. **El canal óptico.** El CLI dibuja QR en la pantalla y el teléfono los lee con
   la cámara: [`qyro_fountain`](rust/crates/qyro_fountain/src/lib.rs) (código
   fuente sin patente) y [`qyro_eye`](rust/crates/qyro_eye/src/lib.rs) (luma
   entra, archivo sale). **Sin red de ninguna clase.**
4. **El canal serie.** [`qyro_serial`](rust/crates/qyro_serial/src/lib.rs), para
   la máquina que no puede instalar nada.

## Reglas no negociables

- Sin cuentas, anuncios, analítica, trackers, nube, backend o relay obligatorio.
- No inventar builds, pruebas, rendimiento, compatibilidad, marca ni seguridad.
  **No se inventa evidencia de hardware: un hueco en blanco es la verdad.**
- No copiar interfaces, identidad o assets de proyectos de referencia.
- No agregar servicios remotos ni criptografía propia.
- No agregar GPL, AGPL, LGPL o licencia desconocida sin autorización.
- Una función está terminada solo con implementación, test, plataforma y
  documentación coherentes.
- **`key.properties` y cualquier keystore nunca se rastrean.** El repositorio es
  público; una clave de firma en un repositorio público no es una clave de firma.

## Arquitectura

- Flutter/Dart: presentación, accesibilidad y coordinación.
- Rust: el dominio y **el protocolo, que existe y está en el cable**.
- `qyro_ffi`: frontera estrecha sin lógica de negocio. Es la única `unsafe` que
  cruza el producto junto con `qyro_win_dpapi` (ADR-0024 §1); el resto de crates
  llevan `#![forbid(unsafe_code)]`.
- Kotlin, Swift y C++: capacidades exclusivas de plataforma.
- Dependencias hacia el dominio; el dominio no conoce Flutter ni APIs nativas.
- Cambios de arquitectura requieren ADR.

## Git

**Todo va a `main`. Nunca una rama.** El autor de cada commit es
`M1gu3hb <118588634+M1gu3hb@users.noreply.github.com>`; nunca `Claude`, nunca
`Co-Authored-By`. **Jamás force-push, jamás reescribir historia.** Commits
pequeños, Conventional Commits.

## Flujo obligatorio

1. Leer STATUS.md, ESTADO-ACTUAL.md, HANDOFF.md y NEXT_STEPS.md.
2. Ejecutar doctor y baseline relevante.
3. Para comportamiento nuevo: **test rojo, fallo correcto**, implementación
   mínima, verde y refactor. **Arreglar sin una prueba que falle antes está
   prohibido**: sin ella nadie sabe si el arreglo arregla.
4. Ejecutar formato, análisis y tests tras cada cambio.
5. Actualizar STATUS.md y documentos afectados sin duplicar estado.
   **`ESTADO-ACTUAL.md` se actualiza dentro del commit de contenido**, nunca en
   un `chore(status)` aparte.
6. Registrar fallos reales en BUGS_PENDING.md.

## El calibre se ajusta al riesgo

| Riesgo | Qué llevan | Ejemplos |
|---|---|---|
| **Alto** | todo el ceremonial: ADR congelada antes del código, barrido, prueba roja, refutación | cripto, protocolo en el cable, frontera C, `unsafe`, identidad |
| **Medio** | ADR sólo si decide algo | motor, canales, CI |
| **Bajo** | compila + una prueba que falle sin el cambio + la puerta | pantallas, textos, Kotlin de interfaz, manifiestos, empaquetado, documentos |

## Comandos

    bash scripts/doctor.sh
    bash scripts/bootstrap.sh
    bash scripts/test_all.sh

PowerShell dispone de `doctor.ps1`, `bootstrap.ps1` y `test_all.ps1` equivalentes.

**La puerta**, que corre los mismos comandos que CI porque los lee de
`.github/workflows/ci.yml` en vez de llevar su propia lista:

    pwsh -File scripts/gate.ps1

La puerta queda **verde en el commit que el informe nombra**. Si se commitea
después, se vuelve a correr.

## Seguridad

- Limitar longitudes **antes** de reservar memoria.
- Validar rutas, tamaños, versiones y entradas externas. El CVE del sector es
  escribir fuera del destino: `qyro_fs::safe_path` y `qyro_manifest::path`.
- Redactar diagnósticos; no exponer claves, contenido ni rutas completas.
- Un nombre de archivo es texto de un tercero y **un terminal es un intérprete**:
  pasa por `qyro_session::safe_terminal_name` antes de imprimirse.
- Fijar versiones y actualizar LICENSE_AUDIT/THIRD_PARTY_NOTICES.
