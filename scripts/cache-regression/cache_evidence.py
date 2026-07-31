#!/usr/bin/env python3
"""Deterministic cache regression run-plan and evidence identities."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any


RESULT_SCHEMA_VERSION = "whalecode-cache-hit-regression-v3"


def canonical_json_sha256(value: Any) -> str:
    encoded = json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def file_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def expected_run_plan(contract: dict[str, Any]) -> dict[str, Any]:
    live = contract["live_regression"]
    return {
        "model": live["model"],
        "sample": live["sample"],
        "arms": live["arms"],
        "repeat": live["repeat"],
        "planned_sample_runs": live["planned_sample_runs"],
        "automatic_retries": live["automatic_retries"],
        "thresholds": {
            "min_request_2_plus_count": live["min_request_2_plus_count"],
            "min_trace_coverage": live["min_trace_coverage"],
            "absolute_floor": live["absolute_floor"],
            "max_drop_from_live_baseline": live["max_drop_from_live_baseline"],
        },
    }


def evidence_manifest(arms: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [
        {"arm": arm["arm"], "artifact_sha256": arm["artifact_sha256"]}
        for arm in arms
    ]
