# Releases

Estado: builds debug comprobados; no hay release ni artefactos retenidos.

Run 30938946789 generó APK debug, qyro.exe debug y Runner.app iOS sin firma en runners efímeros. No son paquetes de distribución.

Tags vX.Y.Z deberán validar versión/changelog, probar, construir, generar checksums, SBOM, notices y build-info, y adjuntar artefactos solo con credenciales válidas.

- Android: APK/AAB según configuración comprobada.
- Windows: MSIX y ZIP portable x64.
- iOS: xcarchive cuando sea viable; IPA solo firmado.
- checksums: SHA256SUMS.txt; firma solo con clave autorizada.

Nunca almacenar certificados o secretos ni publicar automáticamente a tiendas.
