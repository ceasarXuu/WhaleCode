import importlib.util
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check_manual_actions_only.py"
SPEC = importlib.util.spec_from_file_location("manual_actions_only", SCRIPT)
assert SPEC and SPEC.loader
manual_actions_only = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(manual_actions_only)


class ManualActionsOnlyTests(unittest.TestCase):
    def setUp(self):
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        self.workflows = self.root / ".github/workflows"
        self.workflows.mkdir(parents=True)

    def tearDown(self):
        self.tempdir.cleanup()

    def write_workflow(self, trigger: str) -> None:
        (self.workflows / "ci.yml").write_text(
            f"name: ci\non:\n{trigger}\njobs:\n  check:\n    runs-on: ubuntu-latest\n",
            encoding="utf-8",
        )

    def test_accepts_manual_dispatch_only(self):
        self.write_workflow("  workflow_dispatch:\n    inputs:\n      version:\n")
        manual_actions_only.validate(self.root)

    def test_rejects_push_trigger(self):
        self.write_workflow("  workflow_dispatch:\n  push:\n")
        with self.assertRaisesRegex(ValueError, "only workflow_dispatch"):
            manual_actions_only.validate(self.root)

    def test_rejects_pull_request_trigger(self):
        self.write_workflow("  pull_request:\n  workflow_dispatch:\n")
        with self.assertRaisesRegex(ValueError, "only workflow_dispatch"):
            manual_actions_only.validate(self.root)

    def test_rejects_inline_trigger(self):
        (self.workflows / "ci.yml").write_text(
            "name: ci\non: workflow_dispatch\njobs: {}\n", encoding="utf-8"
        )
        with self.assertRaisesRegex(ValueError, "trigger block"):
            manual_actions_only.validate(self.root)


if __name__ == "__main__":
    unittest.main()
