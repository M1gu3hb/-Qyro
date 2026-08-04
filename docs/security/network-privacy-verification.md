# Verificación de privacidad de red

Estado: procedimiento diseñado; no ejecutado porque no existe aplicación transferible.

## Preparación

1. Build reproducible sin analytics, fuentes o assets remotos.
2. Dispositivo/VM aislado con captura de interfaz, DNS y firewall.
3. Dos peers en LAN sin salida a internet y archivos de prueba no sensibles.
4. Limpiar caché DNS, historial y logs; registrar commit/hardware/SO.

## Casos

- arranque y navegación inactiva;
- discovery;
- pairing QR/IP;
- transferencia y reanudación;
- diagnóstico;
- modo óptico con radios de red deshabilitadas.

## Aserciones

- cero DNS externo;
- cero HTTP(S)/QUIC hacia internet;
- discovery limitado a enlaces/grupos locales documentados;
- datos solo entre IPs de peers;
- cero crash logs, telemetría, actualización, fuentes o assets remotos.

## Evidencia

Guardar pcap sanitizado, reglas, hash del build, comandos, duración y resumen reproducible en docs/security/evidence/. Si aparece tráfico externo, bloquear release, crear bug P0 y rastrear dependencia/callsite. Nunca subir contenido o claves.
