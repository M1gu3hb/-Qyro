# Fase 17 — Windows 7 y 32 bits

**Rama:** `main` · **2026-08-19**

**Puerta corrida con `scripts/gate.ps1`** (comprobaciones 16, 17 y 18).

---

## 1. Qué hay, y qué no

ADR-0049, congelada en `29cf52b` antes de una línea de código.

| Lo que la fase pedía | Estado |
|---|---|
| La ADR con las cinco decisiones | **HECHA** |
| Job de CI con `-Z build-std` y los dos targets win7 | **HECHO**, `win7-builds.yml` |
| Comprobación de imports **con su control**, por código de salida | **HECHA y vista fallar** |
| Los targets de 32 bits | **HECHO** en la matriz del job |
| **Un binario de win7 compilado aquí** | **NO.** §3 |

---

## 2. Comprobación 14 — llamante de producción

| Capacidad | Llamante | Consumidor |
|---|---|---|
| `check_win7_imports.ps1` | `.github/workflows/win7-builds.yml` (paso «Los imports, con su control») | **CI** |
| `win7_imports_contract_test.ps1` | `.github/workflows/ci.yml:246`, en el job `scripts` | **CI** |

Los dos tienen llamante. Y el segundo existe **para el primero**: un script de
puerta que nadie ha visto fallar puede estar saliendo 0 por la razón equivocada.

---

## 3. Lo que NO se compiló aquí, y por qué

**No hay un binario de Windows 7 en esta máquina.**

`-Z build-std` necesita nightly y `rust-src`: **alrededor de 1,5 GB en el disco de
sistema**, y el de esta máquina va justo — el propietario tuvo que interrumpir una
sesión ese mismo día para liberar espacio. Instalar un toolchain entero para
producir un artefacto que CI produce igual habría sido gastar el disco de otra
persona en comodidad propia.

**El job lo compila en un runner**, que lo tira al terminar, y sube el binario y
su tabla de imports como artefactos.

**La consecuencia, dicha:** ADR-0049 §3 deja escrito que la confirmación sobre
`msvc` está **PENDIENTE** — `R8` §10 midió sobre `-gnu`, el código de `std` es el
mismo, y eso es un argumento y no una medida. **Hasta que ese `dumpbin` se ejecute
y su salida esté pegada en un informe, este proyecto no afirma que Windows 7
funcione.** El paso «La tabla de imports, para el informe» existe justamente para
producir esa evidencia.

---

## 4. El control, que es la mitad que importa

`check_win7_imports.ps1` comprueba dos cosas:

1. El binario de win7 **no** importa `api-ms-win-core-synch-l1-2-0.dll`,
   `vcruntime140.dll` ni `msvcp140.dll`.
2. **El binario del target normal SÍ importa alguno.**

Sin (2), un patrón mal escrito, un `dumpbin` ausente o una ruta equivocada
pasarían en verde **para siempre**, diciendo exactamente lo mismo que una
comprobación que funciona.

Y el script **no se pasa en verde por no poder mirar**: si no encuentra `dumpbin`,
sale 1. Una comprobación que no puede ejecutarse es una que no se ejecutó.

**Visto fallar, aquí, con las tres entradas que tiene que rechazar:**

```
[ok]   un binario con el import de Windows 8 en el hueco de win7 fallo, como debe
[ok]   un binario que no existe fallo, como debe
[ok]   un control que apunta a un archivo que no existe fallo, como debe
```

---

## 5. Windows XP: descartada, con la respuesta al lado

No hay target de Rust para XP y no lo va a haber.

**Y eso no deja a esa máquina fuera del producto.** A una XP no se le lleva Qyro:
se le lleva un archivo **por el puerto serie desde HyperTerminal**, que ya está
instalado allí desde el día que salió. Eso es la fase 16, ya está hecho, y
`qyro serial` imprime el receptor para pegar.

La respuesta a «¿y XP?» es un procedimiento que funciona, no una excusa.

---

## 6. Lo que esta fase NO promete

- **Que Qyro arranque en un Windows 7.** Ninguno lo ha ejecutado. Fase 19.
- **Que el target Tier 3 compile.** El job existe y **no se ha ejecutado nunca**:
  se ejecutará en el primer push que toque `rust/`. Si falla, falla ahí y con log.
- **32 bits en hardware de 32 bits.** Se compila; funcionar es la fase 19.
- **Garantía de nadie.** Tier 3 significa, en palabras del proyecto Rust, que no
  hay builds oficiales. La Release lo dirá así (ADR-0049 §5).
