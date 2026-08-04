import hashlib
import importlib.util
import json
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("generate.py")
SPEC = importlib.util.spec_from_file_location("logo_ascii_generator", MODULE_PATH)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)


class LogoAsciiGeneratorTest(unittest.TestCase):
    def test_generates_deterministic_model_text_and_png_preview(self):
        pixels = [
            (0, 0, 0, 0),
            (22, 139, 255, 255),
            (81, 200, 255, 255),
            (0, 0, 0, 0),
        ]
        source = GENERATOR.encode_rgba_png(2, 2, pixels)

        first = GENERATOR.generate_assets(
            source,
            source_name="fixture.png",
            target_width=4,
        )
        second = GENERATOR.generate_assets(
            source,
            source_name="fixture.png",
            target_width=4,
        )
        model = json.loads(first.json_text)

        self.assertEqual(first, second)
        self.assertEqual(model["width"], 4)
        self.assertGreater(model["height"], 0)
        self.assertEqual(
            model["sourceChecksum"],
            "sha256:" + hashlib.sha256(source).hexdigest(),
        )
        self.assertEqual(model["generatorVersion"], "1.0.0")
        self.assertEqual(len(model["characterCells"]), model["height"])
        self.assertEqual(len(model["mask"]), model["height"])
        self.assertTrue(first.preview_png.startswith(b"\x89PNG\r\n\x1a\n"))

    def test_committed_assets_match_the_source_logo(self):
        root = Path(__file__).resolve().parents[2]
        source_path = root / "design/brand/source/qyro-logo.png"
        output = root / "apps/qyro/assets/generated"
        expected = GENERATOR.generate_assets(
            source_path.read_bytes(),
            source_name="design/brand/source/qyro-logo.png",
        )

        self.assertEqual(
            (output / "logo_ascii.json").read_text(encoding="utf-8"),
            expected.json_text,
        )
        self.assertEqual(
            (output / "logo_ascii.txt").read_text(encoding="utf-8"),
            expected.text,
        )
        self.assertEqual(
            (output / "logo_ascii_preview.png").read_bytes(),
            expected.preview_png,
        )


if __name__ == "__main__":
    unittest.main()
