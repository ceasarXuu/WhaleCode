#!/usr/bin/env python3

from __future__ import annotations

import copy
import subprocess
import tempfile
import unittest
from pathlib import Path

from cache_evidence import (
    RESULT_SCHEMA_VERSION,
    canonical_json_sha256,
    evidence_manifest,
    expected_run_plan,
)
from cache_surface import load_contract, surface_snapshot, write_json
from cache_run_analysis import analyze_artifacts
from promote_cache_baseline import validate_promotion_result


PROMOTER = Path(__file__).resolve().parent / "promote_cache_baseline.py"


def run(*args: str, cwd: Path) -> str:
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


class PromoteCacheBaselineTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name)
        run("git", "init", "-q", cwd=self.repo)
        run("git", "config", "user.email", "test@example.com", cwd=self.repo)
        run("git", "config", "user.name", "Test", cwd=self.repo)
        (self.repo / "prompt").mkdir()
        (self.repo / "prompt/base.md").write_text("stable\n", encoding="utf-8")
        self.contract_path = (
            self.repo / "benchmarks/cache-regression/cache-surface-contract.json"
        )
        self.contract_path.parent.mkdir(parents=True)
        contract = {
            "schema_version": "whalecode-cache-surface-v1",
            "baseline": {
                "surface_sha256": "",
                "status": "live_regression_failed",
                "source_commit": "fixture",
                "live_result_path": None,
            },
            "surface_rules": [
                {"id": "prompt", "globs": ["prompt/**"], "reason": "prompt"}
            ],
            "live_regression": {
                "model": "deepseek-v4-flash",
                "sample": "fixture-sample",
                "arms": ["standard", "map-request"],
                "repeat": 1,
                "planned_sample_runs": 2,
                "automatic_retries": 0,
                "min_request_2_plus_count": 1,
                "min_trace_coverage": 1.0,
                "absolute_floor": {"standard": 0.85, "map-request": 0.75},
                "max_drop_from_live_baseline": 0.05,
            },
        }
        write_json(self.contract_path, contract)
        run("git", "add", ".", cwd=self.repo)
        surface, _ = surface_snapshot(self.repo, contract, "index")
        contract["baseline"]["surface_sha256"] = surface
        write_json(self.contract_path, contract)
        run("git", "add", ".", cwd=self.repo)
        run("git", "commit", "-qm", "fixture", cwd=self.repo)
        self.contract = load_contract(self.contract_path)
        self.head = run("git", "rev-parse", "HEAD", cwd=self.repo)
        self.result_path = (
            self.repo / "benchmarks/cache-regression/results/fixture-result.json"
        )
        self.result_path.parent.mkdir()
        self.result = self.make_result()
        write_json(self.result_path, self.result)
        self.write_ledger(self.result)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def make_arm(self, arm: str, side: str, hit_rate: float) -> dict:
        artifacts = self.repo / f"target/run/pair-001/{side}/artifacts"
        artifacts.mkdir(parents=True)
        cache = artifacts / "provider-cache-trace-summary.json"
        request = artifacts / "request-summary.json"
        metrics = artifacts / "metrics.json"
        request_2_plus_cached = round(hit_rate * 100)
        write_json(
            cache,
            {
                "provider_request_count": 3,
                "request_2_plus_count": 2,
                "request_2_plus_cached_input_tokens": request_2_plus_cached,
                "request_2_plus_uncached_input_tokens": 100 - request_2_plus_cached,
                "request_2_plus_hit_rate": hit_rate,
                "trace_coverage": 1.0,
                "cache_usage_missing_count": 0,
            },
        )
        write_json(
            request,
            {
                "rollout_trace": {
                    "input_tokens": 1000,
                    "cached_input_tokens": 900,
                    "output_tokens": 20,
                }
            },
        )
        write_json(
            metrics,
            {
                "logical_mode": "standard" if arm == "standard" else "taskspace",
                "business_success": True,
            },
        )
        result = analyze_artifacts(cache, request, metrics, arm)
        result["passed"] = True
        return result

    def make_result(self) -> dict:
        arms = [
            self.make_arm("standard", "left", 0.9),
            self.make_arm("map-request", "right", 0.8),
        ]
        plan = expected_run_plan(self.contract)
        return {
            "schema_version": RESULT_SCHEMA_VERSION,
            "record_id": "WAR-FIXTURE",
            "status": "pass",
            "subject_commit": self.head,
            "surface_sha256": self.contract["baseline"]["surface_sha256"],
            "authorization_reference": "fixture-approval",
            "run_plan": plan,
            "policy_sha256": canonical_json_sha256(self.contract["live_regression"]),
            "actual_sample_runs": 2,
            "arms": arms,
            "evidence_sha256": canonical_json_sha256(evidence_manifest(arms)),
            "result_path": str(self.result_path.relative_to(self.repo)),
        }

    def write_ledger(
        self, result: dict, authorization: str = "fixture-approval"
    ) -> None:
        plan = result["run_plan"]
        write_json(
            self.repo / "benchmarks/whale-agent-run-ledger.json",
            {
                "entries": [
                    {
                        "record_id": result["record_id"],
                        "status": "settled",
                        "authorization": {
                            "status": "granted",
                            "reference": authorization,
                        },
                        "execution": {
                            "model": plan["model"],
                            "sample_ids": [plan["sample"]],
                            "arm_ids": plan["arms"],
                            "repeats_per_arm_per_sample": plan["repeat"],
                            "planned_sample_runs": plan["planned_sample_runs"],
                            "actual_sample_runs": plan["planned_sample_runs"],
                        },
                        "evidence": {
                            "result_path": result["result_path"],
                            "subject_commit": result["subject_commit"],
                            "surface_sha256": result["surface_sha256"],
                        },
                    }
                ]
            },
        )

    def validate(self, result: dict) -> str:
        return validate_promotion_result(
            self.repo, self.contract, self.result_path, result
        )

    def test_accepts_complete_matching_evidence(self) -> None:
        self.assertEqual(self.validate(self.result), self.result["surface_sha256"])

    def test_cli_promotes_complete_matching_evidence(self) -> None:
        completed = subprocess.run(
            [
                "python3",
                str(PROMOTER),
                str(self.result_path),
                "--repo-root",
                str(self.repo),
            ],
            cwd=self.repo,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
        promoted = load_contract(self.contract_path)
        self.assertEqual(promoted["baseline"]["status"], "live_verified")
        self.assertEqual(
            promoted["baseline"]["source_commit"], self.result["subject_commit"]
        )

    def test_rejects_missing_arm(self) -> None:
        result = copy.deepcopy(self.result)
        result["arms"].pop()
        with self.assertRaisesRegex(ValueError, "sample count"):
            self.validate(result)

    def test_rejects_run_plan_mismatch(self) -> None:
        result = copy.deepcopy(self.result)
        result["run_plan"]["repeat"] = 2
        with self.assertRaisesRegex(ValueError, "run plan"):
            self.validate(result)

    def test_rejects_artifact_tampering(self) -> None:
        metrics = Path(self.result["arms"][0]["artifacts"]["metrics"])
        metrics.write_text("{}\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "artifact digest"):
            self.validate(self.result)

    def test_rejects_fabricated_metrics(self) -> None:
        result = copy.deepcopy(self.result)
        result["arms"][0]["request_2_plus_hit_rate"] = 1.0
        with self.assertRaisesRegex(ValueError, "artifact metrics"):
            self.validate(result)

    def test_rejects_fabricated_request_2_plus_tokens(self) -> None:
        result = copy.deepcopy(self.result)
        result["arms"][0]["request_2_plus_cached_input_tokens"] += 1
        with self.assertRaisesRegex(ValueError, "artifact metrics"):
            self.validate(result)

    def test_rejects_self_consistent_evidence_below_threshold(self) -> None:
        result = copy.deepcopy(self.result)
        cache_path = Path(result["arms"][0]["artifacts"]["cache_summary"])
        write_json(
            cache_path,
            {
                "provider_request_count": 3,
                "request_2_plus_count": 2,
                "request_2_plus_cached_input_tokens": 50,
                "request_2_plus_uncached_input_tokens": 50,
                "request_2_plus_hit_rate": 0.5,
                "trace_coverage": 1.0,
                "cache_usage_missing_count": 0,
            },
        )
        artifacts = result["arms"][0]["artifacts"]
        low_arm = analyze_artifacts(
            Path(artifacts["cache_summary"]),
            Path(artifacts["request_summary"]),
            Path(artifacts["metrics"]),
            "standard",
        )
        low_arm["passed"] = True
        result["arms"][0] = low_arm
        result["evidence_sha256"] = canonical_json_sha256(
            evidence_manifest(result["arms"])
        )
        with self.assertRaisesRegex(ValueError, "thresholds"):
            self.validate(result)

    def test_rejects_evidence_manifest_mismatch(self) -> None:
        result = copy.deepcopy(self.result)
        result["evidence_sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "evidence digest"):
            self.validate(result)

    def test_rejects_subject_mismatch(self) -> None:
        result = copy.deepcopy(self.result)
        result["subject_commit"] = "0" * 40
        with self.assertRaisesRegex(ValueError, "current HEAD"):
            self.validate(result)

    def test_rejects_ledger_authorization_mismatch(self) -> None:
        self.write_ledger(self.result, authorization="different-approval")
        with self.assertRaisesRegex(ValueError, "authorization"):
            self.validate(self.result)


if __name__ == "__main__":
    unittest.main()
