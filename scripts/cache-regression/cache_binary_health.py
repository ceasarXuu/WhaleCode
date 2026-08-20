#!/usr/bin/env python3
"""Run the shared Whale binary-health contract before provider preflight."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from cache_evidence import file_sha256
from cache_json import strict_json_loads
from cache_process_control import BenchmarkTimeoutError, run_captured_command


def validate_binary_health(value: object) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError("Whale binary health evidence must be an object")
    findings = value.get("findings")
    if (
        value.get("schema_version") != 1
        or value.get("status") not in {"pass", "fail"}
        or value.get("run_validity") not in {"valid", "invalid_harness"}
        or not isinstance(findings, list)
    ):
        raise ValueError("Whale binary health evidence is invalid")
    return value


def run_whale_binary_health_preflight(
    repo: Path,
    whale_bin: Path,
    output_path: Path,
    timeout_seconds: int = 60,
) -> dict[str, Any]:
    repo = repo.resolve()
    output_path = (
        output_path.resolve()
        if output_path.is_absolute()
        else (repo / output_path).resolve()
    )
    output_path.parent.mkdir(parents=True, exist_ok=True)
    command = [
        "pwsh",
        "-NoProfile",
        "-File",
        str(repo / "scripts/taskspace-benchmark/check-whale-binary-health.ps1"),
        "-WhaleBin",
        str(whale_bin),
        "-RepoRoot",
        str(repo),
        "-OutputPath",
        str(output_path),
    ]
    try:
        completed = run_captured_command(command, repo, timeout_seconds)
    except BenchmarkTimeoutError as error:
        if not error.process_tree_termination.get(
            "descendants_guaranteed_terminated", False
        ):
            raise ValueError(
                "Whale binary preflight cleanup could not be verified"
            ) from error
        raise ValueError("Whale binary preflight timed out") from error
    if not output_path.is_file():
        raise ValueError("Whale binary preflight did not write health evidence")
    health = validate_binary_health(
        strict_json_loads(output_path.read_text(encoding="utf-8-sig"))
    )
    if completed.returncode != 0 or health["status"] != "pass":
        hard_findings = [
            finding
            for finding in health["findings"]
            if isinstance(finding, dict) and finding.get("severity") == "fail"
        ]
        stable_code = (
            hard_findings[0].get("stable_code", "unknown")
            if hard_findings
            else "unknown"
        )
        raise ValueError(
            f"Whale binary preflight failed before provider route: {stable_code}"
        )
    return {
        "status": "passed",
        "whale_binary_sha256": health["whale_binary_sha256"],
        "artifact_path": output_path.relative_to(repo).as_posix(),
        "artifact_sha256": file_sha256(output_path),
    }
