from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "workspace_context.py"
SPEC = importlib.util.spec_from_file_location("workspace_context_exec", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
workspace_context = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(workspace_context)


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo), *args], check=True, capture_output=True, text=True
    ).stdout.strip()


class WorkspaceExecTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.home = self.base / "home"
        self.environment = {
            **os.environ,
            "HOME": str(self.home),
            "XDG_STATE_HOME": str(self.home / "state"),
            "XDG_DATA_HOME": str(self.home / "data"),
            "WHALE_HOME": "/legacy/whale",
            "CODEX_SQLITE_HOME": "/legacy/sqlite",
            "CODEX_HOME": "/official/codex",
            "FIXTURE_SECRET": "must-remain-in-child-not-log",
        }
        self.root = self.make_repo("repo", "main")
        self.bootstrap_with_binary(self.root)

    def make_repo(self, name: str, branch: str) -> Path:
        root = self.base / name
        root.mkdir()
        git(root, "init", "-q", "-b", branch)
        git(root, "config", "user.name", "Exec Test")
        git(root, "config", "user.email", "exec@example.invalid")
        (root / "README.md").write_text("fixture\n", encoding="utf-8")
        git(root, "add", "README.md")
        git(root, "commit", "-q", "-m", "initial")
        return root

    def bootstrap_with_binary(self, root: Path) -> dict[str, object]:
        plan = workspace_context.build_plan(root, self.environment)
        workspace_context.apply_plan(root, plan["fingerprint"], self.environment)
        context = workspace_context.resolve_context(root, self.environment)
        binary = Path(context["resources"]["binary_dir"]) / "whale"
        binary.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        binary.chmod(0o700)
        attestation = {
            "schema_version": 2,
            "status": "pass",
            "repo_root": context["canonical_root"],
            "worktree_clean": True,
            "whale_bin": str(binary),
            "whale_binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        }
        Path(f"{binary}.build-attestation.json").write_text(json.dumps(attestation))
        return context

    def test_child_environment_is_isolated_and_parent_is_unchanged(self) -> None:
        parent_before = dict(self.environment)
        captured = {}

        def executor(command, **kwargs):
            captured.update(kwargs)
            return subprocess.CompletedProcess(command, 0)

        exit_code = workspace_context.exec_ready(
            self.root, ["whale", "--version"], self.environment, executor=executor
        )
        context = workspace_context.resolve_context(self.root, self.environment)
        child = captured["env"]

        self.assertEqual(0, exit_code)
        self.assertEqual(context["resources"]["runtime_home"], child["WHALE_HOME"])
        self.assertEqual(context["resources"]["runtime_home"], child["CODEX_SQLITE_HOME"])
        self.assertNotIn("CODEX_HOME", child)
        self.assertEqual(context["resources"]["binary_dir"], child["PATH"].split(os.pathsep)[0])
        self.assertEqual("must-remain-in-child-not-log", child["FIXTURE_SECRET"])
        changed_keys = {
            key
            for key in set(parent_before) | set(child)
            if parent_before.get(key) != child.get(key)
        }
        self.assertEqual(workspace_context.workspace_runtime.MANAGED_ENV_KEYS, changed_keys)
        self.assertEqual(parent_before, self.environment)

    def test_stale_and_invalid_binary_never_launch(self) -> None:
        calls = []

        def executor(*args, **kwargs):
            calls.append((args, kwargs))
            return subprocess.CompletedProcess(args[0], 0)

        git(self.root, "checkout", "-q", "-b", "other")
        with self.assertRaises(workspace_context.ExecError) as stale:
            workspace_context.exec_ready(self.root, ["whale"], self.environment, executor=executor)
        self.assertEqual("workspace_not_ready", stale.exception.code)
        self.assertEqual([], calls)

        git(self.root, "checkout", "-q", "main")
        context = workspace_context.resolve_context(self.root, self.environment)
        Path(f"{context['resources']['binary_dir']}/whale.build-attestation.json").write_text("{}")
        with self.assertRaises(workspace_context.ExecError) as invalid:
            workspace_context.exec_ready(self.root, ["whale"], self.environment, executor=executor)
        self.assertEqual("workspace_not_ready", invalid.exception.code)
        self.assertEqual([], calls)

    def test_two_workspaces_write_to_distinct_runtime_homes(self) -> None:
        other = self.make_repo("other", "feature")
        other_context = self.bootstrap_with_binary(other)
        first_context = workspace_context.resolve_context(self.root, self.environment)
        script = "from pathlib import Path; import os; Path(os.environ['WHALE_HOME'],'probe').write_text(os.getcwd())"

        with ThreadPoolExecutor(max_workers=2) as pool:
            results = list(
                pool.map(
                    lambda root: workspace_context.exec_ready(
                        root, ["python3", "-c", script], self.environment
                    ),
                    [self.root, other],
                )
            )

        first_probe = Path(first_context["resources"]["runtime_home"]) / "probe"
        other_probe = Path(other_context["resources"]["runtime_home"]) / "probe"
        self.assertEqual([0, 0], results)
        self.assertNotEqual(first_probe, other_probe)
        self.assertNotEqual(first_probe.stat().st_ino, other_probe.stat().st_ino)
        self.assertEqual(str(self.root.resolve()), first_probe.read_text())
        self.assertEqual(str(other.resolve()), other_probe.read_text())

    def test_audit_log_never_records_command_or_environment_values(self) -> None:
        workspace_context.exec_ready(
            self.root, ["whale", "secret-command-argument"], self.environment
        )
        context = workspace_context.resolve_context(self.root, self.environment)
        log = (Path(context["resources"]["state_root"]) / "workspace-events.jsonl").read_text()

        self.assertNotIn("secret-command-argument", log)
        self.assertNotIn("must-remain-in-child-not-log", log)

    def test_cli_propagates_child_exit_code(self) -> None:
        completed = subprocess.run(
            [
                "python3", str(MODULE_PATH), "exec", "--repo-root", str(self.root),
                "--", "python3", "-c", "raise SystemExit(7)",
            ],
            check=False,
            env=self.environment,
        )
        self.assertEqual(7, completed.returncode)


if __name__ == "__main__":
    unittest.main()
