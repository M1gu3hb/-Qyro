# Fuzzing de la ruta criptográfica

Qué se fuzzea, con qué, durante cuánto, y qué **no** demuestra.

## Lo que no demuestra

Dos minutos por target encuentran defectos superficiales. No es una revisión de
seguridad, no es exhaustivo, y no sustituye a que alguien lea el código. Es un
suelo que CI puede sostener.

## Los targets

| Target | Superficie | Invariante que sostiene |
|---|---|---|
| `frame_decoder` | framing QYRO/1 | ningún frame aceptado viola los límites, sea cual sea la fragmentación |
| `encrypted_envelope` | la forma que consume el AEAD | un sobre vuelve a codificarse en los bytes de los que salió |
| `frame_opener` | AEAD y ventana de replay | solo sale texto claro de frames que autentican, y un fallo de autenticación no mueve la ventana |
| `replay_window` | transiciones de la ventana | `check` no muta; una secuencia aceptada no vuelve a aceptarse |
| `manifest_decoder` | manifest | todo manifest aceptado lleva solo rutas seguras y re-codifica igual |
| `relative_path` | parser de rutas | no entra en pánico ni reescribe su entrada |

## La sesión determinista

`frame_opener` necesita claves que encajen con los frames que recibe. Con una
sesión aleatoria, prácticamente toda entrada muere en `WrongSession` antes de que
el AEAD corra, y el target gasta su presupuesto en comparar ocho bytes.

La sesión fija vive en `qyro_crypto::fuzzing` y existe **solo bajo `--cfg
fuzzing`**.

No es una feature de Cargo, y la diferencia importa. Las features son aditivas:
cualquier crate del grafo de dependencias puede encender una para todo el mundo,
y el crate que guarda las claves no se entera. Una feature pública
`test-vectors` estaría a una línea de `Cargo.toml` de meter un constructor
determinista en un build de release. `--cfg fuzzing` lo pone cargo-fuzz en la
línea de órdenes para una compilación concreta: no se puede pedir desde una
dependencia, no aparece en ningún manifest, y no existe en `cargo build`, `cargo
test` ni `cargo install`.

El módulo expone una sesión, no material de clave. No hay accesor a un secreto de
tráfico, a una clave AEAD ni a un prefijo de nonce, porque un target no los
necesita y cada salida adicional es otra cosa que se puede hacer mal.

## Ejecutar una campaña

Desde la raíz del repositorio:

    rustup toolchain install nightly
    cargo install cargo-fuzz --locked --version 0.13.1
    cargo +nightly fuzz run --fuzz-dir rust/fuzz frame_opener \
        -- -max_total_time=300 -print_final_stats=1

`--fuzz-dir` no es opcional: sin él cargo-fuzz busca `<raíz>/fuzz` y falla con un
mensaje sobre un manifest que no existe.

`-print_final_stats=1` hace que el log diga cuántas ejecuciones hubo. Sin eso,
«se fuzzeó» es una afirmación en lugar de un número, y una campaña que murió a
los dos segundos se ve igual que una que corrió dos minutos.

## En CI

`.github/workflows/crypto-fuzz.yml`, semanal y bajo demanda, un job por target y
sin `fail-fast` para que un crash en uno no oculte si los demás también fallan.
Falla si un target deja artefactos de crash. Retiene el corpus y cualquier
entrada que haya provocado un fallo durante 30 días.

Corpus y crash inputs son cadenas de bytes que eligió el fuzzer. No llevan
material de clave: la única sesión en juego es la fija, cuyas semillas están
publicadas en este repositorio y comprometidas por definición.

## Las dos primeras campañas

120 segundos por target, los seis en success y sin artefactos de crash en ambas.
Ejecuciones y tamaño final del corpus:

| Target | Run 31051840079 (`358c64f`) | Run 31052486806 (`2c3b3b5`) |
|---|---|---|
| `frame_decoder` | 7 096 629 / 243 | 15 026 991 / 239 |
| `encrypted_envelope` | 11 689 945 / 207 | 8 836 525 / 209 |
| `frame_opener` | **63 381** / 62 | **77 956** / 67 |
| `replay_window` | 17 214 634 / 112 | 16 803 668 / 149 |
| `manifest_decoder` | 17 522 591 / 349 | 14 043 820 / 391 |
| `relative_path` | 20 574 340 / 201 | 17 748 248 / 201 |

`frame_opener` va **dos órdenes de magnitud más lento** que el resto, y se
reproduce en las dos campañas. Es lo esperado: cada iteración deriva una sesión,
sella un frame real y lo abre, así que ejecuta ChaCha20-Poly1305 dos veces por
caso mientras los demás solo parsean. Se anota porque un número pequeño en una
fila invita a pensar que algo falló, y lo que dice en realidad es que en el mismo
presupuesto ese target explora mucho menos. **Su cobertura es la más baja de las
seis, y es el que cubre el AEAD.**

La variación entre campañas —`frame_decoder` dobla, `manifest_decoder` baja un
20 %— es ruido de una máquina compartida, no una señal. Con este presupuesto los
números sirven para decir «esto se ejecutó tantas veces», no para comparar
versiones.

Cero crashes no es una buena noticia por sí sola: con este presupuesto es también
el resultado que daría un target que no ejercita nada. Lo que sostiene que sí
ejercitan algo es el crecimiento del corpus —libFuzzer solo guarda una entrada
cuando abre cobertura nueva— y las aserciones de cada target, no el hecho de que
no se cayeran.

## Qué hacer con un hallazgo

1. Añadir la entrada al corpus **antes** de corregir, para que el smoke la cubra
   a partir de entonces.
2. Corregir.
3. Comprobar borrando la corrección que alguna prueba falla. Si ninguna falla, la
   propiedad no estaba cubierta.
