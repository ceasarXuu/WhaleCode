#!/usr/bin/env python3
"""Mechanical final result construction for paid cache runs."""

from __future__ import annotations

import time
from typing import Any, Callable

from cache_evidence import canonical_json_sha256
from cache_request_accounting import summarize_request_accounting
from cache_run_ledger import now


def finalize_run_result(
    result: dict[str, Any],
    matrix: list[dict[str, Any]],
    stop_at: str | None,
    *,
    cleanup_failed: bool,
    supervision_failed: bool,
    cancelled: bool,
    started: float,
    execution_completed: Callable[
        [list[dict[str, Any]], list[dict[str, Any]], list[dict[str, Any]]], bool
    ],
) -> None:
    _aggregate(
        result,
        matrix,
        stop_at,
        cleanup_failed=cleanup_failed,
        supervision_failed=supervision_failed,
        cancelled=cancelled,
        execution_completed=execution_completed,
    )
    result["ended_at"] = now()
    result["elapsed_seconds"] = round(time.time() - started, 3)


def _aggregate(
    result: dict[str, Any],
    matrix: list[dict[str, Any]],
    stop_at: str | None,
    *,
    cleanup_failed: bool,
    supervision_failed: bool,
    cancelled: bool,
    execution_completed: Callable[
        [list[dict[str, Any]], list[dict[str, Any]], list[dict[str, Any]]], bool
    ],
) -> None:
    attempted_keys = {
        (item["sample"], item["arm"], item["repeat"]) for item in result["attempts"]
    }
    result["unverified_scope"] = [
        item
        for item in matrix
        if (item["sample"], item["arm"], item["repeat"]) not in attempted_keys
    ]
    result["stop_reason"] = stop_at
    request_minimum, request_status = summarize_request_accounting(
        result["attempts"], result["actual_sample_runs"]
    )
    result["provider_boundary_requests_minimum"] = request_minimum
    result["provider_boundary_accounting_status"] = request_status
    if cleanup_failed or supervision_failed:
        result["status"] = "failed"
    elif cancelled:
        result["status"] = "cancelled"
    elif execution_completed(matrix, result["attempts"], result["observations"]):
        result["status"] = "completed"
    elif result["attempts"]:
        result["status"] = "partial"
    else:
        result["status"] = "failed"
    result["evidence_sha256"] = canonical_json_sha256(
        [
            {
                "sample": item["sample"],
                "arm": item["arm"],
                "repeat": item["repeat"],
                "artifact_sha256": item["artifact_sha256"],
            }
            for item in result["observations"]
        ]
    )
    result["runner_exit_code"] = (
        0 if result["status"] == "completed" else 130 if cancelled else 3
    )
