from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import tempfile
import time
import unittest
from unittest import mock
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "workspace_context.py"
SPEC = importlib.util.spec_from_file_location("workspace_context_gate", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
workspace_context = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(workspace_context)


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo), *args], check=True, capture_output=True, text=True
    ).stdout.strip()


def tree_snapshot(root: Path) -> dict[str, tuple[int, int]]:
    if not root.exists():
        return {}
    return {
        str(path.relative_to(root)): (path.stat().st_size, path.stat().st_mtime_ns)
        for path in root.rglob("*")
    }


class RequireReadyTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "repo"
        self.root.mkdir()
        git(self.root, "init", "-q", "-b", "main")
        git(self.root, "config", "user.name", "Gate Test")
        git(self.root, "config", "user.email", "gate@example.invalid")
        (self.root / "README.md").write_text("fixture\n")
        git(self.root, "add", "README.md")
        git(self.root, "commit", "-q", "-m", "initial")
        self.home = Path(self.temp.name) / "home"
        self.environment = {
            "HOME": str(self.home),
            "XDG_STATE_HOME": str(self.home / "state"),
            "XDG_DATA_HOME": str(self.home / "data"),
        }

    def apply(self) -> Path:
        plan = workspace_context.build_plan(self.root, self.environment)
        workspace_context.apply_plan(self.root, plan["fingerprint"], self.environment)
        return Path(plan["context"]["marker_path"])

    def test_unbootstrapped_gate_is_zero_write(self) -> None:
        before = tree_snapshot(self.home)
        result = workspace_context.require_ready(self.root, self.environment)

        self.assertFalse(result["ready"])
        self.assertEqual("Unbootstrapped", result["state"])
        self.assertEqual(before, tree_snapshot(self.home))

    def test_ready_gate_does_not_touch_audit_or_marker(self) -> None:
        marker_path = self.apply()
        before = tree_snapshot(self.home)
        result = workspace_context.require_ready(self.root, self.environment)

        self.assertTrue(result["ready"])
        self.assertEqual("Ready", result["state"])
        self.assertEqual(before, tree_snapshot(self.home))
        self.assertEqual(0o600, marker_path.stat().st_mode & 0o777)

    def test_gate_git_reads_are_limited_to_root_and_branch(self) -> None:
        self.apply()
        calls = []
        original = workspace_context._git

        def recording_git(repo, *args, **kwargs):
            calls.append(args)
            return original(repo, *args, **kwargs)

        with mock.patch.object(workspace_context, "_git", side_effect=recording_git):
            result = workspace_context.require_ready(self.root, self.environment)

        self.assertTrue(result["ready"])
        self.assertEqual(
            [("rev-parse", "--show-toplevel"), ("symbolic-ref", "--quiet", "--short", "HEAD")],
            calls,
        )

    def test_stale_conflict_and_doctor_failed_are_distinct(self) -> None:
        marker_path = self.apply()
        marker = json.loads(marker_path.read_text())
        git(self.root, "checkout", "-q", "-b", "other")
        stale = workspace_context.require_ready(self.root, self.environment)
        git(self.root, "checkout", "-q", "main")
        marker_path.write_text(json.dumps({**marker, "canonical_root": "/other"}))
        conflict = workspace_context.require_ready(self.root, self.environment)
        marker_path.write_text(
            json.dumps({**marker, "last_doctor": {"status": "failed", "diagnostic_codes": ["fixture"]}})
        )
        failed = workspace_context.require_ready(self.root, self.environment)

        self.assertEqual("Stale", stale["state"])
        self.assertEqual("Conflict", conflict["state"])
        self.assertEqual("DoctorFailed", failed["state"])

    def test_gate_ignores_deep_resource_checks_by_contract(self) -> None:
        marker_path = self.apply()
        marker = json.loads(marker_path.read_text())
        marker["resources"] = {"deliberately": "not-checked-by-fast-gate"}
        marker_path.write_text(json.dumps(marker))

        self.assertTrue(workspace_context.require_ready(self.root, self.environment)["ready"])

    def test_cli_exit_codes_json_schema_and_latency(self) -> None:
        cli_environment = {**os.environ, **self.environment}
        blocked = subprocess.run(
            ["python3", str(MODULE_PATH), "require-ready", "--repo-root", str(self.root), "--json"],
            check=False, capture_output=True, text=True, env=cli_environment,
        )
        self.assertEqual(7, blocked.returncode)
        self.apply()
        ready = subprocess.run(
            ["python3", str(MODULE_PATH), "require-ready", "--repo-root", str(self.root), "--json"],
            check=False, capture_output=True, text=True, env=cli_environment,
        )
        result = json.loads(ready.stdout)
        schema = json.loads(
            (MODULE_PATH.parent / "schemas/workspace-ready.schema.json").read_text()
        )
        started = time.perf_counter()
        for _ in range(10):
            self.assertTrue(workspace_context.require_ready(self.root, self.environment)["ready"])
        elapsed = time.perf_counter() - started

        self.assertEqual(0, ready.returncode)
        self.assertEqual(set(schema["required"]), set(result))
        self.assertLess(elapsed / 10, 0.1)


if __name__ == "__main__":
    unittest.main()
