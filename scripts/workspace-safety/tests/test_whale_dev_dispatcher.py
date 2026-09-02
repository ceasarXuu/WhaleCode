from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CONTEXT_PATH = ROOT / "scripts/workspace-safety/workspace_context.py"
DISPATCHER_PATH = ROOT / "scripts/workspace-safety/whale_dev_dispatcher.py"


def load(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


context_api = load(CONTEXT_PATH, "whale_dev_context_fixture")
dispatcher = load(DISPATCHER_PATH, "whale_dev_dispatcher_fixture")


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo), *args], check=True, capture_output=True, text=True
    ).stdout.strip()


class WhaleDevDispatcherTest(unittest.TestCase):
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
            "WHALE_HOME": "/release/home",
            "CODEX_SQLITE_HOME": "/release/sqlite",
            "CODEX_HOME": "/codex/home",
        }
        self.root = self.base / "repo"
        self.root.mkdir()
        git(self.root, "init", "-q", "-b", "main")
        git(self.root, "config", "user.name", "Whale Dev Test")
        git(self.root, "config", "user.email", "whale-dev@example.invalid")
        (self.root / "README.md").write_text("fixture\n", encoding="utf-8")
        git(self.root, "add", "README.md")
        git(self.root, "commit", "-q", "-m", "initial")

    def bootstrap_binary(self, root: Path, label: str) -> dict[str, object]:
        plan = context_api.build_plan(root, self.environment)
        context_api.apply_plan(root, plan["fingerprint"], self.environment)
        context = context_api.resolve_context(root, self.environment)
        binary = Path(context["resources"]["binary_dir"]) / "whale"
        binary.write_text(
            "#!/usr/bin/env python3\n"
            "import json, os, sys\n"
            f"label = {label!r}\n"
            "if sys.argv[1:] == ['--version']:\n"
            "    print(f'whale {label}')\n"
            "else:\n"
            "    print(json.dumps({'label': label, 'cwd': os.getcwd(), "
            "'whale_home': os.environ.get('WHALE_HOME'), "
            "'sqlite_home': os.environ.get('CODEX_SQLITE_HOME'), "
            "'codex_home': os.environ.get('CODEX_HOME')}))\n",
            encoding="utf-8",
        )
        binary.chmod(0o700)
        attestation = {
            "schema_version": 2,
            "status": "pass",
            "repo_root": context["canonical_root"],
            "worktree_clean": True,
            "whale_bin": str(binary),
            "whale_binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        }
        Path(f"{binary}.build-attestation.json").write_text(
            json.dumps(attestation), encoding="utf-8"
        )
        return context

    def run_dispatcher(self, cwd: Path, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["python3", str(DISPATCHER_PATH), *args],
            cwd=cwd,
            env=self.environment,
            check=False,
            capture_output=True,
            text=True,
        )

    def test_version_routes_linked_worktrees_to_distinct_slots(self) -> None:
        git(self.root, "branch", "feature")
        linked = self.base / "linked"
        git(self.root, "worktree", "add", "-q", str(linked), "feature")
        root_context = self.bootstrap_binary(self.root, "root-build")
        linked_context = self.bootstrap_binary(linked, "linked-build")

        root_result = self.run_dispatcher(self.root, "--version")
        linked_result = self.run_dispatcher(linked, "--version")

        self.assertEqual(0, root_result.returncode, root_result.stderr)
        self.assertEqual(0, linked_result.returncode, linked_result.stderr)
        self.assertIn("whale-dev whale root-build", root_result.stdout)
        self.assertIn(str(root_context["workspace_id"]), root_result.stdout)
        self.assertIn("whale-dev whale linked-build", linked_result.stdout)
        self.assertIn(str(linked_context["workspace_id"]), linked_result.stdout)
        self.assertNotEqual(
            root_context["resources"]["binary_dir"],
            linked_context["resources"]["binary_dir"],
        )
        self.assertNotEqual(
            root_context["resources"]["runtime_home"],
            linked_context["resources"]["runtime_home"],
        )

    def test_launch_preserves_subdirectory_and_overrides_release_homes(self) -> None:
        context = self.bootstrap_binary(self.root, "root-build")
        subdirectory = self.root / "nested"
        subdirectory.mkdir()

        result = self.run_dispatcher(subdirectory, "probe")
        payload = json.loads(result.stdout)

        self.assertEqual(0, result.returncode, result.stderr)
        self.assertEqual(str(subdirectory), payload["cwd"])
        self.assertEqual(context["resources"]["runtime_home"], payload["whale_home"])
        self.assertEqual(context["resources"]["runtime_home"], payload["sqlite_home"])
        self.assertIsNone(payload["codex_home"])

    def test_outside_workspace_fails_without_path_fallback(self) -> None:
        outside = self.base / "outside"
        outside.mkdir()
        fallback_marker = self.base / "release-was-run"
        fallback_dir = self.base / "fallback-bin"
        fallback_dir.mkdir()
        release = fallback_dir / "whale"
        release.write_text(f"#!/bin/sh\ntouch {fallback_marker}\n", encoding="utf-8")
        release.chmod(0o700)
        self.environment["PATH"] = (
            f"{fallback_dir}{os.pathsep}{self.environment['PATH']}"
        )

        result = self.run_dispatcher(outside, "--version")

        self.assertEqual(2, result.returncode)
        self.assertIn("workspace_not_found", result.stderr)
        self.assertFalse(fallback_marker.exists())

    def test_stale_marker_and_invalid_attestation_fail_closed(self) -> None:
        context = self.bootstrap_binary(self.root, "root-build")
        git(self.root, "checkout", "-q", "-b", "other")
        stale = self.run_dispatcher(self.root, "--version")
        self.assertEqual(2, stale.returncode)
        self.assertIn("workspace_marker_stale", stale.stderr)

        git(self.root, "checkout", "-q", "main")
        binary = Path(context["resources"]["binary_dir"]) / "whale"
        binary.write_text("#!/bin/sh\necho tampered\n", encoding="utf-8")
        invalid = self.run_dispatcher(self.root, "--version")
        self.assertEqual(2, invalid.returncode)
        self.assertIn("workspace_attestation_invalid", invalid.stderr)


if __name__ == "__main__":
    unittest.main()
