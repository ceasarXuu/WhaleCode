#!/usr/bin/env python3

from __future__ import annotations

import contextlib
import io
import json
import unittest
from unittest.mock import patch

from cache_baseline_test_support import write_provider_boundary_evidence
from cache_evidence import RESULT_SCHEMA_VERSION, file_sha256
from cache_arm_identity import fixture_arm_identity
from cache_provider_boundary_evidence import persist_provider_boundary_accounting
from cache_run_analysis import analyze_artifacts
from cache_run_execution_test_support import CacheRunExecutionFixture
from cache_surface import write_json
from run_cache_hit_regression import (
    main,
    persist_observation_artifacts,
)


class CacheRunExecutionTest(CacheRunExecutionFixture):
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

        def fake_persist_boundary(*_args, **_kwargs):
            ledger = json.loads(
                (self.repo / "benchmarks/whale-agent-run-ledger.json").read_text(
                    encoding="utf-8"
                )
            )
            execution = ledger["entries"][0]["execution"]
            self.assertIsNone(execution["api_requests"])
            self.assertEqual(execution["api_requests_minimum"], len(calls) * 3)
            self.assertEqual(execution["api_requests_evidence_status"], "partial")
            return {
                "provider_boundary_request_count": 3,
                "provider_boundary_evidence_path": "fixture-boundary.json",
                "provider_boundary_evidence_sha256": "e" * 64,
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
            patch(
                "run_cache_hit_regression.run_benchmark_command", side_effect=fake_run
            ),
            patch(
                "run_cache_hit_regression.cleanup_labeled_containers",
                return_value={
                    "status": "verified_absent",
                    "container_ids": [],
                    "stable_empty_polls": 3,
                    "network_cleanup_status": "verified_absent",
                    "network_ids": [],
                    "secret_cleanup_status": "verified_absent",
                    "secret_paths": [],
                    "error": "",
                },
            ),
            patch(
                "run_cache_hit_regression.find_run_dir_by_id",
                return_value=self.repo,
            ),
            patch("run_cache_hit_regression.analyze_arm", side_effect=fake_analyze),
            patch(
                "run_cache_hit_regression.read_provider_boundary_request_count",
                return_value=3,
            ),
            patch(
                "run_cache_hit_regression.persist_provider_boundary_accounting",
                side_effect=fake_persist_boundary,
            ),
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
        self.assertEqual(entry["execution"]["api_requests_evidence_status"], "complete")
        result = json.loads(
            next(
                (self.repo / "benchmarks/cache-regression/results").glob("*.json")
            ).read_text(encoding="utf-8")
        )
        self.assertEqual(result["schema_version"], RESULT_SCHEMA_VERSION)
        self.assertEqual(result["status"], "completed")
        self.assertEqual(result["unverified_scope"], [])

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
            "secret_cleanup_status": "removed_verified",
            "secret_paths": ["deepseek-fixture.secret"],
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
            patch(
                "run_cache_hit_regression.run_benchmark_command",
                side_effect=KeyboardInterrupt,
            ),
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
        self.assertEqual(
            cleanup_call.call_args.args[2],
            self.repo
            / "target/cache-hit-regression"
            / cleanup_run_id.rsplit("-CACHE-", 1)[0],
        )
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

    def test_keyboard_interrupt_with_unverified_cleanup_is_a_failed_run(self) -> None:
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
            "status": "failed",
            "container_ids": ["agent"],
            "stable_empty_polls": 0,
            "network_cleanup_status": "not_attempted",
            "network_ids": [],
            "secret_cleanup_status": "not_attempted",
            "secret_paths": [],
            "error": "cleanup grace expired",
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
            patch(
                "run_cache_hit_regression.run_benchmark_command",
                side_effect=KeyboardInterrupt,
            ),
            patch(
                "run_cache_hit_regression.cleanup_labeled_containers",
                return_value=cleanup,
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            exit_code = main()

        self.assertEqual(exit_code, 3)
        result = json.loads(
            next(
                (self.repo / "benchmarks/cache-regression/results").glob("*.json")
            ).read_text(encoding="utf-8")
        )
        self.assertEqual(result["status"], "failed")
        self.assertEqual(result["stop_reason"], "cancelled_cleanup_failed")
        self.assertEqual(result["attempts"][0]["post_run_cleanup"], cleanup)

    def _assert_post_wait_interrupt_is_settled(self, interrupt_stage: str) -> None:
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
            "status": "verified_absent",
            "container_ids": [],
            "stable_empty_polls": 3,
            "network_cleanup_status": "verified_absent",
            "network_ids": [],
            "secret_cleanup_status": "verified_absent",
            "secret_paths": [],
            "error": "",
        }
        cleanup_effect = (
            [KeyboardInterrupt(), cleanup]
            if interrupt_stage == "cleanup"
            else [cleanup, cleanup]
        )
        run_dir_effect = (
            KeyboardInterrupt() if interrupt_stage == "evidence" else self.repo
        )
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
            patch(
                "run_cache_hit_regression.run_benchmark_command",
                return_value=type("Completed", (), {"returncode": 0})(),
            ),
            patch(
                "run_cache_hit_regression.cleanup_labeled_containers",
                side_effect=cleanup_effect,
            ) as cleanup_call,
            patch(
                "run_cache_hit_regression.find_run_dir_by_id",
                side_effect=run_dir_effect,
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            exit_code = main()

        self.assertEqual(exit_code, 3)
        self.assertEqual(cleanup_call.call_count, 2)
        result = json.loads(
            next(
                (self.repo / "benchmarks/cache-regression/results").glob("*.json")
            ).read_text(encoding="utf-8")
        )
        self.assertEqual(result["status"], "failed")
        self.assertEqual(result["attempts"][0]["post_run_cleanup"], cleanup)
        ledger = json.loads(
            (self.repo / "benchmarks/whale-agent-run-ledger.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(ledger["entries"][0]["status"], "failed")

    def test_cleanup_stage_interrupt_is_cleaned_and_settled(self) -> None:
        self._assert_post_wait_interrupt_is_settled("cleanup")

    def test_evidence_stage_interrupt_is_cleaned_and_settled(self) -> None:
        self._assert_post_wait_interrupt_is_settled("evidence")

    def test_persists_recomputable_artifacts_outside_target(self) -> None:
        source = self.repo / "target/run/pair-001/left/artifacts"
        source.mkdir(parents=True)
        cache = source / "provider-cache-trace-summary.json"
        request = source / "request-summary.json"
        metrics = source / "metrics.json"
        boundary = source / "provider-boundary-evidence.json"
        execution_argv = source / "whale-argv.json"
        logical_mode_map = source.parent.parent / "logical-mode-map.json"
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
        argv_value, mode_map_value = fixture_arm_identity("standard")
        write_json(execution_argv, argv_value)
        write_json(logical_mode_map, mode_map_value)
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

    def test_persists_boundary_accounting_before_full_reconciliation(self) -> None:
        run_dir = self.repo / "target/run/simple/CACHE-001"
        artifacts = run_dir / "pair-001/left/artifacts"
        artifacts.mkdir(parents=True)
        boundary = artifacts / "provider-boundary-evidence.json"
        write_provider_boundary_evidence(boundary, 2)
        value = json.loads(boundary.read_text(encoding="utf-8"))
        value["status"] = "mismatch"
        value["wire_requests"] = []
        value["wire_request_count"] = 0
        value["errors"] = ["provider_dispatch_trace_mismatch"]
        write_json(boundary, value)

        accounting = persist_provider_boundary_accounting(
            self.repo,
            "WAR-FIXTURE",
            "CACHE-001",
            run_dir,
            "left",
            "deepseek-v4-flash",
        )

        self.assertEqual(accounting["provider_boundary_request_count"], 2)
        self.assertTrue(
            (self.repo / accounting["provider_boundary_evidence_path"]).is_file()
        )


if __name__ == "__main__":
    unittest.main()
