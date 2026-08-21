from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
CONTEXT_PATH = ROOT / "scripts/workspace-safety/workspace_context.py"
ENTRYPOINT_PATH = ROOT / "scripts/workspace-safety/workspace_entrypoint.py"


def load(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


context_api = load(CONTEXT_PATH, "entrypoint_context_fixture")
entrypoint = load(ENTRYPOINT_PATH, "workspace_entrypoint_fixture")


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


class WorkspaceEntrypointTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "repo"
        self.root.mkdir()
        git(self.root, "init", "-q", "-b", "main")
        git(self.root, "config", "user.name", "Entrypoint Test")
        git(self.root, "config", "user.email", "entrypoint@example.invalid")
        (self.root / "README.md").write_text("fixture\n")
        git(self.root, "add", "README.md")
        git(self.root, "commit", "-q", "-m", "initial")
        self.environment = {
            "HOME": str(Path(self.temp.name) / "home"),
            "XDG_STATE_HOME": str(Path(self.temp.name) / "state"),
            "XDG_DATA_HOME": str(Path(self.temp.name) / "data"),
        }

    def bootstrap_binary(self) -> Path:
        plan = context_api.build_plan(self.root, self.environment)
        context_api.apply_plan(self.root, plan["fingerprint"], self.environment)
        context = context_api.resolve_context(self.root, self.environment)
        binary = Path(context["resources"]["binary_dir"]) / "whale"
        binary.write_text("#!/bin/sh\necho whale fixture\n")
        binary.chmod(0o700)
        attestation = {
            "schema_version": 2,
            "status": "pass",
            "repo_root": str(self.root.resolve()),
            "worktree_clean": True,
            "whale_bin": str(binary),
            "whale_binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        }
        Path(f"{binary}.build-attestation.json").write_text(json.dumps(attestation))
        return binary

    def test_default_resolves_only_ready_attested_slot(self) -> None:
        with patch.dict(os.environ, self.environment, clear=False):
            with self.assertRaisesRegex(entrypoint.WorkspacePreflightError, "workspace_not_ready"):
                entrypoint.resolve_workspace_binary(self.root, None)
            binary = self.bootstrap_binary()
            self.assertEqual(binary, entrypoint.resolve_workspace_binary(self.root, None))

    def test_explicit_binary_cannot_escape_workspace_slot(self) -> None:
        with patch.dict(os.environ, self.environment, clear=False):
            self.bootstrap_binary()
            with self.assertRaisesRegex(
                entrypoint.WorkspacePreflightError, "outside_workspace_slot"
            ):
                entrypoint.resolve_workspace_binary(self.root, Path("/other/whale"))


if __name__ == "__main__":
    unittest.main()
