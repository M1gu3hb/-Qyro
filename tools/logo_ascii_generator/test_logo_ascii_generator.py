import hashlib
import importlib.util
import json
import sys
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("generate.py")

# The owner supplied this file on main as design/brand/source/logo.png. It is the
# only authorized production logo source. See docs/adr/ADR-0014-canonical-logo.md.
CANONICAL_LOGO = "design/brand/source/logo.png"
CANONICAL_LOGO_SHA256 = (
    "e8413410d53958fe399c3e37ed73e85030b41c1dbe456ca3a5bad2491e6d4f39"
)
# Superseded placeholder the owner explicitly renamed to mark it unusable. It must
# never reach app assets, generated previews, or releases.
REJECTED_LOGO = "design/brand/source/no usar este logo"
REJECTED_LOGO_SHA256 = (
    "52107d9e88fcc50838e7c9fcef928592529eea6aaed367597fcfc4547488258d"
)
SHIPPED_LOGO_ASSET = "apps/qyro/assets/brand/qyro-logo.png"
SPEC = importlib.util.spec_from_file_location("logo_ascii_generator", MODULE_PATH)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = GENERATOR
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
        source_path = root / CANONICAL_LOGO
        output = root / "apps/qyro/assets/generated"
        expected = GENERATOR.generate_assets(
            source_path.read_bytes(),
            source_name=CANONICAL_LOGO,
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


class CanonicalLogoTest(unittest.TestCase):
    """Binds the authorized logo bytes so the rejected placeholder cannot return."""

    def setUp(self):
        self.root = Path(__file__).resolve().parents[2]

    def _digest(self, relative_path):
        return hashlib.sha256((self.root / relative_path).read_bytes()).hexdigest()

    def test_canonical_source_is_the_owner_supplied_logo(self):
        self.assertEqual(self._digest(CANONICAL_LOGO), CANONICAL_LOGO_SHA256)

    def test_shipped_asset_is_byte_identical_to_the_canonical_source(self):
        self.assertEqual(self._digest(SHIPPED_LOGO_ASSET), CANONICAL_LOGO_SHA256)

    def test_generated_model_pins_the_canonical_source_and_checksum(self):
        model = json.loads(
            (
                self.root / "apps/qyro/assets/generated/logo_ascii.json"
            ).read_text(encoding="utf-8")
        )
        self.assertEqual(model["source"], CANONICAL_LOGO)
        self.assertEqual(model["sourceChecksum"], "sha256:" + CANONICAL_LOGO_SHA256)

    def test_rejected_placeholder_is_preserved_but_still_the_old_bytes(self):
        rejected = self.root / REJECTED_LOGO
        self.assertTrue(rejected.is_file(), f"{REJECTED_LOGO} must not be deleted")
        self.assertEqual(self._digest(REJECTED_LOGO), REJECTED_LOGO_SHA256)
        self.assertNotEqual(REJECTED_LOGO_SHA256, CANONICAL_LOGO_SHA256)

    def test_rejected_placeholder_bytes_never_reach_shipped_assets(self):
        assets = self.root / "apps/qyro/assets"
        offenders = [
            path.relative_to(self.root).as_posix()
            for path in sorted(assets.rglob("*"))
            if path.is_file()
            and hashlib.sha256(path.read_bytes()).hexdigest() == REJECTED_LOGO_SHA256
        ]
        self.assertEqual(offenders, [])


if __name__ == "__main__":
    unittest.main()
