import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check_release_identity.py"
SPEC = importlib.util.spec_from_file_location("release_identity", SCRIPT)
assert SPEC and SPEC.loader
release_identity = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release_identity)


class ReleaseIdentityTests(unittest.TestCase):
    def setUp(self):
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        (self.root / "third_party/codex-cli/codex-rs").mkdir(parents=True)
        (self.root / "docs/releases/v0.0.6/release-preparation").mkdir(
            parents=True
        )
        (self.root / "docs/v0.0.5/codex-upstream-sync").mkdir(parents=True)
        (self.root / "third_party/codex-cli/codex-rs/Cargo.toml").write_text(
            '[workspace]\n[workspace.package]\nversion = "0.0.6"\n',
            encoding="utf-8",
        )
        self.manifest_path = (
            self.root / "docs/releases/v0.0.6/release-preparation/release.json"
        )
        self.candidate_path = (
            self.root / "docs/v0.0.5/codex-upstream-sync/upstream-candidate.json"
        )
        self.manifest = {
            "schema_version": 1,
            "product": "WhaleCode",
            "release": {
                "version": "0.0.6",
                "tag": "v0.0.6",
                "status": "preparing",
                "publish_authorized": False,
            },
            "upstream_substrate": {
                "version": "0.149.0",
                "tag": "rust-v0.149.0",
                "candidate_manifest": (
                    "docs/v0.0.5/codex-upstream-sync/upstream-candidate.json"
                ),
            },
        }
        self.candidate = {"release_tag": "rust-v0.149.0"}
        self.write_fixtures()

    def tearDown(self):
        self.tempdir.cleanup()

    def write_fixtures(self):
        self.manifest_path.write_text(json.dumps(self.manifest), encoding="utf-8")
        self.candidate_path.write_text(json.dumps(self.candidate), encoding="utf-8")

    def test_accepts_separate_whale_and_codex_identities(self):
        self.assertEqual(
            release_identity.validate(self.root, "v0.0.6"),
            ("v0.0.6", "rust-v0.149.0"),
        )

    def test_rejects_codex_tag_as_whale_tag(self):
        with self.assertRaisesRegex(
            release_identity.IdentityError, "supplied Whale tag"
        ):
            release_identity.validate(self.root, "rust-v0.149.0")

    def test_rejects_cargo_substrate_version_as_whale_version(self):
        cargo = self.root / "third_party/codex-cli/codex-rs/Cargo.toml"
        cargo.write_text(
            '[workspace]\n[workspace.package]\nversion = "0.149.0"\n',
            encoding="utf-8",
        )
        with self.assertRaisesRegex(release_identity.IdentityError, "Cargo Whale"):
            release_identity.validate(self.root, "v0.0.6")

    def test_rejects_candidate_registration_drift(self):
        self.candidate["release_tag"] = "rust-v0.147.0"
        self.write_fixtures()
        with self.assertRaisesRegex(release_identity.IdentityError, "candidate tag"):
            release_identity.validate(self.root)

    def test_preparing_release_cannot_be_publish_authorized(self):
        self.manifest["release"]["publish_authorized"] = True
        self.write_fixtures()
        with self.assertRaisesRegex(release_identity.IdentityError, "publish_authorized"):
            release_identity.validate(self.root)


if __name__ == "__main__":
    unittest.main()
