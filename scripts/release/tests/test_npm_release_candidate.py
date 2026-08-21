import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check_npm_release_candidate.py"
SPEC = importlib.util.spec_from_file_location("npm_release_candidate", SCRIPT)
assert SPEC and SPEC.loader
npm_release_candidate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(npm_release_candidate)


class NpmReleaseCandidateTests(unittest.TestCase):
    def manifest(self):
        version = "0.0.5"
        return {
            "name": "@ceasarxuu/whalecode",
            "version": version,
            "bin": {"whale": "bin/whale.js"},
            "files": ["bin/whale.js"],
            "optionalDependencies": (
                npm_release_candidate.expected_optional_dependencies(version)
            ),
        }

    def test_accepts_whale_staged_manifest(self):
        npm_release_candidate.validate_staged_manifest(self.manifest(), "0.0.5")

    def test_rejects_codex_package_name(self):
        manifest = self.manifest()
        manifest["name"] = "@openai/codex"
        with self.assertRaisesRegex(
            npm_release_candidate.NpmCandidateError,
            "staged npm manifest name",
        ):
            npm_release_candidate.validate_staged_manifest(manifest, "0.0.5")

    def test_rejects_incomplete_tarball_inventory(self):
        output = [
            {
                "name": "@ceasarxuu/whalecode",
                "version": "0.0.5",
                "files": [{"path": "package.json"}],
                "shasum": "sha1",
                "integrity": "sha512-value",
            }
        ]
        with self.assertRaisesRegex(
            npm_release_candidate.NpmCandidateError,
            "npm tarball files",
        ):
            npm_release_candidate.validate_pack_output(output, "0.0.5")


if __name__ == "__main__":
    unittest.main()
