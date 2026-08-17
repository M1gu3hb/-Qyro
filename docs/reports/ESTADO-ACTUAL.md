# Estado actual — fase 13 en curso

**Última actualización:** 2026-08-17. **Rama:** `claude/qyro-cerrar-cadena-12`.
**Nunca `main`. Nunca force-push. Nunca reescribir historia.**

---

## 1. Dónde está esto

**Fase 12 cerrada.** Un archivo cruza entre dos procesos usando sólo el código
que el receptor publica. Release corregida y republicada desde `2c01de0`.

**Fase 13 en curso.** `qyro`, el binario de terminal, **funciona**: `whoami`
imprime huella y código, el menú se niega si no hay TTY, todo se dibuja con `\r`
y `\n`. Falta: comprobación de imports estáticos con su control, pipeline de
cuatro targets, y el informe.

**Después, sin parar: 14 (sin router), 15 (QR), 16 (serie).**

---

## 2. Qué existe

| | |
|---|---|
| Motor | Rust 1.88.0, once crates + `qyro_cli` |
| Frontera | **24** símbolos C, ninguno cruza un tipo |
| GUI | Tres pantallas (historial retirado, QYR-0358), dos idiomas |
| **CLI** | `qyro` 653 KB. `send`/`recv`/`whoami`/menú. **Sólo inglés** (ADR-0042 §6) |
| Emparejamiento | Código tecleado. Puerto fijo **49517** (ADR-0041) |
| Descubrimiento | **NO alcanzable.** Fase 14 |
| Identidad | DPAPI en Windows; sandbox por UID en Android |
| Pruebas | Rust ~700; Dart **106, 0 saltadas** |
| Ledger | **159 fichas, 0 abiertas** |

---

## 3. Las reglas que no cambian

1. **Nunca `main`**, nunca force-push, nunca reescribir historia.
2. **No se inventa evidencia de hardware.** Un hueco en blanco es la verdad.
3. **ADR congelada antes del código, en su propio commit.**
4. **Terminar una fase no es motivo para parar.** Se abre la siguiente en el
   mismo turno.
5. Dos destinos para una ficha: **cerrada, o descartada con argumento.**
6. Una ficha se cierra **respondiendo a la pregunta que hace**.
7. IDs nuevos desde **QYR-0360 en adelante**.

---

## 4. La puerta — quince comprobaciones, por exit code

```bash
cargo test --workspace && cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p qyro_session -p qyro_ffi --all-targets --target aarch64-linux-android -- -D warnings
cd apps/qyro && flutter analyze && flutter test && dart format --set-exit-if-changed .
bash scripts/check_docs_consistency.sh
powershell -NoProfile -File scripts/check_docs_consistency.ps1
```

**14 — el llamante de producción.** Tabla `capacidad | símbolo | llamante |
archivo:línea`. Ha encontrado **cuatro** capacidades muertas en dos fases. Se usa
**antes** de escribir el informe. **Desde la fase 13 hay dos consumidores del
motor** (GUI y CLI): la tabla lleva columna de cuál.

**15 — la cadena desde el gesto** hasta el byte, sin saltos.

---

## 5. El entorno

| | |
|---|---|
| Repo | `D:\Qyro\repo` |
| Flutter | `D:\flutter` 3.44.8 — **no está en PATH** |
| Android SDK | `D:\android-sdk`, sin NDK |
| Keystore | `D:\qyro-release\qyro-release.jks`, alias `qyro` |
| cargo-audit | `D:\tools\cargo\bin` |

```powershell
$env:PATH="D:\flutter\bin;D:\tools\cargo\bin;"+$env:PATH
```

---

## 6. Las trampas, ya pagadas

1. **`flutter test` a secas salta 9 pruebas** — las que cruzan el FFI. Con
   `QYRO_FFI_LIBRARY_PATH` y `QYRO_NET_SMOKE_PATH`: **106, cero saltadas**.
2. **`flutter build` no corre aquí** (Modo Desarrollador apagado, QYR-0324). Los
   artefactos los hace `release.yml`.
3. **Un artefacto se reconstruye del commit que se publica, siempre** (QYR-0359).
   Un hash correcto no prueba que el binario haga lo que las notas dicen.
4. **En heredocs de Bash, `\$` y `\t` llegan mal a Python.** Usar `chr(36)`,
   `chr(92)`, o la herramienta Edit. Ha costado cuatro vueltas.
5. **`cargo test` en Windows no compila lo que hay tras `cfg(unix)`.** Por eso la
   comprobación 13 usa `--target aarch64-linux-android`.
6. **Las guardas textuales pierden contra la sintaxis.** Que salten comentarios y
   literales, y que tengan control en los dos sentidos.
7. **`panic = "abort"` está prohibido** — mata `catch_unwind` en la frontera C
   (QYR-0305). R8 §6 lo recomienda y **el invariante gana**: 590 KB con, 653 sin.
8. **Una identidad por proceso.** Un segundo `open` con otra ruta es
   `bad_argument`, y los tests que abren una por caso fallan por eso.

---

## 7. Lo siguiente

Cerrar la 13 —imports estáticos con control, pipeline de cuatro targets,
informe— y seguir con 14, 15 y 16 sin parar entre ellas.
