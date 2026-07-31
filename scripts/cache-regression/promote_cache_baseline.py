#!/usr/bin/env python3
"""Promote one passing live result to the cache surface baseline."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any

from cache_evidence import (
    RESULT_SCHEMA_VERSION,
    canonical_json_sha256,
    evidence_manifest,
    expected_run_plan,
    file_sha256,
)
from cache_surface import load_contract, surface_snapshot, write_json
from run_cache_hit_regression import analyze_artifacts, arm_passes


ARM_EVIDENCE_KEYS = (
    "logical_mode",
    "provider_requests",
    "request_2_plus_count",
    "request_2_plus_hit_rate",
    "trace_coverage",
    "cache_usage_missing_count",
    "input_tokens",
    "cached_input_tokens",
    "uncached_input_tokens",
    "output_tokens",
    "business_success",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def evidence_path(repo: Path, raw_path: str) -> Path:
    path = Path(raw_path)
    resolved = (path if path.is_absolute() else repo / path).resolve()
    try:
        resolved.relative_to(repo)
    except ValueError as error:
        raise ValueError("cache evidence must be inside the repository") from error
    return resolved


def validate_ledger(
    repo: Path, result_path: Path, result: dict[str, Any], plan: dict[str, Any]
) -> None:
    ledger = load_contract_json(repo / "benchmarks/whale-agent-run-ledger.json")
    entries = [
        entry
        for entry in ledger["entries"]
        if entry.get("record_id") == result.get("record_id")
    ]
    require(len(entries) == 1, "result must have exactly one matching ledger entry")
    entry = entries[0]
    authorization = entry.get("authorization", {})
    execution = entry.get("execution", {})
    evidence = entry.get("evidence", {})
    require(entry.get("status") == "settled", "ledger entry is not settled")
    require(authorization.get("status") == "granted", "ledger authorization is not granted")
    require(
        authorization.get("reference") == result.get("authorization_reference"),
        "ledger authorization does not match result",
    )
    require(execution.get("model") == plan["model"], "ledger model does not match plan")
    require(execution.get("sample_ids") == [plan["sample"]], "ledger sample does not match plan")
    require(execution.get("arm_ids") == plan["arms"], "ledger arms do not match plan")
    require(
        execution.get("repeats_per_arm_per_sample") == plan["repeat"],
        "ledger repeat does not match plan",
    )
    require(
        execution.get("planned_sample_runs") == plan["planned_sample_runs"]
        and execution.get("actual_sample_runs") == plan["planned_sample_runs"],
        "ledger sample count does not match plan",
    )
    expected_result_path = str(result_path.relative_to(repo))
    require(evidence.get("result_path") == expected_result_path, "ledger result path mismatch")
    require(evidence.get("subject_commit") == result["subject_commit"], "ledger subject mismatch")
    require(evidence.get("surface_sha256") == result["surface_sha256"], "ledger surface mismatch")


def load_contract_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def validate_promotion_result(
    repo: Path,
    contract: dict[str, Any],
    result_path: Path,
    result: dict[str, Any],
) -> str:
    require(result.get("schema_version") == RESULT_SCHEMA_VERSION, "invalid result schema")
    require(result.get("status") == "pass", "cannot promote a non-passing result")
    plan = expected_run_plan(contract)
    require(result.get("run_plan") == plan, "result run plan does not match contract")
    require(
        result.get("policy_sha256") == canonical_json_sha256(contract["live_regression"]),
        "result policy digest does not match contract",
    )
    head = subprocess.check_output(
        ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True
    ).strip()
    require(result.get("subject_commit") == head, "result subject is not current HEAD")
    current_hash, _ = surface_snapshot(repo, contract, "worktree")
    require(result.get("surface_sha256") == current_hash, "result surface is not current")
    arms = result.get("arms")
    require(isinstance(arms, list), "result arms are missing")
    require(
        result.get("actual_sample_runs") == plan["planned_sample_runs"] == len(arms),
        "actual sample count does not match approved plan",
    )
    require([arm.get("arm") for arm in arms] == plan["arms"], "result arms do not match approved plan")
    for arm in arms:
        artifacts = arm.get("artifacts", {})
        hashes = arm.get("artifact_sha256", {})
        paths = {key: evidence_path(repo, artifacts[key]) for key in ("cache_summary", "request_summary", "metrics")}
        require(
            all(file_sha256(path) == hashes.get(key) for key, path in paths.items()),
            f"artifact digest mismatch for {arm['arm']}",
        )
        recomputed = analyze_artifacts(
            paths["cache_summary"], paths["request_summary"], paths["metrics"], arm["arm"]
        )
        require(
            all(arm.get(key) == recomputed[key] for key in ARM_EVIDENCE_KEYS),
            f"artifact metrics mismatch for {arm['arm']}",
        )
        require(arm.get("passed") is True, f"arm {arm['arm']} is not marked passing")
        require(
            arm_passes(recomputed, contract["live_regression"], contract.get("baseline")),
            f"arm {arm['arm']} does not satisfy current thresholds",
        )
    require(
        result.get("evidence_sha256") == canonical_json_sha256(evidence_manifest(arms)),
        "result evidence digest mismatch",
    )
    validate_ledger(repo, result_path, result, plan)
    return current_hash


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("result", type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--contract",
        type=Path,
        default=Path("benchmarks/cache-regression/cache-surface-contract.json"),
    )
    args = parser.parse_args()
    repo = args.repo_root.resolve()
    contract_path = args.contract if args.contract.is_absolute() else repo / args.contract
    result_path = args.result if args.result.is_absolute() else repo / args.result
    contract = load_contract(contract_path)
    result = load_contract_json(result_path)
    try:
        current_hash = validate_promotion_result(repo, contract, result_path, result)
    except (KeyError, TypeError, ValueError) as error:
        raise SystemExit(str(error)) from error
    contract["baseline"] = {
        "surface_sha256": current_hash,
        "status": "live_verified",
        "source_commit": result["subject_commit"],
        "live_result_path": str(result_path.relative_to(repo)),
        "note": "由 Standard + map-request 最简缓存回归晋升。",
        "request_2_plus_hit_rate": {
            arm["arm"]: arm["request_2_plus_hit_rate"] for arm in result["arms"]
        },
    }
    write_json(contract_path, contract)
    print(f"promoted cache surface baseline: {current_hash}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
