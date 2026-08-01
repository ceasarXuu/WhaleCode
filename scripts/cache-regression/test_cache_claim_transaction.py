#!/usr/bin/env python3

from __future__ import annotations

import contextlib
import io
import json
import unittest
from unittest.mock import patch

from cache_run_execution_test_support import CacheRunExecutionFixture
from cache_run_ledger import claim_entry
from run_cache_hit_regression import main


class CacheClaimTransactionTest(CacheRunExecutionFixture):
    def test_interrupt_after_durable_claim_is_settled(self) -> None:
        argv = [
            "run_cache_hit_regression.py",
            "--repo-root",
            str(self.repo),
            "--proposal",
            str(self.proposal_path),
            "--authorization",
            str(self.authorization_path),
        ]

        def claim_then_interrupt(path, entry):
            claim_entry(path, entry)
            raise KeyboardInterrupt

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
                "run_cache_hit_regression.claim_entry",
                side_effect=claim_then_interrupt,
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            exit_code = main()

        self.assertEqual(exit_code, 3)
        ledger = json.loads(
            (self.repo / "benchmarks/whale-agent-run-ledger.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(ledger["entries"][0]["status"], "failed")
        result = json.loads(
            next(
                (self.repo / "benchmarks/cache-regression/results").glob("*.json")
            ).read_text(encoding="utf-8")
        )
        self.assertEqual(result["status"], "failed")
        self.assertEqual(result["stop_reason"], "supervisor_interrupted")
        self.assertEqual(result["actual_sample_runs"], 0)


if __name__ == "__main__":
    unittest.main()
