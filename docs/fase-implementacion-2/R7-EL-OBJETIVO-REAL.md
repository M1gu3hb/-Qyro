# R7 — El objetivo real de Qyro

> **Este documento manda sobre todos los demás.** Cuando una decisión de alcance
> tenga dos salidas razonables, gana la que acerque a lo que está escrito aquí.
> Dictado por el propietario el 2026-08-17, después de siete meses de proyecto, con
> estas palabras: *«creo que hemos perdido el rumbo».*

---

## 1. Qué es Qyro, en una frase

**Un programa que mueve un archivo de un aparato a otro sin nube, sin cuentas, sin
servidor, sin instalación y sin depender de que los dos aparatos estén conectados a
nada — funcionando en cualquier máquina, incluidas las que ya nadie soporta.**

---

## 2. La escena que hay que resolver

Palabras del propietario, y son el requisito, no una anécdota:

> «Tengo computadoras viejas que necesito hacer cosas y no puedo subirles ni darles
> nada. Los puertos que tiene ya no sirven, entonces no le puedo ni conectar USBs.
> Entonces, ¿de qué manera les comparto archivos? Varios softwares de transferencia
> de archivos no son compatibles, o no sirven.»

Y la forma que quiere que tenga la solución:

> «Con este software, en su terminal pongo el comando, listo, sale el logo, toda la
> intro en el CMD, en la terminal, le das en recibir o enviar. Pregunta ahí la forma
> en que va a recibir. Listo, ya. Y lo recibe y le da guardar. Es todo.»

**Léelo dos veces.** No dice «abre la aplicación». Dice **terminal**. No dice
«instala». Dice **pon el comando**. La máquina de la escena **no puede instalar
nada, no tiene tienda, no tiene USB, y probablemente no tiene una GUI moderna que
pueda ejecutar Flutter.**

---

## 3. Los cinco requisitos, en orden de importancia

### R7.1 — Universal antes que bonito

Una máquina que Qyro no alcanza es un fallo del producto, no del usuario. El orden
de prioridad cuando algo choque es: **alcanzar más máquinas > más velocidad > más
funciones > mejor aspecto.**

### R7.2 — Cero terceros, cero infraestructura

Sin nube, sin relay, sin cuenta, sin servidor de señalización, sin telemetría, sin
tienda de aplicaciones, sin dependencia de que exista internet. Esto ya se cumple y
**no se negocia nunca**, ni siquiera a cambio de conveniencia.

### R7.3 — Sin instalación

El binario se copia y se ejecuta. No pide permisos de administrador para funcionar.
No escribe en el registro. No deja nada al desinstalarse porque no se instala.

### R7.4 — No hace falta que los dos aparatos estén conectados

Ésta es la que se había perdido, y es la que hace a Qyro distinto de todo lo demás.

> «El objetivo también es que no es necesario que los dispositivos estén literalmente
> conectados. Para eso también es lo de los QRs.»

Hay una **escalera de canales**, y Qyro debe bajarla hasta encontrar uno que
funcione:

| Nivel | Canal | Cuándo |
|---|---|---|
| 1 | **TCP sobre la red que ya existe** | Los dos en la misma Wi-Fi o LAN. Ya existe |
| 2 | **TCP sobre un enlace directo sin router** | Cable Ethernet entre los dos, o link-local. **Fase 14** |
| 3 | **Óptico: QR animado, pantalla → cámara** | No hay red de ninguna clase. **Fase 15** |
| 4 | **Serie: RS-232 null-modem** | La máquina no puede leer un QR porque no tiene cámara. **Fase 16** |

**Cada nivel es una respuesta a una pregunta física distinta**, no una alternativa
de gusto. El nivel 3 existe porque a veces no hay cable ni red. El nivel 4 existe
porque un PC de sobremesa de 2006 puede *mostrar* un QR pero no puede *leer* uno.

### R7.5 — Cualquier archivo

Notas, texto, fotos, vídeos, documentos, carpetas enteras. **Con una salvedad
honesta que hay que decir en la interfaz y no en una nota al pie:** el canal óptico
mueve entre 1 y 10 KB/s. Sirve para texto, configuración, código, claves y
documentos pequeños. **Una foto tarda entre seis y diecisiete minutos. Un vídeo es
imposible.** Ver `R8` §4. Qyro debe **elegir el canal más rápido disponible** y, si
el usuario fuerza uno lento con un archivo grande, **decirle cuánto va a tardar
antes de empezar y dejarle cancelar.**

---

## 4. La consecuencia arquitectónica, dicha sin rodeos

El motor de Rust ya hace todo lo difícil: handshake autenticado, cifrado por frame,
verificación por archivo, confianza por huella, identidad persistente. **Eso no se
toca.**

Lo que falta es que ese motor tenga **dos caras y cuatro canales**:

```
                       ┌─────────────────────────┐
   cara 1  ────────────┤                         │
   Flutter GUI         │      MOTOR DE RUST      │
   (Android, Windows)  │   ya construido y       │
                       │   probado. No se toca.  │
   cara 2  ────────────┤                         │
   qyro CLI            └───────────┬─────────────┘
   (un binario, todas              │
    las máquinas)      ┌───────────┴─────────────┐
                       │   canal 1  TCP LAN      │  hecho
                       │   canal 2  sin router   │  fase 14
                       │   canal 3  QR óptico    │  fase 15
                       │   canal 4  serie        │  fase 16
                       └─────────────────────────┘
```

**La cara 2 es la que resuelve la escena del §2.** Sin ella el proyecto no ha
resuelto el problema del que nació, por muy buena que sea la GUI.

---

## 5. Lo que Qyro NO es, para que nadie lo convierta en eso

- **No es una app de mensajería.** No hay chat, no hay perfiles, no hay avatares.
- **No es un sincronizador.** No hay carpetas espejo ni «Qyro Drive».
- **No es un servidor.** No corre en segundo plano esperando. Se abre, se usa, se
  cierra.
- **No es un producto de tienda.** No hay analítica, no hay onboarding, no hay
  cuentas, no hay actualizaciones automáticas.
- **No añade funciones que nadie pidió.** Sin temas, sin ajustes, sin plugins.

---

## 6. El criterio único para decidir

Cuando dudes, pregunta esto:

> **¿Esto acerca el día en que alguien mete un archivo en un PC viejo, sin USB, sin
> tienda y sin red, tecleando un comando?**

Si la respuesta es sí, hazlo. Si es no, va a `deuda-de-calidad.md` o no va.
