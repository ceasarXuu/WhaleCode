#!/usr/bin/env python3
"""Validate durable cache runner envelope fields shared by consumers."""

from __future__ import annotations

from datetime import datetime
from typing import Any

from cache_elapsed import is_elapsed_number


def parse_timestamp(value: Any, label: str) -> datetime:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{label} is missing")
    try:
        return datetime.fromisoformat(value)
    except ValueError as error:
        raise ValueError(f"{label} is invalid") from error


def validate_result_envelope(
    result: dict[str, Any], result_path: str, *, require_success: bool = True
) -> None:
    started = parse_timestamp(result.get("started_at"), "cache result started_at")
    ended = parse_timestamp(result.get("ended_at"), "cache result ended_at")
    expected_exit_codes = {
        "completed": 0,
        "partial": 3,
        "failed": 3,
        "cancelled": 130,
    }
    status = result.get("status")
    valid = (
        ended >= started
        and is_elapsed_number(result.get("elapsed_seconds"))
        and result["elapsed_seconds"] >= 0
        and type(result.get("runner_exit_code")) is int
        and status in expected_exit_codes
        and result["runner_exit_code"] == expected_exit_codes[status]
        and (status == "completed" if require_success else True)
        and result.get("result_path") == result_path
        and isinstance(result.get("run_root"), str)
        and result["run_root"].strip()
        and isinstance(result.get("credential_source"), str)
        and result["credential_source"].strip()
    )
    if not valid:
        raise ValueError("cache result envelope is incomplete")
