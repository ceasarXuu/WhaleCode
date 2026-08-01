#!/usr/bin/env python3
"""Mechanical integrity checks for completed cache run results."""

from __future__ import annotations

from typing import Any

from cache_cleanup_contract import cleanup_verified
from cache_elapsed import is_elapsed_number
from cache_json import exact_json_equal
from cache_provider_route import (
    validate_route_profile_binding,
    validate_route_summary,
)


def validate_completed_result_integrity(
    result: dict[str, Any], expected_matrix: list[dict[str, Any]] | None = None
) -> None:
    if result.get("status") != "completed":
        return
    actual = result.get("actual_sample_runs")
    attempts = result.get("attempts")
    observations = result.get("observations")
    route = validate_route_summary(
        result.get("provider_route_attestation"),
        result.get("observed_scope", {}).get("model"),
    )
    if (
        type(actual) is not int
        or actual <= 0
        or not isinstance(attempts, list)
        or not isinstance(observations, list)
        or len(attempts) != actual
        or len(observations) != actual
        or result.get("unverified_scope") != []
    ):
        raise ValueError("completed cache result evidence is inconsistent")
    attempt_matrix = [
        {key: attempt.get(key) for key in ("sample", "arm", "repeat")}
        for attempt in attempts
        if isinstance(attempt, dict)
    ]
    if expected_matrix is not None and not exact_json_equal(
        attempt_matrix, expected_matrix
    ):
        raise ValueError("completed cache result scope is unauthorized")
    for attempt, observation in zip(attempts, observations):
        if not isinstance(attempt, dict) or not isinstance(observation, dict):
            raise ValueError("completed cache result evidence is inconsistent")
        scope = {key: attempt.get(key) for key in ("sample", "arm", "repeat")}
        observed_scope = {
            key: observation.get(key) for key in ("sample", "arm", "repeat")
        }
        token_values = [
            observation.get(key)
            for key in (
                "provider_requests",
                "input_tokens",
                "cached_input_tokens",
                "uncached_input_tokens",
                "output_tokens",
            )
        ]
        valid = (
            isinstance(scope["sample"], str)
            and bool(scope["sample"].strip())
            and isinstance(scope["arm"], str)
            and bool(scope["arm"].strip())
            and type(scope["repeat"]) is int
            and scope["repeat"] > 0
            and attempt.get("status") == "completed"
            and type(attempt.get("exit_code")) is int
            and attempt["exit_code"] == 0
            and attempt.get("timed_out") is False
            and isinstance(attempt.get("run_id"), str)
            and bool(attempt["run_id"].strip())
            and is_elapsed_number(attempt.get("elapsed_seconds"))
            and attempt["elapsed_seconds"] >= 0
            and cleanup_verified(attempt.get("post_run_cleanup", {}))
            and "execution_error" not in attempt
            and "evidence_error" not in attempt
            and exact_json_equal(scope, observed_scope)
            and observation.get("run_id") == attempt["run_id"]
            and type(attempt.get("provider_boundary_request_count")) is int
            and attempt["provider_boundary_request_count"] >= 0
            and all(type(value) is int and value >= 0 for value in token_values)
            and observation["input_tokens"]
            == observation["cached_input_tokens"] + observation["uncached_input_tokens"]
            and observation["provider_requests"]
            == attempt["provider_boundary_request_count"]
        )
        if not valid:
            raise ValueError("completed cache result evidence is inconsistent")
        if not exact_json_equal(
            observation.get("provider_routing"), route["provider_routing"]
        ):
            raise ValueError("completed cache result provider route is inconsistent")
        validate_route_profile_binding(
            observation.get("provider_route_profile"), route, scope["arm"]
        )
