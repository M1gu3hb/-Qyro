# Aquí van los dos archivos de la prueba

**Esta carpeta está vacía a propósito, y estarlo es la verdad.**

Los dos artefactos —`qyro.exe` y `app-release.apk`— **no se pueden construir en
la máquina donde se preparó esta tanda**: es un contenedor de Linux sin Flutter,
sin SDK de Android, sin PowerShell y sin el objetivo `x86_64-pc-windows-msvc`.
Construirlos en otra parte y decir que son éstos sería exactamente lo que este
proyecto ya hizo una vez y le costó una retractación pública (QYR-0359).

**Los construye el propietario**, con los comandos exactos de
[`docs/GUIA-DE-PRUEBA.md`](../../docs/GUIA-DE-PRUEBA.md) §2, y quedan aquí:

```
release/prueba-en-hardware/
  qyro.exe                 <- el binario de terminal para Windows
  app-release.apk          <- la aplicación para el teléfono
  SHA256SUMS.txt           <- los dos hashes
  BUILD-INFO.txt           <- de qué commit salieron
```

`SHA256SUMS.txt` y `BUILD-INFO.txt` se generan con los comandos de la guía §2.4.
Los dos archivos binarios **no se commitean**: `release/windows/*` y
`release/android/*` ya están en `.gitignore` por la misma razón, y un binario en
el árbol se desincroniza del código en el primer commit que no lo regenere.

Ver también `release/README.md`.
