#!/usr/bin/env python3
"""Execute one exactly authorized cache smoke proposal and settle its ledger."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import time
import uuid
from datetime import datetime
from pathlib import Path
from typing import Any

from cache_evidence import (
    RESULT_SCHEMA_VERSION,
    canonical_json_sha256,
    file_sha256,
)
from cache_run_analysis import (
    analyze_arm,
    analyze_artifacts,
    budget_observation_exceeded,
)
from cache_run_contract import (
    benchmark_command,
    execution_matrix,
    load_authorized_proposal,
)
from cache_run_ledger import claim_entry, now, planned_entry, settle_entry, store_entry
from cache_surface import load_contract, write_json


def ensure_deepseek_api_key(repo: Path) -> str:
    if os.environ.get("DEEPSEEK_API_KEY", "").strip():
        return "process_environment"
    env_path = repo / ".env.local"
    if not env_path.is_file():
        raise RuntimeError("DEEPSEEK_API_KEY is missing and .env.local does not exist")
    for raw_line in env_path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("export "):
            line = line.removeprefix("export ").lstrip()
        key, separator, value = line.partition("=")
        if separator and key.strip() == "DEEPSEEK_API_KEY":
            value = value.strip()
            if len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}:
                value = value[1:-1]
            if not value:
                break
            os.environ["DEEPSEEK_API_KEY"] = value
            return ".env.local"
    raise RuntimeError("DEEPSEEK_API_KEY is missing from .env.local")


def find_run_dir_by_id(run_root: Path, run_id: str) -> Path:
    candidates = [path for path in run_root.glob(f"*/{run_id}") if path.is_dir()]
    if len(candidates) != 1:
        raise RuntimeError(
            f"benchmark run id {run_id} resolved to {len(candidates)} directories"
        )
    return candidates[0]


def persist_observation_artifacts(
    repo: Path,
    record_id: str,
    run_id: str,
    arm: str,
    observation: dict[str, Any],
) -> dict[str, Any]:
    destination = repo / "benchmarks/cache-regression/evidence" / record_id / run_id
    destination.mkdir(parents=True, exist_ok=False)
    artifact_names = {
        "cache_summary": "provider-cache-trace-summary.json",
        "request_summary": "request-summary.json",
        "metrics": "metrics.json",
    }
    persisted = {}
    for key, filename in artifact_names.items():
        source = Path(observation["artifacts"][key])
        if not source.is_file():
            raise FileNotFoundError(f"cache observation artifact is missing: {key}")
        target = destination / filename
        shutil.copyfile(source, target)
        persisted[key] = target
    durable = analyze_artifacts(
        persisted["cache_summary"],
        persisted["request_summary"],
        persisted["metrics"],
        arm,
    )
    observed_values = {
        key: value
        for key, value in observation.items()
        if key not in {"artifacts", "artifact_sha256"}
    }
    durable_values = {
        key: value
        for key, value in durable.items()
        if key not in {"artifacts", "artifact_sha256"}
    }
    if (
        durable_values != observed_values
        or durable["artifact_sha256"] != observation["artifact_sha256"]
    ):
        raise ValueError("persisted cache observation does not match source artifacts")
    durable["artifacts"] = {
        key: path.relative_to(repo).as_posix() for key, path in persisted.items()
    }
    return durable


def stop_reason(
    conditions: list[str],
    run_failed: bool,
    observation: dict[str, Any] | None,
) -> str | None:
    del conditions
    if run_failed:
        if observation is None:
            return "run_failure"
        if not observation["business_success"]:
            return "business_failure"
        if observation["cache_usage_missing_count"] > 0:
            return "usage_gap"
        if observation["budget_observation_exceeded"]:
            return "budget_observation_exceeded"
        return "run_failure"
    return None


def execution_completed(
    matrix: list[dict[str, Any]],
    attempts: list[dict[str, Any]],
    observations: list[dict[str, Any]],
) -> bool:
    return (
        len(attempts) == len(matrix)
        and len(observations) == len(matrix)
        and all(attempt["status"] == "completed" for attempt in attempts)
        and all(
            not observation["budget_observation_exceeded"]
            for observation in observations
        )
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--proposal", type=Path, required=True)
    parser.add_argument("--authorization", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--whale-bin", type=Path, default=Path.home() / ".whale/bin/whale"
    )
    parser.add_argument("--run-root", type=Path)
    args = parser.parse_args()
    repo = args.repo_root.resolve()
    contract_path = repo / "benchmarks/cache-regression/cache-surface-contract.json"
    contract = load_contract(contract_path)
    try:
        proposal, authorization, proposal_path, authorization_path = (
            load_authorized_proposal(repo, contract, args.proposal, args.authorization)
        )
        matrix = execution_matrix(proposal)
    except (KeyError, TypeError, ValueError) as error:
        raise SystemExit(str(error)) from error
    credential_source = ensure_deepseek_api_key(repo)
    stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    record_id = f"WAR-{stamp}-CACHE-REGRESSION-{uuid.uuid4().hex[:8].upper()}"
    run_root = args.run_root or repo / "target/cache-hit-regression" / record_id
    ledger_path = repo / "benchmarks/whale-agent-run-ledger.json"
    entry = planned_entry(
        record_id,
        proposal,
        authorization,
        proposal_path,
        authorization_path,
        repo,
        run_root,
    )
    try:
        claim_entry(ledger_path, entry)
    except ValueError as error:
        raise SystemExit(str(error)) from error

    started = time.time()
    selection = proposal["selection"]
    limits = proposal["per_sample_run_limits"]
    result = {
        "schema_version": RESULT_SCHEMA_VERSION,
        "record_id": record_id,
        "status": "failed",
        "started_at": now(),
        "subject_commit": proposal["subject_commit"],
        "surface_sha256": proposal["surface_sha256"],
        "proposal_id": proposal["proposal_id"],
        "proposal_sha256": proposal["proposal_sha256"],
        "authorization_reference": authorization["approval_reference"],
        "authorization_sha256": file_sha256(authorization_path),
        "observed_scope": selection,
        "unverified_scope": [],
        "evidence_boundary": proposal["evidence_boundary"],
        "actual_sample_runs": 0,
        "credential_source": credential_source,
        "run_root": str(run_root.relative_to(repo)),
        "attempts": [],
        "observations": [],
    }
    stop_at = None
    cancelled = False
    for index, execution in enumerate(matrix, start=1):
        run_id = f"CACHE-{index:03d}"
        command = benchmark_command(
            repo, args.whale_bin, run_root, run_id, proposal, execution
        )
        attempt = {**execution, "run_id": run_id, "status": "running"}
        result["attempts"].append(attempt)
        result["actual_sample_runs"] += 1
        entry["status"] = "running"
        entry["started_at"] = entry["started_at"] or now()
        entry["execution"]["actual_sample_runs"] = result["actual_sample_runs"]
        store_entry(ledger_path, entry)
        run_started = time.time()
        run_failed = False
        observation = None
        try:
            completed = subprocess.run(
                command,
                cwd=repo,
                check=False,
                timeout=limits["elapsed_seconds"],
            )
            attempt["exit_code"] = completed.returncode
            attempt["timed_out"] = False
            run_failed = completed.returncode != 0
        except subprocess.TimeoutExpired:
            attempt["exit_code"] = None
            attempt["timed_out"] = True
            run_failed = True
        except OSError as error:
            attempt["exit_code"] = None
            attempt["timed_out"] = False
            attempt["execution_error"] = f"{type(error).__name__}: {error}"
            run_failed = True
        except KeyboardInterrupt:
            attempt["exit_code"] = None
            attempt["timed_out"] = False
            attempt["status"] = "cancelled"
            attempt["execution_error"] = "KeyboardInterrupt: run cancelled"
            cancelled = True
            stop_at = "cancelled"
            break
        attempt["elapsed_seconds"] = round(time.time() - run_started, 3)
        try:
            run_dir = find_run_dir_by_id(run_root, run_id)
            side = "left" if execution["arm"] == "standard" else "right"
            observation = analyze_arm(run_dir, side, execution["arm"])
            observation = persist_observation_artifacts(
                repo,
                record_id,
                run_id,
                execution["arm"],
                observation,
            )
            observation.update(execution)
            observation["run_id"] = run_id
            observation["elapsed_seconds"] = attempt["elapsed_seconds"]
            observation["budget_observation_exceeded"] = budget_observation_exceeded(
                observation, limits
            )
            result["observations"].append(observation)
            attempt["run_dir"] = str(run_dir.relative_to(repo))
        except (
            FileNotFoundError,
            KeyError,
            RuntimeError,
            TypeError,
            ValueError,
        ) as error:
            attempt["evidence_error"] = f"{type(error).__name__}: {error}"
            run_failed = True
        observation_failed = bool(
            observation
            and (
                not observation["business_success"]
                or observation["cache_usage_missing_count"] > 0
                or observation["budget_observation_exceeded"]
            )
        )
        run_failed = run_failed or observation_failed
        attempt["status"] = "failed" if run_failed else "completed"
        stop_at = stop_reason(selection["stop_conditions"], run_failed, observation)
        if stop_at is not None:
            attempt["stop_reason"] = stop_at
            break

    attempted_keys = {
        (item["sample"], item["arm"], item["repeat"]) for item in result["attempts"]
    }
    result["unverified_scope"] = [
        item
        for item in matrix
        if (item["sample"], item["arm"], item["repeat"]) not in attempted_keys
    ]
    result["stop_reason"] = stop_at
    if cancelled:
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

    result_dir = repo / "benchmarks/cache-regression/results"
    result_path = result_dir / f"{record_id}.json"
    result["result_path"] = str(result_path.relative_to(repo))
    write_json(result_path, result)
    settle_entry(entry, result, started)
    store_entry(ledger_path, entry)
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return result["runner_exit_code"]


if __name__ == "__main__":
    raise SystemExit(main())
