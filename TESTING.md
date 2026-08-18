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
- Criptografía: KAT contra los RFC (7748, 8032, 4231, 8439) y vectores propios
  con schema estricto, regeneración byte a byte y verificación independiente
  desde las primitivas. Los vectores del AEAD están encadenados a los del
  handshake, y una prueba comprueba el encadenamiento campo a campo en lugar de
  afirmarlo en prosa.
- Criptografía por plataforma: `qyro_crypto` se compila para Android x86_64 y
  arm64, iOS device y simulator, Windows x64 y Linux, y el harness de
  `rust/tools/qyro_crypto_smoke` lo **ejecuta** en cuatro de esas seis. Ver
  `docs/testing/crypto-platform-matrix.md`, que distingue compilar de ejecutar
  fila por fila porque no son lo mismo.
- Fuzzing acotado: seis targets, semanal y bajo demanda. Ver
  `docs/testing/crypto-fuzzing.md`, que empieza por lo que **no** demuestra.
- Intro: unit/widget/golden según corresponda.

## Cómo se comprueba una invariante

Escribir la prueba no basta. **Borra la corrección y comprueba que alguna prueba
falla.** Si ninguna falla, la propiedad no estaba cubierta, por convincente que
sea el argumento a favor.

Ese método encontró que el enlace de la firma del iniciador sobre la del
respondedor no aportaba nada con Ed25519 determinista, que cinco de doce
codificaciones «de orden bajo» de X25519 no lo son, que dos reglas nuevas del
verificador documental no comprobaban lo que decían comprobar, y —en el sprint
4C— que quitar la dirección de la etiqueta de derivación del AEAD no rompía
ninguna de las treinta y tres pruebas, porque la propiedad estaba apoyada una
capa más arriba.

## Ejecutar no es compilar

El sprint 4C.1 existe porque cuatro workflows en verde no decían nada sobre
`qyro_crypto` fuera de x86_64 Linux. Todos compilaban y ejecutaban `qyro_ffi`,
que deliberadamente no puede alcanzar `qyro_crypto`.

La regla que queda: **el nombre del paquete y el target se comprueban juntos.**
`--package qyro_ffi --target aarch64-linux-android` no es evidencia sobre
`qyro_crypto`, y `scripts/check_crypto_platform_evidence.{sh,ps1}` rechaza esa
sustitución con un contrato que la hace a propósito para ver si el checker la
detecta.

La segunda regla: **un emulador y un simulador no son hardware.** Se declaran
como lo que son.

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

Hasta el sprint 4C.1 esta sección decía «el fuzzing real **no se ha
ejecutado**», y era peor que eso: los targets ni siquiera compilaban, porque
`rust/fuzz` no declaraba `[workspace]` y CI solo les pasaba `rustfmt --check`.
Ahora los seis targets se construyen y `.github/workflows/crypto-fuzz.yml` los
ejecuta, con un presupuesto acotado y sus estadísticas finales en el log. El
corpus de 94 entradas se sigue reproduciendo como smoke test en cada commit,
que es lo que corre en stable. Detalles, comandos y —sobre todo— lo que la
campaña **no** demuestra, en `docs/security/parser-threats.md` y
`docs/testing/crypto-fuzzing.md`.

## Golden tests

No implementados todavía. La secuencia de arranque se cubre con contratos de
widget y de painter, que verifican comportamiento pero no apariencia.
