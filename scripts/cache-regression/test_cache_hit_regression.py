#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from cache_baseline_test_support import write_provider_boundary_evidence
from cache_usage_contract import (
    aggregate_usage_records,
    load_provider_usage_fixture,
    normalized_fixture_cases,
    validate_cache_artifacts,
)
from cache_run_analysis import (
    analyze_arm,
    analyze_artifacts,
    budget_observation_exceeded,
)
from run_cache_hit_regression import (
    ensure_deepseek_api_key,
    execution_completed,
    stop_reason,
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
            write_provider_boundary_evidence(
                artifacts / "provider-boundary-evidence.json", 3
            )
            arm = analyze_arm(
                run_dir, "left", "standard", "deepseek-v4-flash"
            )
            self.assertEqual(arm["uncached_input_tokens"], 100)
            self.assertEqual(arm["request_2_plus_hit_rate"], 0.91)

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
        with self.assertRaisesRegex(ValueError, "request_2_plus_cached_input_tokens"):
            validate_cache_artifacts(cache, request)

    def test_analyzer_rejects_inconsistent_request_2_plus_rate(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            artifacts = Path(root)
            cache_path = artifacts / "cache.json"
            request_path = artifacts / "request.json"
            metrics_path = artifacts / "metrics.json"
            boundary_path = artifacts / "provider-boundary-evidence.json"
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
            write_provider_boundary_evidence(boundary_path, 2)
            with self.assertRaisesRegex(ValueError, "does not match token evidence"):
                analyze_artifacts(
                    cache_path,
                    request_path,
                    metrics_path,
                    boundary_path,
                    "standard",
                    "deepseek-v4-flash",
                )

    def test_analyzer_rejects_unmatched_provider_boundary_dispatch(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            artifacts = Path(root)
            cache = artifacts / "cache.json"
            request = artifacts / "request.json"
            metrics = artifacts / "metrics.json"
            boundary = artifacts / "provider-boundary-evidence.json"
            cache.write_text(
                json.dumps(
                    {
                        "provider_request_count": 2,
                        "request_2_plus_count": 1,
                        "request_2_plus_cached_input_tokens": 80,
                        "request_2_plus_uncached_input_tokens": 20,
                        "request_2_plus_hit_rate": 0.8,
                        "trace_coverage": 1.0,
                        "cache_usage_missing_count": 0,
                    }
                ),
                encoding="utf-8",
            )
            request.write_text(
                json.dumps(
                    {
                        "rollout_trace": {
                            "input_tokens": 100,
                            "cached_input_tokens": 80,
                            "output_tokens": 10,
                        }
                    }
                ),
                encoding="utf-8",
            )
            metrics.write_text(
                json.dumps({"logical_mode": "standard", "business_success": True}),
                encoding="utf-8",
            )
            write_provider_boundary_evidence(boundary, 2)
            evidence = json.loads(boundary.read_text(encoding="utf-8"))
            evidence["boundary_requests"].append(
                {
                    "count": 3,
                    "method": "POST",
                    "path": "/responses",
                    "model": "deepseek-v4-flash",
                    "body_sha256": "f" * 64,
                }
            )
            evidence["boundary_request_count"] = 3
            boundary.write_text(json.dumps(evidence), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "request count"):
                analyze_artifacts(
                    cache,
                    request,
                    metrics,
                    boundary,
                    "standard",
                    "deepseek-v4-flash",
                )

    def test_budget_observation_is_explicit_and_always_stops(self) -> None:
        observation = {
            "provider_requests": 11,
            "input_tokens": 90,
            "output_tokens": 20,
            "elapsed_seconds": 5,
            "business_success": True,
            "cache_usage_missing_count": 0,
        }
        exceeded = budget_observation_exceeded(
            observation,
            {
                "provider_requests": 10,
                "elapsed_seconds": 10,
            },
            {"input_tokens": 100, "output_tokens": 20},
        )
        self.assertEqual(exceeded, ["provider_requests"])
        observation["budget_observation_exceeded"] = exceeded
        self.assertEqual(
            stop_reason([], True, observation),
            "budget_observation_exceeded",
        )

    def test_run_failure_stop_is_mechanical(self) -> None:
        self.assertEqual(
            stop_reason(["after_any_run_failure"], True, None), "run_failure"
        )
        self.assertEqual(stop_reason([], True, None), "run_failure")

    def test_failed_or_over_budget_attempt_is_not_complete(self) -> None:
        matrix = [{"sample": "simple", "arm": "standard", "repeat": 1}]
        observation = {**matrix[0], "budget_observation_exceeded": []}
        failed = [{**matrix[0], "status": "failed", "exit_code": 7}]
        self.assertFalse(execution_completed(matrix, failed, [observation]))
        completed = [{**matrix[0], "status": "completed", "exit_code": 0}]
        observation["budget_observation_exceeded"] = ["input_tokens"]
        self.assertFalse(execution_completed(matrix, completed, [observation]))


if __name__ == "__main__":
    unittest.main()
