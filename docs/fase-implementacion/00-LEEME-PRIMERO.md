# Qyro — Fase de implementación hasta la v1.0

**Este directorio es el plan completo.** No hay nada fuera de él que haya que
adivinar. Si algo no está aquí, no está en el alcance.

Escrito el 2026-08-12 por el supervisor técnico, tras auditar el árbol fusionado
de las ramas `claude/qyro-net-6a` y `codex/qyro-trust-5d`.

---

## 1. Cómo se usa este directorio

**Orden de lectura, la primera vez y sólo una vez:**

1. `R6-ESTADO-BASE.md` — qué existe hoy, con números que puedes reproducir.
2. `R1-REGLAS-NO-NEGOCIABLES.md` — lo que no se hace nunca, pase lo que pase.
3. `R2-PROTOCOLO-DE-PUERTA.md` — la auto-auditoría que cierra cada paso.
4. `R3-COMO-AUDITAR.md` — mutación, clases de evidencia, los anti-patrones.
5. `R4-COMO-REGISTRAR-BUGS.md` — la disciplina del ledger.
6. `R5-PLANTILLA-DE-REPORTE.md` — qué lleva el informe de cada fase.

**Después, una fase cada vez, en orden numérico:**

| # | Fase | Qué desbloquea |
|---|---|---|
| 01 | El FFI del motor | Que Dart pueda pedir algo |
| 02 | Progreso hasta Dart y transferencia conducida desde Dart | La primera prueba de producto |
| 03 | El selector de archivos | Que el usuario elija qué mandar |
| 04 | El descubrimiento y el emparejamiento en red | Que dos aparatos se encuentren |
| 05 | La interfaz de transferencia, y los botones | El producto |
| 06 | Identidad persistente en Android y iOS | Que la identidad sobreviva al reinicio |
| 07 | Hardware físico | La primera evidencia real |
| 08 | Permisos, empaquetado y firma | Poder instalarlo |
| 09 | Cierre de deuda y endurecimiento | Poder llamarlo v1.0 |
| 10 | v1.0 | El final |

**No saltes fases.** Cada una declara en su §2 de qué depende, y esas
dependencias son reales: la 05 no puede existir sin la 01, y la 07 no significa
nada sin la 03.

---

## 2. Qué es una fase

Una fase es un tramo de trabajo que:

- **se termina entera o no se termina** — no hay «fase 04 a medias»;
- **cierra con una puerta** (`R2`), que es una auto-auditoría de doce
  comprobaciones que hay que pasar antes de tocar la fase siguiente;
- **produce un informe** en `docs/reports/fase-NN-<nombre>.md` (`R5`);
- **deja el árbol en verde**: `cargo test --workspace`, `clippy -D warnings`,
  `fmt --check`, los `check_*` y los workflows.

**Una fase declarada cerrada que no lo está envenena todo lo que viene detrás.**
Ya pasó en este proyecto: QYR-0071 hizo que cuatro sprints de evidencia
estructural midieran menos de lo que decían. Por eso la puerta es obligatoria y
por eso no se negocia.

---

## 3. La regla que gobierna todo

> **No confíes en tu memoria. Confía en el código.**

Vas a trabajar muchas horas seguidas. Lo que era cierto en la fase 02 puede haber
dejado de serlo en la 06. Por tanto:

- Antes de afirmar algo, **ejecútalo o léelo en el archivo**. No lo recuerdes.
- Cuando digas un número, **di el comando con el que lo obtuviste**.
- Antes de cerrar una puerta, **relee las secciones del informe que la fase
  pudiera haber invalidado** y corrígelas contra el código actual.

Esto no es paranoia. Ya ha fallado dos veces por escrito en este repositorio:
un informe donde §4 decía 63 paquetes y §12 decía 62, y una tabla de runs que
omitía doce fallos.

---

## 4. Lo que ya no se discute

Estas decisiones están tomadas, con la investigación hecha y las fuentes
primarias citadas en las fases correspondientes. **No las vuelvas a abrir.**

| Tema | Decisión | Dónde está el porqué |
|---|---|---|
| Transporte | `std::net` + hilos. **Sin async, sin tokio.** | ADR-0028 |
| Puente Dart↔Rust | `dart:ffi` a mano + `NativeCallable.listener`. **Sin `flutter_rust_bridge`** (47–60 crates) | Fase 01 §6 |
| Selector de archivos | En Dart con `file_selector`; a Rust cruza un **fd** en Android/iOS y una **ruta** en Windows. **Sin COM a mano** | Fase 03 §6 |
| Descubrimiento | Nativo por plataforma tras un trait. **Nada de sockets mDNS crudos en móvil** | Fase 04 §6 |
| Historial | Log append-only con el formato enmarcado propio. **Sin SQLite** | Ya implementado en `qyro_fs::history` |
| Dependencias | **Cero externas.** `Cargo.lock` tiene 63 paquetes y todos son de primera parte | `R1` §2 |

---

## 5. El objetivo final, escrito una vez

**Qyro v1.0 es:** una persona abre Qyro en su teléfono y otra en su computadora,
en la misma red Wi-Fi. **Se ven el uno al otro.** Comparan una huella corta en voz
alta y se marcan como conocidos. Uno pulsa **Enviar**, elige archivos con el
selector de su sistema, y el otro ve **quién** le manda **qué** y acepta o
rechaza. Los archivos viajan cifrados, con progreso visible, y se pueden pausar y
reanudar. Al terminar, el receptor tiene los archivos verificados por SHA-256 y
los dos tienen una entrada en su historial local.

**Sin nube. Sin cuentas. Sin anuncios. Sin telemetría. Sin servidor.**

Y funciona en **Android, iOS y Windows**, instalable por quien tenga el APK, el
IPA ad-hoc o el `.exe`.

**Cuando eso ocurra en tres aparatos físicos, y no antes, es la v1.0.**

---

## 6. La línea que no se cruza hasta la fase 05

**Los botones Enviar y Recibir están `onPressed: null` a propósito**, con un texto
en pantalla que lo explica. Se habilitan **en la fase 05 y sólo si** la fase 02
demostró una transferencia real conducida desde Dart y la fase 04 demostró que dos
aparatos se encuentran.

**Habilitarlos antes es la única forma de mentirle al usuario que este proyecto ha
evitado durante siete meses.** No lo hagas.
