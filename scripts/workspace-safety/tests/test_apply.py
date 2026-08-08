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
SPEC = importlib.util.spec_from_file_location("workspace_context_apply", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
workspace_context = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(workspace_context)


def git(repo: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(repo), *args], check=True, capture_output=True, text=True
    )
    return completed.stdout.strip()


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class WorkspaceApplyTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "repo"
        self.root.mkdir()
        git(self.root, "init", "-q", "-b", "main")
        git(self.root, "config", "user.name", "Apply Test")
        git(self.root, "config", "user.email", "apply@example.invalid")
        (self.root / "README.md").write_text("# Fixture\n", encoding="utf-8")
        git(self.root, "add", "README.md")
        git(self.root, "commit", "-q", "-m", "initial")
        self.home = Path(self.temp.name) / "home"
        self.environment = {
            "HOME": str(self.home),
            "XDG_STATE_HOME": str(self.home / "state"),
            "XDG_DATA_HOME": str(self.home / "data"),
        }

    def plan(self) -> dict[str, object]:
        return workspace_context.build_plan(self.root, self.environment)

    def test_stale_fingerprint_fails_before_writes(self) -> None:
        with self.assertRaisesRegex(workspace_context.ApplyError, "fingerprint") as raised:
            workspace_context.apply_plan(self.root, "0" * 64, self.environment)

        self.assertEqual("plan_fingerprint_mismatch", raised.exception.code)
        self.assertFalse(self.home.exists())

    def test_apply_creates_private_resources_and_ready_marker(self) -> None:
        plan = self.plan()
        result = workspace_context.apply_plan(
            self.root, plan["fingerprint"], self.environment
        )
        marker_path = Path(plan["context"]["marker_path"])
        marker = json.loads(marker_path.read_text(encoding="utf-8"))

        self.assertEqual("Ready", result["state"]["code"])
        self.assertEqual("passed", marker["last_doctor"]["status"])
        self.assertEqual(0o600, marker_path.stat().st_mode & 0o777)
        for path in plan["context"]["resources"].values():
            self.assertEqual(0o700, Path(path).stat().st_mode & 0o777)

    def test_repeated_apply_reuses_identical_marker(self) -> None:
        first_plan = self.plan()
        workspace_context.apply_plan(
            self.root, first_plan["fingerprint"], self.environment
        )
        second_plan = self.plan()
        marker_path = Path(second_plan["context"]["marker_path"])
        before_inode = marker_path.stat().st_ino
        before_digest = digest(marker_path)

        result = workspace_context.apply_plan(
            self.root, second_plan["fingerprint"], self.environment
        )

        self.assertEqual("reused", result["marker_disposition"])
        self.assertEqual(before_inode, marker_path.stat().st_ino)
        self.assertEqual(before_digest, digest(marker_path))

    def test_branch_change_invalidates_confirmed_plan_without_writes(self) -> None:
        plan = self.plan()
        git(self.root, "checkout", "-q", "-b", "other")

        with self.assertRaises(workspace_context.ApplyError) as raised:
            workspace_context.apply_plan(
                self.root, plan["fingerprint"], self.environment
            )

        self.assertEqual("plan_fingerprint_mismatch", raised.exception.code)
        self.assertFalse(self.home.exists())

    def test_legacy_home_and_git_are_untouched(self) -> None:
        legacy = self.home / ".whale"
        legacy.mkdir(parents=True)
        legacy_file = legacy / "auth.json"
        legacy_file.write_text("fixture-secret", encoding="utf-8")
        before_digest = digest(legacy_file)
        before_mtime = legacy_file.stat().st_mtime_ns
        before_status = git(self.root, "status", "--porcelain")
        plan = self.plan()

        workspace_context.apply_plan(
            self.root, plan["fingerprint"], self.environment
        )

        self.assertEqual(before_digest, digest(legacy_file))
        self.assertEqual(before_mtime, legacy_file.stat().st_mtime_ns)
        self.assertEqual(before_status, git(self.root, "status", "--porcelain"))

    def test_non_directory_conflict_preserves_partial_state(self) -> None:
        plan = self.plan()
        runtime_home = Path(plan["context"]["resources"]["runtime_home"])
        runtime_home.parent.mkdir(parents=True)
        runtime_home.write_text("conflict", encoding="utf-8")

        with self.assertRaises(workspace_context.ApplyError) as raised:
            workspace_context.apply_plan(
                self.root, plan["fingerprint"], self.environment
            )

        self.assertEqual("resource_not_directory", raised.exception.code)
        self.assertEqual("conflict", runtime_home.read_text(encoding="utf-8"))
        self.assertFalse(Path(plan["context"]["marker_path"]).exists())

    def test_cli_apply_requires_expected_fingerprint(self) -> None:
        environment = {**os.environ, **self.environment}
        completed = subprocess.run(
            [
                "python3",
                str(MODULE_PATH),
                "bootstrap",
                "apply",
                "--repo-root",
                str(self.root),
                "--expect",
                "0" * 64,
            ],
            check=False,
            capture_output=True,
            text=True,
            env=environment,
        )

        self.assertEqual(4, completed.returncode)
        self.assertIn("plan_fingerprint_mismatch", completed.stderr)
        self.assertFalse(self.home.exists())


if __name__ == "__main__":
    unittest.main()
