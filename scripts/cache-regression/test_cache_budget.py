#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from cache_budget import build_budget_proposal, validate_budget_proposal
from cache_surface import load_contract, surface_snapshot, write_json


CLI = Path(__file__).resolve().parent / "propose_cache_regression_budget.py"


def run(*args: str, cwd: Path) -> str:
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


class CacheBudgetProposalTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name)
        run("git", "init", "-q", cwd=self.repo)
        run("git", "config", "user.email", "test@example.com", cwd=self.repo)
        run("git", "config", "user.name", "Test", cwd=self.repo)
        (self.repo / "prompt").mkdir()
        (self.repo / "prompt/base.md").write_text("stable\n", encoding="utf-8")
        contract_path = (
            self.repo / "benchmarks/cache-regression/cache-surface-contract.json"
        )
        contract_path.parent.mkdir(parents=True)
        write_json(
            contract_path,
            {
                "schema_version": "whalecode-cache-surface-v1",
                "baseline": {
                    "surface_sha256": "fixture",
                    "status": "live_regression_failed",
                },
                "surface_rules": [
                    {"id": "prompt", "globs": ["prompt/**"], "reason": "prompt"}
                ],
                "pricing_snapshot": {
                    "currency": "USD",
                    "uncached_input_per_million": 0.14,
                    "output_per_million": 0.28,
                },
            },
        )
        self.report_path = self.repo / "benchmarks/cache-regression/change-report.json"
        write_json(
            self.report_path,
            {
                "schema_version": "whalecode-cache-regression-gate-v1",
                "status": "blocked",
                "discovery_state": "changed",
                "free_validation": {
                    "passed": False,
                    "commands": [{"id": "final_wire_matrix", "status": "fail"}],
                },
            },
        )
        run("git", "add", ".", cwd=self.repo)
        run("git", "commit", "-qm", "fixture", cwd=self.repo)
        self.contract = load_contract(contract_path)
        self.head = run("git", "rev-parse", "HEAD", cwd=self.repo)
        report = json.loads(self.report_path.read_text(encoding="utf-8"))
        report["subject_commit"] = self.head
        report["actual_surface_sha256"] = surface_snapshot(
            self.repo, self.contract, "worktree"
        )[0]
        write_json(self.report_path, report)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def proposal(self) -> dict:
        return build_budget_proposal(
            repo=self.repo,
            contract=self.contract,
            gate_report_path=self.report_path,
            gate_report=json.loads(self.report_path.read_text(encoding="utf-8")),
            subject_commit=self.head,
            model="deepseek-v4-flash",
            samples=["simple", "complex"],
            arms=["standard", "map-request"],
            repeat=2,
            retry_sample_run_limit=1,
            max_provider_requests_per_run=10,
            max_input_tokens_per_run=100_000,
            max_output_tokens_per_run=5_000,
            max_seconds_per_run=120,
            stop_conditions=["after_any_business_failure"],
            selection_reason="human selected representative paths",
        )

    def test_computes_declared_maximums_without_cache_discount(self) -> None:
        proposal = self.proposal()
        validate_budget_proposal(proposal)
        self.assertEqual(proposal["selection"]["planned_sample_runs"], 8)
        self.assertEqual(proposal["selection"]["maximum_sample_runs"], 9)
        self.assertEqual(proposal["maximums"]["provider_requests"], 90)
        self.assertEqual(proposal["maximums"]["input_tokens"], 900_000)
        self.assertEqual(proposal["maximums"]["output_tokens"], 45_000)
        self.assertEqual(proposal["maximums"]["elapsed_seconds"], 1080)
        self.assertEqual(proposal["maximums"]["estimated_cost"], 0.1386)

    def test_rejects_unblocked_or_uncomparable_gate(self) -> None:
        report = json.loads(self.report_path.read_text(encoding="utf-8"))
        report["free_validation"]["passed"] = True
        with self.assertRaisesRegex(ValueError, "did not detect"):
            build_budget_proposal(**{**self._kwargs(), "gate_report": report})
        report["free_validation"]["passed"] = False
        report["discovery_state"] = "uncomparable"
        with self.assertRaisesRegex(ValueError, "comparable"):
            build_budget_proposal(**{**self._kwargs(), "gate_report": report})

    def test_accepts_explicit_clean_revalidation_without_changed_scenarios(
        self,
    ) -> None:
        report = json.loads(self.report_path.read_text(encoding="utf-8"))
        report.update(
            {
                "discovery_state": "revalidation_requested",
                "baseline_status": "live_regression_failed",
                "revalidation_requested": True,
                "require_live_baseline": True,
                "require_clean_subject": True,
                "sensitive_changes": [],
            }
        )
        report["free_validation"]["passed"] = True
        report["free_validation"]["commands"][0]["status"] = "pass"
        proposal = build_budget_proposal(**{**self._kwargs(), "gate_report": report})
        self.assertEqual(proposal["trigger"]["failed_free_commands"], [])

    def test_rejects_gate_report_from_another_surface(self) -> None:
        report = json.loads(self.report_path.read_text(encoding="utf-8"))
        report["actual_surface_sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "does not match"):
            build_budget_proposal(**{**self._kwargs(), "gate_report": report})

    def test_rejects_unknown_arm_and_duplicate_selection(self) -> None:
        with self.assertRaisesRegex(ValueError, "unsupported arm"):
            build_budget_proposal(**{**self._kwargs(), "arms": ["unknown"]})
        with self.assertRaisesRegex(ValueError, "duplicates"):
            build_budget_proposal(**{**self._kwargs(), "samples": ["simple", "simple"]})
        with self.assertRaisesRegex(ValueError, "unsupported stop condition"):
            build_budget_proposal(
                **{**self._kwargs(), "stop_conditions": ["interpret this prose"]}
            )

    def test_digest_detects_plan_tampering(self) -> None:
        proposal = self.proposal()
        proposal["selection"]["repeat"] = 99
        with self.assertRaisesRegex(ValueError, "digest"):
            validate_budget_proposal(proposal)

    def test_cli_has_no_key_or_ledger_side_effect(self) -> None:
        ledger = self.repo / "benchmarks/whale-agent-run-ledger.json"
        before = set(self.repo.rglob("*"))
        with patch.dict(os.environ, {}, clear=True):
            completed = subprocess.run(
                [
                    "python3",
                    str(CLI),
                    "--repo-root",
                    str(self.repo),
                    "--gate-report",
                    str(self.report_path),
                    "--model",
                    "deepseek-v4-flash",
                    "--sample",
                    "simple",
                    "--arm",
                    "map-request",
                    "--repeat",
                    "1",
                    "--max-provider-requests-per-run",
                    "10",
                    "--max-input-tokens-per-run",
                    "100000",
                    "--max-output-tokens-per-run",
                    "5000",
                    "--max-seconds-per-run",
                    "120",
                    "--stop-condition",
                    "after_any_run_failure",
                    "--selection-reason",
                    "human choice",
                ],
                cwd=self.repo,
                text=True,
                capture_output=True,
                check=False,
            )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(
            json.loads(completed.stdout)["selection"]["maximum_sample_runs"], 1
        )
        self.assertFalse(ledger.exists())
        self.assertEqual(set(self.repo.rglob("*")), before)

    def _kwargs(self) -> dict:
        return {
            "repo": self.repo,
            "contract": self.contract,
            "gate_report_path": self.report_path,
            "gate_report": json.loads(self.report_path.read_text(encoding="utf-8")),
            "subject_commit": self.head,
            "model": "deepseek-v4-flash",
            "samples": ["simple"],
            "arms": ["map-request"],
            "repeat": 1,
            "retry_sample_run_limit": 0,
            "max_provider_requests_per_run": 10,
            "max_input_tokens_per_run": 100_000,
            "max_output_tokens_per_run": 5_000,
            "max_seconds_per_run": 120,
            "stop_conditions": ["after_any_run_failure"],
            "selection_reason": "human choice",
        }


if __name__ == "__main__":
    unittest.main()
