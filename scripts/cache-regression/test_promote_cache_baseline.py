#!/usr/bin/env python3

from __future__ import annotations

import copy
import json
import math
import subprocess
import unittest
from pathlib import Path

from cache_evidence import canonical_json_sha256, file_sha256
from cache_arm_identity import fixture_arm_identity
from cache_gate_evidence import changed_scenarios
from cache_run_analysis import analyze_artifacts
from cache_run_contract import execution_matrix
from cache_surface import load_contract, write_json
from promote_cache_baseline import (
    promote,
)
from promote_cache_baseline_test_support import PromoteCacheBaselineFixture


def run(*args: str, cwd: Path) -> str:
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


class PromoteCacheBaselineTest(PromoteCacheBaselineFixture, unittest.TestCase):
    def _replace_observation_artifact(
        self, result: dict, observation_index: int, key: str, value: dict
    ) -> None:
        observation = result["observations"][observation_index]
        path = self.repo / observation["artifacts"][key]
        write_json(path, value)
        observation["artifact_sha256"][key] = file_sha256(path)
        result["evidence_sha256"] = canonical_json_sha256(
            [
                {**scope, "artifact_sha256": item["artifact_sha256"]}
                for scope, item in zip(
                    execution_matrix(self.proposal), result["observations"]
                )
            ]
        )

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

        with self.assertRaisesRegex(
            ValueError, "provider accounting|not durable|logical_mode"
        ):
            self.validate(result=self.result, acceptance=self.acceptance)

    def test_rejects_incomplete_post_run_cleanup_proof(self) -> None:
        for field in ("network_cleanup_status", "secret_cleanup_status"):
            with self.subTest(field=field):
                result = copy.deepcopy(self.result)
                result["attempts"][0]["post_run_cleanup"].pop(field)
                with self.assertRaisesRegex(ValueError, "failed attempt"):
                    self.validate(result=result)

        result = copy.deepcopy(self.result)
        result["attempts"][0]["post_run_cleanup"]["secret_cleanup_status"] = "failed"
        with self.assertRaisesRegex(ValueError, "failed attempt"):
            self.validate(result=result)

    def test_rejects_cleanup_success_with_contradictory_residue(self) -> None:
        for field, value in (
            ("container_ids", ["remaining-container"]),
            ("network_ids", ["remaining-network"]),
            ("secret_paths", ["remaining-secret"]),
            ("error", "cleanup reported an error"),
        ):
            with self.subTest(field=field):
                result = copy.deepcopy(self.result)
                result["attempts"][0]["post_run_cleanup"][field] = value
                with self.assertRaisesRegex(ValueError, "failed attempt"):
                    self.validate(result=result)

    def test_rejects_tampered_provider_accounting_on_attempt(self) -> None:
        result = copy.deepcopy(self.result)
        result["attempts"][0]["provider_boundary_request_count"] += 1
        with self.assertRaisesRegex(ValueError, "provider accounting"):
            self.validate(result=result)

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
        with self.assertRaisesRegex(ValueError, "failed attempt|exceeded"):
            self.validate(result=result)

        result = copy.deepcopy(self.result)
        result["observations"][0]["elapsed_seconds"] = 121.0
        result["attempts"][0]["elapsed_seconds"] = 121.0
        with self.assertRaisesRegex(ValueError, "failed attempt|elapsed|exceeded"):
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

    def test_rejects_ledger_monetary_cost_mismatch(self) -> None:
        ledger_path = self.repo / "benchmarks/whale-agent-run-ledger.json"
        ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
        ledger["entries"][0]["monetary_cost"].update(
            {
                "amount": 999.0,
                "currency": "USD",
                "components": {"cached_input": 999.0},
                "pricing_snapshot": self.pricing,
                "formula": "forged",
            }
        )
        write_json(ledger_path, ledger)
        with self.assertRaisesRegex(ValueError, "cost"):
            self.validate()

    def test_rejects_result_elapsed_shorter_than_attempt_total(self) -> None:
        result = copy.deepcopy(self.result)
        result["elapsed_seconds"] = 0.0
        ledger_path = self.repo / "benchmarks/whale-agent-run-ledger.json"
        ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
        ledger["entries"][0]["elapsed_calendar_seconds"] = 0.0
        write_json(ledger_path, ledger)

        with self.assertRaisesRegex(ValueError, "elapsed"):
            self.validate(result=result)

    def test_rejects_result_elapsed_inconsistent_with_timestamps(self) -> None:
        result = copy.deepcopy(self.result)
        result["ended_at"] = result["started_at"]

        with self.assertRaisesRegex(ValueError, "timestamps"):
            self.validate(result=result)

    def test_rejects_boolean_elapsed_values_in_full_promotion(self) -> None:
        result = copy.deepcopy(self.result)
        result["elapsed_seconds"] = True
        for attempt, observation in zip(result["attempts"], result["observations"]):
            attempt["elapsed_seconds"] = False
            observation["elapsed_seconds"] = False
        self.result = result
        self.write_ledger()

        with self.assertRaisesRegex(ValueError, "envelope|elapsed|failed attempt"):
            self.validate(result=result)

    def test_rejects_false_runner_exit_code_in_full_promotion(self) -> None:
        result = copy.deepcopy(self.result)
        result["runner_exit_code"] = False
        self.result = result
        self.write_ledger()

        with self.assertRaisesRegex(ValueError, "envelope"):
            self.validate(result=result)

    def test_rejects_boolean_attempt_exit_code_in_full_promotion(self) -> None:
        result = copy.deepcopy(self.result)
        result["attempts"][0]["exit_code"] = False

        with self.assertRaisesRegex(ValueError, "failed attempt"):
            self.validate(result=result)

    def test_rejects_boolean_ledger_integer_evidence(self) -> None:
        ledger_path = self.repo / "benchmarks/whale-agent-run-ledger.json"
        mutations = (
            (("execution", "repeats_per_arm_per_sample"), True, "ledger execution"),
            (("evidence", "runner_exit_code"), False, "ledger evidence"),
            (("tokens", "input"), False, "token totals"),
        )
        for path, value, message in mutations:
            with self.subTest(path=path):
                self.write_ledger()
                ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
                ledger["entries"][0][path[0]][path[1]] = value
                write_json(ledger_path, ledger)
                with self.assertRaisesRegex(ValueError, message):
                    self.validate()

    def test_rejects_nan_business_success_in_full_promotion(self) -> None:
        result = copy.deepcopy(self.result)
        self._replace_observation_artifact(
            result,
            0,
            "metrics",
            {"logical_mode": "standard", "business_success": math.nan},
        )
        with self.assertRaisesRegex(ValueError, "invalid|constant|boolean"):
            self.validate(result=result)

    def test_rejects_infinite_trace_coverage_in_full_promotion(self) -> None:
        result = copy.deepcopy(self.result)
        self._replace_observation_artifact(
            result,
            0,
            "cache_summary",
            {
                "provider_request_count": 3,
                "request_2_plus_count": 2,
                "request_2_plus_cached_input_tokens": 90,
                "request_2_plus_uncached_input_tokens": 10,
                "request_2_plus_hit_rate": 0.9,
                "trace_coverage": math.inf,
                "cache_usage_missing_count": 0,
            },
        )
        with self.assertRaisesRegex(ValueError, "invalid|constant|finite"):
            self.validate(result=result)

    def test_rejects_taskspace_arm_with_standard_launch_identity(self) -> None:
        result = copy.deepcopy(self.result)
        standard_argv, _ = fixture_arm_identity("standard")
        self._replace_observation_artifact(result, 1, "execution_argv", standard_argv)

        with self.assertRaisesRegex(ValueError, "execution identity|taskspace"):
            self.validate(result=result)

    def test_rejects_cross_arm_reuse_of_provider_wire_evidence(self) -> None:
        result = copy.deepcopy(self.result)
        boundary_path = (
            self.repo / result["observations"][0]["artifacts"]["provider_boundary"]
        )
        self._replace_observation_artifact(
            result,
            1,
            "provider_boundary",
            json.loads(boundary_path.read_text(encoding="utf-8")),
        )
        result["attempts"][1]["provider_boundary_evidence_sha256"] = result[
            "observations"
        ][1]["artifact_sha256"]["provider_boundary"]
        result["observations"][1]["provider_payload_sha256"] = list(
            result["observations"][0]["provider_payload_sha256"]
        )

        with self.assertRaisesRegex(ValueError, "identical provider wire"):
            self.validate(result=result)

    def test_rejects_float_request_count_in_observation(self) -> None:
        result = copy.deepcopy(self.result)
        result["observations"][0]["provider_requests"] = 3.0
        with self.assertRaisesRegex(ValueError, "observation metrics mismatch"):
            self.validate(result=result)

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
            self.repo / observation["artifacts"]["provider_boundary"],
            observation["arm"],
            "deepseek-v4-flash",
        )
        recomputed["artifacts"] = observation["artifacts"]
        recomputed["artifact_sha256"].update(
            {
                key: observation["artifact_sha256"][key]
                for key in ("execution_argv", "logical_mode_map")
            }
        )
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
