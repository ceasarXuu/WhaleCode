#!/usr/bin/env python3

from __future__ import annotations

import contextlib
import io
import json
from unittest.mock import patch

from cache_run_execution_test_support import CacheRunExecutionFixture
from run_cache_hit_regression import main


class CacheRunAggregationTest(CacheRunExecutionFixture):
    def _run_with_finalization_interrupt(self, patch_target: str) -> tuple[int, dict]:
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
                return_value=type("Completed", (), {"returncode": 1})(),
            ),
            patch(
                "run_cache_hit_regression.cleanup_labeled_containers",
                return_value=cleanup,
            ),
            patch(patch_target, side_effect=KeyboardInterrupt),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            exit_code = main()
        result_path = next(
            (self.repo / "benchmarks/cache-regression/results").glob("*.json")
        )
        return exit_code, json.loads(result_path.read_text(encoding="utf-8"))

    def test_aggregation_interrupt_settles_failed_result_and_ledger(self) -> None:
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
                return_value=type("Completed", (), {"returncode": 1})(),
            ),
            patch(
                "run_cache_hit_regression.cleanup_labeled_containers",
                return_value=cleanup,
            ),
            patch(
                "cache_run_result.canonical_json_sha256",
                side_effect=KeyboardInterrupt,
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            exit_code = main()

        self.assertEqual(exit_code, 3)
        result_path = next(
            (self.repo / "benchmarks/cache-regression/results").glob("*.json")
        )
        result = json.loads(result_path.read_text(encoding="utf-8"))
        self.assertEqual(result["status"], "failed")
        self.assertIn("KeyboardInterrupt", result["finalization_error"])
        ledger = json.loads(
            (self.repo / "benchmarks/whale-agent-run-ledger.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(ledger["entries"][0]["status"], "failed")

    def test_timestamp_interrupt_is_inside_final_settlement_transaction(self) -> None:
        exit_code, result = self._run_with_finalization_interrupt(
            "cache_run_result.now"
        )

        self.assertEqual(exit_code, 3)
        self.assertEqual(result["status"], "failed")
        ledger = json.loads(
            (self.repo / "benchmarks/whale-agent-run-ledger.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(ledger["entries"][0]["status"], "failed")


if __name__ == "__main__":
    import unittest

    unittest.main()
