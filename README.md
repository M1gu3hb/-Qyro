# Qyro

**Manda un archivo de un aparato a otro por la red local.** Sin nube, sin
cuentas, sin servidor, sin anuncios, sin telemetría.

**La v1.0 son Android y Windows** (ADR-0039): iOS está **aplazado**, no
cancelado, porque Xcode exige macOS y este proyecto no tiene ninguno. El núcleo
de Rust ya compila para iOS en CI y el trabajo hecho se conserva; reincorporarlo
pide un Mac y una cuenta de desarrollador de Apple.

> **Nada de esto se ha ejecutado nunca en hardware físico.** Está probado en
> unidad, en integración, entre dos procesos reales y en CI sobre Linux y
> Windows. Ningún teléfono ha ejecutado nunca esta aplicación y ninguna
> transferencia ha cruzado una Wi-Fi de verdad. Los veinte escenarios que
> cerrarían ese hueco están escritos y **sin marcar** en
> [docs/testing/hardware-protocol.md](docs/testing/hardware-protocol.md).
>
> Trátalo como software que funciona en las pruebas y que nadie ha usado.

## Qué hace

- Dos aparatos se encuentran solos por mDNS/NSD, o con un código de
  emparejamiento tecleado — el camino que funciona también con aislamiento de
  cliente. **No se escanea: no hay cámara.**
- Eliges archivos con el selector del sistema. En Android, sin un solo permiso de
  almacenamiento y **sin copiar el archivo** para leerlo.
- Ves con quién hablas: una huella corta para comparar en voz alta, y su estado.
  Si un aparato conocido presenta otra clave, Qyro **se niega** y no ofrece
  «continuar de todos modos».
- El receptor decide, siempre. **Nada se acepta solo, nunca.**
- Handshake autenticado, ChaCha20-Poly1305 por frame, SHA-256 por archivo. Un
  archivo que no verifica **no se entrega**.
- Español e inglés.

Qué **no** hace, y por qué: [docs/release/v1.0.md](docs/release/v1.0.md).
Qué defiende y qué no: [THREAT_MODEL.md](THREAT_MODEL.md).

## Estado verificable

La fuente canónica es [STATUS.md](STATUS.md), que distingue entre compilar,
ejecutar, empaquetar y probar, con runs y bloqueos reales. El registro de
hallazgos es [BUGS_PENDING.md](BUGS_PENDING.md); las decisiones, con lo que cada
una descartó y por qué, están en [docs/adr/](docs/adr).

## Instalar

Los artefactos y sus SHA-256 están en
[docs/release/v1.0.md](docs/release/v1.0.md). **Compara el hash antes de
instalar nada.**

## Desarrollo

    bash scripts/doctor.sh
    bash scripts/bootstrap.sh
    bash scripts/test_all.sh

Equivalentes PowerShell:

    pwsh -NoProfile -File scripts/doctor.ps1
    pwsh -NoProfile -File scripts/bootstrap.ps1
    pwsh -NoProfile -File scripts/test_all.ps1

Para ejecutar la app:

    cd apps/qyro
    flutter run

En Windows, `flutter build` y `flutter run` con plugins necesitan el Modo
Desarrollador activado, porque Flutter usa enlaces simbólicos para el registrante
de plugins (`start ms-settings:developers`).

Lee AGENTS.md, STATUS.md, HANDOFF.md y NEXT_STEPS.md antes de modificar código.
