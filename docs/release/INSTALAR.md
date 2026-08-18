# Instalar Qyro

**No se instala.** Es un archivo. Descárgalo, cópialo donde quieras, y ejecútalo.

1. Descarga `qyro.exe` de la Release.
2. **Cópialo a un USB con formato FAT32 o exFAT.** Windows deja de avisar — la
   marca de «esto vino de internet» vive en un flujo de NTFS, y esos formatos no
   tienen dónde guardarla.
3. En la otra máquina, ábrelo desde el USB o cópialo al escritorio.
4. Doble clic, o `qyro` en una terminal.
5. Comprueba que es el que publicamos:
   `(Get-FileHash qyro.exe -Algorithm SHA256).Hash` — tiene que coincidir con el
   `sha256` del `BUILD-INFO.txt` que viene al lado.

---

**Si ya tienes un Qyro corriendo en otra máquina**, no hace falta el USB:

```
qyro send --self --to <el código de la máquina nueva>
```

Qyro se lleva a sí mismo. Son unos 800 KB — por un cable serie, alrededor de
ochenta segundos.

---

**Este binario no está firmado.** `docs/release/DECISION-DE-FIRMA.md` explica por
qué y qué costaría cambiarlo. Windows enseñará una advertencia si lo descargaste
por internet y lo ejecutas sin pasar por el USB; el paso 2 la evita.

**No escribe fuera de su carpeta.** La identidad del aparato queda en un archivo
junto al ejecutable, así que borrar la carpeta lo borra entero.
