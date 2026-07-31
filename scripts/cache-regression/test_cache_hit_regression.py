#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from cache_usage_contract import (
    aggregate_usage_records,
    load_provider_usage_fixture,
    normalized_fixture_cases,
    validate_cache_artifacts,
)
from run_cache_hit_regression import (
    analyze_artifacts,
    analyze_arm,
    arm_passes,
    ensure_deepseek_api_key,
    record_failed_baseline,
    should_record_failed_baseline,
)


REPO = Path(__file__).resolve().parents[2]
PROVIDER_USAGE_FIXTURE = (
    REPO
    / "third_party/codex-cli/codex-rs/codex-api/tests/fixtures/provider_usage_contract.json"
)


class CacheHitRegressionAnalysisTest(unittest.TestCase):
    def test_loads_only_deepseek_key_from_env_local(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            repo = Path(root)
            (repo / ".env.local").write_text(
                "IGNORED=value\nexport DEEPSEEK_API_KEY='fixture-secret'\n",
                encoding="utf-8",
            )
            with patch.dict(os.environ, {}, clear=True):
                source = ensure_deepseek_api_key(repo)
                self.assertEqual(source, ".env.local")
                self.assertEqual(os.environ["DEEPSEEK_API_KEY"], "fixture-secret")
                self.assertNotIn("IGNORED", os.environ)

    def test_prefers_existing_environment_key(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            with patch.dict(
                os.environ, {"DEEPSEEK_API_KEY": "existing-secret"}, clear=True
            ):
                source = ensure_deepseek_api_key(Path(root))
                self.assertEqual(source, "process_environment")
                self.assertEqual(os.environ["DEEPSEEK_API_KEY"], "existing-secret")

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
                        "request_2_plus_cached_input_tokens": 819,
                        "request_2_plus_uncached_input_tokens": 81,
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

    def test_rust_fixture_has_one_cross_language_usage_contract(self) -> None:
        fixture = load_provider_usage_fixture(PROVIDER_USAGE_FIXTURE)
        chat = normalized_fixture_cases(fixture, "chat_completions")
        responses = normalized_fixture_cases(fixture, "responses")
        self.assertEqual(chat, responses)
        self.assertIsNone(chat["invalid_cached_type"])

        for cases in (chat, responses):
            aggregate = aggregate_usage_records([cases["miss"], cases["hit"]])
            self.assertEqual(aggregate["provider_request_count"], 2)
            self.assertEqual(aggregate["input_tokens"], 190)
            self.assertEqual(aggregate["cached_input_tokens"], 80)
            self.assertEqual(aggregate["output_tokens"], 30)
            self.assertEqual(aggregate["request_2_plus_count"], 1)
            self.assertEqual(aggregate["request_2_plus_cached_input_tokens"], 80)
            self.assertEqual(aggregate["request_2_plus_uncached_input_tokens"], 20)
            self.assertEqual(aggregate["request_2_plus_hit_rate"], 0.8)

    def test_missing_or_invalid_usage_is_not_comparable(self) -> None:
        fixture = load_provider_usage_fixture(PROVIDER_USAGE_FIXTURE)
        cases = normalized_fixture_cases(fixture, "chat_completions")
        with self.assertRaisesRegex(ValueError, "missing or undecodable"):
            aggregate_usage_records([cases["hit"], cases["invalid_cached_type"]])

    def test_analyzer_rejects_missing_request_2_plus_token_evidence(self) -> None:
        cache = {
            "provider_request_count": 2,
            "request_2_plus_count": 1,
            "request_2_plus_hit_rate": 0.8,
            "cache_usage_missing_count": 0,
        }
        request = {
            "input_tokens": 190,
            "cached_input_tokens": 80,
            "output_tokens": 30,
        }
        with self.assertRaisesRegex(
            ValueError, "request_2_plus_cached_input_tokens"
        ):
            validate_cache_artifacts(cache, request)

    def test_analyzer_rejects_inconsistent_request_2_plus_rate(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            artifacts = Path(root)
            cache_path = artifacts / "cache.json"
            request_path = artifacts / "request.json"
            metrics_path = artifacts / "metrics.json"
            cache_path.write_text(
                json.dumps(
                    {
                        "provider_request_count": 2,
                        "request_2_plus_count": 1,
                        "request_2_plus_cached_input_tokens": 80,
                        "request_2_plus_uncached_input_tokens": 20,
                        "request_2_plus_hit_rate": 0.5,
                        "trace_coverage": 1.0,
                        "cache_usage_missing_count": 0,
                    }
                ),
                encoding="utf-8",
            )
            request_path.write_text(
                json.dumps(
                    {
                        "rollout_trace": {
                            "input_tokens": 190,
                            "cached_input_tokens": 80,
                            "output_tokens": 30,
                        }
                    }
                ),
                encoding="utf-8",
            )
            metrics_path.write_text(
                json.dumps({"logical_mode": "standard", "business_success": True}),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ValueError, "does not match token evidence"):
                analyze_artifacts(cache_path, request_path, metrics_path, "standard")

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

    def test_preflight_failure_does_not_poison_live_baseline(self) -> None:
        self.assertFalse(
            should_record_failed_baseline(
                {"status": "fail", "actual_sample_runs": 0}
            )
        )
        self.assertTrue(
            should_record_failed_baseline(
                {"status": "fail", "actual_sample_runs": 1}
            )
        )


if __name__ == "__main__":
    unittest.main()
