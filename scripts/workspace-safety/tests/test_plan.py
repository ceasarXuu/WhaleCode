from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "workspace_context.py"
SPEC = importlib.util.spec_from_file_location("workspace_context_plan", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
workspace_context = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(workspace_context)


def git(repo: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(repo), *args],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return completed.stdout.strip()


def snapshot(root: Path) -> dict[str, str]:
    result = {}
    if not root.exists():
        return result
    for path in sorted(root.rglob("*")):
        relative = str(path.relative_to(root))
        result[relative] = (
            hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else "directory"
        )
    return result


class WorkspacePlanTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "repo"
        self.root.mkdir()
        git(self.root, "init", "-q", "-b", "main")
        git(self.root, "config", "user.name", "Plan Test")
        git(self.root, "config", "user.email", "plan@example.invalid")
        (self.root / "README.md").write_text("# Fixture\n", encoding="utf-8")
        git(self.root, "add", "README.md")
        git(self.root, "commit", "-q", "-m", "initial")
        self.home = Path(self.temp.name) / "home"
        self.environment = {
            "HOME": str(self.home),
            "XDG_STATE_HOME": str(self.home / "state"),
            "XDG_DATA_HOME": str(self.home / "data"),
        }

    def test_plan_is_deterministic_and_zero_write(self) -> None:
        before_repo = snapshot(self.root)
        before_home = snapshot(self.home)

        first = workspace_context.build_plan(self.root, self.environment)
        second = workspace_context.build_plan(self.root / "README.md", self.environment)

        self.assertEqual(first, second)
        self.assertEqual(before_repo, snapshot(self.root))
        self.assertEqual(before_home, snapshot(self.home))
        self.assertEqual("Unbootstrapped", first["state"]["code"])
        self.assertTrue(first["can_apply"])
        self.assertEqual("main", first["context"]["branch"])
        self.assertFalse(Path(first["context"]["marker_path"]).exists())

    def test_linked_worktree_has_distinct_root_and_shared_common_dir(self) -> None:
        linked = Path(self.temp.name) / "linked"
        git(self.root, "branch", "feature")
        git(self.root, "worktree", "add", "-q", str(linked), "feature")

        main = workspace_context.build_plan(self.root, self.environment)
        worktree = workspace_context.build_plan(linked, self.environment)

        self.assertEqual(main["context"]["git_common_dir"], worktree["context"]["git_common_dir"])
        self.assertNotEqual(main["context"]["git_dir"], worktree["context"]["git_dir"])
        self.assertNotEqual(main["context"]["workspace_id"], worktree["context"]["workspace_id"])
        self.assertEqual("feature", worktree["context"]["branch"])

    def test_detached_head_returns_plan_but_blocks_apply(self) -> None:
        git(self.root, "checkout", "-q", "--detach")
        plan = workspace_context.build_plan(self.root, self.environment)

        self.assertTrue(plan["context"]["detached_head"])
        self.assertFalse(plan["can_apply"])
        self.assertIn("detached_head", plan["blocking_reason_codes"])

    def test_existing_marker_changes_state_and_fingerprint(self) -> None:
        unbootstrapped = workspace_context.build_plan(self.root, self.environment)
        context = unbootstrapped["context"]
        marker_path = Path(context["marker_path"])
        marker_path.parent.mkdir(parents=True)
        marker = {
            "schema_version": 1,
            "workspace_id": context["workspace_id"],
            "canonical_root": context["canonical_root"],
            "git_common_dir": context["git_common_dir"],
            "branch": context["branch"],
            "resources": context["resources"],
            "last_doctor": {"status": "passed", "diagnostic_codes": []},
        }
        marker_path.write_text(json.dumps(marker), encoding="utf-8")

        ready = workspace_context.build_plan(self.root, self.environment)

        self.assertEqual("Ready", ready["state"]["code"])
        self.assertNotEqual(unbootstrapped["fingerprint"], ready["fingerprint"])
        marker_action = next(item for item in ready["actions"] if item["action"] == "write_marker")
        self.assertEqual("reuse", marker_action["disposition"])

    def test_invalid_marker_is_redacted_conflict(self) -> None:
        initial = workspace_context.build_plan(self.root, self.environment)
        marker_path = Path(initial["context"]["marker_path"])
        marker_path.parent.mkdir(parents=True)
        marker_path.write_text('{"secret":"must-not-appear"', encoding="utf-8")

        plan = workspace_context.build_plan(self.root, self.environment)
        rendered = workspace_context.render_json(plan)

        self.assertEqual("Conflict", plan["state"]["code"])
        self.assertFalse(plan["can_apply"])
        self.assertEqual("invalid_json", plan["existing_marker"]["parse_status"])
        self.assertNotIn("must-not-appear", rendered)

    def test_relative_xdg_path_fails_closed(self) -> None:
        environment = {**self.environment, "XDG_STATE_HOME": "relative/state"}
        with self.assertRaisesRegex(workspace_context.ContextError, "absolute path"):
            workspace_context.build_plan(self.root, environment)

    def test_schema_contract_matches_plan_shape(self) -> None:
        schema_path = MODULE_PATH.parent / "schemas" / "workspace-plan.schema.json"
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
        plan = workspace_context.build_plan(self.root, self.environment)

        self.assertEqual(set(schema["required"]), set(plan))
        self.assertEqual(set(schema["properties"]["context"]["required"]), set(plan["context"]))
        self.assertRegex(plan["fingerprint"], r"^[a-f0-9]{64}$")

    def test_cli_emits_json_without_creating_home(self) -> None:
        environment = {**os.environ, **self.environment}
        completed = subprocess.run(
            [
                "python3",
                str(MODULE_PATH),
                "bootstrap",
                "plan",
                "--repo-root",
                str(self.root),
                "--json",
            ],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=environment,
        )

        self.assertEqual(0, completed.returncode, completed.stderr)
        self.assertEqual("Unbootstrapped", json.loads(completed.stdout)["state"]["code"])
        self.assertFalse(self.home.exists())


if __name__ == "__main__":
    unittest.main()
