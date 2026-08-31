# Releases

**Estado: v1.0.0 está etiquetada y publicada**, con sus artefactos y sus
SHA-256 en `docs/release/v1.0.md`. Lo que ese documento describe es **esa
versión**, y varias de sus frases ya no son ciertas de `main`: lo dice su propia
cabecera.

CI sigue construyendo APK debug, `qyro.exe` debug y `Runner.app` de iOS sin
firmar en runners efímeros. Eso **no** son paquetes de distribución y nunca lo
fueron.

> **La frase que estuvo aquí hasta 2026-08-31 decía «no hay release ni
> artefactos retenidos», con v1.0.0 etiquetada y publicada.** Se deja escrita
> para que no vuelva (QYR-0395).

Tags vX.Y.Z deberán validar versión/changelog, probar, construir, generar checksums, SBOM, notices y build-info, y adjuntar artefactos solo con credenciales válidas.

- Android: APK/AAB según configuración comprobada.
- Windows: MSIX y ZIP portable x64.
- iOS: **fuera de la v1.0** (ADR-0039). CI sigue construyendo `Runner.app` sin
  firmar, que es la prueba de que la puerta sigue abierta; xcarchive e IPA
  firmado esperan a que exista un Mac y una cuenta de desarrollador.
- checksums: SHA256SUMS.txt; firma solo con clave autorizada.

Nunca almacenar certificados o secretos ni publicar automáticamente a tiendas.
