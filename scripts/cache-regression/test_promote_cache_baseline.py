#!/usr/bin/env python3

from __future__ import annotations

import copy
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from cache_budget import build_budget_proposal
from cache_baseline_test_support import write_settled_ledger
from cache_evidence import RESULT_SCHEMA_VERSION, canonical_json_sha256, file_sha256
from cache_run_analysis import analyze_artifacts
from cache_run_contract import AUTHORIZATION_SCHEMA_VERSION, execution_matrix
from cache_surface import load_contract, surface_snapshot, write_json
from accepted_cache_baseline import ACCEPTANCE_SCHEMA_VERSION, changed_scenarios
from promote_cache_baseline import (
    promote,
    validate_promotion,
)


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
        (self.repo / "prompt/base.md").write_text("changed product\n", encoding="utf-8")
        runner = self.repo / "scripts/taskspace-benchmark/run-taskspace-benchmark.ps1"
        runner.parent.mkdir(parents=True)
        runner.write_text("runner\n", encoding="utf-8")
        scenario = self.repo / "benchmarks/taskspace/scenarios/simple"
        scenario.mkdir(parents=True)
        (scenario / "scenario.json").write_text("{}\n", encoding="utf-8")
        self.snapshot_path = self.repo / "snapshots/final_wire__changed.snap"
        self.snapshot_path.parent.mkdir()
        self.before_payload = {"request_1": {"input": ["before"]}}
        self.after_payload = {"request_1": {"input": ["after"]}}
        self.snapshot_path.write_text(
            "---\nsource: fixture.rs\n---\n"
            + json.dumps(self.before_payload, indent=2)
            + "\n",
            encoding="utf-8",
        )
        self.contract_path = (
            self.repo / "benchmarks/cache-regression/cache-surface-contract.json"
        )
        self.contract_path.parent.mkdir(parents=True)
        self.pricing = {
            "currency": "USD",
            "cached_input_per_million": 0.0028,
            "uncached_input_per_million": 0.14,
            "output_per_million": 0.28,
        }
        write_json(
            self.contract_path,
            {
                "schema_version": "whalecode-cache-surface-v1",
                "baseline": {
                    "surface_sha256": "old",
                    "status": "live_regression_failed",
                },
                "surface_rules": [
                    {"id": "prompt", "globs": ["prompt/**"], "reason": "prompt"}
                ],
                "free_validation": {"semantic_baseline_globs": ["snapshots/*.snap"]},
                "pricing_snapshot": self.pricing,
                "provider_hard_limits": {
                    "deepseek-v4-flash": {
                        "max_input_tokens_per_request": 1_000_000,
                        "max_output_tokens_per_request": 384_000,
                    }
                },
            },
        )
        self.gate_path = self.repo / "benchmarks/cache-regression/gate.json"
        self.scenario = {
            "scenario_id": "changed",
            "comparison_object": "normalized_final_wire_snapshot",
            "baseline_path": "snapshots/final_wire__changed.snap",
            "status": "changed",
            "first_difference": "/request_1/input/0",
            "before_payload_sha256": canonical_json_sha256(self.before_payload),
            "after_payload_sha256": canonical_json_sha256(self.after_payload),
            "candidate_payload": self.after_payload,
        }
        write_json(
            self.gate_path,
            {
                "schema_version": "whalecode-cache-regression-gate-v1",
                "status": "blocked",
                "discovery_state": "changed",
                "free_validation": {
                    "passed": False,
                    "commands": [
                        {
                            "id": "wire",
                            "status": "fail",
                            "change_report": {
                                "status": "changed",
                                "scenarios": [self.scenario],
                            },
                        }
                    ],
                },
            },
        )
        run("git", "add", ".", cwd=self.repo)
        run("git", "commit", "-qm", "subject", cwd=self.repo)
        self.head = run("git", "rev-parse", "HEAD", cwd=self.repo)
        self.contract = load_contract(self.contract_path)
        gate = json.loads(self.gate_path.read_text(encoding="utf-8"))
        gate["subject_commit"] = self.head
        gate["actual_surface_sha256"] = surface_snapshot(
            self.repo, self.contract, "worktree"
        )[0]
        write_json(self.gate_path, gate)
        self.proposal = build_budget_proposal(
            repo=self.repo,
            contract=self.contract,
            gate_report_path=self.gate_path,
            gate_report=json.loads(self.gate_path.read_text(encoding="utf-8")),
            subject_commit=self.head,
            model="deepseek-v4-flash",
            samples=["simple"],
            arms=["standard", "map-request"],
            repeat=1,
            retry_sample_run_limit=0,
            max_provider_requests_per_run=10,
            observed_input_tokens_per_run=100_000,
            observed_output_tokens_per_run=5_000,
            max_seconds_per_run=120,
            stop_conditions=["after_any_run_failure"],
            selection_reason="human selected smoke",
        )
        self.proposal_path = self.repo / "benchmarks/cache-regression/proposal.json"
        write_json(self.proposal_path, self.proposal)
        self.authorization = {
            "schema_version": AUTHORIZATION_SCHEMA_VERSION,
            "status": "granted",
            "approved_by": "user",
            "authorization_id": "CBA-FIXTURE-001",
            "approval_reference": "fixture run approval",
            "approved_at": self.proposal["created_at"],
            "proposal_id": self.proposal["proposal_id"],
            "proposal_sha256": self.proposal["proposal_sha256"],
            "approved_selection": self.proposal["selection"],
            "approved_maximums": self.proposal["maximums"],
        }
        self.authorization_path = (
            self.repo / "benchmarks/cache-regression/authorization.json"
        )
        write_json(self.authorization_path, self.authorization)
        observations = [
            self.make_observation("standard", "left"),
            self.make_observation("map-request", "right"),
        ]
        evidence = [
            {
                "sample": item["sample"],
                "arm": item["arm"],
                "repeat": item["repeat"],
                "artifact_sha256": item["artifact_sha256"],
            }
            for item in observations
        ]
        self.result = {
            "schema_version": RESULT_SCHEMA_VERSION,
            "record_id": "WAR-FIXTURE",
            "status": "completed",
            "started_at": self.proposal["created_at"],
            "ended_at": self.proposal["created_at"],
            "elapsed_seconds": 2.0,
            "subject_commit": self.head,
            "surface_sha256": self.proposal["surface_sha256"],
            "proposal_id": self.proposal["proposal_id"],
            "proposal_sha256": self.proposal["proposal_sha256"],
            "authorization_reference": self.authorization["approval_reference"],
            "authorization_sha256": file_sha256(self.authorization_path),
            "observed_scope": self.proposal["selection"],
            "unverified_scope": [],
            "actual_sample_runs": 2,
            "credential_source": "fixture",
            "run_root": "target/cache-fixture",
            "observations": observations,
            "attempts": [
                {
                    **scope,
                    "run_id": f"CACHE-{index:03d}",
                    "status": "completed",
                    "exit_code": 0,
                    "timed_out": False,
                    "elapsed_seconds": 1.0,
                    "post_run_cleanup": {
                        "status": "verified_absent",
                        "container_ids": [],
                        "stable_empty_polls": 3,
                        "error": "",
                    },
                }
                for index, scope in enumerate(execution_matrix(self.proposal), start=1)
            ],
            "evidence_sha256": canonical_json_sha256(evidence),
        }
        self.result_path = self.repo / "benchmarks/cache-regression/results/result.json"
        self.result_path.parent.mkdir()
        self.result["result_path"] = self.result_path.relative_to(self.repo).as_posix()
        self.result["runner_exit_code"] = 0
        write_json(self.result_path, self.result)
        self.write_ledger()
        self.acceptance = self.make_acceptance()
        self.acceptance_path = self.repo / "benchmarks/cache-regression/acceptance.json"
        write_json(self.acceptance_path, self.acceptance)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def make_observation(self, arm: str, side: str) -> dict:
        del side
        run_id = "CACHE-001" if arm == "standard" else "CACHE-002"
        artifacts = self.repo / (
            f"benchmarks/cache-regression/evidence/WAR-FIXTURE/{run_id}"
        )
        artifacts.mkdir(parents=True)
        cache = artifacts / "provider-cache-trace-summary.json"
        request = artifacts / "request-summary.json"
        metrics = artifacts / "metrics.json"
        write_json(
            cache,
            {
                "provider_request_count": 3,
                "request_2_plus_count": 2,
                "request_2_plus_cached_input_tokens": 90,
                "request_2_plus_uncached_input_tokens": 10,
                "request_2_plus_hit_rate": 0.9,
                "trace_coverage": 1.0,
                "cache_usage_missing_count": 0,
            },
        )
        write_json(
            request,
            {
                "rollout_trace": {
                    "input_tokens": 100,
                    "cached_input_tokens": 90,
                    "output_tokens": 10,
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
        observation = analyze_artifacts(cache, request, metrics, arm)
        observation["artifacts"] = {
            key: Path(path).relative_to(self.repo).as_posix()
            for key, path in observation["artifacts"].items()
        }
        observation.update(
            {
                "sample": "simple",
                "repeat": 1,
                "elapsed_seconds": 1.0,
                "budget_observation_exceeded": [],
                "run_id": run_id,
            }
        )
        return observation

    def test_rejects_standard_artifacts_relabelled_as_taskspace(self) -> None:
        forged = copy.deepcopy(self.result["observations"][0])
        forged.update(
            {
                "arm": "map-request",
                "run_id": self.result["observations"][1]["run_id"],
            }
        )
        self.result["observations"][1] = forged
        self.result["evidence_sha256"] = canonical_json_sha256(
            [
                {
                    **scope,
                    "artifact_sha256": observation["artifact_sha256"],
                }
                for scope, observation in zip(
                    execution_matrix(self.proposal), self.result["observations"]
                )
            ]
        )
        write_json(self.result_path, self.result)
        self.acceptance = self.make_acceptance()
        write_json(self.acceptance_path, self.acceptance)
        self.write_ledger()

        with self.assertRaisesRegex(ValueError, "not durable|logical_mode"):
            self.validate(result=self.result, acceptance=self.acceptance)

    def write_ledger(self) -> None:
        write_settled_ledger(
            self.repo,
            self.result,
            self.proposal,
            self.authorization,
            self.result_path,
            self.proposal_path,
            self.authorization_path,
            api_requests=6,
            input_tokens=200,
            cached_input_tokens=180,
            output_tokens=20,
        )

    def make_acceptance(self) -> dict:
        return {
            "schema_version": ACCEPTANCE_SCHEMA_VERSION,
            "status": "accepted",
            "accepted_by": "user",
            "accepted_at": self.proposal["created_at"],
            "acceptance_reference": "user accepted exact result in thread",
            "result_path": self.result_path.relative_to(self.repo).as_posix(),
            "result_sha256": file_sha256(self.result_path),
            "proposal_path": self.proposal_path.relative_to(self.repo).as_posix(),
            "authorization_path": self.authorization_path.relative_to(
                self.repo
            ).as_posix(),
            "accepted_scope": self.result["observed_scope"],
            "acknowledged_unverified_scope": [],
            "accepted_scenarios": [
                {
                    "scenario_id": "changed",
                    "after_payload_sha256": self.scenario["after_payload_sha256"],
                }
            ],
        }

    def validate(self, result=None, acceptance=None):
        result = copy.deepcopy(result or self.result)
        write_json(self.result_path, result)
        acceptance = copy.deepcopy(acceptance or self.acceptance)
        acceptance["result_sha256"] = file_sha256(self.result_path)
        write_json(self.acceptance_path, acceptance)
        return validate_promotion(
            self.repo,
            self.contract,
            self.result_path,
            self.acceptance_path,
        )

    def test_accepts_complete_exact_evidence(self) -> None:
        validated = self.validate()
        self.assertEqual(
            [item["scenario_id"] for item in validated["scenarios"]], ["changed"]
        )
        self.assertEqual(validated["proposal"], self.proposal)

    def test_promotes_snapshot_and_precise_smoke_boundary(self) -> None:
        promote(
            self.repo,
            self.contract_path,
            self.contract,
            self.result_path,
            self.acceptance_path,
        )
        promoted = load_contract(self.contract_path)
        self.assertEqual(promoted["baseline"]["status"], "accepted")
        self.assertEqual(
            promoted["baseline"]["smoke_evidence"]["arms"], ["standard", "map-request"]
        )
        self.assertEqual(promoted["baseline"]["smoke_evidence"]["unverified_scope"], [])
        self.assertEqual(
            promoted["baseline"]["acceptance_evidence"]["sha256"],
            file_sha256(self.acceptance_path),
        )
        self.assertEqual(len(promoted["baseline"]["final_wire_manifest"]), 1)
        self.assertIn('"after"', self.snapshot_path.read_text(encoding="utf-8"))

    def test_rejects_duplicate_changed_scenario(self) -> None:
        gate = json.loads(self.gate_path.read_text(encoding="utf-8"))
        gate["free_validation"]["commands"].append(
            copy.deepcopy(gate["free_validation"]["commands"][0])
        )
        with self.assertRaisesRegex(ValueError, "changed scenarios are invalid"):
            changed_scenarios(gate)

    def test_revalidation_has_no_snapshot_scenarios(self) -> None:
        self.assertEqual(
            changed_scenarios(
                {
                    "discovery_state": "revalidation_requested",
                    "free_validation": {"commands": []},
                }
            ),
            [],
        )

    def test_rejects_incomplete_or_tampered_result(self) -> None:
        result = copy.deepcopy(self.result)
        result["status"] = "partial"
        with self.assertRaisesRegex(ValueError, "incomplete"):
            self.validate(result=result)
        (self.repo / self.result["observations"][0]["artifacts"]["metrics"]).write_text(
            "{}\n", encoding="utf-8"
        )
        with self.assertRaisesRegex(ValueError, "evidence digest"):
            self.validate()

    def test_rejects_scope_or_scenario_expansion(self) -> None:
        acceptance = copy.deepcopy(self.acceptance)
        acceptance["accepted_scope"]["repeat"] = 2
        with self.assertRaisesRegex(ValueError, "scope"):
            self.validate(acceptance=acceptance)
        acceptance = copy.deepcopy(self.acceptance)
        acceptance["accepted_scenarios"] = []
        with self.assertRaisesRegex(ValueError, "scenarios"):
            self.validate(acceptance=acceptance)

    def test_rejects_result_evidence_or_ledger_mismatch(self) -> None:
        result = copy.deepcopy(self.result)
        result["evidence_sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "evidence digest"):
            self.validate(result=result)
        ledger_path = self.repo / "benchmarks/whale-agent-run-ledger.json"
        ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
        ledger["entries"][0]["execution"]["actual_sample_runs"] = 1
        write_json(ledger_path, ledger)
        with self.assertRaisesRegex(ValueError, "ledger execution"):
            self.validate()

    def test_rejects_failed_attempt_or_over_budget_observation(self) -> None:
        result = copy.deepcopy(self.result)
        result["attempts"][0]["status"] = "failed"
        with self.assertRaisesRegex(ValueError, "failed attempt"):
            self.validate(result=result)

        result = copy.deepcopy(self.result)
        result["observations"][0]["elapsed_seconds"] = 121.0
        result["attempts"][0]["elapsed_seconds"] = 121.0
        result["observations"][0]["budget_observation_exceeded"] = ["elapsed_seconds"]
        with self.assertRaisesRegex(ValueError, "exceeded"):
            self.validate(result=result)

    def test_rejects_ledger_token_or_gate_source_mismatch(self) -> None:
        ledger_path = self.repo / "benchmarks/whale-agent-run-ledger.json"
        ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
        ledger["entries"][0]["tokens"]["input"] = 199
        write_json(ledger_path, ledger)
        with self.assertRaisesRegex(ValueError, "token totals"):
            self.validate()

        self.write_ledger()
        gate = json.loads(self.gate_path.read_text(encoding="utf-8"))
        gate["subject_commit"] = "0" * 40
        write_json(self.gate_path, gate)
        with self.assertRaisesRegex(ValueError, "gate report digest"):
            self.validate()

    def test_rejects_incomplete_runner_or_authorization_envelope(self) -> None:
        result = copy.deepcopy(self.result)
        result.pop("ended_at")
        with self.assertRaisesRegex(ValueError, "ended_at"):
            self.validate(result=result)

        authorization = copy.deepcopy(self.authorization)
        authorization.pop("approved_at")
        write_json(self.authorization_path, authorization)
        result = copy.deepcopy(self.result)
        result["authorization_sha256"] = file_sha256(self.authorization_path)
        with self.assertRaisesRegex(ValueError, "timestamp"):
            self.validate(result=result)

        self.write_ledger()
        write_json(self.authorization_path, self.authorization)
        result = copy.deepcopy(self.result)
        result["attempts"][0].pop("post_run_cleanup")
        with self.assertRaisesRegex(ValueError, "failed attempt"):
            self.validate(result=result)

    def test_rejects_reversed_evidence_timeline(self) -> None:
        acceptance = copy.deepcopy(self.acceptance)
        acceptance["accepted_at"] = "2020-01-01T00:00:00+00:00"
        with self.assertRaisesRegex(ValueError, "acceptance precedes run end"):
            self.validate(acceptance=acceptance)

    def test_rejects_duplicate_authorization_in_ledger(self) -> None:
        ledger_path = self.repo / "benchmarks/whale-agent-run-ledger.json"
        ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
        duplicate = copy.deepcopy(ledger["entries"][0])
        duplicate["record_id"] = "WAR-DUPLICATE"
        ledger["entries"].append(duplicate)
        write_json(ledger_path, ledger)
        with self.assertRaisesRegex(ValueError, "not unique"):
            self.validate()

    def test_rejects_business_failure_even_with_consistent_artifacts(self) -> None:
        result = copy.deepcopy(self.result)
        observation = result["observations"][0]
        metrics_path = self.repo / observation["artifacts"]["metrics"]
        write_json(
            metrics_path, {"logical_mode": "standard", "business_success": False}
        )
        recomputed = analyze_artifacts(
            self.repo / observation["artifacts"]["cache_summary"],
            self.repo / observation["artifacts"]["request_summary"],
            metrics_path,
            observation["arm"],
        )
        recomputed["artifacts"] = observation["artifacts"]
        recomputed.update(
            {
                "sample": "simple",
                "repeat": 1,
                "elapsed_seconds": 1.0,
                "budget_observation_exceeded": [],
                "run_id": observation["run_id"],
            }
        )
        result["observations"][0] = recomputed
        with self.assertRaisesRegex(ValueError, "not promotable"):
            self.validate(result=result)

    def test_promotion_rejects_snapshot_changed_after_discovery(self) -> None:
        self.snapshot_path.write_text(
            '---\nsource: fixture.rs\n---\n{"drifted": true}\n',
            encoding="utf-8",
        )
        with self.assertRaisesRegex(ValueError, "changed after discovery"):
            promote(
                self.repo,
                self.contract_path,
                self.contract,
                self.result_path,
                self.acceptance_path,
            )


if __name__ == "__main__":
    unittest.main()
