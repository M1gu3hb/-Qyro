"""Que el inspector sepa fallar, probado con APK sintéticos.

**Una comprobación que nunca ha estado roja no es una comprobación.** Este
repositorio ya publicó un artefacto que no era el que decía, y la lección fue que
una medida sin control es una opinión con formato de dato. Así que cada
afirmación de `inspect_apk` se prueba en los dos sentidos: un APK que la cumple y
uno que no.

Los `.so` se **fabrican aquí en Python**, byte a byte, en vez de guardarse como
ficheros de prueba. Dos razones y la segunda es la que decide: un binario en el
árbol se desincroniza del código a la primera, y —lo que importa— un ELF
fabricado deja **elegir** el `p_align`, que es justo la propiedad bajo prueba.
Con un `.so` real habría que tener un NDK para producir el caso malo.

    python3 -m unittest tools/apk_inspector/test_inspect_apk.py
"""

from __future__ import annotations

import os
import struct
import sys
import tempfile
import unittest
import zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from inspect_apk import NotAnElf, inspect, load_alignments  # noqa: E402


def elf64(alignments: list[int]) -> bytes:
    """Un ELF de 64 bits con un `PT_LOAD` por alineación pedida.

    Sólo lo que `load_alignments` lee: la cabecera y las de programa. No es una
    biblioteca cargable y no tiene que serlo — lo que se prueba es el lector.
    """
    phentsize = 56
    phoff = 64
    header = bytearray(64)
    header[0:4] = b"\x7fELF"
    header[4] = 2  # 64 bits
    header[5] = 1  # little endian
    header[6] = 1  # EI_VERSION
    struct.pack_into("<H", header, 16, 3)  # e_type = ET_DYN
    struct.pack_into("<Q", header, 0x20, phoff)
    struct.pack_into("<H", header, 0x36, phentsize)
    struct.pack_into("<H", header, 0x38, len(alignments))

    body = bytearray()
    for align in alignments:
        entry = bytearray(phentsize)
        struct.pack_into("<I", entry, 0, 1)  # p_type = PT_LOAD
        struct.pack_into("<Q", entry, 0x30, align)  # p_align
        body += entry
    return bytes(header) + bytes(body)


def elf32(alignments: list[int]) -> bytes:
    """El mismo, de 32 bits: `armeabi-v7a` y `x86` lo son, y sus campos difieren."""
    phentsize = 32
    phoff = 52
    header = bytearray(52)
    header[0:4] = b"\x7fELF"
    header[4] = 1  # 32 bits
    header[5] = 1
    header[6] = 1
    struct.pack_into("<H", header, 16, 3)
    struct.pack_into("<I", header, 0x1C, phoff)
    struct.pack_into("<H", header, 0x2A, phentsize)
    struct.pack_into("<H", header, 0x2C, len(alignments))

    body = bytearray()
    for align in alignments:
        entry = bytearray(phentsize)
        struct.pack_into("<I", entry, 0, 1)
        struct.pack_into("<I", entry, 0x1C, align)
        body += entry
    return bytes(header) + bytes(body)


def apk(tmp: str, name: str, libraries: dict[str, bytes], *, stored=False, pad=0) -> str:
    path = os.path.join(tmp, name)
    with zipfile.ZipFile(path, "w") as archive:
        if pad:
            archive.writestr("res/padding.bin", b"x" * pad)
        for arc, blob in libraries.items():
            archive.writestr(
                arc,
                blob,
                compress_type=zipfile.ZIP_STORED if stored else zipfile.ZIP_DEFLATED,
            )
    return path


class AlignmentReader(unittest.TestCase):
    def test_reads_a_64_bit_load_alignment(self):
        self.assertEqual(load_alignments(elf64([0x4000, 0x4000])), [0x4000, 0x4000])

    def test_reads_a_32_bit_load_alignment(self):
        # `armeabi-v7a` es de 32 bits, y sus cabeceras están en otros desplazamientos.
        # Leerlas con la forma de 64 daría basura que pasaría por una alineación.
        self.assertEqual(load_alignments(elf32([0x4000])), [0x4000])

    def test_refuses_something_that_is_not_an_elf(self):
        # El control del lector: si aceptara cualquier cosa, un `.so` corrupto
        # saldría con alineación 0 o con la del archivo de al lado.
        with self.assertRaises(NotAnElf):
            load_alignments(b"PK\x03\x04 esto es un zip, no un ELF")
        with self.assertRaises(NotAnElf):
            load_alignments(b"")


class SixteenKilobytePages(unittest.TestCase):
    """Android 15 corre con páginas de 16 KB, y un `.so` a 4 KB no carga."""

    def test_a_library_aligned_to_16k_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = apk(tmp, "ok.apk", {"lib/arm64-v8a/libqyro_ffi.so": elf64([0x4000])})
            report = inspect(path)
            self.assertEqual(report.problems, [])
            self.assertEqual(report.abis, {"arm64-v8a"})

    def test_a_library_aligned_to_4k_is_refused_and_says_why(self):
        # El caso que Android 15 rechaza en `dlopen`, con la aplicación ya
        # instalada y arrancando: el motor no carga y la pantalla muere al
        # primer uso. Es el defecto que sólo se ve en el aparato, y por eso se
        # mide en el paquete.
        with tempfile.TemporaryDirectory() as tmp:
            path = apk(tmp, "old.apk", {"lib/arm64-v8a/libqyro_ffi.so": elf64([0x1000])})
            report = inspect(path)
            self.assertEqual(len(report.problems), 1)
            self.assertIn("0x1000", report.problems[0])
            self.assertIn("max-page-size=16384", report.problems[0])

    def test_one_bad_segment_among_good_ones_is_enough(self):
        # `dlopen` mapea todos los `PT_LOAD`. Mirar sólo el primero, o la media,
        # dejaría pasar el caso real: un enlazador que alineó el texto y no los
        # datos.
        with tempfile.TemporaryDirectory() as tmp:
            path = apk(
                tmp,
                "mixed.apk",
                {"lib/arm64-v8a/libqyro_ffi.so": elf64([0x4000, 0x4000, 0x1000])},
            )
            self.assertEqual(len(inspect(path).problems), 1)


class WhichAbisAreInside(unittest.TestCase):
    def test_it_names_every_abi_it_found(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = apk(
                tmp,
                "two.apk",
                {
                    "lib/arm64-v8a/libqyro_ffi.so": elf64([0x4000]),
                    "lib/armeabi-v7a/libqyro_ffi.so": elf32([0x4000]),
                },
            )
            self.assertEqual(inspect(path).abis, {"arm64-v8a", "armeabi-v7a"})

    def test_an_apk_with_no_native_library_reports_none(self):
        # El control. Un APK sin `lib/` es un APK sin motor, y un informe vacío
        # que dijera «todo bien» sería el peor resultado posible.
        with tempfile.TemporaryDirectory() as tmp:
            path = apk(tmp, "empty.apk", {})
            report = inspect(path)
            self.assertEqual(report.objects, [])
            self.assertEqual(report.abis, set())

    def test_a_file_under_lib_that_is_not_an_elf_is_a_problem(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = apk(tmp, "junk.apk", {"lib/arm64-v8a/libqyro_ffi.so": b"not an elf"})
            self.assertIn("no es un ELF", inspect(path).problems[0])


class PositionInsideTheZip(unittest.TestCase):
    """Con `extractNativeLibs=false` el `.so` se mapea desde el propio APK."""

    def test_a_stored_library_at_an_odd_offset_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = apk(
                tmp,
                "unaligned.apk",
                {"lib/arm64-v8a/libqyro_ffi.so": elf64([0x4000])},
                stored=True,
                pad=7,
            )
            problems = inspect(path).problems
            self.assertEqual(len(problems), 1)
            self.assertIn("zipalign", problems[0])

    def test_a_compressed_library_is_not_judged_on_its_offset(self):
        # Y el control que impide que la comprobación de arriba sea un falso
        # positivo permanente: comprimido significa que Android lo extrae al
        # instalar, y entonces dónde estaba dentro del zip da igual. Un APK de
        # Flutter sin `useLegacyPackaging=false` es exactamente este caso.
        with tempfile.TemporaryDirectory() as tmp:
            path = apk(
                tmp,
                "compressed.apk",
                {"lib/arm64-v8a/libqyro_ffi.so": elf64([0x4000])},
                stored=False,
                pad=7,
            )
            self.assertEqual(inspect(path).problems, [])


if __name__ == "__main__":
    unittest.main()
