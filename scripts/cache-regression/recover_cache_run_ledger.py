#!/usr/bin/env python3
"""Idempotently settle a claimed cache-run ledger entry from a durable result."""

from __future__ import annotations

import argparse
from pathlib import Path

from cache_evidence import RESULT_SCHEMA_VERSION
from cache_json import strict_json_loads
from cache_result_envelope import validate_result_envelope
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
    _validate_settlement_payload(result)

    def settle(entry: dict) -> str:
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
    for attempt in attempts:
        if not isinstance(attempt, dict):
            raise ValueError("cache recovery attempt must be an object")
        value = attempt.get("provider_boundary_request_count")
        if type(value) is not int or value < 0:
            raise ValueError("cache recovery request accounting is invalid")


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
