# ADR-0019: Nombre visible derivado de la ruta

- Estado: aceptada
- Fecha: 2026-08-05
- Implementa: `rust/crates/qyro_manifest`
- Sube `MANIFEST_VERSION` de 1 a 2

## Contexto

`ManifestItem` llevaba un `display_name` propio en el wire format, separado de la
ruta. Los constructores `file()` y `directory()` lo derivaban del último segmento,
pero `ManifestItem::new` aceptaba cualquier cadena, y el decoder también.

Eso permite un ataque directo: un peer envía un item cuya ruta es
`factura.pdf.exe` y cuyo `display_name` es `factura.pdf`. La interfaz muestra el
nombre inocuo, el receptor acepta, y en disco aparece un ejecutable. El manifest
sería técnicamente válido y todas las reglas de ruta se cumplirían.

El campo tampoco aportaba nada: no existe ningún caso en el que el nombre visible
deba diferir de dónde van a caer los bytes.

## Decisión

**Se elimina `display_name` del formato.** El nombre visible se deriva siempre de
`RelativePath::file_name()`.

Consecuencias directas:

- No hay un segundo nombre que pueda discrepar del primero, así que la clase de
  ataque desaparece por construcción en vez de por validación.
- El manifest serializado encoge: un campo con prefijo de longitud menos por item.
- La interfaz puede seguir presentando una forma segura derivada (truncada,
  con marcas de dirección neutralizadas), pero siempre a partir de la ruta
  validada, nunca de algo que el peer eligió por separado.

`MANIFEST_VERSION` pasa a **2**. El formato nunca se publicó y no hay peers
desplegados, así que no se conserva un decoder de la versión 1: mantener dos
representaciones de lo mismo solo añadiría superficie ambigua. Un manifest que
declare la versión 1 se rechaza con `UnsupportedVersion`.

## Alternativas descartadas

- **Validar que `display_name == file_name()` al decodificar.** Funcionaría, pero
  deja el campo en el wire, gasta bytes, y obliga a que cada implementación
  futura recuerde la comprobación. Un campo que solo puede tener un valor no
  debería viajar.
- **Permitir un nombre visible distinto pero marcarlo en la interfaz.** Traslada
  al usuario una decisión de seguridad que el formato puede evitarle.

## Cumplimiento

Pruebas que fijan la decisión:

- El nombre visible siempre coincide con el último segmento de la ruta.
- No existe constructor público que produzca discrepancia.
- Un `.exe` no puede presentarse como `.pdf`.
- Round-trip canónico con la versión 2.
- Un manifest de versión 1 se rechaza.
