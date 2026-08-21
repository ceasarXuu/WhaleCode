import importlib.util
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check_distribution_identity.py"
SPEC = importlib.util.spec_from_file_location("distribution_identity", SCRIPT)
assert SPEC and SPEC.loader
distribution_identity = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(distribution_identity)
SOURCE_ROOT = Path(__file__).resolve().parents[3]
BUILDER = (
    SOURCE_ROOT
    / "third_party/codex-cli/codex-cli/scripts/build_npm_package.py"
)


class DistributionIdentityTests(unittest.TestCase):
    def setUp(self):
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        paths = set(distribution_identity.ACTIVE_FILES)
        paths.update(distribution_identity.REQUIRED_TEXT)
        paths.add("third_party/codex-cli/DISTRIBUTION_QUARANTINE.md")
        for relative in paths:
            source = SOURCE_ROOT / relative
            target = self.root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
        workflow_source = SOURCE_ROOT / ".github/workflows"
        workflow_target = self.root / ".github/workflows"
        workflow_target.mkdir(parents=True)
        for source in workflow_source.glob("*.y*ml"):
            shutil.copy2(source, workflow_target / source.name)

    def tearDown(self):
        self.tempdir.cleanup()

    def test_accepts_whale_owned_distribution_routes(self):
        distribution_identity.validate(self.root)

    def test_rejects_openai_runtime_update_target(self):
        path = (
            self.root
            / "third_party/codex-cli/codex-rs/tui/src/update_action.rs"
        )
        path.write_text(
            path.read_text(encoding="utf-8")
            + '\nconst BAD: &str = "npm install -g @openai/codex";\n',
            encoding="utf-8",
        )
        with self.assertRaisesRegex(
            distribution_identity.DistributionIdentityError,
            "forbidden OpenAI distribution target",
        ):
            distribution_identity.validate(self.root)

    def test_rejects_vendor_release_workflow_activation(self):
        workflow = self.root / ".github/workflows/bad.yml"
        workflow.write_text(
            "uses: ./third_party/codex-cli/.github/workflows/rust-release.yml\n",
            encoding="utf-8",
        )
        with self.assertRaisesRegex(
            distribution_identity.DistributionIdentityError,
            "quarantined vendor release code",
        ):
            distribution_identity.validate(self.root)

    def test_rejects_codex_launcher(self):
        launcher = self.root / "third_party/codex-cli/codex-cli/bin/codex.js"
        launcher.write_text("console.log('codex')\n", encoding="utf-8")
        with self.assertRaisesRegex(
            distribution_identity.DistributionIdentityError,
            "must not expose bin/codex.js",
        ):
            distribution_identity.validate(self.root)

    def test_stages_all_whale_platform_packages(self):
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            vendor = root / "vendor"
            packages = {
                "whalecode-linux-x64": ("x86_64-unknown-linux-musl", "linux", "x64"),
                "whalecode-linux-arm64": ("aarch64-unknown-linux-musl", "linux", "arm64"),
                "whalecode-darwin-x64": ("x86_64-apple-darwin", "darwin", "x64"),
                "whalecode-darwin-arm64": ("aarch64-apple-darwin", "darwin", "arm64"),
                "whalecode-win32-x64": ("x86_64-pc-windows-msvc", "win32", "x64"),
                "whalecode-win32-arm64": ("aarch64-pc-windows-msvc", "win32", "arm64"),
            }
            for target, _os_name, _cpu in packages.values():
                binary = vendor / target / "bin" / (
                    "whale.exe" if "windows" in target else "whale"
                )
                binary.parent.mkdir(parents=True)
                binary.write_bytes(b"whale-test-binary")

            meta_stage = root / "meta"
            self._stage("whalecode", meta_stage)
            meta = json.loads((meta_stage / "package.json").read_text())
            self.assertEqual(meta["name"], "@ceasarxuu/whalecode")
            self.assertEqual(meta["bin"], {"whale": "bin/whale.js"})
            self.assertEqual(len(meta["optionalDependencies"]), 6)

            for package, (target, os_name, cpu) in packages.items():
                stage = root / package
                self._stage(package, stage, vendor)
                manifest = json.loads((stage / "package.json").read_text())
                tag = package.removeprefix("whalecode-")
                self.assertEqual(manifest["name"], "@ceasarxuu/whalecode")
                self.assertEqual(manifest["version"], f"0.0.5-{tag}")
                self.assertEqual(manifest["os"], [os_name])
                self.assertEqual(manifest["cpu"], [cpu])
                self.assertTrue((stage / "vendor" / target / "bin").is_dir())

    @staticmethod
    def _stage(package: str, stage: Path, vendor: Path | None = None):
        command = [
            sys.executable,
            str(BUILDER),
            "--package",
            package,
            "--release-version",
            "0.0.5",
            "--staging-dir",
            str(stage),
        ]
        if vendor is not None:
            command.extend(["--vendor-src", str(vendor)])
        subprocess.run(command, check=True, capture_output=True, text=True)


if __name__ == "__main__":
    unittest.main()
