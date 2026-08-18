# Orden final — de aquí al objetivo de `R7`

> Sustituye a `01-ORDEN-ACTUALIZADO.md`. Medido sobre `c2d9a80`, en `main`,
> 2026-08-18. **Una sola rama. Todo va a `main`.**

---

## 1. Dónde está esto, verificado compilando

| | |
|---|---|
| Rama | **`main`, la única.** 506 commits, `v1.0.0` intacta |
| Rust | **750 pruebas / 0 fallos en Linux**, 739 en Windows |
| Dart | 122 pruebas |
| Ledger | 167 fichas, **1 abierta** (QYR-0365) |
| Paridad GUI/CLI | 13 capacidades, comprobada por script |
| Símbolos C | 25 |

**Fases cerradas:** 12, 13, 14, 15, 16, 21. **Abierta:** 22, con ADR-0047 congelada
y tres entregables hechos.

### Lo que la aplicación ya hace

- **GUI (Android, Windows):** enseña su huella y su código, teclea el del otro,
  **envía y recibe**, ve qué le ofrecen antes de aceptar, rechaza por motivo, y
  marca en rojo un peer cuya clave cambió. Español e inglés.
- **CLI (`qyro`, 653 KB, un archivo, sin instalar):** `send`, `recv`, `whoami`,
  `find`, `qr`, `beam`, `serial`, `how`, y el menú.
- **Las cuatro combinaciones app↔terminal**, probadas byte a byte.

---

## 2. Los cinco arreglos, y van primero

Auditoría del 2026-08-18 sobre `c2d9a80`. Ninguno es una fase; son una mañana.

1. **`cargo clippy --workspace --all-targets -- -D warnings` falla en Linux.**
   `qyro_session/src/discovery.rs:130`, `clippy::ptr_arg` sobre el stub
   `&mut Vec<FoundPeer>`. **Reproducido dos veces sobre árbol limpio con la
   toolchain fijada; el arreglo es una línea y está verificado.** Y la
   **comprobación 17 no lo caza**, porque usa `cargo check` y CI usa `clippy`:
   corrige también la comprobación, o vuelve a pasar.
2. **Los cuatro enlaces de documentación de la Release publicada son 404.**
   Apuntan a `blob/claude/qyro-cerrar-cadena-12/…`, la rama borrada. Comprobado.
3. **`ci.yml:39`** dice «No `paths:` filter, deliberately» justo debajo del bloque
   `paths:` que lo contradice.
4. **QYR-0088 y QYR-0089 llevan dos `- Estado:`** (`cerrado` y un `abierto` viejo).
   Por eso un recuento ingenuo dice 3 abiertas y el canónico dice 1.
5. **`STATUS.md` da un solo número de pruebas de Rust.** Son dos: 750 en Linux, 739
   en Windows. Di los dos.

---

## 3. El orden, de aquí al final

```
los cinco arreglos → 24 → 22 → 17 → 18 → 19 → 20 → 23
```

| Fase | Qué | Por qué ahí |
|---|---|---|
| **24** | **El ojo**: el teléfono lee los QR | **La última capacidad que falta.** Cierra el nivel 3 de la escalera de `R7`. Todo lo demás es robustez, plataforma, verdad o empaquetado |
| **22** | Lo que la gente hace: 4 escenarios + QYR-0365 | Puede cambiar el protocolo, y cambiarlo después de la 18 obliga a reescribir el modelo de amenazas dos veces |
| **17** | Windows 7 y 32 bits | Es un pipeline, no una funcionalidad. Va cuando ya no cambia el código |
| **18** | La verdad: modelo de amenazas y documentos | Después de todo lo que crea afirmaciones nuevas |
| **19** | Hardware: los cuatro canales | Necesita al propietario y dos aparatos |
| **20** | Distribución y firma | Dinero: decisión del propietario |
| **23** | La v2.0 | Sólo si alguien ha mandado un archivo en hardware |

**QYR-0365 va dentro de la 22 y es lo primero de ella.** Está abierta, severidad
alta, y **diagnosticada hasta el final**: a partir de ~50 archivos el reloj de 60 s
convierte una transferencia completa en un fallo; el emisor gasta 75 lecturas
vencidas contra 1 del receptor; `qyro_transfer` ya tiene ventana
(`WINDOW_CHUNKS = 16`), así que **es el bucle de sesión el que serializa**. El
arreglo está acotado a una sola opción con las otras dos descartadas por escrito.

---

## 4. Qué falta para decir «terminado», y son cinco frases

`R7` §6 da el criterio único. Traducido a lo que queda:

1. **El teléfono lee un QR.** → fase 24.
2. **Doscientos archivos cruzan sin mentir.** → QYR-0365, fase 22.
3. **El binario arranca en la máquina vieja.** → fase 17.
4. **Alguien ha mandado un archivo de verdad, en hardware.** → fase 19. **Es lo
   único que la v1.0 no tenía y por lo que su etiqueta valía menos de lo que
   parecía.**
5. **Lo que se publica es verdad.** → fases 18 y 20.

Nada más. El motor, los cuatro canales, las dos caras y la identidad persistente
están hechos.

---

## 5. Las reglas, y una nueva

Todo lo de `00-LEEME-PRIMERO` §4 y §5 sigue vigente. Y se añade:

### Comprobación 18 — **la puerta se corre con el comando que corre CI**

> Si CI ejecuta `cargo clippy --workspace --all-targets -- -D warnings` en Ubuntu,
> la puerta ejecuta **eso**, no `cargo check`. Una comprobación que no es el mismo
> comando no comprueba lo mismo.

Sale de que la comprobación 17 se escribió para impedir «verificar sólo en
Windows» y usa un comando distinto del que CI usa — así que el fallo de clippy en
Linux sobrevivió a la comprobación creada para cazarlo.

### Y las que ya se ganaron, porque han acertado todas las veces

- **La comprobación 14** —por cada capacidad, el llamante de producción con archivo
  y línea— lleva **nueve** capacidades muertas encontradas.
- **Dejar que una guarda te contradiga.** Once pararon el trabajo en la sesión del
  17 y acertaron las once, incluida una que el propio implementador acababa de
  escribir.
- **Ejecutar lo generado.** Correr el script del receptor serie destapó que 512
  bytes producen Base64 inválido al concatenar.
- **Borrar tu propia explicación cuando la medida la refuta.** Pasó con el disco en
  QYR-0365: 4,9 ms medidos contra «un segundo» escrito.
