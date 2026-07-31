#!/usr/bin/env python3

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from run_cache_hit_regression import analyze_arm, arm_passes, record_failed_baseline


class CacheHitRegressionAnalysisTest(unittest.TestCase):
    def test_analyzes_provider_request_2_plus_cache(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            run_dir = Path(root)
            artifacts = run_dir / "pair-001/left/artifacts"
            artifacts.mkdir(parents=True)
            (artifacts / "provider-cache-trace-summary.json").write_text(
                json.dumps(
                    {
                        "provider_request_count": 3,
                        "request_2_plus_count": 2,
                        "request_2_plus_hit_rate": 0.91,
                        "trace_coverage": 1.0,
                        "cache_usage_missing_count": 0,
                    }
                ),
                encoding="utf-8",
            )
            (artifacts / "request-summary.json").write_text(
                json.dumps(
                    {
                        "rollout_trace": {
                            "input_tokens": 1000,
                            "cached_input_tokens": 900,
                            "output_tokens": 25,
                        }
                    }
                ),
                encoding="utf-8",
            )
            (artifacts / "metrics.json").write_text(
                json.dumps({"logical_mode": "standard", "business_success": True}),
                encoding="utf-8",
            )
            arm = analyze_arm(run_dir, "left", "standard")
            self.assertEqual(arm["uncached_input_tokens"], 100)
            self.assertEqual(arm["request_2_plus_hit_rate"], 0.91)
            self.assertTrue(
                arm_passes(
                    arm,
                    {
                        "absolute_floor": {"standard": 0.85},
                        "min_request_2_plus_count": 1,
                        "min_trace_coverage": 1.0,
                    },
                )
            )

    def test_rejects_missing_cache_coverage(self) -> None:
        arm = {
            "arm": "map-request",
            "business_success": True,
            "provider_requests": 4,
            "request_2_plus_count": 3,
            "request_2_plus_hit_rate": 0.99,
            "trace_coverage": 0.5,
            "cache_usage_missing_count": 1,
        }
        self.assertFalse(
            arm_passes(
                arm,
                {
                    "absolute_floor": {"map-request": 0.75},
                    "min_request_2_plus_count": 1,
                    "min_trace_coverage": 1.0,
                },
            )
        )

    def test_rejects_drop_from_live_baseline(self) -> None:
        arm = {
            "arm": "standard",
            "business_success": True,
            "provider_requests": 3,
            "request_2_plus_count": 2,
            "request_2_plus_hit_rate": 0.86,
            "trace_coverage": 1.0,
            "cache_usage_missing_count": 0,
        }
        policy = {
            "absolute_floor": {"standard": 0.85},
            "min_request_2_plus_count": 1,
            "min_trace_coverage": 1.0,
            "max_drop_from_live_baseline": 0.05,
        }
        baseline = {
            "status": "live_verified",
            "request_2_plus_hit_rate": {"standard": 0.93},
        }
        self.assertFalse(arm_passes(arm, policy, baseline))

    def test_rejects_business_failure(self) -> None:
        arm = {
            "arm": "standard",
            "business_success": False,
            "provider_requests": 3,
            "request_2_plus_count": 2,
            "request_2_plus_hit_rate": 0.95,
            "trace_coverage": 1.0,
            "cache_usage_missing_count": 0,
        }
        policy = {
            "absolute_floor": {"standard": 0.85},
            "min_request_2_plus_count": 1,
            "min_trace_coverage": 1.0,
        }
        self.assertFalse(arm_passes(arm, policy))

    def test_failed_result_marks_surface_blocked(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            contract_path = Path(root) / "contract.json"
            contract = {
                "schema_version": "whalecode-cache-surface-v1",
                "baseline": {
                    "surface_sha256": "abc",
                    "status": "structural_bootstrap",
                    "live_result_path": None,
                },
                "surface_rules": [],
                "live_regression": {},
            }
            result = {"result_path": "benchmarks/cache-regression/results/fail.json"}
            record_failed_baseline(contract_path, contract, result)
            saved = json.loads(contract_path.read_text(encoding="utf-8"))
            self.assertEqual(saved["baseline"]["status"], "live_regression_failed")
            self.assertEqual(saved["baseline"]["surface_sha256"], "abc")


if __name__ == "__main__":
    unittest.main()
