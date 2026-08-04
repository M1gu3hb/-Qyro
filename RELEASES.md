# Releases

Estado: no hay artefactos ni release.

Tags vX.Y.Z deberán validar versión/changelog, ejecutar pruebas, construir plataformas disponibles, generar checksums, SBOM, notices y build-info, y adjuntar artefactos solo con credenciales válidas.

## Objetivos

- Android: APK/AAB según configuración comprobada.
- Windows: MSIX y ZIP portable x64.
- iOS: xcarchive cuando sea viable; IPA solo con firma/provisioning válidos.
- checksums: SHA256SUMS.txt y firma solo si existe clave autorizada.

Nunca almacenar certificados o secretos. No publicar automáticamente a tiendas. Cada build-info registra commit, herramientas, plataforma, fecha UTC y resultados.
