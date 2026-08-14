# Releases

Estado: builds debug comprobados; no hay release ni artefactos retenidos.

Run 30938946789 generó APK debug, qyro.exe debug y Runner.app iOS sin firma en runners efímeros. No son paquetes de distribución.

Tags vX.Y.Z deberán validar versión/changelog, probar, construir, generar checksums, SBOM, notices y build-info, y adjuntar artefactos solo con credenciales válidas.

- Android: APK/AAB según configuración comprobada.
- Windows: MSIX y ZIP portable x64.
- iOS: **fuera de la v1.0** (ADR-0039). CI sigue construyendo `Runner.app` sin
  firmar, que es la prueba de que la puerta sigue abierta; xcarchive e IPA
  firmado esperan a que exista un Mac y una cuenta de desarrollador.
- checksums: SHA256SUMS.txt; firma solo con clave autorizada.

Nunca almacenar certificados o secretos ni publicar automáticamente a tiendas.
