from __future__ import annotations

import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "workspace_inventory.py"
SPEC = importlib.util.spec_from_file_location("workspace_inventory", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
workspace_inventory = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(workspace_inventory)


def git(repo: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(repo), *args],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return completed.stdout.strip()


class WorkspaceInventoryTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "repo"
        self.root.mkdir()
        git(self.root, "init", "-q", "-b", "main")
        git(self.root, "config", "user.name", "Inventory Test")
        git(self.root, "config", "user.email", "inventory@example.invalid")
        (self.root / "README.md").write_text("# Fixture\n", encoding="utf-8")
        git(self.root, "add", "README.md")
        git(self.root, "commit", "-q", "-m", "initial")

    def _add_fixture_surfaces(self) -> None:
        rust_root = self.root / "nested" / "rust"
        rust_root.mkdir(parents=True)
        (rust_root / "Cargo.toml").write_text(
            '[workspace]\nresolver = "2"\nmembers = []\n', encoding="utf-8"
        )
        bazel_root = self.root / "vendor"
        bazel_root.mkdir()
        (bazel_root / "MODULE.bazel").write_text(
            'module(name = "fixture")\n', encoding="utf-8"
        )
        scripts = self.root / "scripts"
        scripts.mkdir()
        (scripts / "run_agent.py").write_text(
            '#!/usr/bin/env python3\n'
            'from pathlib import Path\n'
            'binary = Path.home() / ".whale/bin/whale"\n'
            'key_name = "DEEPSEEK_API_KEY"\n'
            'if __name__ == "__main__":\n'
            '    print(binary, key_name)\n',
            encoding="utf-8",
        )
        (scripts / "test_agent.py").write_text(
            '#!/usr/bin/env python3\nkey_name = "DEEPSEEK_API_KEY"\n',
            encoding="utf-8",
        )

    def test_collects_build_roots_and_sensitive_entrypoints(self) -> None:
        self._add_fixture_surfaces()
        before = sorted(str(path.relative_to(self.root)) for path in self.root.rglob("*"))
        result = workspace_inventory.collect_inventory(self.root)
        after = sorted(str(path.relative_to(self.root)) for path in self.root.rglob("*"))

        self.assertEqual(before, after)
        self.assertEqual(1, result["schema_version"])
        self.assertEqual(
            ["nested/rust/Cargo.toml", "vendor/MODULE.bazel"],
            [item["manifest"] for item in result["build_roots"]],
        )
        entry = next(item for item in result["entrypoints"] if item["path"] == "scripts/run_agent.py")
        self.assertEqual("possible", entry["model_request_risk"])
        self.assertIn("legacy-user-slot", entry["binary_resolution"])
        self.assertEqual(
            {"legacy-whale-binary", "model-credential"},
            set(entry["matched_rule_ids"]),
        )
        self.assertNotIn("scripts/test_agent.py", {item["path"] for item in result["entrypoints"]})

    def test_linked_worktree_shares_common_dir_but_not_git_dir(self) -> None:
        linked = Path(self.temp.name) / "linked"
        git(self.root, "branch", "feature")
        git(self.root, "worktree", "add", "-q", str(linked), "feature")

        main = workspace_inventory.collect_inventory(self.root)["repository"]
        worktree = workspace_inventory.collect_inventory(linked)["repository"]

        self.assertFalse(main["linked_worktree"])
        self.assertTrue(worktree["linked_worktree"])
        self.assertEqual(main["git_common_dir"], worktree["git_common_dir"])
        self.assertNotEqual(main["git_dir"], worktree["git_dir"])
        self.assertEqual("feature", worktree["branch"])

    def test_remote_credentials_and_query_are_redacted(self) -> None:
        git(
            self.root,
            "remote",
            "add",
            "origin",
            "https://user:secret@example.com/org/repo.git?token=hidden",
        )
        rendered = workspace_inventory.render(
            workspace_inventory.collect_inventory(self.root)
        )
        document = json.loads(rendered)

        self.assertNotIn("user", rendered)
        self.assertNotIn("secret", rendered)
        self.assertNotIn("token", rendered)
        self.assertEqual(
            "https://example.com/org/repo.git",
            document["repository"]["remotes"][0]["endpoints"][0]["endpoint"],
        )

    def test_schema_contract_tracks_top_level_document(self) -> None:
        schema_path = MODULE_PATH.parent / "schemas" / "workspace-inventory.schema.json"
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
        self._add_fixture_surfaces()
        document = workspace_inventory.collect_inventory(self.root)

        self.assertEqual(schema["properties"]["schema_version"]["const"], document["schema_version"])
        self.assertEqual(set(schema["required"]), set(document))
        self.assertEqual(
            set(schema["properties"]["repository"]["required"]),
            set(document["repository"]),
        )
        for key in ("build_roots", "entrypoints", "shared_resource_references"):
            item_schema = schema["properties"][key]["items"]
            for item in document[key]:
                self.assertEqual(set(item_schema["required"]), set(item))
        self.assertEqual(
            set(schema["properties"]["summary"]["required"]),
            set(document["summary"]),
        )


if __name__ == "__main__":
    unittest.main()
