#!/usr/bin/env python3
"""Lo que de verdad va dentro de un APK, medido sobre el APK.

Este proyecto ya publicó una vez un artefacto que no era el que decía, así que
las tres cosas que deciden si Qyro arranca en un teléfono real se miden **sobre
el paquete que se instala** y no sobre lo que salió del compilador:

1. **Qué ABIs lleva.** Un APK con sólo `arm64-v8a` no instala en un teléfono de
   32 bits y no corre en el emulador de x86_64. El fallo llega como «la
   aplicación no está disponible para tu dispositivo» en la tienda, o como
   `UnsatisfiedLinkError` al arrancar si el APK se instaló a mano.

2. **La alineación de página de cada `.so`.** Android 15 corre con páginas de
   **16 KB** en aparatos nuevos, y una biblioteca cuyos segmentos `PT_LOAD`
   están alineados a `0x1000` **no carga**: `dlopen` falla y la aplicación
   muere al primer uso del motor. El NDK r28 lo pone bien por defecto; el r27 y
   anteriores necesitan `-Wl,-z,max-page-size=16384`. Se mide aquí, sobre el
   `.so` **extraído del APK**, porque entre el enlazador y el paquete hay un
   empaquetador que puede no conservar lo que el enlazador dejó.

3. **Si ese `.so` se carga desde dentro del APK, su posición en el zip.** Con
   `extractNativeLibs=false` —el valor por omisión desde AGP 4.2— el `.so` no se
   copia a disco: se mapea desde el propio APK. Para eso su entrada tiene que
   estar **sin comprimir** y empezar en un múltiplo del tamaño de página. Es lo
   que hace `zipalign -P 16`, y es una condición distinta de la (2): un `.so`
   perfectamente alineado por dentro no carga si el zip lo dejó en una posición
   impar.

Se escribe en Python y no en PowerShell ni en Bash a propósito: es **una sola**
implementación, corre igual en el runner de Linux y en la máquina Windows del
propietario, y este repositorio ya ejecuta Python en CI. Dos implementaciones de
la misma comprobación son dos comprobaciones el día que alguien toca una.

    python3 tools/apk_inspector/inspect_apk.py app-release.apk \\
        --require-abi arm64-v8a --require-abi armeabi-v7a --page-size 16384

Sin `--require-abi` no exige ninguna y sólo informa.
"""

from __future__ import annotations

import argparse
import struct
import sys
import zipfile
from dataclasses import dataclass, field

# El tamaño de página que Android 15 puede usar. Un `PT_LOAD` con menos no carga.
ANDROID_16K_PAGE = 16384

_PT_LOAD = 1
_ELF_MAGIC = b"\x7fELF"


class NotAnElf(Exception):
    """El archivo no empieza por `\\x7fELF`, o se acaba antes de su cabecera."""


@dataclass
class SharedObject:
    """Un `.so` dentro del APK, con lo que hace falta para juzgarlo."""

    path: str
    abi: str
    load_alignments: list[int]
    compressed: bool
    header_offset: int
    data_offset: int | None = None

    @property
    def worst_alignment(self) -> int:
        """La peor alineación de sus `PT_LOAD`; una sola mala tumba la carga."""
        return min(self.load_alignments) if self.load_alignments else 0


@dataclass
class Report:
    objects: list[SharedObject] = field(default_factory=list)
    problems: list[str] = field(default_factory=list)

    @property
    def abis(self) -> set[str]:
        return {obj.abi for obj in self.objects}


def load_alignments(blob: bytes) -> list[int]:
    """Los `p_align` de cada `PT_LOAD`, leídos de las cabeceras de programa.

    Se analiza el ELF a mano en vez de llamar a `readelf`: el propietario está en
    Windows y no tiene binutils, y una comprobación que sólo corre donde ya
    funcionaba todo no comprueba nada.

    Soporta ELF de 32 y de 64 bits, que son las dos que Android usa
    (`armeabi-v7a` y `x86` son de 32).
    """
    if len(blob) < 64 or blob[:4] != _ELF_MAGIC:
        raise NotAnElf("no empieza por \\x7fELF")

    is_64 = blob[4] == 2
    little = blob[5] == 1
    endian = "<" if little else ">"

    if is_64:
        # e_phoff en 0x20, e_phentsize en 0x36, e_phnum en 0x38.
        (phoff,) = struct.unpack_from(f"{endian}Q", blob, 0x20)
        phentsize, phnum = struct.unpack_from(f"{endian}HH", blob, 0x36)
    else:
        (phoff,) = struct.unpack_from(f"{endian}I", blob, 0x1C)
        phentsize, phnum = struct.unpack_from(f"{endian}HH", blob, 0x2A)

    alignments: list[int] = []
    for index in range(phnum):
        base = phoff + index * phentsize
        if base + phentsize > len(blob):
            raise NotAnElf("las cabeceras de programa se salen del archivo")
        (p_type,) = struct.unpack_from(f"{endian}I", blob, base)
        if p_type != _PT_LOAD:
            continue
        # p_align es el último campo: 0x30 en 64 bits, 0x1C en 32.
        if is_64:
            (align,) = struct.unpack_from(f"{endian}Q", blob, base + 0x30)
        else:
            (align,) = struct.unpack_from(f"{endian}I", blob, base + 0x1C)
        alignments.append(align)
    return alignments


def _data_offset(apk: zipfile.ZipFile, info: zipfile.ZipInfo) -> int | None:
    """Dónde empiezan de verdad los bytes de la entrada, no su cabecera local.

    `header_offset` apunta a la cabecera local, cuyo tamaño depende del nombre y
    del campo extra que `zipalign` usa precisamente para empujar los datos hasta
    el múltiplo que toca. Así que hay que leerla.
    """
    handle = apk.fp
    if handle is None:
        return None
    handle.seek(info.header_offset)
    local = handle.read(30)
    if len(local) < 30 or local[:4] != b"PK\x03\x04":
        return None
    name_len, extra_len = struct.unpack_from("<HH", local, 26)
    return info.header_offset + 30 + name_len + extra_len


def inspect(path: str, page_size: int = ANDROID_16K_PAGE) -> Report:
    report = Report()
    with zipfile.ZipFile(path) as apk:
        for info in apk.infolist():
            parts = info.filename.split("/")
            if len(parts) != 3 or parts[0] != "lib" or not parts[2].endswith(".so"):
                continue
            blob = apk.read(info.filename)
            try:
                alignments = load_alignments(blob)
            except NotAnElf as error:
                report.problems.append(f"{info.filename}: no es un ELF ({error})")
                continue
            if not alignments:
                report.problems.append(
                    f"{info.filename}: ningun segmento PT_LOAD, asi que no es una "
                    "biblioteca que Android pueda cargar"
                )
                continue
            obj = SharedObject(
                path=info.filename,
                abi=parts[1],
                load_alignments=alignments,
                compressed=info.compress_type != zipfile.ZIP_STORED,
                header_offset=info.header_offset,
                data_offset=_data_offset(apk, info),
            )
            report.objects.append(obj)

            if obj.worst_alignment < page_size:
                report.problems.append(
                    f"{obj.path}: PT_LOAD alineado a 0x{obj.worst_alignment:x}, y "
                    f"Android 15 con paginas de {page_size // 1024} KB necesita "
                    f"0x{page_size:x}. dlopen fallaria y la aplicacion moriria al "
                    "primer uso del motor. Construye con NDK r28 o anade "
                    "-Wl,-z,max-page-size=16384"
                )
            # Sólo si va sin comprimir: comprimido significa que Android lo
            # extrae al instalar, y entonces su posición en el zip da igual.
            if not obj.compressed and obj.data_offset is not None:
                if obj.data_offset % page_size != 0:
                    report.problems.append(
                        f"{obj.path}: va sin comprimir (se mapea desde el APK) y "
                        f"empieza en el byte {obj.data_offset}, que no es multiplo "
                        f"de {page_size}. Falta `zipalign -P "
                        f"{page_size // 1024} -f`"
                    )
    return report


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("apk")
    parser.add_argument(
        "--require-abi",
        action="append",
        default=[],
        metavar="ABI",
        help="una ABI que el APK tiene que llevar; repetible",
    )
    parser.add_argument(
        "--page-size",
        type=int,
        default=ANDROID_16K_PAGE,
        help=f"tamano de pagina exigido (por omision {ANDROID_16K_PAGE})",
    )
    args = parser.parse_args(argv)

    report = inspect(args.apk, args.page_size)

    if not report.objects:
        print(f"[BLOCKER] {args.apk} no lleva ninguna biblioteca nativa en lib/")
        return 1

    print(f"=== {args.apk} ===")
    for obj in sorted(report.objects, key=lambda o: o.path):
        packing = "comprimido" if obj.compressed else "sin comprimir"
        print(
            f"  {obj.path}  PT_LOAD align=0x{obj.worst_alignment:x}  {packing}"
            + (f"  datos en {obj.data_offset}" if obj.data_offset is not None else "")
        )

    missing = [abi for abi in args.require_abi if abi not in report.abis]
    if missing:
        report.problems.append(
            f"faltan las ABIs {missing}; el APK lleva {sorted(report.abis)}. "
            "Un telefono de esa arquitectura no puede instalarlo, o arranca y "
            "muere con UnsatisfiedLinkError"
        )

    if report.problems:
        print()
        for problem in report.problems:
            print(f"[BLOCKER] {problem}")
        return 1

    print(f"\n[OK] ABIs {sorted(report.abis)}, todo PT_LOAD >= 0x{args.page_size:x}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
