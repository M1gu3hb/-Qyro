# La decisión de firmar, o no

**2026-08-19** · Fase 20 §3.5

> **Esta decisión no la toma el implementador.** Cuesta dinero, y el dinero es una
> de las cuatro excepciones. Lo que hay aquí son los números y las consecuencias,
> ordenados para que se pueda decidir en cinco minutos.

---

## 1. Qué pasa hoy, sin firmar

Un `.exe` descargado de GitHub llega con **Mark of the Web**, y Windows lo trata
como venido de internet:

- **SmartScreen** enseña una pantalla azul de «Windows protegió tu PC» con el
  botón real escondido detrás de «Más información». Mucha gente para ahí.
- **Smart App Control** (Windows 11, si está encendido) **lo bloquea sin
  pregunta**. No hay «ejecutar de todos modos».
- **AppLocker** o una política corporativa lo bloquea si la política dice que sí.

---

## 2. La vía que ya funciona, y es gratis

**Mark of the Web vive en un flujo alternativo de NTFS.** Copiar el `.exe` a un
USB con **FAT32 o exFAT lo borra**, porque esos sistemas de archivos no tienen
dónde guardarlo. El archivo llega a la otra máquina **sin marca**, y SmartScreen
no dice nada.

Para máquinas viejas y bloqueadas —que son exactamente las de `R7` §2— **la ruta
USB es la más fiable que existe**, y no cuesta nada.

Y desde esta fase hay una segunda: **`qyro send --self`**. Una vez que hay un
Qyro corriendo en una máquina, se lleva a sí mismo a la siguiente. Son unos 800 KB
— por serie, alrededor de ochenta segundos.

---

## 3. Qué costaría firmar, y qué compra exactamente

| | |
|---|---|
| **Certificado OV** (validación de organización) | **Orden de 200–400 € al año**, según emisor. **Compruébalo antes de decidir: este documento no cotiza nada, y un precio de hace meses no es un precio.** Desde junio de 2023 exige almacenamiento en hardware —token o HSM—, así que hay un coste inicial además de la cuota |
| **Certificado EV** (validación extendida) | Más caro, y **es el que da reputación de SmartScreen desde el primer día** |

### Qué resuelve

- **Smart App Control: sí.** Un binario firmado deja de bloquearse sin pregunta.
- **SmartScreen: parcialmente, y con el tiempo.** Con OV, la reputación se
  construye con descargas; hasta que la haya, **la pantalla azul sigue saliendo**.
  Con EV es inmediata.
- **La sensación de «esto es de alguien»**, que no es técnica y sí importa.

### Qué **no** resuelve

- **AppLocker y las políticas corporativas.** Si la política dice que sólo corre
  lo que está en una lista, un certificado no cambia nada. **La máquina difícil
  de `R7` §2 puede seguir sin poder ejecutarlo.**
- **Nada de lo de arriba si el archivo llega por USB**, porque ahí ya no había
  problema.

---

## 4. Las tres salidas, y qué implica cada una

**A — No firmar nunca. Distribuir por USB y por `--self`.**
Coste: 0 €. La documentación explica la ruta USB en dos líneas. Es lo que hay
hoy, y para el caso de uso real de este producto **puede que sea suficiente para
siempre**.

**B — Firmar con OV.**
Coste anual, más el token. Resuelve Smart App Control desde el día uno y
SmartScreen con el tiempo. **No resuelve** las políticas corporativas.

**C — Firmar con EV.**
Más caro. Añade sobre B la reputación inmediata de SmartScreen.

---

## 5. Lo que el implementador sí puede decir

**El caso de uso de este producto empuja hacia A**, y la razón es concreta: la
máquina que Qyro existe para servir es la que **no puede instalar nada** — sin
permisos de administrador y a menudo con política corporativa. A esa máquina el
archivo llega por USB o por el propio Qyro, y en las dos rutas **el certificado no
cambia nada.**

Firmar compra sobre todo la primera impresión de quien descarga desde GitHub en
una máquina normal, que es un público distinto del que este producto persigue.

**Pero eso es una opinión sobre un producto, y quien decide es su propietario.**

---

## 6. Lo que hay que hacer si la respuesta es A

Ya está hecho, salvo una línea: la página de la Release tiene que decir, **arriba
del todo**, que el binario no está firmado y qué hacer con eso.

Redacción propuesta, para copiar:

> **Este binario no está firmado.** Windows enseñará una advertencia al
> ejecutarlo. Si lo copias a un USB con formato FAT32 o exFAT y lo ejecutas
> desde ahí, la advertencia no aparece — Windows sólo marca lo que llega por
> internet, y ese formato no guarda la marca.
