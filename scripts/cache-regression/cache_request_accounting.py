#!/usr/bin/env python3
"""Shared mechanical request-accounting facts for cache run evidence."""

from __future__ import annotations

from typing import Any


def summarize_request_accounting(
    attempts: object, actual_sample_runs: object
) -> tuple[int, str]:
    if not isinstance(attempts, list) or type(actual_sample_runs) is not int:
        raise ValueError("cache request accounting inputs are invalid")
    if actual_sample_runs < 0 or not all(isinstance(item, dict) for item in attempts):
        raise ValueError("cache request accounting inputs are invalid")
    accounted = [
        item["provider_boundary_request_count"]
        for item in attempts
        if type(item.get("provider_boundary_request_count")) is int
        and item["provider_boundary_request_count"] >= 0
    ]
    complete = len(attempts) == actual_sample_runs and len(accounted) == len(attempts)
    status = "complete" if complete else "partial" if accounted else "unavailable"
    return sum(accounted), status


def validate_result_request_accounting(result: dict[str, Any]) -> None:
    minimum, status = summarize_request_accounting(
        result.get("attempts"), result.get("actual_sample_runs")
    )
    reported_minimum = result.get("provider_boundary_requests_minimum")
    if (
        type(reported_minimum) is not int
        or reported_minimum != minimum
        or result.get("provider_boundary_accounting_status") != status
    ):
        raise ValueError("cache result request accounting is inconsistent")
