#!/usr/bin/env python3

from __future__ import annotations

import contextlib
import io
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from cache_baseline_test_support import write_provider_boundary_evidence
from cache_evidence import RESULT_SCHEMA_VERSION, file_sha256
from cache_run_analysis import analyze_artifacts
from cache_surface import write_json
from run_cache_hit_regression import (
    cleanup_labeled_containers,
    cleanup_verified,
    main,
    persist_observation_artifacts,
)
from cache_process_control import (
    BenchmarkTimeoutError,
    _terminate_process_tree,
    run_benchmark_command,
)


class CacheRunExecutionTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name)
        contract_path = (
            self.repo / "benchmarks/cache-regression/cache-surface-contract.json"
        )
        contract_path.parent.mkdir(parents=True)
        self.pricing = {
            "currency": "USD",
            "cached_input_per_million": 0.0028,
            "uncached_input_per_million": 0.14,
            "output_per_million": 0.28,
        }
        write_json(
            contract_path,
            {
                "schema_version": "whalecode-cache-surface-v1",
                "baseline": {
                    "surface_sha256": "fixture",
                    "status": "live_regression_failed",
                },
                "surface_rules": [],
                "pricing_snapshot": self.pricing,
            },
        )
        write_json(
            self.repo / "benchmarks/whale-agent-run-ledger.json",
            {
                "schema_version": "whale-agent-run-ledger-v1",
                "updated_at": "2026-08-01T00:00:00+08:00",
                "entries": [],
            },
        )
        self.proposal_path = self.repo / "benchmarks/cache-regression/proposal.json"
        self.authorization_path = (
            self.repo / "benchmarks/cache-regression/authorization.json"
        )
        self.proposal = {
            "proposal_id": "CBP-FIXTURE",
            "proposal_sha256": "a" * 64,
            "subject_commit": "b" * 40,
            "surface_sha256": "c" * 64,
            "selection": {
                "model": "deepseek-v4-flash",
                "samples": ["simple"],
                "arms": ["standard", "map-request"],
                "repeat": 1,
                "planned_sample_runs": 2,
                "retry_sample_run_limit": 0,
                "maximum_sample_runs": 2,
                "stop_conditions": [],
                "selection_reason": "approved fixture",
            },
            "per_sample_run_limits": {
                "provider_requests": 10,
                "elapsed_seconds": 60,
                "cleanup_grace_seconds": 120,
            },
            "per_sample_run_observation_thresholds": {
                "input_tokens": 1000,
                "output_tokens": 100,
            },
            "maximums": {
                "provider_requests": 20,
                "input_tokens": 20_000_000,
                "output_tokens": 7_680_000,
                "elapsed_seconds": 360,
                "estimated_cost": 4.9504,
                "currency": "USD",
            },
            "provider_hard_limits": {
                "max_input_tokens_per_request": 1_000_000,
                "max_output_tokens_per_request": 384_000,
            },
            "pricing_snapshot": self.pricing,
            "evidence_boundary": "fixture scope only",
        }
        self.authorization = {
            "authorization_id": "CBA-FIXTURE-001",
            "approval_reference": "fixture approval",
        }
        write_json(self.proposal_path, self.proposal)
        write_json(self.authorization_path, self.authorization)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_runner_executes_exact_matrix_and_records_started_attempts(self) -> None:
        calls = []

        def fake_run(command, *_args, **kwargs):
            ledger = json.loads(
                (self.repo / "benchmarks/whale-agent-run-ledger.json").read_text(
                    encoding="utf-8"
                )
            )
            self.assertEqual(ledger["entries"][0]["status"], "running")
            self.assertEqual(
                ledger["entries"][0]["execution"]["actual_sample_runs"],
                len(calls) + 1,
            )
            calls.append(command)
            return type("Completed", (), {"returncode": 0})()

        def fake_analyze(_run_dir, _side, arm, _model):
            return {
                "arm": arm,
                "provider_usage_contract_version": "fixture",
                "logical_mode": "standard" if arm == "standard" else "taskspace",
                "provider_model": "deepseek-v4-flash",
                "provider_requests": 3,
                "request_2_plus_count": 2,
                "request_2_plus_hit_rate": 0.9,
                "request_2_plus_cached_input_tokens": 90,
                "request_2_plus_uncached_input_tokens": 10,
                "trace_coverage": 1.0,
                "cache_usage_missing_count": 0,
                "input_tokens": 100,
                "cached_input_tokens": 90,
                "uncached_input_tokens": 10,
                "output_tokens": 10,
                "business_success": True,
                "artifacts": {"metrics": "fixture"},
                "artifact_sha256": {"metrics": "d" * 64},
            }

        argv = [
            "run_cache_hit_regression.py",
            "--repo-root",
            str(self.repo),
            "--proposal",
            str(self.proposal_path),
            "--authorization",
            str(self.authorization_path),
        ]
        with (
            patch("sys.argv", argv),
            patch(
                "run_cache_hit_regression.load_authorized_proposal",
                return_value=(
                    self.proposal,
                    self.authorization,
                    self.proposal_path,
                    self.authorization_path,
                ),
            ),
            patch(
                "run_cache_hit_regression.ensure_deepseek_api_key",
                return_value="fixture",
            ),
            patch("run_cache_hit_regression.run_benchmark_command", side_effect=fake_run),
            patch(
                "run_cache_hit_regression.cleanup_labeled_containers",
                return_value={
                    "status": "verified_absent",
                    "container_ids": [],
                    "stable_empty_polls": 3,
                    "network_cleanup_status": "verified_absent",
                    "network_ids": [],
                    "error": "",
                },
            ),
            patch(
                "run_cache_hit_regression.find_run_dir_by_id",
                return_value=self.repo,
            ),
            patch("run_cache_hit_regression.analyze_arm", side_effect=fake_analyze),
            patch(
                "run_cache_hit_regression.persist_observation_artifacts",
                side_effect=lambda _repo, _record, _run, _arm, _model, value: value,
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            exit_code = main()

        self.assertEqual(exit_code, 0)
        self.assertEqual(len(calls), 2)
        self.assertTrue(all(call[call.index("-Repeats") + 1] == "1" for call in calls))
        run_ids = [call[call.index("-RunId") + 1] for call in calls]
        self.assertEqual(len(set(run_ids)), 2)
        self.assertTrue(run_ids[0].startswith("WAR-"))
        self.assertTrue(run_ids[0].endswith("-CACHE-001"))
        self.assertTrue(run_ids[1].endswith("-CACHE-002"))
        self.assertEqual(
            run_ids[0].rsplit("-CACHE-", maxsplit=1)[0],
            run_ids[1].rsplit("-CACHE-", maxsplit=1)[0],
        )
        ledger = json.loads(
            (self.repo / "benchmarks/whale-agent-run-ledger.json").read_text(
                encoding="utf-8"
            )
        )
        entry = ledger["entries"][0]
        self.assertEqual(entry["status"], "settled")
        self.assertEqual(entry["execution"]["actual_sample_runs"], 2)
        self.assertEqual(entry["execution"]["api_requests"], 6)
        result = json.loads(
            next(
                (self.repo / "benchmarks/cache-regression/results").glob("*.json")
            ).read_text(encoding="utf-8")
        )
        self.assertEqual(result["schema_version"], RESULT_SCHEMA_VERSION)
        self.assertEqual(result["status"], "completed")
        self.assertEqual(result["unverified_scope"], [])

    def test_timeout_cleanup_removes_all_run_labeled_containers(self) -> None:
        completed = [
            type(
                "Completed", (), {"returncode": 0, "stdout": "one\ntwo\n", "stderr": ""}
            )(),
            type(
                "Completed", (), {"returncode": 0, "stdout": "one\ntwo\n", "stderr": ""}
            )(),
            type("Completed", (), {"returncode": 0, "stdout": "", "stderr": ""})(),
            type("Completed", (), {"returncode": 0, "stdout": "", "stderr": ""})(),
            type("Completed", (), {"returncode": 0, "stdout": "", "stderr": ""})(),
            type("Completed", (), {"returncode": 0, "stdout": "", "stderr": ""})(),
        ]
        with (
            patch("cache_process_control.subprocess.run", side_effect=completed) as run,
            patch("cache_process_control.time.sleep"),
        ):
            result = cleanup_labeled_containers("CACHE-001", 10)
        self.assertEqual(result["status"], "removed_verified")
        self.assertEqual(result["container_ids"], ["one", "two"])
        self.assertEqual(
            run.call_args_list[0].args[0][-1],
            "label=whalecode.run_id=CACHE-001",
        )
        self.assertEqual(
            run.call_args_list[1].args[0], ["docker", "rm", "--force", "one", "two"]
        )
        self.assertEqual(result["stable_empty_polls"], 3)

    def test_timeout_terminates_the_benchmark_process_group(self) -> None:
        process = unittest.mock.Mock()
        process.pid = 123
        process.poll.return_value = None
        process.wait.side_effect = [
            subprocess.TimeoutExpired(["pwsh"], 10),
            143,
        ]
        process.returncode = 143
        with (
            patch("cache_process_control.subprocess.Popen", return_value=process) as popen,
            patch("cache_process_control.os.getpgid", return_value=123),
            patch("cache_process_control.os.killpg") as killpg,
        ):
            with self.assertRaises(BenchmarkTimeoutError) as raised:
                run_benchmark_command(["pwsh", "runner.ps1"], self.repo, 10)
        self.assertTrue(popen.call_args.kwargs["start_new_session"])
        killpg.assert_called_once_with(123, unittest.mock.ANY)
        self.assertEqual(
            raised.exception.process_tree_termination["status"], "terminated"
        )

    def test_windows_timeout_terminates_entire_process_tree(self) -> None:
        process = unittest.mock.Mock()
        process.pid = 456
        process.poll.return_value = None
        process.wait.side_effect = [
            subprocess.TimeoutExpired(["pwsh"], 10),
            1,
        ]
        process.returncode = 1
        job = unittest.mock.Mock()
        with (
            patch("cache_process_control.os.name", "nt"),
            patch("cache_process_control.subprocess.Popen", return_value=process) as popen,
            patch("cache_process_control._new_windows_job", return_value=job),
        ):
            with self.assertRaises(BenchmarkTimeoutError) as raised:
                run_benchmark_command(["pwsh", "runner.ps1"], self.repo, 10)
        self.assertEqual(
            raised.exception.process_tree_termination["status"], "terminated"
        )
        self.assertEqual(
            raised.exception.process_tree_termination["method"],
            "windows_job_object",
        )
        self.assertTrue(
            raised.exception.process_tree_termination[
                "descendants_guaranteed_terminated"
            ]
        )
        job.assign.assert_called_once_with(process)
        job.close.assert_called()
        self.assertIn("creationflags", popen.call_args.kwargs)
        process.terminate.assert_not_called()
        process.kill.assert_not_called()

    def test_windows_taskkill_fallback_never_accepts_parent_exit_as_tree_proof(self) -> None:
        process = unittest.mock.Mock()
        process.pid = 789
        process.poll.side_effect = [None, 1]
        failed = subprocess.CompletedProcess(
            ["taskkill", "/PID", "789", "/T", "/F"], 128, "", "not found"
        )
        with (
            patch("cache_process_control.os.name", "nt"),
            patch("cache_process_control.subprocess.run", return_value=failed),
        ):
            result = _terminate_process_tree(process)
        self.assertEqual(result["status"], "failed")
        self.assertFalse(result["descendants_guaranteed_terminated"])

    def test_cleanup_catches_container_that_appears_after_first_empty_poll(self) -> None:
        completed = [
            type("Completed", (), {"returncode": 0, "stdout": "", "stderr": ""})(),
            type("Completed", (), {"returncode": 0, "stdout": "late\n", "stderr": ""})(),
            type("Completed", (), {"returncode": 0, "stdout": "late\n", "stderr": ""})(),
            *[
                type("Completed", (), {"returncode": 0, "stdout": "", "stderr": ""})()
                for _ in range(4)
            ],
        ]
        with (
            patch("cache_process_control.subprocess.run", side_effect=completed) as run,
            patch("cache_process_control.time.sleep"),
        ):
            result = cleanup_labeled_containers("CACHE-LATE", 10)
        self.assertEqual(result["status"], "removed_verified")
        self.assertEqual(result["container_ids"], ["late"])
        self.assertEqual(run.call_args_list[2].args[0], ["docker", "rm", "--force", "late"])

    def test_cleanup_removes_provider_boundary_networks(self) -> None:
        completed = [
            *[
                type("Completed", (), {"returncode": 0, "stdout": "", "stderr": ""})()
                for _ in range(3)
            ],
            type("Completed", (), {"returncode": 0, "stdout": "net-one\n", "stderr": ""})(),
            type("Completed", (), {"returncode": 0, "stdout": "net-one\n", "stderr": ""})(),
        ]
        with (
            patch("cache_process_control.subprocess.run", side_effect=completed) as run,
            patch("cache_process_control.time.sleep"),
        ):
            result = cleanup_labeled_containers("CACHE-NETWORK", 10)
        self.assertEqual(result["network_cleanup_status"], "removed_verified")
        self.assertEqual(result["network_ids"], ["net-one"])
        self.assertEqual(
            run.call_args_list[4].args[0],
            ["docker", "network", "rm", "net-one"],
        )

    def test_only_verified_cleanup_statuses_allow_completion(self) -> None:
        self.assertTrue(
            cleanup_verified(
                {
                    "status": "verified_absent",
                    "stable_empty_polls": 3,
                    "network_cleanup_status": "verified_absent",
                }
            )
        )
        self.assertTrue(
            cleanup_verified(
                {
                    "status": "removed_verified",
                    "stable_empty_polls": 3,
                    "network_cleanup_status": "removed_verified",
                }
            )
        )
        self.assertFalse(cleanup_verified({"status": "verified_absent", "stable_empty_polls": 1}))
        self.assertFalse(cleanup_verified({"status": "failed"}))
        self.assertFalse(cleanup_verified({"status": "removed"}))

    def test_keyboard_interrupt_cleans_run_containers_and_settles_cancelled(
        self,
    ) -> None:
        argv = [
            "run_cache_hit_regression.py",
            "--repo-root",
            str(self.repo),
            "--proposal",
            str(self.proposal_path),
            "--authorization",
            str(self.authorization_path),
        ]
        cleanup = {
            "status": "removed_verified",
            "container_ids": ["agent"],
            "stable_empty_polls": 3,
            "network_cleanup_status": "removed_verified",
            "network_ids": ["provider-boundary"],
            "error": "",
        }
        with (
            patch("sys.argv", argv),
            patch(
                "run_cache_hit_regression.load_authorized_proposal",
                return_value=(
                    self.proposal,
                    self.authorization,
                    self.proposal_path,
                    self.authorization_path,
                ),
            ),
            patch(
                "run_cache_hit_regression.ensure_deepseek_api_key",
                return_value="fixture",
            ),
            patch("run_cache_hit_regression.run_benchmark_command", side_effect=KeyboardInterrupt),
            patch(
                "run_cache_hit_regression.cleanup_labeled_containers",
                return_value=cleanup,
            ) as cleanup_call,
            contextlib.redirect_stdout(io.StringIO()),
        ):
            exit_code = main()

        self.assertEqual(exit_code, 130)
        cleanup_run_id = cleanup_call.call_args.args[0]
        self.assertTrue(cleanup_run_id.startswith("WAR-"))
        self.assertTrue(cleanup_run_id.endswith("-CACHE-001"))
        self.assertEqual(cleanup_call.call_args.args[1], 120)
        result = json.loads(
            next(
                (self.repo / "benchmarks/cache-regression/results").glob("*.json")
            ).read_text(encoding="utf-8")
        )
        self.assertEqual(result["status"], "cancelled")
        self.assertEqual(result["attempts"][0]["interrupt_cleanup"], cleanup)
        ledger = json.loads(
            (self.repo / "benchmarks/whale-agent-run-ledger.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(ledger["entries"][0]["status"], "cancelled")
        self.assertEqual(ledger["entries"][0]["monetary_cost"]["status"], "unavailable")

    def test_persists_recomputable_artifacts_outside_target(self) -> None:
        source = self.repo / "target/source"
        source.mkdir(parents=True)
        cache = source / "provider-cache-trace-summary.json"
        request = source / "request-summary.json"
        metrics = source / "metrics.json"
        boundary = source / "provider-boundary-evidence.json"
        write_json(
            cache,
            {
                "provider_request_count": 2,
                "request_2_plus_count": 1,
                "request_2_plus_cached_input_tokens": 80,
                "request_2_plus_uncached_input_tokens": 20,
                "request_2_plus_hit_rate": 0.8,
                "trace_coverage": 1.0,
                "cache_usage_missing_count": 0,
            },
        )
        write_json(
            request,
            {
                "rollout_trace": {
                    "input_tokens": 100,
                    "cached_input_tokens": 80,
                    "output_tokens": 10,
                }
            },
        )
        write_json(
            metrics,
            {"logical_mode": "standard", "business_success": True},
        )
        write_provider_boundary_evidence(boundary, 2)
        observation = analyze_artifacts(
            cache,
            request,
            metrics,
            boundary,
            "standard",
            "deepseek-v4-flash",
        )

        durable = persist_observation_artifacts(
            self.repo,
            "WAR-FIXTURE",
            "CACHE-001",
            "standard",
            "deepseek-v4-flash",
            observation,
        )

        for key, path in durable["artifacts"].items():
            artifact = self.repo / path
            self.assertTrue(artifact.is_file(), key)
            self.assertIn("benchmarks/cache-regression/evidence", artifact.as_posix())
            self.assertEqual(file_sha256(artifact), durable["artifact_sha256"][key])


if __name__ == "__main__":
    unittest.main()
