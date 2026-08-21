import importlib.util
import shutil
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check_brand_identity.py"
SPEC = importlib.util.spec_from_file_location("brand_identity", SCRIPT)
assert SPEC and SPEC.loader
brand_identity = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(brand_identity)
SOURCE_ROOT = Path(__file__).resolve().parents[3]


class BrandIdentityTests(unittest.TestCase):
    def setUp(self):
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        for relative_root in brand_identity.SOURCE_ROOTS:
            source = SOURCE_ROOT / relative_root
            shutil.copytree(source, self.root / relative_root)
        for relative_file in brand_identity.EXTRA_FILES:
            source = SOURCE_ROOT / relative_file
            destination = self.root / relative_file
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)

    def tearDown(self):
        self.tempdir.cleanup()

    def test_accepts_whale_user_surfaces(self):
        brand_identity.validate(self.root)

    def test_rejects_codex_product_brand(self):
        path = self.root / "third_party/codex-cli/codex-rs/cli/src/app_cmd.rs"
        path.write_text(
            path.read_text(encoding="utf-8") + '\nconst BAD: &str = "OpenAI Codex";\n',
            encoding="utf-8",
        )
        with self.assertRaisesRegex(
            brand_identity.BrandIdentityError, "inherited brand exposure"
        ):
            brand_identity.validate(self.root)

    def test_rejects_codex_user_command(self):
        path = self.root / "third_party/codex-cli/codex-rs/cli/src/app_cmd.rs"
        path.write_text(
            path.read_text(encoding="utf-8")
            + '\nconst BAD: &str = "Run codex doctor";\n',
            encoding="utf-8",
        )
        with self.assertRaisesRegex(
            brand_identity.BrandIdentityError, "inherited brand exposure"
        ):
            brand_identity.validate(self.root)

    def test_rejects_codex_tooltip_command(self):
        path = self.root / "third_party/codex-cli/codex-rs/tui/tooltips.txt"
        path.write_text(
            path.read_text(encoding="utf-8")
            + "\nResume with `codex resume`.\n",
            encoding="utf-8",
        )
        with self.assertRaisesRegex(
            brand_identity.BrandIdentityError, "inherited brand exposure"
        ):
            brand_identity.validate(self.root)


if __name__ == "__main__":
    unittest.main()
