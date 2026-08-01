from __future__ import annotations

import copy
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_DIR))

from classification import classify  # noqa: E402
from git_snapshot import diff_stats  # noqa: E402
from metadata_contract import (  # noqa: E402
    validate_backlog,
    validate_inventory,
    validate_ledger,
    validate_ledger_paths,
    validate_patch_digest,
)


class ClassificationTests(unittest.TestCase):
    def test_taskspace_host_hook_uses_changed_lines(self) -> None:
        before = b"stable TaskSpace mention\nfn old() {}\n"
        after = b"stable TaskSpace mention\nfn new() {}\n"
        result = classify("codex-rs/core/src/util.rs", before, after, set())
        self.assertNotIn("taskspace_host_hooks", result.categories)

    def test_taskspace_domain_path_wins_over_host_hook(self) -> None:
        result = classify(
            "codex-rs/core/src/action_map/model.rs",
            None,
            b"pub struct ActionMap;\n",
            set(),
        )
        self.assertIn("taskspace_domain", result.categories)
        self.assertNotIn("taskspace_host_hooks", result.categories)

    def test_backport_is_multi_label(self) -> None:
        path = "codex-rs/tui/src/wrapping.rs"
        result = classify(path, b"old\n", b"new\n", {path})
        self.assertIn("upstream_backport", result.categories)
        self.assertIn("tui_experience", result.categories)


class GitSnapshotTests(unittest.TestCase):
    def test_diff_stats_preserves_tabs_and_newlines_in_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            repo = Path(temp_dir)
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
            subprocess.run(
                ["git", "-C", str(repo), "config", "user.email", "test@example.com"],
                check=True,
            )
            subprocess.run(
                ["git", "-C", str(repo), "config", "user.name", "Test"], check=True
            )
            odd_path = repo / "odd\tname\n.rs"
            odd_path.write_text("before\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", "."], check=True)
            subprocess.run(
                ["git", "-C", str(repo), "commit", "-qm", "baseline"], check=True
            )
            baseline = subprocess.check_output(
                ["git", "-C", str(repo), "rev-parse", "HEAD^{tree}"], text=True
            ).strip()
            odd_path.write_text("after\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", "."], check=True)
            current = subprocess.check_output(
                ["git", "-C", str(repo), "write-tree"], text=True
            ).strip()
            result = diff_stats(repo, baseline, current)
            self.assertEqual(["odd\tname\n.rs"], list(result))
            self.assertEqual("modified", result["odd\tname\n.rs"].status)


class ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        repo = SCRIPT_DIR.parents[1]
        topic = repo / "docs/v0.0.5/codex-upstream-sync"
        cls.inventory = json.loads((topic / "overlay-inventory.json").read_text())
        cls.ledger = json.loads((topic / "backport-ledger.json").read_text())
        cls.backlog = json.loads(
            (topic / "backport-provenance-backlog.json").read_text()
        )

    def test_current_documents_are_structurally_valid(self) -> None:
        self.assertEqual([], validate_inventory(self.inventory))
        self.assertEqual([], validate_ledger(self.ledger))
        self.assertEqual([], validate_backlog(self.backlog))

    def test_duplicate_active_upstream_is_rejected(self) -> None:
        document = copy.deepcopy(self.ledger)
        document["entries"].append(copy.deepcopy(document["entries"][0]))
        errors = validate_ledger(document)
        self.assertTrue(any("duplicate active" in error for error in errors))

    def test_inventory_digest_tamper_is_rejected(self) -> None:
        document = copy.deepcopy(self.inventory)
        document["entries"][0]["current_sha256"] = "bad"
        errors = validate_inventory(document)
        self.assertTrue(any("current_sha256" in error for error in errors))

    def test_inventory_summary_tamper_is_rejected(self) -> None:
        document = copy.deepcopy(self.inventory)
        document["summary"]["path_count"] += 1
        errors = validate_inventory(document)
        self.assertIn("inventory summary does not match entries", errors)

    def test_patch_digest_tamper_is_rejected(self) -> None:
        entry = {
            "upstream_commit": "a" * 40,
            "patch_sha256": "0" * 64,
        }
        errors = validate_patch_digest(entry, b"official patch")
        self.assertEqual([f"patch digest mismatch for {'a' * 40}"], errors)

    def test_missing_local_and_upstream_paths_are_rejected(self) -> None:
        entry = {
            "upstream_commit": "a" * 40,
            "paths": ["local.rs"],
            "upstream_paths": ["upstream.rs"],
        }
        errors = validate_ledger_paths(entry, set(), set(), set())
        self.assertEqual(2, len(errors))
        self.assertTrue(any("upstream commit tree" in error for error in errors))
        self.assertTrue(any("baseline and current tree" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
