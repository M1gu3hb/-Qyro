# Próximos pasos

## P0

1. Generar runners Flutter Android, iOS y Windows.
   - Aceptación: Android debug y Windows debug compilan; iOS compila sin firma en macOS o queda bloqueo reproducible.
2. Conectar Dart con qyro_ffi.
   - Aceptación: test de integración lee QYRO/1 desde la biblioteca real en al menos Windows y Android.
3. Implementar doctor/bootstrap/test_all en Bash y PowerShell con tests.
   - Aceptación: salidas OK/advertencia/bloqueo/no aplica y códigos de salida comprobados.
4. Crear modelo QYRO/1 y validación de manifest mediante TDD.
   - Aceptación: round-trip, límites, Unicode y path traversal comprobados.

## P1

- Pantalla boot y Home con Enviar/Recibir, skip y reduced motion.
- Configuración de branding y aviso cuando permanece com.owner.qyro.
- CI por Windows/macOS/Android.
- Lockfiles y auditoría completa de dependencias.

## P2

- Generador logo→ASCII y golden tests.
- Persistencia SQLite y migración 0001.
- LAN e IP manual.

## P3

- RaptorQ/QR adaptativo.
- Wi-Fi Direct, Multipeer y Bluetooth experimental.
