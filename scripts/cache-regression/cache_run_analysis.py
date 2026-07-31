#!/usr/bin/env python3
"""Recompute cache smoke observations from benchmark artifacts."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from cache_evidence import file_sha256
from cache_usage_contract import SCHEMA_VERSION as PROVIDER_USAGE_CONTRACT_VERSION
from cache_usage_contract import validate_cache_artifacts


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def analyze_artifacts(
    cache_path: Path, request_path: Path, metrics_path: Path, arm: str
) -> dict[str, Any]:
    cache = read_json(cache_path)
    request = read_json(request_path)["rollout_trace"]
    metrics = read_json(metrics_path)
    usage = validate_cache_artifacts(cache, request)
    return {
        "arm": arm,
        "provider_usage_contract_version": PROVIDER_USAGE_CONTRACT_VERSION,
        "logical_mode": metrics["logical_mode"],
        "provider_requests": usage["provider_request_count"],
        "request_2_plus_count": usage["request_2_plus_count"],
        "request_2_plus_hit_rate": usage["request_2_plus_hit_rate"],
        "request_2_plus_cached_input_tokens": usage[
            "request_2_plus_cached_input_tokens"
        ],
        "request_2_plus_uncached_input_tokens": usage[
            "request_2_plus_uncached_input_tokens"
        ],
        "trace_coverage": float(cache["trace_coverage"]),
        "cache_usage_missing_count": usage["cache_usage_missing_count"],
        "input_tokens": usage["input_tokens"],
        "cached_input_tokens": usage["cached_input_tokens"],
        "uncached_input_tokens": usage["uncached_input_tokens"],
        "output_tokens": usage["output_tokens"],
        "business_success": bool(metrics["business_success"]),
        "artifacts": {
            "cache_summary": str(cache_path),
            "request_summary": str(request_path),
            "metrics": str(metrics_path),
        },
        "artifact_sha256": {
            "cache_summary": file_sha256(cache_path),
            "request_summary": file_sha256(request_path),
            "metrics": file_sha256(metrics_path),
        },
    }


def analyze_arm(run_dir: Path, side: str, arm: str) -> dict[str, Any]:
    artifacts = run_dir / "pair-001" / side / "artifacts"
    return analyze_artifacts(
        artifacts / "provider-cache-trace-summary.json",
        artifacts / "request-summary.json",
        artifacts / "metrics.json",
        arm,
    )


def observation_meets_policy(
    observation: dict[str, Any],
    policy: dict[str, Any],
    baseline: dict[str, Any] | None = None,
) -> bool:
    floor = float(policy["absolute_floor"][observation["arm"]])
    absolute_pass = (
        observation["business_success"]
        and observation["provider_requests"] >= 2
        and observation["request_2_plus_count"]
        >= int(policy["min_request_2_plus_count"])
        and observation["trace_coverage"] >= float(policy["min_trace_coverage"])
        and observation["cache_usage_missing_count"] == 0
        and observation["request_2_plus_hit_rate"] >= floor
    )
    if not absolute_pass or not baseline or baseline.get("status") != "live_verified":
        return absolute_pass
    prior_rate = baseline.get("request_2_plus_hit_rate", {}).get(observation["arm"])
    if prior_rate is None:
        return absolute_pass
    return observation["request_2_plus_hit_rate"] >= float(prior_rate) - float(
        policy["max_drop_from_live_baseline"]
    )
