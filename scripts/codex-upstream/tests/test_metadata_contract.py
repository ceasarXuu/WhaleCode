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
    validate_candidate,
    validate_backlog,
    validate_inventory,
    validate_ledger,
    validate_ledger_paths,
    validate_patch_digest,
    validate_replay_ledger,
    validate_tui_baseline,
    validate_upstream_delta,
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
        cls.tui_baseline = json.loads((topic / "tui-baseline.json").read_text())
        cls.candidate_path = topic / "upstream-candidate.json"
        cls.delta_path = topic / "upstream-delta-inventory.json"
        cls.replay_path = topic / "overlay-replay-ledger.json"

    def test_current_documents_are_structurally_valid(self) -> None:
        self.assertEqual([], validate_inventory(self.inventory))
        self.assertEqual([], validate_ledger(self.ledger))
        self.assertEqual([], validate_backlog(self.backlog))
        self.assertEqual([], validate_tui_baseline(self.tui_baseline))
        if self.candidate_path.exists():
            self.assertEqual([], validate_candidate(json.loads(self.candidate_path.read_text())))
        if self.delta_path.exists():
            self.assertEqual([], validate_upstream_delta(json.loads(self.delta_path.read_text())))
        if self.replay_path.exists():
            self.assertEqual([], validate_replay_ledger(json.loads(self.replay_path.read_text())))

    def test_candidate_rejects_absolute_evidence_path(self) -> None:
        document = {
            "schema_version": 1,
            "release_tag": "rust-v0.147.0",
            "commit_sha": "a" * 40,
            "tree_sha": "b" * 40,
            "release_date": "2026-08-07",
            "license_path": "LICENSE",
            "license_sha256": "c" * 64,
            "source_method": "git-archive",
            "source_object_verified": True,
            "toolchain": {"rustc": "x", "cargo": "y", "nextest": "z"},
            "qualification_commands": [{"id": "fmt", "command": ["cargo", "fmt"], "cwd": "codex-rs", "environment": {"INSTA_UPDATE": "no", "NEXTEST_PROFILE": "local", "RUST_MIN_STACK": "8388608"}, "result": "passed", "exit_code": 0, "evidence": "/tmp/log"}],
            "production_vendor_unchanged": True,
            "model_request_count": 0,
            "summary": {"command_count": 1, "by_result": {"passed": 1}},
        }
        self.assertTrue(any("evidence must be relative" in error for error in validate_candidate(document)))

    def test_delta_summary_tamper_is_rejected(self) -> None:
        document = {
            "schema_version": 1,
            "baseline_commit": "a" * 40,
            "baseline_tree": "b" * 40,
            "target_commit": "c" * 40,
            "target_tree": "d" * 40,
            "source": "git-tree",
            "entries": [],
            "summary": {"path_count": 1, "by_status": {}, "by_crate_owner": {}, "by_generated_kind": {}},
        }
        self.assertIn("upstream delta summary does not match entries", validate_upstream_delta(document))

    def test_regenerate_requires_lineage(self) -> None:
        document = {
            "schema_version": 1,
            "baseline_commit": "a" * 40,
            "target_commit": "b" * 40,
            "overlay_tree": "c" * 40,
            "cutover_batches": [
                {"id": "final-generated-release", "depends_on": []}
            ],
            "entries": [{
                "path": "schema.json",
                "categories": ["generated_artifact"],
                "disposition": "regenerate",
                "decision_basis": ["generated"],
                "verification": ["generator check"],
                "cutover_batch": "final-generated-release",
                "owner_domain": "generated",
                "depends_on": [],
                "generated_lineage": None,
            }],
            "summary": {"path_count": 1, "by_disposition": {"regenerate": 1}, "by_cutover_batch": {"final-generated-release": 1}},
        }
        self.assertTrue(any("requires generated_lineage" in error for error in validate_replay_ledger(document)))

    def test_replay_batch_cycle_is_rejected(self) -> None:
        document = {
            "schema_version": 1,
            "baseline_commit": "a" * 40,
            "target_commit": "b" * 40,
            "overlay_tree": "c" * 40,
            "cutover_batches": [
                {"id": "a", "depends_on": ["b"]},
                {"id": "b", "depends_on": ["a"]},
            ],
            "entries": [],
            "summary": {
                "path_count": 0,
                "by_disposition": {},
                "by_cutover_batch": {},
            },
        }
        self.assertTrue(
            any("dependency cycle" in error for error in validate_replay_ledger(document))
        )

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

    def test_tui_summary_tamper_is_rejected(self) -> None:
        document = copy.deepcopy(self.tui_baseline)
        document["summary"]["test_count"] += 1
        errors = validate_tui_baseline(document)
        self.assertIn("TUI baseline summary does not match entries", errors)


if __name__ == "__main__":
    unittest.main()
