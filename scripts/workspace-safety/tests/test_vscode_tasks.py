from __future__ import annotations

import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
TASKS_PATH = ROOT / ".vscode/tasks.json"


class VscodeTasksTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        document = json.loads(TASKS_PATH.read_text(encoding="utf-8"))
        cls.document = document
        cls.tasks = {task["label"]: task for task in document["tasks"]}

    def test_expected_tasks_are_present(self) -> None:
        self.assertEqual(
            set(self.tasks),
            {
                "Workspace: Bootstrap Plan",
                "Workspace: Bootstrap Apply",
                "Workspace: Doctor",
                "Rust: Check codex-cli",
            },
        )

    def test_workspace_tasks_call_the_authoritative_cli(self) -> None:
        script = "${workspaceFolder}/scripts/workspace-safety/workspace_context.py"
        for label in (
            "Workspace: Bootstrap Plan",
            "Workspace: Bootstrap Apply",
            "Workspace: Doctor",
        ):
            task = self.tasks[label]
            self.assertEqual(task["type"], "process")
            self.assertEqual(task["command"], "python3")
            self.assertEqual(task["args"][0], script)
            self.assertIn("--repo-root", task["args"])
            self.assertIn("${workspaceFolder}", task["args"])

    def test_apply_requires_the_plan_fingerprint(self) -> None:
        apply_args = self.tasks["Workspace: Bootstrap Apply"]["args"]
        self.assertEqual(
            apply_args[apply_args.index("--expect") + 1],
            "${input:workspacePlanFingerprint}",
        )
        inputs = {item["id"]: item for item in self.document["inputs"]}
        self.assertEqual(inputs["workspacePlanFingerprint"]["type"], "promptString")

    def test_doctor_requires_workspace_binary(self) -> None:
        self.assertIn("--require-binary", self.tasks["Workspace: Doctor"]["args"])

    def test_rust_check_uses_the_real_workspace(self) -> None:
        task = self.tasks["Rust: Check codex-cli"]
        self.assertEqual(task["command"], "cargo")
        self.assertEqual(task["args"], ["check", "-p", "codex-cli", "--locked"])
        self.assertEqual(
            task["options"]["cwd"],
            "${workspaceFolder}/third_party/codex-cli/codex-rs",
        )
        self.assertTrue((ROOT / "third_party/codex-cli/codex-rs/Cargo.toml").is_file())


if __name__ == "__main__":
    unittest.main()
