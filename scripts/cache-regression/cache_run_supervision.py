#!/usr/bin/env python3
"""Fail-closed cleanup and settlement helpers for paid cache runs."""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any, Callable

from cache_run_ledger import atomic_write_json, now, settle_entry, store_entry


def cleanup_failure(error: BaseException) -> dict[str, Any]:
    return {
        "status": "failed",
        "container_ids": [],
        "stable_empty_polls": 0,
        "network_cleanup_status": "not_attempted",
        "network_ids": [],
        "secret_cleanup_status": "not_attempted",
        "secret_paths": [],
        "error": f"{type(error).__name__}: {error}",
    }


def emergency_cleanup(
    cleanup: Callable[[str, int, Path], dict[str, Any]],
    run_id: str,
    grace_seconds: int,
    run_root: Path,
) -> dict[str, Any]:
    try:
        return cleanup(run_id, grace_seconds, run_root)
    except BaseException as error:
        return cleanup_failure(error)


def persist_final_settlement(
    entry: dict[str, Any],
    result: dict[str, Any],
    result_path: Path,
    ledger_path: Path,
) -> None:
    try:
        _persist(entry, result, result_path, ledger_path)
    except BaseException as error:
        result["status"] = "failed"
        result["runner_exit_code"] = 3
        result["finalization_error"] = f"{type(error).__name__}: {error}"
        _persist(entry, result, result_path, ledger_path)


def finalize_and_persist(
    entry: dict[str, Any],
    result: dict[str, Any],
    result_path: Path,
    ledger_path: Path,
    started: float,
    finalize: Callable[[], None],
) -> None:
    try:
        finalize()
        persist_final_settlement(entry, result, result_path, ledger_path)
    except BaseException as error:
        result["status"] = "failed"
        result["runner_exit_code"] = 3
        result["finalization_error"] = f"{type(error).__name__}: {error}"
        result["ended_at"] = result.get("ended_at") or now()
        result["elapsed_seconds"] = result.get("elapsed_seconds") or round(
            time.time() - started, 3
        )
        persist_final_settlement(entry, result, result_path, ledger_path)


def _persist(
    entry: dict[str, Any],
    result: dict[str, Any],
    result_path: Path,
    ledger_path: Path,
) -> None:
    atomic_write_json(result_path, result)
    settle_entry(entry, result)
    store_entry(ledger_path, entry)
