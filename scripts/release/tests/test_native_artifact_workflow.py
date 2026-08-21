import importlib.util
import shutil
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check_native_artifact_workflow.py"
SPEC = importlib.util.spec_from_file_location("native_artifact_workflow", SCRIPT)
assert SPEC and SPEC.loader
native_artifact_workflow = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(native_artifact_workflow)
SOURCE_ROOT = Path(__file__).resolve().parents[3]


class NativeArtifactWorkflowTests(unittest.TestCase):
    def setUp(self):
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        target = self.root / ".github/workflows/whale-native-artifacts.yml"
        target.parent.mkdir(parents=True)
        shutil.copy2(
            SOURCE_ROOT / ".github/workflows/whale-native-artifacts.yml", target
        )
        helper = self.root / "scripts/release/prepare_rusty_v8.py"
        helper.parent.mkdir(parents=True)
        shutil.copy2(SOURCE_ROOT / "scripts/release/prepare_rusty_v8.py", helper)

    def tearDown(self):
        self.tempdir.cleanup()

    def test_accepts_whale_artifact_only_workflow(self):
        native_artifact_workflow.validate(self.root)

    def test_rejects_publish_capability(self):
        workflow = self.root / ".github/workflows/whale-native-artifacts.yml"
        workflow.write_text(workflow.read_text() + "\n# npm publish\n")
        with self.assertRaisesRegex(ValueError, "forbidden publish/vendor capability"):
            native_artifact_workflow.validate(self.root)

    def test_rejects_automatic_push_trigger(self):
        workflow = self.root / ".github/workflows/whale-native-artifacts.yml"
        workflow.write_text(workflow.read_text() + "\npush:\n")
        with self.assertRaisesRegex(ValueError, "must not have a push trigger"):
            native_artifact_workflow.validate(self.root)


if __name__ == "__main__":
    unittest.main()
