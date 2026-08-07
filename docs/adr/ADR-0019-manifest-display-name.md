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

## Enmienda — sprint 4C.2 (QYR-0021): la regla Unicode de las rutas

**Corrige una afirmación anterior de este ADR.** El apartado «Cumplimiento» dice
«Un `.exe` no puede presentarse como `.pdf`». Era falso, y de la forma que peor
se detecta: la decisión de derivar el nombre visible de la ruta es correcta y
sigue en pie, pero no bastaba, porque la ruta misma podía mentir.

`RelativePath::parse` filtraba con `char::is_control()`, que cubre la categoría
general Unicode `Cc` y nada más. La categoría `Cf` —los caracteres de formato
invisibles y los controles bidireccionales— pasaba entera, se guardaba tal cual
y sobrevivía a `codec::encode`/`codec::decode` byte a byte.

`RelativePath::parse("invoice\u{202E}fdp.exe")` devolvía `Ok`. Todo renderizador
consciente de bidi muestra ese nombre como `invoiceexe.pdf`, incluidos los
selectores de archivo y las terminales donde un receptor confirma la
transferencia. `lib.rs` y `THREAT_MODEL.md` afirmaban lo mismo que este ADR.

### La regla

> `RelativePath::parse` rechaza todo carácter de la categoría general Unicode
> `Cf`, con `PathError::FormatCharacter`, además de los `Cc` que ya rechazaba.

La tabla está transcrita de `extracted/DerivedGeneralCategory.txt` de Unicode
16.0.0 (2024-04-30): veintiún rangos, 170 puntos de código, citados en el propio
código fuente y comprobados contra el archivo.

### Por qué la categoría entera, y no solo los overrides

Unicode UTR #36 §2.5.1 dice **«Never allow bidi override characters»** en
identificadores, y trata los `Cf` invisibles como un peligro de suplantación
visual en general. Un nombre de archivo es exactamente la cadena que una persona
lee antes de aceptar una transferencia, así que aquí la propiedad de seguridad
*es* la representación.

Rechazar solo `U+202D`/`U+202E` dejaría fuera `U+200B` entre el nombre y su
extensión, `U+FEFF` al principio de un nombre que así solo se distingue de otro
por bytes invisibles, y los aislantes `U+2066`–`U+2069`, que consiguen el mismo
efecto visual que un override por otro mecanismo.

### Decisión explícita sobre `U+200C` y `U+200D`

UTR #36 los exceptúa: la ortografía índica y persa los necesita junto a un
virama, y una regla que los elimine cambia la palabra.

**Se rechazan igualmente.** Es una decisión, no un descuido:

- Un nombre de archivo no es un identificador lingüístico. No se compara por
  reglas de escritura; se escribe en un sistema de archivos y se muestra en una
  lista.
- La postura declarada del crate es **rechazar en vez de sanear**, porque
  reescribir una ruta hostil suele producir otra ruta hostil.
- Aceptar un carácter que se renderiza como nada permitiría dos nombres
  visualmente idénticos en un mismo manifest.
- La asimetría del coste decide: un emisor que necesite uno recibe un error
  claro y renombra; un receptor que no puede distinguir dos entradas no tiene
  ese recurso.

### Coste aceptado

Una versión futura de Unicode puede añadir rangos a `Cf`. Un punto de código
añadido después de 16.0.0 se aceptaría hasta que la tabla se actualice. Es una
obsolescencia acotada y visible, y se prefiere a una dependencia nueva en la
ruta que analiza bytes de un peer antes de tocar el disco.

### Cumplimiento

`rust/crates/qyro_manifest/tests/unicode_path_contract.rs`, más dos pruebas
unitarias en `path.rs` que fijan que la tabla está ordenada, es disjunta y
responde en ambos extremos de cada rango: una búsqueda binaria sobre una tabla
mal formada falla en silencio.
