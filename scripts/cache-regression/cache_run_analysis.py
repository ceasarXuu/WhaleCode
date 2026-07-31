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


def budget_observation_exceeded(
    observation: dict[str, Any], limits: dict[str, Any], thresholds: dict[str, Any]
) -> list[str]:
    exceeded = []
    for observed_key, limit_key in (("provider_requests", "provider_requests"),):
        if observation[observed_key] > limits[limit_key]:
            exceeded.append(observed_key)
    for key in ("input_tokens", "output_tokens"):
        if observation[key] > thresholds[key]:
            exceeded.append(key)
    return exceeded


def analyze_artifacts(
    cache_path: Path, request_path: Path, metrics_path: Path, arm: str
) -> dict[str, Any]:
    cache = read_json(cache_path)
    request = read_json(request_path)["rollout_trace"]
    metrics = read_json(metrics_path)
    return analyze_artifact_values(
        cache,
        request,
        metrics,
        arm,
        {
            "cache_summary": str(cache_path),
            "request_summary": str(request_path),
            "metrics": str(metrics_path),
        },
        {
            "cache_summary": file_sha256(cache_path),
            "request_summary": file_sha256(request_path),
            "metrics": file_sha256(metrics_path),
        },
    )


def analyze_artifact_values(
    cache: dict[str, Any],
    request: dict[str, Any],
    metrics: dict[str, Any],
    arm: str,
    artifacts: dict[str, str],
    artifact_sha256: dict[str, str],
) -> dict[str, Any]:
    usage = validate_cache_artifacts(cache, request)
    expected_logical_mode = "standard" if arm == "standard" else "taskspace"
    if metrics.get("logical_mode") != expected_logical_mode:
        raise ValueError(
            f"cache artifact logical_mode does not match arm {arm}: "
            f"expected {expected_logical_mode}"
        )
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
        "artifacts": artifacts,
        "artifact_sha256": artifact_sha256,
    }


def analyze_arm(run_dir: Path, side: str, arm: str) -> dict[str, Any]:
    artifacts = run_dir / "pair-001" / side / "artifacts"
    return analyze_artifacts(
        artifacts / "provider-cache-trace-summary.json",
        artifacts / "request-summary.json",
        artifacts / "metrics.json",
        arm,
    )
