"""Repara UTF-8 que se leyo como Windows-1252 y se volvio a guardar.

D1 de `deuda-de-calidad.md`.

**Windows-1252 y no Latin-1, y esa es la parte que costo un intento.** A simple
vista el mojibake parece Latin-1, pero las secuencias traen caracteres como
U+201A y U+2013 que **no existen en Latin-1**: viven en el rango 0x80-0x9F, que
en Latin-1 son controles y en cp1252 son comillas y guiones. Una vuelta por
Latin-1 falla en esos bytes y deja el archivo igual, que es exactamente lo que
paso.

La vuelta es exacta: los bytes originales siguen ahi, solo se interpretaron mal,
asi que `encode('cp1252').decode('utf-8')` los devuelve tal cual. Cuando ese par
falla se deja lo que habia: adivinar seria peor que un mojibake visible.

Se aplica **hasta que deje de cambiar**, con tope, porque una secuencia puede
estar codificada dos veces y una sola vuelta la deja a medias.

    python tools/fix_mojibake.py            # todo el arbol de Rust
    python tools/fix_mojibake.py ruta.rs    # un archivo
"""

import glob
import io
import re
import sys

# Las marcas con que empieza este defecto, por punto de codigo para que este
# archivo no contenga el mojibake que repara.
#
# La tercera, U+00E2, se anadio despues de mirar: los dos primeros arreglan el
# simbolo de seccion, y el guion largo empieza por ella -- `E2 80 94` mal
# decodificado. Una lista de dos habria dejado el archivo a medias y con pinta
# de terminado.
MARKERS = (chr(0xC3), chr(0xC2), chr(0xE2))

# Todo lo no-ASCII que cp1252 sabe codificar. Construido en vez de escrito: una
# clase a mano se deja fuera justo el caracter que rompe el arreglo.
_ENCODABLE = []
for _point in range(0x80, 0x2100):
    _char = chr(_point)
    # Los del rango 0x80-0x9F entran aunque cp1252 no los defina: es justo
    # donde vive el byte que hacia fallar la vuelta.
    if 0x80 <= _point <= 0x9F:
        _ENCODABLE.append(_char)
        continue
    try:
        _char.encode("cp1252")
    except UnicodeEncodeError:
        continue
    _ENCODABLE.append(_char)

PATTERN = re.compile(
    "[" + re.escape("".join(MARKERS)) + "][" + re.escape("".join(_ENCODABLE)) + "]{1,4}"
)

MAX_ROUNDS = 4


def _to_bytes(chunk):
    """cp1252, con reserva a Latin-1 para los bytes que cp1252 no define.

    **La codificacion original era mixta y ese fue el segundo intento fallido.**
    La mayoria de los caracteres vienen de cp1252, pero aparece U+009D, que cp1252
    **no define**: ese byte se decodifico como Latin-1. Un `encode('cp1252')` a
    secas revienta ahi y deja el archivo a medias.

    Asi que se codifica caracter a caracter: cp1252 primero, y si no puede y el
    punto cabe en un byte, el byte crudo.
    """
    out = bytearray()
    for char in chunk:
        try:
            out.extend(char.encode("cp1252"))
        except UnicodeEncodeError:
            if ord(char) < 0x100:
                out.append(ord(char))
            else:
                raise
    return bytes(out)


def repair_once(text):
    def one(match):
        try:
            return _to_bytes(match.group(0)).decode("utf-8")
        except (UnicodeEncodeError, UnicodeDecodeError):
            return match.group(0)

    return PATTERN.sub(one, text)


def repair(text):
    for _ in range(MAX_ROUNDS):
        once = repair_once(text)
        if once == text:
            return text
        text = once
    return text


def marks(text):
    return sum(text.count(marker) for marker in MARKERS)


def main():
    targets = sys.argv[1:] or glob.glob("rust/crates/**/*.rs", recursive=True)
    total = 0
    stubborn = 0
    for path in targets:
        before = io.open(path, encoding="utf-8", errors="replace").read()
        if not marks(before):
            continue
        after = repair(before)
        if after == before:
            print("%s: %d marcas que esta vuelta no sabe reparar" % (path, marks(before)))
            stubborn += marks(before)
            continue
        io.open(path, "w", encoding="utf-8", newline="\n").write(after)
        fixed = marks(before) - marks(after)
        total += fixed
        stubborn += marks(after)
        print("%s: %d reparadas, %d restantes" % (path, fixed, marks(after)))
    print("total reparadas: %d, sin reparar: %d" % (total, stubborn))
    return 1 if stubborn else 0


sys.exit(main())
