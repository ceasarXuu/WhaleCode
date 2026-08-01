#!/usr/bin/env python3
"""Execute one exactly authorized cache smoke proposal and settle its ledger."""

from __future__ import annotations

import argparse
import json
import shutil
import time
import uuid
from datetime import datetime
from pathlib import Path
from typing import Any

from cache_cleanup_contract import cleanup_verified
from cache_evidence import RESULT_SCHEMA_VERSION, file_sha256
from cache_arm_identity import validate_arm_identity
from cache_json import strict_json_loads
from cache_process_control import (
    cleanup_labeled_containers,
    run_benchmark_command,
)
from cache_provider_boundary_evidence import (
    persist_provider_boundary_accounting,
    read_provider_boundary_request_count,
)
from cache_run_environment import ensure_deepseek_api_key, find_run_dir_by_id
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
from cache_run_ledger import (
    checkpoint_request_count,
    claim_entry,
    entry_exists,
    now,
    planned_entry,
    store_entry,
)
from cache_run_result import finalize_run_result
from cache_run_supervision import emergency_cleanup, finalize_and_persist
from cache_surface import load_contract


def persist_observation_artifacts(
    repo: Path,
    record_id: str,
    run_id: str,
    arm: str,
    expected_model: str,
    observation: dict[str, Any],
) -> dict[str, Any]:
    destination = repo / "benchmarks/cache-regression/evidence" / record_id / run_id
    destination.mkdir(parents=True, exist_ok=True)
    artifact_names = {
        "cache_summary": "provider-cache-trace-summary.json",
        "request_summary": "request-summary.json",
        "metrics": "metrics.json",
        "provider_boundary": "provider-boundary-evidence.json",
        "execution_argv": "whale-argv.json",
        "logical_mode_map": "logical-mode-map.json",
    }
    metrics_source = Path(observation["artifacts"]["metrics"])
    source_paths = {
        **{
            key: Path(observation["artifacts"][key]) for key in observation["artifacts"]
        },
        "execution_argv": metrics_source.parent / "whale-argv.json",
        "logical_mode_map": metrics_source.parent.parent.parent
        / "logical-mode-map.json",
    }
    persisted = {}
    for key, filename in artifact_names.items():
        source = source_paths[key]
        if not source.is_file():
            raise FileNotFoundError(f"cache observation artifact is missing: {key}")
        target = destination / filename
        if target.is_file():
            if file_sha256(target) != file_sha256(source):
                raise ValueError(f"persisted cache artifact changed: {key}")
        else:
            shutil.copyfile(source, target)
        persisted[key] = target
    durable = analyze_artifacts(
        persisted["cache_summary"],
        persisted["request_summary"],
        persisted["metrics"],
        persisted["provider_boundary"],
        arm,
        expected_model,
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
    validate_arm_identity(
        strict_json_loads(persisted["execution_argv"].read_text(encoding="utf-8-sig")),
        strict_json_loads(
            persisted["logical_mode_map"].read_text(encoding="utf-8-sig")
        ),
        arm,
    )
    durable["artifacts"] = {
        key: path.relative_to(repo).as_posix() for key, path in persisted.items()
    }
    durable["artifact_sha256"] = {
        key: file_sha256(path) for key, path in persisted.items()
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


def execute_attempts(
    repo: Path,
    whale_bin: Path,
    run_root: Path,
    record_id: str,
    proposal: dict[str, Any],
    matrix: list[dict[str, Any]],
    result: dict[str, Any],
    entry: dict[str, Any],
    ledger_path: Path,
) -> tuple[str | None, bool, bool, bool]:
    selection = proposal["selection"]
    limits = proposal["per_sample_run_limits"]
    thresholds = proposal["per_sample_run_observation_thresholds"]
    stop_at = None
    cancelled = False
    cleanup_failed = False
    supervision_failed = False
    for index, execution in enumerate(matrix, start=1):
        run_id = f"{record_id}-CACHE-{index:03d}"
        command = benchmark_command(
            repo, whale_bin, run_root, run_id, proposal, execution
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
        cleanup = None
        try:
            completed = run_benchmark_command(command, repo, limits["elapsed_seconds"])
            attempt["exit_code"] = completed.returncode
            attempt["timed_out"] = False
            run_failed = completed.returncode != 0
        except TimeoutError as error:
            attempt["exit_code"] = None
            attempt["timed_out"] = True
            attempt["process_tree_termination"] = getattr(
                error, "process_tree_termination", {"status": "unknown"}
            )
            cleanup = cleanup_labeled_containers(
                run_id, limits["cleanup_grace_seconds"], run_root
            )
            attempt["timeout_cleanup"] = cleanup
            run_failed = True
        except OSError as error:
            attempt["exit_code"] = None
            attempt["timed_out"] = False
            attempt["execution_error"] = f"{type(error).__name__}: {error}"
            run_failed = True
        except KeyboardInterrupt as error:
            attempt["exit_code"] = None
            attempt["timed_out"] = False
            attempt["execution_error"] = "KeyboardInterrupt: run cancelled"
            attempt["process_tree_termination"] = getattr(
                error, "process_tree_termination", {"status": "unknown"}
            )
            interrupt_cleanup = cleanup_labeled_containers(
                run_id, limits["cleanup_grace_seconds"], run_root
            )
            attempt["interrupt_cleanup"] = interrupt_cleanup
            attempt["post_run_cleanup"] = interrupt_cleanup
            attempt["elapsed_seconds"] = round(time.time() - run_started, 3)
            if cleanup_verified(interrupt_cleanup):
                attempt["status"] = "cancelled"
                cancelled = True
                stop_at = "cancelled"
            else:
                attempt["status"] = "failed"
                attempt["cleanup_error"] = interrupt_cleanup.get(
                    "error", "cleanup could not be verified"
                )
                cleanup_failed = True
                stop_at = "cancelled_cleanup_failed"
            break
        try:
            attempt["elapsed_seconds"] = round(time.time() - run_started, 3)
            cleanup = cleanup or cleanup_labeled_containers(
                run_id, limits["cleanup_grace_seconds"], run_root
            )
            attempt["post_run_cleanup"] = cleanup
            if not cleanup_verified(cleanup):
                run_failed = True
            try:
                run_dir = find_run_dir_by_id(run_root, run_id)
                side = "left" if execution["arm"] == "standard" else "right"
                request_count = read_provider_boundary_request_count(
                    run_dir, side, selection["model"]
                )
                attempt["provider_boundary_request_count"] = request_count
                checkpoint_request_count(entry, request_count)
                store_entry(ledger_path, entry)
                attempt.update(
                    persist_provider_boundary_accounting(
                        repo,
                        record_id,
                        run_id,
                        run_dir,
                        side,
                        selection["model"],
                        request_count,
                    )
                )
                observation = analyze_arm(
                    run_dir, side, execution["arm"], selection["model"]
                )
                observation = persist_observation_artifacts(
                    repo,
                    record_id,
                    run_id,
                    execution["arm"],
                    selection["model"],
                    observation,
                )
                observation.update(execution)
                observation["run_id"] = run_id
                observation["elapsed_seconds"] = attempt["elapsed_seconds"]
                observation["budget_observation_exceeded"] = (
                    budget_observation_exceeded(observation, limits, thresholds)
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
        except (KeyboardInterrupt, OSError) as error:
            supervision_failed = True
            attempt["elapsed_seconds"] = round(time.time() - run_started, 3)
            attempt["execution_error"] = f"{type(error).__name__}: {error}"
            recovery = emergency_cleanup(
                cleanup_labeled_containers,
                run_id,
                limits["cleanup_grace_seconds"],
                run_root,
            )
            attempt["supervisor_cleanup"] = recovery
            attempt["post_run_cleanup"] = recovery
            attempt["status"] = "failed"
            cleanup_failed = cleanup_failed or not cleanup_verified(recovery)
            stop_at = (
                "supervisor_interrupted"
                if isinstance(error, KeyboardInterrupt)
                else "supervisor_failure"
            )
            attempt["stop_reason"] = stop_at
            break
    return stop_at, cancelled, cleanup_failed, supervision_failed


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
    result_path = repo / "benchmarks/cache-regression/results" / f"{record_id}.json"
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
        "result_path": str(result_path.relative_to(repo)),
        "attempts": [],
        "observations": [],
    }
    stop_at = None
    cancelled = False
    cleanup_failed = False
    supervision_failed = False
    claim_error = None
    try:
        try:
            claim_entry(ledger_path, entry)
        except ValueError as error:
            claim_error = error
        if claim_error is None:
            stop_at, cancelled, cleanup_failed, supervision_failed = execute_attempts(
                repo,
                args.whale_bin,
                run_root,
                record_id,
                proposal,
                matrix,
                result,
                entry,
                ledger_path,
            )
    except BaseException as error:
        supervision_failed = True
        stop_at = (
            "supervisor_interrupted"
            if isinstance(error, KeyboardInterrupt)
            else "supervisor_failure"
        )
        if result["attempts"]:
            attempt = result["attempts"][-1]
            recovery = emergency_cleanup(
                cleanup_labeled_containers,
                attempt["run_id"],
                limits["cleanup_grace_seconds"],
                run_root,
            )
            attempt["supervisor_cleanup"] = recovery
            attempt["post_run_cleanup"] = recovery
            attempt["status"] = "failed"
            attempt["execution_error"] = f"{type(error).__name__}: {error}"
            attempt["stop_reason"] = stop_at
            cleanup_failed = not cleanup_verified(recovery)
    finally:
        if claim_error is None and entry_exists(ledger_path, record_id):
            finalize_and_persist(
                entry,
                result,
                result_path,
                ledger_path,
                started,
                lambda: finalize_run_result(
                    result,
                    matrix,
                    stop_at,
                    cleanup_failed=cleanup_failed,
                    supervision_failed=supervision_failed,
                    cancelled=cancelled,
                    started=started,
                    execution_completed=execution_completed,
                ),
            )
    if claim_error is not None:
        raise SystemExit(str(claim_error)) from claim_error
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return result["runner_exit_code"]


if __name__ == "__main__":
    raise SystemExit(main())
