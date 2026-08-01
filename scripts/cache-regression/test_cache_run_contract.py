#!/usr/bin/env python3

from __future__ import annotations

import copy
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from cache_budget import build_budget_proposal
from cache_run_contract import (
    AUTHORIZATION_SCHEMA_VERSION,
    benchmark_command,
    execution_matrix,
    load_authorized_proposal,
    validate_authorization,
)
from cache_surface import load_contract, surface_snapshot, write_json


BENCHMARK_RUNNER = (
    Path(__file__).resolve().parents[1]
    / "taskspace-benchmark/run-taskspace-benchmark.ps1"
)


def run(*args: str, cwd: Path) -> str:
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


class CacheRunContractTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name)
        run("git", "init", "-q", cwd=self.repo)
        run("git", "config", "user.email", "test@example.com", cwd=self.repo)
        run("git", "config", "user.name", "Test", cwd=self.repo)
        (self.repo / "prompt").mkdir()
        (self.repo / "prompt/base.md").write_text("stable\n", encoding="utf-8")
        runner = self.repo / "scripts/taskspace-benchmark/run-taskspace-benchmark.ps1"
        runner.parent.mkdir(parents=True)
        runner.write_text("runner\n", encoding="utf-8")
        scenario = self.repo / "benchmarks/taskspace/scenarios/simple"
        scenario.mkdir(parents=True)
        (scenario / "scenario.json").write_text("{}\n", encoding="utf-8")
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
                "provider_hard_limits": {
                    "deepseek-v4-flash": {
                        "max_input_tokens_per_request": 1_000_000,
                        "max_output_tokens_per_request": 384_000,
                    }
                },
            },
        )
        self.gate_path = self.repo / "benchmarks/cache-regression/gate.json"
        write_json(
            self.gate_path,
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
            repeat=2,
            retry_sample_run_limit=0,
            max_provider_requests_per_run=10,
            observed_input_tokens_per_run=100_000,
            observed_output_tokens_per_run=5_000,
            max_seconds_per_run=120,
            stop_conditions=["after_any_run_failure"],
            selection_reason="human selected this smoke",
        )
        self.proposal_path = self.repo / "benchmarks/cache-regression/proposal.json"
        write_json(self.proposal_path, self.proposal)
        self.authorization = {
            "schema_version": AUTHORIZATION_SCHEMA_VERSION,
            "status": "granted",
            "approved_by": "user",
            "authorization_id": "CBA-FIXTURE-001",
            "approval_reference": "user approved exact proposal in thread",
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

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_loads_only_exact_current_authorized_proposal(self) -> None:
        proposal, authorization, _, _ = load_authorized_proposal(
            self.repo,
            self.contract,
            self.proposal_path,
            self.authorization_path,
        )
        self.assertEqual(proposal, self.proposal)
        self.assertEqual(authorization, self.authorization)

    def test_rejects_duplicate_authorization_keys(self) -> None:
        content = self.authorization_path.read_text(encoding="utf-8")
        content = content.replace(
            '"approved_maximums": {',
            '"approved_maximums": {},\n  "approved_maximums": {',
            1,
        )
        self.authorization_path.write_text(content, encoding="utf-8")

        with self.assertRaisesRegex(ValueError, "duplicate JSON object key"):
            load_authorized_proposal(
                self.repo,
                self.contract,
                self.proposal_path,
                self.authorization_path,
            )

    def test_authorization_cannot_expand_selection_or_maximums(self) -> None:
        authorization = copy.deepcopy(self.authorization)
        authorization["approved_selection"]["repeat"] = 3
        with self.assertRaisesRegex(ValueError, "selection"):
            validate_authorization(self.proposal, authorization)
        authorization = copy.deepcopy(self.authorization)
        authorization["approved_maximums"]["provider_requests"] += 1
        with self.assertRaisesRegex(ValueError, "maximums"):
            validate_authorization(self.proposal, authorization)

    def test_authorization_cannot_predate_proposal(self) -> None:
        authorization = copy.deepcopy(self.authorization)
        authorization["approved_at"] = "2020-01-01T00:00:00+00:00"
        with self.assertRaisesRegex(ValueError, "authorization precedes proposal"):
            validate_authorization(self.proposal, authorization)

    def test_stale_subject_or_gate_report_is_rejected(self) -> None:
        (self.repo / "new.txt").write_text("new\n", encoding="utf-8")
        run("git", "add", ".", cwd=self.repo)
        run("git", "commit", "-qm", "advance", cwd=self.repo)
        with self.assertRaisesRegex(ValueError, "current HEAD"):
            load_authorized_proposal(
                self.repo,
                self.contract,
                self.proposal_path,
                self.authorization_path,
            )

    def test_authorized_runner_or_scenario_cannot_drift(self) -> None:
        runner = self.repo / "scripts/taskspace-benchmark/run-taskspace-benchmark.ps1"
        runner.write_text("changed runner\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "current evidence"):
            load_authorized_proposal(
                self.repo,
                self.contract,
                self.proposal_path,
                self.authorization_path,
            )

        runner.write_text("runner\n", encoding="utf-8")
        control = self.repo / "scripts/cache-regression/cache_run_contract.py"
        control.parent.mkdir(parents=True, exist_ok=True)
        control.write_text("new control code\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "current evidence"):
            load_authorized_proposal(
                self.repo,
                self.contract,
                self.proposal_path,
                self.authorization_path,
            )

        control.unlink()
        scenario = self.repo / "benchmarks/taskspace/scenarios/simple/scenario.json"
        scenario.write_text('{"changed": true}\n', encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "current evidence"):
            load_authorized_proposal(
                self.repo,
                self.contract,
                self.proposal_path,
                self.authorization_path,
            )

    def test_execution_matrix_is_exact_cartesian_selection(self) -> None:
        matrix = execution_matrix(self.proposal)
        self.assertEqual(
            matrix,
            [
                {"sample": "simple", "arm": "standard", "repeat": 1},
                {"sample": "simple", "arm": "standard", "repeat": 2},
                {"sample": "simple", "arm": "map-request", "repeat": 1},
                {"sample": "simple", "arm": "map-request", "repeat": 2},
            ],
        )

    def test_command_maps_arm_without_changing_generic_benchmark(self) -> None:
        command = benchmark_command(
            self.repo,
            Path("/tmp/whale"),
            Path("/tmp/run"),
            "CACHE-001",
            self.proposal,
            {"sample": "simple", "arm": "map-append", "repeat": 1},
        )
        self.assertEqual(command[command.index("-RunSide") + 1], "right")
        self.assertEqual(
            command[command.index("-TaskSpaceProjectionPolicy") + 1], "map-append"
        )
        self.assertEqual(command[command.index("-Repeats") + 1], "1")
        self.assertEqual(command[command.index("-RunId") + 1], "CACHE-001")
        self.assertEqual(command[command.index("-ProviderRequestHardLimit") + 1], "10")

    def test_benchmark_runner_routes_hard_limit_through_isolated_boundary(self) -> None:
        source = BENCHMARK_RUNNER.read_text(encoding="utf-8")
        self.assertIn("[int]$ProviderRequestHardLimit = 0", source)
        self.assertIn(
            "Get-TaskspaceProviderBoundaryConfigOverrides",
            source,
        )
        self.assertNotIn("model_providers.deepseek.base_url=", source)
        self.assertIn(
            '$childEnvironment["WHALE_PROVIDER_BOUNDARY"] = "docker-isolated-proxy-v1"',
            source,
        )
        self.assertIn("$ProviderRequestHardLimit $Model $TimeoutSeconds", source)
        self.assertNotIn("provider-request-hard-limit.count", source)

    def test_benchmark_runner_preserves_explicit_run_id(self) -> None:
        source = BENCHMARK_RUNNER.read_text(encoding="utf-8")
        self.assertIn(
            "$runDir = New-TaskspaceBenchmarkRun $RunRoot $manifest.Id $RunId",
            source,
        )


if __name__ == "__main__":
    unittest.main()
