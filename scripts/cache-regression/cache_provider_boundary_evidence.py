#!/usr/bin/env python3
"""Read and durably copy provider-boundary request evidence."""

from __future__ import annotations

import shutil
from pathlib import Path
from typing import Any

from cache_evidence import file_sha256
from cache_json import strict_json_loads
from cache_run_analysis import validate_provider_boundary_accounting


def boundary_source(run_dir: Path, side: str) -> Path:
    return run_dir / "pair-001" / side / "artifacts" / "provider-boundary-evidence.json"


def read_provider_boundary_request_count(
    run_dir: Path, side: str, expected_model: str
) -> int:
    source = boundary_source(run_dir, side)
    if not source.is_file():
        raise FileNotFoundError("provider boundary accounting evidence is missing")
    boundary = strict_json_loads(source.read_text(encoding="utf-8-sig"))
    return validate_provider_boundary_accounting(boundary, expected_model)


def persist_provider_boundary_accounting(
    repo: Path,
    record_id: str,
    run_id: str,
    run_dir: Path,
    side: str,
    expected_model: str,
    expected_request_count: int | None = None,
) -> dict[str, Any]:
    source = boundary_source(run_dir, side)
    request_count = read_provider_boundary_request_count(run_dir, side, expected_model)
    if expected_request_count is not None and request_count != expected_request_count:
        raise ValueError("provider boundary request count changed before persistence")
    destination = repo / "benchmarks/cache-regression/evidence" / record_id / run_id
    destination.mkdir(parents=True, exist_ok=False)
    target = destination / "provider-boundary-evidence.json"
    shutil.copyfile(source, target)
    return {
        "provider_boundary_request_count": request_count,
        "provider_boundary_evidence_path": target.relative_to(repo).as_posix(),
        "provider_boundary_evidence_sha256": file_sha256(target),
    }
