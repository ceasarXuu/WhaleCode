#!/usr/bin/env python3
"""Build the exact local execution-input manifest for an authorized cache run."""

from __future__ import annotations

import hashlib
from pathlib import Path
from typing import Any

from cache_evidence import canonical_json_sha256


SCHEMA_VERSION = "whalecode-cache-execution-identity-v1"
RUNNER_INPUTS = (
    "scripts/taskspace-benchmark/run-taskspace-benchmark.ps1",
    "scripts/taskspace-benchmark/run-taskspace-benchmark-pairs.ps1",
    "scripts/taskspace-benchmark/invoke-provider-route-preflight.ps1",
    "scripts/taskspace-benchmark/lib",
    "scripts/taskspace-benchmark/docker",
    "scripts/action-map-real-user-e2e-lib.ps1",
    "scripts/action-map-graph-health-lib.ps1",
    "benchmarks/taskspace/container-runtime-contract.json",
)


def _files(root: Path) -> list[Path]:
    if root.is_file():
        return [root]
    if root.is_dir():
        return sorted(path for path in root.rglob("*") if path.is_file())
    return []


def build_execution_identity(repo: Path, samples: list[str]) -> dict[str, Any]:
    repo = repo.resolve()
    roots = [repo / path for path in RUNNER_INPUTS]
    control_scripts = repo / "scripts/cache-regression"
    for sample in samples:
        scenario = repo / "benchmarks/taskspace/scenarios" / sample
        if not scenario.is_dir():
            raise ValueError(f"cache benchmark scenario does not exist: {sample}")
        roots.append(scenario)

    paths = {path.resolve() for root in roots for path in _files(root)}
    if control_scripts.is_dir():
        paths.update(
            path.resolve()
            for path in control_scripts.iterdir()
            if path.is_file()
            and path.suffix in {".py", ".ps1"}
            and not path.name.startswith("test_")
        )
    paths = sorted(paths)
    if not paths:
        raise ValueError("cache execution identity contains no files")
    entries = []
    for path in paths:
        try:
            relative = path.relative_to(repo).as_posix()
        except ValueError as error:
            raise ValueError("cache execution input escapes repository") from error
        entries.append(
            {
                "path": relative,
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            }
        )
    return {
        "schema_version": SCHEMA_VERSION,
        "entries": entries,
        "sha256": canonical_json_sha256(entries),
    }
