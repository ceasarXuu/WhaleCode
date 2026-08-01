#!/usr/bin/env python3
"""Idempotently settle a claimed cache-run ledger entry from a durable result."""

from __future__ import annotations

import argparse
from pathlib import Path

from cache_budget import validate_budget_selection
from cache_evidence import RESULT_SCHEMA_VERSION
from cache_json import exact_json_equal, strict_json_loads
from cache_result_envelope import validate_result_envelope
from cache_request_accounting import validate_result_request_accounting
from cache_run_ledger import mutate_entry, now, settle_entry


def read_json(path: Path) -> dict:
    value = strict_json_loads(path.read_text(encoding="utf-8-sig"))
    if not isinstance(value, dict):
        raise ValueError("cache recovery input must be an object")
    return value


def recover(repo: Path, ledger_path: Path, result_path: Path) -> str:
    result = read_json(result_path)
    if result.get("schema_version") != RESULT_SCHEMA_VERSION:
        raise ValueError("unsupported cache result schema")
    expected_path = result_path.resolve().relative_to(repo).as_posix()
    validate_result_envelope(result, expected_path, require_success=False)

    def settle(entry: dict) -> str:
        _validate_settlement_payload(result)
        _validate_claim_identity(entry, result)
        if (
            entry.get("status") in {"settled", "failed", "cancelled"}
            and entry.get("evidence", {}).get("result_path") == expected_path
        ):
            return "already_settled"
        if entry.get("status") not in {"planned", "running"}:
            raise ValueError("cache ledger record cannot be recovered")
        settle_entry(entry, result)
        return "settled"

    return mutate_entry(ledger_path, result["record_id"], settle)


def _validate_settlement_payload(result: dict) -> None:
    actual = result.get("actual_sample_runs")
    observations = result.get("observations")
    attempts = result.get("attempts")
    if (
        type(actual) is not int
        or actual < 0
        or not isinstance(observations, list)
        or not isinstance(attempts, list)
        or len(attempts) != actual
    ):
        raise ValueError("cache recovery result has an invalid run count")
    for observation in observations:
        if not isinstance(observation, dict):
            raise ValueError("cache recovery observation must be an object")
        for field in (
            "provider_requests",
            "input_tokens",
            "cached_input_tokens",
            "uncached_input_tokens",
            "output_tokens",
        ):
            value = observation.get(field)
            if type(value) is not int or value < 0:
                raise ValueError(f"cache recovery {field} must be nonnegative integer")
    accounted = 0
    for attempt in attempts:
        if not isinstance(attempt, dict):
            raise ValueError("cache recovery attempt must be an object")
        value = attempt.get("provider_boundary_request_count")
        if value is None and result.get("status") != "completed":
            continue
        if type(value) is not int or value < 0:
            raise ValueError("cache recovery request accounting is invalid")
        accounted += 1
    if result.get("status") == "completed" and accounted != actual:
        raise ValueError("completed cache recovery accounting is incomplete")
    validate_result_request_accounting(result)


def _validate_claim_identity(entry: dict, result: dict) -> None:
    execution = entry.get("execution", {})
    evidence = entry.get("evidence", {})
    authorization = entry.get("authorization", {})
    scope = evidence.get("approved_selection")
    validate_budget_selection(scope)
    expected_execution = {
        "model": scope.get("model"),
        "sample_ids": scope.get("samples"),
        "arm_ids": scope.get("arms"),
        "repeats_per_arm_per_sample": scope.get("repeat"),
        "planned_sample_runs": scope.get("planned_sample_runs"),
    }
    if (
        any(
            not exact_json_equal(execution.get(key), value)
            for key, value in expected_execution.items()
        )
        or result.get("subject_commit") != evidence.get("subject_commit")
        or result.get("surface_sha256") != evidence.get("surface_sha256")
        or result.get("proposal_id") != evidence.get("proposal_id")
        or result.get("proposal_sha256") != evidence.get("proposal_contract_sha256")
        or result.get("authorization_reference") != authorization.get("reference")
        or result.get("authorization_sha256") != evidence.get("authorization_sha256")
        or not exact_json_equal(result.get("observed_scope"), scope)
        or result.get("evidence_boundary") != evidence.get("evidence_boundary")
        or result.get("run_root") != evidence.get("planned_run_root")
    ):
        raise ValueError("cache recovery result does not match its durable claim")
    _validate_claimed_matrix(execution, result)
    _validate_claimed_budget(authorization.get("budget_summary"), result)


def _validate_claimed_matrix(execution: dict, result: dict) -> None:
    repeat = execution.get("repeats_per_arm_per_sample")
    samples = execution.get("sample_ids")
    arms = execution.get("arm_ids")
    if (
        type(repeat) is not int
        or repeat <= 0
        or not isinstance(samples, list)
        or not isinstance(arms, list)
    ):
        raise ValueError("cache recovery claim matrix is invalid")
    matrix = [
        {"sample": sample, "arm": arm, "repeat": index}
        for sample in samples
        for arm in arms
        for index in range(1, repeat + 1)
    ]
    actual = result["actual_sample_runs"]
    attempts = result["attempts"]
    attempt_scopes = [
        {key: attempt.get(key) for key in ("sample", "arm", "repeat")}
        for attempt in attempts
    ]
    if (
        actual > execution.get("planned_sample_runs", -1)
        or not exact_json_equal(attempt_scopes, matrix[:actual])
        or len({attempt.get("run_id") for attempt in attempts}) != actual
        or any(
            not isinstance(attempt.get("run_id"), str) or not attempt["run_id"].strip()
            for attempt in attempts
        )
    ):
        raise ValueError("cache recovery result exceeds its approved matrix")
    prior = -1
    for observation in result["observations"]:
        observation_scope = {
            field: observation.get(field) for field in ("sample", "arm", "repeat")
        }
        position = next(
            (
                index
                for index, attempt_scope in enumerate(attempt_scopes)
                if exact_json_equal(observation_scope, attempt_scope)
            ),
            -1,
        )
        if position <= prior or observation.get("run_id") != attempts[position].get(
            "run_id"
        ):
            raise ValueError("cache recovery observation is outside its attempt matrix")
        prior = position


def _validate_claimed_budget(budget: object, result: dict) -> None:
    if not isinstance(budget, dict):
        raise ValueError("cache recovery claim has no budget")
    request_minimum = sum(
        attempt.get("provider_boundary_request_count") or 0
        for attempt in result["attempts"]
    )
    totals = {
        key: sum(observation[key] for observation in result["observations"])
        for key in ("input_tokens", "output_tokens")
    }
    checks = (
        (request_minimum, budget.get("provider_requests")),
        (totals["input_tokens"], budget.get("input_tokens")),
        (totals["output_tokens"], budget.get("output_tokens")),
        (result["elapsed_seconds"], budget.get("elapsed_seconds")),
    )
    if any(
        type(limit) not in (int, float) or type(limit) is bool or value > limit
        for value, limit in checks
    ):
        raise ValueError("cache recovery result exceeds its approved budget")


def mark_unsettled(ledger_path: Path, record_id: str, reason: str) -> str:
    if not reason.strip():
        raise ValueError("unsettled recovery reason is required")

    def mark(entry: dict) -> str:
        if entry.get("status") == "unsettled":
            return "already_unsettled"
        if entry.get("status") not in {"planned", "running"}:
            raise ValueError("only an incomplete cache run can be marked unsettled")
        entry["status"] = "unsettled"
        entry["ended_at"] = now()
        known_requests = entry["execution"].get("api_requests")
        entry["execution"]["api_requests"] = None
        entry["execution"]["api_requests_minimum"] = (
            known_requests if type(known_requests) is int else 0
        )
        entry["execution"]["api_requests_evidence_status"] = "unavailable"
        entry["monetary_cost"].update(
            {
                "status": "unavailable",
                "amount": None,
                "components": None,
                "formula": None,
                "note": "运行在完整 result 落盘前中断；实际费用未知，需人工核账。",
            }
        )
        entry["evidence"].update(
            {
                "outcome": "unsettled",
                "usage_evidence_status": "unavailable",
                "recovery_reason": reason.strip(),
            }
        )
        return "unsettled"

    return mutate_entry(ledger_path, record_id, mark)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("result", type=Path, nargs="?")
    parser.add_argument("--record-id")
    parser.add_argument("--reason")
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--ledger",
        type=Path,
        default=Path("benchmarks/whale-agent-run-ledger.json"),
    )
    args = parser.parse_args()
    repo = args.repo_root.resolve()
    ledger_path = (
        args.ledger if args.ledger.is_absolute() else repo / args.ledger
    ).resolve()
    try:
        if args.result is not None and args.record_id is None:
            result_path = (
                args.result if args.result.is_absolute() else repo / args.result
            ).resolve()
            status = recover(repo, ledger_path, result_path)
        elif args.result is None and args.record_id and args.reason:
            status = mark_unsettled(ledger_path, args.record_id, args.reason)
        else:
            raise ValueError(
                "provide either a result path or --record-id with --reason"
            )
    except (KeyError, OSError, TypeError, ValueError) as error:
        raise SystemExit(str(error)) from error
    print(f"cache ledger recovery: {status}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
