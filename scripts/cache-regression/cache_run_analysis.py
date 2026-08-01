#!/usr/bin/env python3
"""Recompute cache smoke observations from benchmark artifacts."""

from __future__ import annotations

import math
from pathlib import Path
from typing import Any

from cache_evidence import file_sha256
from cache_json import strict_json_loads
from cache_usage_contract import SCHEMA_VERSION as PROVIDER_USAGE_CONTRACT_VERSION
from cache_usage_contract import validate_cache_artifacts

PROVIDER_BOUNDARY_SCHEMA_VERSION = "whalecode-provider-boundary-evidence-v1"
CACHE_OBSERVATION_KEYS = (
    "arm",
    "provider_usage_contract_version",
    "logical_mode",
    "provider_model",
    "provider_requests",
    "request_2_plus_count",
    "request_2_plus_hit_rate",
    "request_2_plus_cached_input_tokens",
    "request_2_plus_uncached_input_tokens",
    "trace_coverage",
    "cache_usage_missing_count",
    "input_tokens",
    "cached_input_tokens",
    "uncached_input_tokens",
    "output_tokens",
    "business_success",
    "artifacts",
    "artifact_sha256",
)


def read_json(path: Path) -> dict[str, Any]:
    value = strict_json_loads(path.read_text(encoding="utf-8-sig"))
    if not isinstance(value, dict):
        raise ValueError("cache artifact must be an object")
    return value


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
    if observation["elapsed_seconds"] > limits["elapsed_seconds"]:
        exceeded.append("elapsed_seconds")
    return exceeded


def analyze_artifacts(
    cache_path: Path,
    request_path: Path,
    metrics_path: Path,
    boundary_path: Path,
    arm: str,
    expected_model: str,
) -> dict[str, Any]:
    cache = read_json(cache_path)
    request = read_json(request_path)["rollout_trace"]
    metrics = read_json(metrics_path)
    boundary = read_json(boundary_path)
    return analyze_artifact_values(
        cache,
        request,
        metrics,
        boundary,
        arm,
        expected_model,
        {
            "cache_summary": str(cache_path),
            "request_summary": str(request_path),
            "metrics": str(metrics_path),
            "provider_boundary": str(boundary_path),
        },
        {
            "cache_summary": file_sha256(cache_path),
            "request_summary": file_sha256(request_path),
            "metrics": file_sha256(metrics_path),
            "provider_boundary": file_sha256(boundary_path),
        },
    )


def analyze_artifact_values(
    cache: dict[str, Any],
    request: dict[str, Any],
    metrics: dict[str, Any],
    boundary: dict[str, Any],
    arm: str,
    expected_model: str,
    artifacts: dict[str, str],
    artifact_sha256: dict[str, str],
) -> dict[str, Any]:
    usage = validate_cache_artifacts(cache, request)
    validate_provider_boundary_evidence(
        boundary, usage["provider_request_count"], expected_model
    )
    expected_logical_mode = "standard" if arm == "standard" else "taskspace"
    if metrics.get("logical_mode") != expected_logical_mode:
        raise ValueError(
            f"cache artifact logical_mode does not match arm {arm}: "
            f"expected {expected_logical_mode}"
        )
    trace_coverage = cache.get("trace_coverage")
    if (
        type(trace_coverage) not in {int, float}
        or not math.isfinite(trace_coverage)
        or not 0 <= trace_coverage <= 1
    ):
        raise ValueError("cache trace coverage must be a finite ratio")
    if type(metrics.get("business_success")) is not bool:
        raise ValueError("cache business success must be boolean")
    return {
        "arm": arm,
        "provider_usage_contract_version": PROVIDER_USAGE_CONTRACT_VERSION,
        "logical_mode": metrics["logical_mode"],
        "provider_model": expected_model,
        "provider_requests": boundary["boundary_request_count"],
        "request_2_plus_count": usage["request_2_plus_count"],
        "request_2_plus_hit_rate": usage["request_2_plus_hit_rate"],
        "request_2_plus_cached_input_tokens": usage[
            "request_2_plus_cached_input_tokens"
        ],
        "request_2_plus_uncached_input_tokens": usage[
            "request_2_plus_uncached_input_tokens"
        ],
        "trace_coverage": float(trace_coverage),
        "cache_usage_missing_count": usage["cache_usage_missing_count"],
        "input_tokens": usage["input_tokens"],
        "cached_input_tokens": usage["cached_input_tokens"],
        "uncached_input_tokens": usage["uncached_input_tokens"],
        "output_tokens": usage["output_tokens"],
        "business_success": metrics["business_success"],
        "artifacts": artifacts,
        "artifact_sha256": artifact_sha256,
    }


def validate_provider_boundary_evidence(
    boundary: dict[str, Any], expected_count: int, expected_model: str
) -> None:
    boundary_count = validate_provider_boundary_accounting(boundary, expected_model)
    if boundary_count != expected_count:
        raise ValueError(
            "provider boundary request count does not match usage evidence"
        )
    if boundary.get("status") != "reconciled" or boundary.get("errors") != []:
        raise ValueError("provider boundary evidence is not reconciled")
    wire_requests = boundary.get("wire_requests")
    if not isinstance(wire_requests, list):
        raise ValueError("provider boundary wire evidence is invalid")
    if boundary.get("wire_request_count") != len(wire_requests):
        raise ValueError("provider boundary wire request count is invalid")
    boundary_requests = boundary["boundary_requests"]
    boundary_hashes = [request.get("body_sha256") for request in boundary_requests]
    wire_hashes = [request.get("provider_payload_sha256") for request in wire_requests]
    if boundary_hashes != wire_hashes:
        raise ValueError(
            "provider boundary request trace does not match Whale wire trace"
        )


def validate_provider_boundary_accounting(
    boundary: dict[str, Any], expected_model: str
) -> int:
    if not isinstance(boundary, dict):
        raise ValueError("provider boundary evidence must be an object")
    if boundary.get("schema_version") != PROVIDER_BOUNDARY_SCHEMA_VERSION:
        raise ValueError("provider boundary evidence schema is invalid")
    if (
        boundary.get("expected_model") != expected_model
        or boundary.get("allowed_method") != "POST"
        or boundary.get("allowed_path") != "/responses"
    ):
        raise ValueError("provider boundary authorization does not match proposal")
    boundary_requests = boundary.get("boundary_requests")
    if not isinstance(boundary_requests, list):
        raise ValueError("provider boundary request evidence is invalid")
    if boundary.get("boundary_request_count") != len(boundary_requests):
        raise ValueError("provider boundary request count is invalid")
    for index, request in enumerate(boundary_requests, 1):
        if (
            request.get("count") != index
            or request.get("method") != "POST"
            or request.get("path") != "/responses"
            or request.get("model") != expected_model
            or not isinstance(request.get("body_sha256"), str)
            or len(request["body_sha256"]) != 64
        ):
            raise ValueError("provider boundary request contract is invalid")
    return len(boundary_requests)


def analyze_arm(
    run_dir: Path, side: str, arm: str, expected_model: str
) -> dict[str, Any]:
    artifacts = run_dir / "pair-001" / side / "artifacts"
    return analyze_artifacts(
        artifacts / "provider-cache-trace-summary.json",
        artifacts / "request-summary.json",
        artifacts / "metrics.json",
        artifacts / "provider-boundary-evidence.json",
        arm,
        expected_model,
    )
