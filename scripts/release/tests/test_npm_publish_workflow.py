import importlib.util
import shutil
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check_npm_publish_workflow.py"
SPEC = importlib.util.spec_from_file_location("npm_publish_workflow", SCRIPT)
assert SPEC and SPEC.loader
npm_publish_workflow = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(npm_publish_workflow)
SOURCE_ROOT = Path(__file__).resolve().parents[3]


class NpmPublishWorkflowTests(unittest.TestCase):
    def setUp(self):
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        target = self.root / ".github/workflows/npm-publish.yml"
        target.parent.mkdir(parents=True)
        shutil.copy2(SOURCE_ROOT / ".github/workflows/npm-publish.yml", target)
        self.workflow = target

    def tearDown(self):
        self.tempdir.cleanup()

    def test_accepts_manual_oidc_publish(self):
        npm_publish_workflow.validate(self.root)

    def test_rejects_persistent_npm_token(self):
        self.workflow.write_text(
            self.workflow.read_text() + "\n# NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}\n"
        )
        with self.assertRaisesRegex(ValueError, "persistent credential"):
            npm_publish_workflow.validate(self.root)

    def test_rejects_missing_oidc_permission(self):
        self.workflow.write_text(
            self.workflow.read_text().replace("  id-token: write\n", "")
        )
        with self.assertRaisesRegex(ValueError, "id-token: write"):
            npm_publish_workflow.validate(self.root)


if __name__ == "__main__":
    unittest.main()
