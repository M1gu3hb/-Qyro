#!/usr/bin/env python3
"""Deterministically converts the provisional Qyro PNG into ASCII assets."""

from __future__ import annotations

import argparse
import binascii
import hashlib
import json
import math
import struct
import zlib
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence

GENERATOR_VERSION = "1.0.0"
DEFAULT_WIDTH = 48
DEFAULT_THRESHOLD = 0.18
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
GLYPHS = " .:-=+*#%@"


@dataclass(frozen=True)
class GeneratedAssets:
    json_text: str
    text: str
    preview_png: bytes


def _chunk(kind: bytes, payload: bytes) -> bytes:
    body = kind + payload
    crc = binascii.crc32(body) & 0xFFFFFFFF
    return struct.pack(">I", len(payload)) + body + struct.pack(">I", crc)


def encode_rgba_png(
    width: int,
    height: int,
    pixels: Sequence[tuple[int, int, int, int]],
) -> bytes:
    if width <= 0 or height <= 0 or len(pixels) != width * height:
        raise ValueError("RGBA dimensions do not match the pixel buffer")
    rows = bytearray()
    for row in range(height):
        rows.append(0)
        start = row * width
        for pixel in pixels[start : start + width]:
            if any(channel < 0 or channel > 255 for channel in pixel):
                raise ValueError("RGBA channels must be in the range 0..255")
            rows.extend(pixel)
    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return (
        PNG_SIGNATURE
        + _chunk(b"IHDR", header)
        + _chunk(b"IDAT", zlib.compress(bytes(rows), level=9))
        + _chunk(b"IEND", b"")
    )


def _paeth(left: int, above: int, upper_left: int) -> int:
    estimate = left + above - upper_left
    left_distance = abs(estimate - left)
    above_distance = abs(estimate - above)
    diagonal_distance = abs(estimate - upper_left)
    if left_distance <= above_distance and left_distance <= diagonal_distance:
        return left
    if above_distance <= diagonal_distance:
        return above
    return upper_left


def _decode_rgba_png(data: bytes) -> tuple[int, int, list[tuple[int, int, int, int]]]:
    if not data.startswith(PNG_SIGNATURE):
        raise ValueError("Source is not a PNG file")

    offset = len(PNG_SIGNATURE)
    width = height = 0
    bit_depth = color_type = interlace = -1
    compressed = bytearray()

    while offset < len(data):
        if offset + 12 > len(data):
            raise ValueError("PNG chunk is truncated")
        length = struct.unpack(">I", data[offset : offset + 4])[0]
        kind = data[offset + 4 : offset + 8]
        payload_start = offset + 8
        payload_end = payload_start + length
        payload = data[payload_start:payload_end]
        expected_crc = struct.unpack(">I", data[payload_end : payload_end + 4])[0]
        actual_crc = binascii.crc32(kind + payload) & 0xFFFFFFFF
        if actual_crc != expected_crc:
            raise ValueError(f"PNG chunk {kind!r} has an invalid CRC")
        offset = payload_end + 4

        if kind == b"IHDR":
            (
                width,
                height,
                bit_depth,
                color_type,
                compression,
                filtering,
                interlace,
            ) = struct.unpack(">IIBBBBB", payload)
            if compression != 0 or filtering != 0:
                raise ValueError("Unsupported PNG compression or filter method")
        elif kind == b"IDAT":
            compressed.extend(payload)
        elif kind == b"IEND":
            break

    channels_by_type = {0: 1, 2: 3, 4: 2, 6: 4}
    channels = channels_by_type.get(color_type)
    if width <= 0 or height <= 0 or bit_depth != 8 or channels is None:
        raise ValueError("Only 8-bit grayscale, RGB, GA, and RGBA PNGs are supported")
    if interlace != 0:
        raise ValueError("Interlaced PNGs are not supported")

    raw = zlib.decompress(bytes(compressed))
    stride = width * channels
    expected_size = height * (stride + 1)
    if len(raw) != expected_size:
        raise ValueError("PNG decompressed size is invalid")

    rows: list[bytearray] = []
    cursor = 0
    previous = bytearray(stride)
    for _ in range(height):
        filter_type = raw[cursor]
        cursor += 1
        encoded = raw[cursor : cursor + stride]
        cursor += stride
        row = bytearray(stride)
        for index, value in enumerate(encoded):
            left = row[index - channels] if index >= channels else 0
            above = previous[index]
            upper_left = previous[index - channels] if index >= channels else 0
            if filter_type == 0:
                restored = value
            elif filter_type == 1:
                restored = value + left
            elif filter_type == 2:
                restored = value + above
            elif filter_type == 3:
                restored = value + ((left + above) // 2)
            elif filter_type == 4:
                restored = value + _paeth(left, above, upper_left)
            else:
                raise ValueError(f"Unsupported PNG filter type {filter_type}")
            row[index] = restored & 0xFF
        rows.append(row)
        previous = row

    pixels: list[tuple[int, int, int, int]] = []
    for row in rows:
        for column in range(width):
            start = column * channels
            if color_type == 0:
                gray = row[start]
                pixels.append((gray, gray, gray, 255))
            elif color_type == 2:
                pixels.append((row[start], row[start + 1], row[start + 2], 255))
            elif color_type == 4:
                gray, alpha = row[start : start + 2]
                pixels.append((gray, gray, gray, alpha))
            else:
                pixels.append(tuple(row[start : start + 4]))
    return width, height, pixels


def _cell_bounds(index: int, count: int, source_size: int) -> tuple[int, int]:
    start = math.floor(index * source_size / count)
    end = math.ceil((index + 1) * source_size / count)
    return start, max(start + 1, end)


def _preview_png(
    width: int,
    height: int,
    densities: Sequence[Sequence[float]],
    mask: Sequence[str],
) -> bytes:
    cell_width = 8
    cell_height = 12
    preview_width = width * cell_width
    preview_height = height * cell_height
    pixels: list[tuple[int, int, int, int]] = []
    for y in range(preview_height):
        row = y // cell_height
        for x in range(preview_width):
            column = x // cell_width
            if mask[row][column] == "0":
                pixels.append((3, 7, 13, 255))
                continue
            intensity = densities[row][column]
            blue = min(255, round(139 + 116 * intensity))
            green = min(255, round(120 + 100 * intensity))
            inset = (x % cell_width in (0, cell_width - 1)) or (
                y % cell_height in (0, cell_height - 1)
            )
            alpha_scale = 0.38 if inset else 0.9
            pixels.append(
                (
                    round(22 * alpha_scale),
                    round(green * alpha_scale),
                    round(blue * alpha_scale),
                    255,
                )
            )
    return encode_rgba_png(preview_width, preview_height, pixels)


def generate_assets(
    source: bytes,
    *,
    source_name: str,
    target_width: int = DEFAULT_WIDTH,
    threshold: float = DEFAULT_THRESHOLD,
) -> GeneratedAssets:
    if target_width < 2 or target_width > 160:
        raise ValueError("target_width must be between 2 and 160")
    if threshold <= 0 or threshold >= 1:
        raise ValueError("threshold must be between 0 and 1")

    source_width, source_height, pixels = _decode_rgba_png(source)
    target_height = max(
        1,
        round(source_height / source_width * target_width * 0.5),
    )
    character_rows: list[str] = []
    mask_rows: list[str] = []
    density_rows: list[list[float]] = []

    for target_y in range(target_height):
        y_start, y_end = _cell_bounds(target_y, target_height, source_height)
        characters: list[str] = []
        masks: list[str] = []
        densities: list[float] = []
        for target_x in range(target_width):
            x_start, x_end = _cell_bounds(target_x, target_width, source_width)
            alpha_sum = 0
            samples = 0
            for source_y in range(y_start, y_end):
                row_start = source_y * source_width
                for source_x in range(x_start, x_end):
                    alpha_sum += pixels[row_start + source_x][3]
                    samples += 1
            density = alpha_sum / (samples * 255)
            rounded_density = round(density, 3)
            visible = density >= threshold
            glyph_index = min(
                len(GLYPHS) - 1,
                max(0, round(density * (len(GLYPHS) - 1))),
            )
            characters.append(GLYPHS[glyph_index] if visible else " ")
            masks.append("1" if visible else "0")
            densities.append(rounded_density)
        character_rows.append("".join(characters))
        mask_rows.append("".join(masks))
        density_rows.append(densities)

    model = {
        "width": target_width,
        "height": target_height,
        "aspectRatio": round(source_width / source_height, 6),
        "characterCells": character_rows,
        "mask": mask_rows,
        "density": density_rows,
        "threshold": threshold,
        "source": source_name,
        "sourceChecksum": "sha256:" + hashlib.sha256(source).hexdigest(),
        "generatorVersion": GENERATOR_VERSION,
        "provisional": True,
    }
    json_text = json.dumps(
        model,
        ensure_ascii=False,
        indent=2,
        separators=(",", ": "),
    ) + "\n"
    text = "\n".join(character_rows) + "\n"
    preview = _preview_png(target_width, target_height, density_rows, mask_rows)
    return GeneratedAssets(json_text=json_text, text=text, preview_png=preview)


def _matches(path: Path, expected: bytes) -> bool:
    return path.is_file() and path.read_bytes() == expected


def _parse_arguments() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--source",
        type=Path,
        default=root / "design/brand/source/qyro-logo.png",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=root / "apps/qyro/assets/generated",
    )
    parser.add_argument("--width", type=int, default=DEFAULT_WIDTH)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def main() -> int:
    arguments = _parse_arguments()
    source_name = arguments.source.resolve().relative_to(
        Path(__file__).resolve().parents[2]
    )
    generated = generate_assets(
        arguments.source.read_bytes(),
        source_name=source_name.as_posix(),
        target_width=arguments.width,
    )
    outputs = {
        arguments.output / "logo_ascii.json": generated.json_text.encode(),
        arguments.output / "logo_ascii.txt": generated.text.encode(),
        arguments.output / "logo_ascii_preview.png": generated.preview_png,
    }

    if arguments.check:
        stale = [str(path) for path, data in outputs.items() if not _matches(path, data)]
        if stale:
            print("[BLOCKER] Generated ASCII logo assets are stale:")
            for path in stale:
                print(f"  - {path}")
            return 1
        print("[PASS] Generated ASCII logo assets are current")
        return 0

    arguments.output.mkdir(parents=True, exist_ok=True)
    for path, data in outputs.items():
        path.write_bytes(data)
    print(f"[PASS] Generated ASCII logo assets in {arguments.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
