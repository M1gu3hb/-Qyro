# Estrategia de pruebas

El inventario ejecutado y sus runs viven en [STATUS.md](STATUS.md).

## Regla

Todo comportamiento comprobable sigue rojo → verde → refactor. El test debe fallar por la causa prevista antes de producción.

## Capas obligatorias del baseline

- Rust: formato, Clippy sin warnings y tests.
- Flutter: formato, análisis y tests.
- FFI: biblioteca real en la plataforma disponible.
- Scripts: contratos Bash y PowerShell.
- Documentación: consistencia con STATUS.md.
- Seguridad/licencias: checks ejecutables, no afirmaciones Markdown.
- Criptografía: KAT contra los RFC (7748, 8032, 4231) y vectores propios con
  schema estricto, regeneración byte a byte y verificación independiente desde
  las primitivas.
- Intro: unit/widget/golden según corresponda.

## Cómo se comprueba una invariante

Escribir la prueba no basta. **Borra la corrección y comprueba que alguna prueba
falla.** Si ninguna falla, la propiedad no estaba cubierta, por convincente que
sea el argumento a favor.

Ese método encontró que el enlace de la firma del iniciador sobre la del
respondedor no aportaba nada con Ed25519 determinista, que cinco de doce
codificaciones «de orden bajo» de X25519 no lo son, y que dos reglas nuevas del
verificador documental no comprobaban lo que decían comprobar.

Un verificador que ignora lo que no entiende es peor que ninguno: informa de
éxito sobre restricciones que nunca comprobó. El validador de schema de los
vectores falla ante cualquier palabra clave desconocida por esa razón.

## Honestidad de plataforma

- Un build no demuestra ejecución.
- Una biblioteca dentro de un APK no demuestra carga runtime.
- Un Runner.app sin firma no demuestra enlace FFI.
- Un test omitido no cuenta como éxito.
- N/A solo es válido para una plataforma no aplicable o funcionalidad futura registrada en STATUS.md.

## Rendimiento

No publicar cifras sin máquina, modo, versión, resolución y metodología. Los tests pueden detectar regresiones graves, no prometer FPS.

## Property tests y fuzzing

Las propiedades de `qyro_protocol` y `qyro_manifest` se comprueban con un
generador sembrado definido en los propios tests, no con `proptest`.

`proptest` se evaluó como pedía el sprint. Su licencia (MIT/Apache-2.0) es
aceptable, pero arrastra 39 paquetes transitivos a un workspace que hoy no tiene
ninguno, ampliando lo que `cargo audit` debe vigilar por una herramienta que solo
se usa en desarrollo. El intercambio queda explícito: se pierde el *shrinking*
automático y, a cambio, cualquier fallo se reproduce con la semilla impresa sin
mantener un archivo de regresiones.

Propiedades cubiertas (~30 000 casos generados por ejecución):

- decodificar lo codificado conserva el valor;
- el decoder incremental coincide con el completo en cualquier punto de corte;
- bytes arbitrarios nunca provocan pánico;
- todo lo que el decoder acepta respeta los límites declarados;
- un manifest válido nunca produce una ruta absoluta ni con travesía;
- el parser de rutas nunca reescribe su entrada.

El fuzzing real **no se ha ejecutado**. Hay targets `cargo-fuzz` y un corpus de
65 entradas que CI reproduce como smoke test. Detalles y comandos en
`docs/security/parser-threats.md`.

## Golden tests

No implementados todavía. La secuencia de arranque se cubre con contratos de
widget y de painter, que verifican comportamiento pero no apariencia.
