#!/usr/bin/env python3
"""Elapsed-time evidence validation for accepted cache runs."""

from __future__ import annotations

from datetime import datetime
from typing import Any


def validate_elapsed_evidence(
    result: dict[str, Any],
    attempts: list[dict[str, Any]],
    started_at: datetime,
    ended_at: datetime,
    maximum_elapsed_seconds: int,
) -> None:
    elapsed = result.get("elapsed_seconds")
    if not (
        isinstance(elapsed, (int, float))
        and elapsed >= 0
        and elapsed <= maximum_elapsed_seconds
    ):
        raise ValueError("cache result elapsed exceeds its approved bound")
    attempt_total = sum(float(item["elapsed_seconds"]) for item in attempts)
    rounding_tolerance = max(0.01, len(attempts) * 0.001)
    if attempt_total > float(elapsed) + rounding_tolerance:
        raise ValueError("cache result elapsed is shorter than its attempt total")
    timestamp_elapsed = (ended_at - started_at).total_seconds()
    if abs(float(elapsed) - timestamp_elapsed) > 1.1:
        raise ValueError("cache result elapsed does not match its timestamps")
