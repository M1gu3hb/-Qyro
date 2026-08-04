# Bugs y pendientes verificados

## QYR-0001 — Falta referencia visual de scramble

- Plataforma: todas
- Severidad: P2
- Reproducción: revisar design/reference/
- Esperado: scramble-decode-reference.jpg del propietario
- Actual: solo existe documentación del pendiente
- Evidencia: activo no suministrado el 2026-08-04
- Posible causa: entrega pendiente
- Workaround: tests deterministas sin golden visual
- Estado: abierto
- Dueño: propietario
- Fecha: 2026-08-04

## QYR-0002 — Runners Flutter no generados

- Plataforma: Android, iOS, Windows
- Severidad: P0
- Reproducción: intentar flutter build
- Esperado: proyectos host válidos
- Actual: no existen runners
- Evidencia: árbol del repositorio
- Posible causa: repositorio iniciado vacío
- Workaround: ninguno para builds
- Estado: abierto
- Dueño: ingeniería
- Fecha: 2026-08-04

## QYR-0003 — Aviso de actions/checkout v4

- Plataforma: CI
- Severidad: P3
- Reproducción: ejecutar GitHub Actions run 30937447915
- Esperado: cero avisos
- Actual: GitHub fuerza Node 24 porque la action declara Node 20
- Evidencia: anotación de ambos jobs
- Posible causa: versión de action
- Workaround: ninguno necesario; el job pasa
- Estado: abierto, revisar checkout v5
- Dueño: release
- Fecha: 2026-08-04
