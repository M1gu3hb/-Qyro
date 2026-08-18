# Deuda de calidad — **reabierta el 2026-08-17**

**Qué es este archivo.** La regla del carril: sólo un P0 detiene una fase, y todo
lo demás se anota aquí para arreglarse en un cierre de deuda. La fase 09 la vació
y **la fase 12 la volvió a llenar**, que es exactamente para lo que existe.

**Se vacía en la fase 18.** Hasta entonces esto crece, y crecer no es un fallo:
lo que sería un fallo es que un hallazgo viviera sólo en un informe de fase que
nadie vuelve a abrir. Eso pasó — la fase 11 anotó en su línea 52 que
`qyro_session_local_address` no tenía llamante y **la observación se quedó en el
informe** hasta que la fase 12 tropezó con ella.

*(El vaciado de la fase 09 —147 fichas, cuatro familias de descarte y las cinco
entradas del carril— está en el historial de git de este archivo. No se copia
aquí: lo que importa hoy es lo que está abierto.)*

---

## 1. Capacidades vivas sin llamante de producción

**El patrón que define este proyecto.** Cuatro en dos fases, todas iguales:
escritas, probadas, e inalcanzables desde el producto. La comprobación 14 existe
por esto y las encuentra en minutos.

| Qué | Símbolo | Estado |
|---|---|---|
| Descubrimiento automático | **ninguno en la superficie C** | **Declarado fuera de la v1.x** (fase 12). `DiscoveryChannel.kt` registrado, ningún Dart abre `dev.qyro/discovery`. Lo conecta la fase 14 |
| Dirección local de una sesión viva | `qyro_session_local_address` | **Sin llamante, a propósito.** ADR-0041 lo hace innecesario hoy —el puerto es fijo— y la fase 14 lo necesitará de verdad al ligar por interfaz |
| Historial | ninguno | **Retirado de la interfaz** (QYR-0358). El motor sigue grabando |
| Materializar lo recibido | `qyro_session_finish` | **Cerrado** (QYR-0357). Tenía cero llamantes y un archivo recibido nunca llegaba |

**Lo que queda abierto de esta tabla:** las dos primeras filas, y las dos tienen
fecha — fase 14. No son deuda difusa: son trabajo con su fase asignada.

---

## 2. Deuda abierta, con su tamaño

| # | Qué | Dónde | Tamaño |
|---|---|---|---|
| D1 | **Mojibake**: 30 secuencias `Ã¢â‚¬â€` y similares, UTF-8 leído como Latin-1 | `rust/crates/qyro_session/src/session.rs`, y algunas en `qyro_ffi` | Mecánico. Un paso de reencodado y una guarda que impida el regreso |
| D2 | **Dos `- Estado:` duplicados** en la misma ficha | QYR-0088 y QYR-0089 de `BUGS_PENDING.md` | Dos líneas. El script de recuento lee el primero, así que el segundo es ruido que puede mentir |
| D3 | `qyro_fs::history` graba y nada lo lee | frontera C | ~150 líneas: un símbolo que emita registros como texto. Desbloquea QYR-0358 |
| D4 | `Progress::item` vale cero siempre | `qyro_session`, `qyro_ffi`, Dart | Superficie nueva del motor. La frase ya está corregida (QYR-0318); el campo sigue sin asignarse |
| D5 | El receptor no informa de progreso por bytes | `qyro_transfer` | Misma superficie que D4 (QYR-0317) |
| D6 | `cargo doc -D warnings` no está en la puerta | CI | Un job. Un enlace intra-doc roto no cambia comportamiento |
| D8 | **Un fallo visto una vez y no reproducido.** `cargo test --workspace` dio `24 passed; 1 failed` en la misma invocación que corrió `cargo fmt --all` antes; tres corridas posteriores en verde | workspace | Se anota en vez de barrerse. La hipótesis —un binario compilado de fuente pre-formato— es plausible y **no está comprobada**, y una hipótesis no comprobada no cierra nada. Si vuelve, el nombre del test es lo primero que hay que capturar |
| D7 | El paquete `http` viaja en el binario de Windows sin que nadie lo llame | `file_selector_platform_interface` | Evitarlo exige `IFileOpenDialog` a mano, que ADR-0034 §4.2 rechaza (QYR-0326) |
| D9 | **`mdns-sd` casi dobla el binario y nadie lo midió.** `qyro.exe` de `x86_64-pc-windows-msvc`: **666 624 bytes en `458d4bd`, 1 295 872 en `3ecebed`** — el commit que añadió `qyro find`. **+614 KB por una dependencia**, en un producto cuyo argumento es un binario portátil que cabe en cualquier sitio | `qyro_net`, `qyro_cli` | Medido con `cargo build --locked --release -p qyro_cli --target x86_64-pc-windows-msvc`, el mismo comando que usa `cli-builds.yml`. **No se toca ahora**: el descubrimiento funciona y quitarlo sin sustituto es cambiar el producto por una cifra. Lo que sí cambia es que ADR-0043 ya trae un `Beacon` propio con `socket2` —multicast y broadcast por interfaz, sin `mdns-sd`— y **ese camino, ya conectado y medido, cuesta 8 192 bytes.** 614 KB contra 8 KB por el mismo trabajo: **si el beacon basta en la fase 19 con red de verdad, esta dependencia deja de tener dueño.** Decisión con fecha y con cifra, no deuda difusa |
| D10 | **El puerto que ADR-0041 congeló estaba escrito dos veces y en ningún sitio del motor**: `qyro_cli::DEFAULT_PORT` y `qyroDefaultPort` en Dart, cada consumidor con su copia privada. El comentario del lado Rust decía literalmente *«**No re-derivado**: dos copias de un número de puerto son dos puertos el día que una cambie»* — siendo la segunda copia | `qyro_net`, `qyro_cli`, `apps/qyro` | **Cerrado en el sitio** (fase 14), porque el beacon iba a escribir la tercera. `qyro_net::QYRO_PORT` es ahora el original, el CLI lo reexporta y la guarda `the_two_consumers_agree_on_the_port` lee el Dart y falla si se separan — **vista fallar a propósito** con 49518 antes de darla por buena |
| D11 | **La reanudacion del canal optico no existe.** ADR-0044 §5 dice que *«checkpoint y reanudación no son opcionales»* para sesiones largas — y no están | `qyro_cli`, `qyro_fountain` | El límite de 20 MB es hoy lo que impide llegar a una sesión que los necesite: a 8 KB/s son ~40 min, y una sesión desatendida de esa longitud falla con probabilidad cercana a 1 (salvapantallas, notificación, throttling). **Con dueño y fecha: fase 22**, que es la que se ocupa de la interrupción. No es deuda difusa: es una decisión de ADR-0044 que se implementa donde tiene sentido probarla |
| D12 | **Sólo se compilaba en Windows**, y por eso un `#[cfg(windows)]` mal pegado tuvo al repositorio sin compilar en Linux durante un día, con 193 ejecuciones de CI en rojo | workspace, CI | **Cerrado en el sitio** (2026-08-18): arreglado, congelado en ADR-0043 enmienda 2, y convertido en la **comprobación 17** — `cargo check --workspace --all-targets` contra Linux por código de salida antes de cualquier informe. En esta máquina, `rustup target add x86_64-unknown-linux-gnu`; `check` no enlaza, así que no hace falta enlazador cruzado |
| D13 | **El registro tenía un estado inventado.** QYR-0362 estaba en «arreglado, evidencia parcial», que no es `cerrado` ni `descartado` | `BUGS_PENDING.md` | **Cerrado**: era un pendiente disfrazado de estado. Se cerró con la evidencia que entretanto apareció —la matriz entrega desde la GUI byte a byte— y se comprobó el vocabulario de las 165 fichas: **ninguna fuera de `cerrado`, `descartado` o `abierto`** |

**Nada de esto detiene una fase.** D1 y D2 son cosméticos con fecha en la 18. D3
tiene dueño y ficha. D4 y D5 son la misma superficie y esperan a que alguien la
necesite de verdad.

---

## 3. Lo que este archivo enseña sobre el proyecto

**Los cuatro defectos graves de las fases 11 y 12 no estaban aquí.** Ninguno era
deuda: eran capacidades que el producto anunciaba y no tenía. La deuda de calidad
es lo que se sabe y se aplaza; **eso otro era lo que no se sabía**, y la
diferencia importa porque se encuentran de formas distintas.

La deuda se encuentra leyendo. Lo otro se encuentra **preguntando quién llama**,
y escribiendo una prueba que ponga al producto en los dos papeles.

---

## 4. Cómo se cuenta

```
python - <<'PY'
import re
t=open('BUGS_PENDING.md', encoding='utf-8').read()
b=[x for x in re.split(r'\n(?=## QYR-)',t) if re.match(r'## QYR-',x)]
def estado(x):
    m=re.search(r'^- Estado: *\*{0,2}(\w+)', x, re.M)
    return m.group(1).lower() if m else '?'
print('total',len(b),'abiertas',len([x for x in b if estado(x)=='abierto']))
PY
```

`BUGS_PENDING.md` al 2026-08-17: **158 fichas, 0 abiertas.** Este archivo lleva
las siete entradas de §2, que no son fichas porque ninguna describe un defecto
con un fallo observable — describen trabajo conocido y aplazado.
