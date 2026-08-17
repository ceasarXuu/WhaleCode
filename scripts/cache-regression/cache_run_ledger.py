#!/usr/bin/env python3
"""Atomic ledger claim and settlement for one cache regression authorization."""

from __future__ import annotations

import json
import os
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from cache_budget import selection_matrix
from cache_cost import settled_monetary_cost
from cache_evidence import file_sha256
from cache_json import exact_json_equal, strict_json_loads
from cache_provider_route import validate_route_profile_binding, validate_route_summary
from cache_request_accounting import summarize_request_accounting
from cache_result_integrity import validate_completed_result_integrity

try:
    import fcntl
except ImportError:  # pragma: no cover - exercised on Windows
    fcntl = None

try:
    import msvcrt
except ImportError:  # pragma: no cover - exercised on POSIX
    msvcrt = None


def now() -> str:
    return datetime.now(timezone.utc).astimezone().isoformat(timespec="seconds")


def _locked_ledger(path: Path, update) -> Any:
    lock_path = path.with_suffix(path.suffix + ".lock")
    with lock_path.open("a+", encoding="utf-8") as lock:
        _lock_file(lock)
        try:
            ledger = strict_json_loads(path.read_text(encoding="utf-8"))
            if not isinstance(ledger, dict) or not isinstance(
                ledger.get("entries"), list
            ):
                raise ValueError("cache ledger must contain an entries list")
            result = update(ledger)
            ledger["updated_at"] = now()
            atomic_write_json(path, ledger)
            return result
        finally:
            _unlock_file(lock)


def _lock_file(lock) -> None:
    if fcntl is not None:
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
        return
    if msvcrt is None:
        raise RuntimeError("no supported file-lock backend")
    lock.seek(0, os.SEEK_END)
    if lock.tell() == 0:
        lock.write("\0")
        lock.flush()
    lock.seek(0)
    msvcrt.locking(lock.fileno(), msvcrt.LK_LOCK, 1)


def _unlock_file(lock) -> None:
    if fcntl is not None:
        fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
        return
    if msvcrt is not None:
        lock.seek(0)
        msvcrt.locking(lock.fileno(), msvcrt.LK_UNLCK, 1)


def atomic_write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    temporary_path = Path(temporary)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(value, handle, ensure_ascii=False, indent=2, allow_nan=False)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary_path, path)
        if os.name == "posix":
            directory = os.open(path.parent, os.O_RDONLY)
            try:
                os.fsync(directory)
            finally:
                os.close(directory)
    finally:
        if temporary_path.exists():
            temporary_path.unlink()


def claim_entry(path: Path, entry: dict[str, Any]) -> None:
    authorization_id = entry["authorization"]["id"]

    def claim(ledger: dict[str, Any]) -> None:
        if any(
            item.get("authorization", {}).get("id") == authorization_id
            for item in ledger["entries"]
        ):
            raise ValueError("cache authorization has already been claimed")
        ledger["entries"].insert(0, entry)

    _locked_ledger(path, claim)


def entry_exists(path: Path, record_id: str) -> bool:
    """Return whether a claim reached durable ledger storage."""
    lock_path = path.with_suffix(path.suffix + ".lock")
    with lock_path.open("a+", encoding="utf-8") as lock:
        _lock_file(lock)
        try:
            ledger = strict_json_loads(path.read_text(encoding="utf-8"))
            return any(item.get("record_id") == record_id for item in ledger["entries"])
        finally:
            _unlock_file(lock)


def store_entry(path: Path, entry: dict[str, Any]) -> None:
    def store(ledger: dict[str, Any]) -> None:
        matches = [
            index
            for index, item in enumerate(ledger["entries"])
            if item.get("record_id") == entry["record_id"]
        ]
        if len(matches) != 1:
            raise ValueError("cache ledger record is missing or duplicated")
        ledger["entries"][matches[0]] = entry

    _locked_ledger(path, store)


def mutate_entry(path: Path, record_id: str, update) -> Any:
    """Read, compare and update one ledger record under the same lock."""

    def mutate(ledger: dict[str, Any]) -> Any:
        matches = [
            item for item in ledger["entries"] if item.get("record_id") == record_id
        ]
        if len(matches) != 1:
            raise ValueError("cache ledger record is missing or duplicated")
        return update(matches[0])

    return _locked_ledger(path, mutate)


def checkpoint_request_count(entry: dict[str, Any], request_count: int) -> None:
    if type(request_count) is not int or request_count < 0:
        raise ValueError("cache request checkpoint must be a nonnegative integer")
    execution = entry["execution"]
    exact = execution.get("api_requests")
    minimum = execution.get("api_requests_minimum")
    known = (
        exact
        if type(exact) is int and exact >= 0
        else minimum
        if type(minimum) is int and minimum >= 0
        else 0
    )
    execution["api_requests"] = None
    execution["api_requests_minimum"] = known + request_count
    execution["api_requests_evidence_status"] = "partial"


def planned_entry(
    record_id: str,
    proposal: dict[str, Any],
    authorization: dict[str, Any],
    proposal_path: Path,
    authorization_path: Path,
    repo: Path,
    run_root: Path,
    provider_route: dict[str, Any],
) -> dict[str, Any]:
    selection = proposal["selection"]
    pricing = proposal["pricing_snapshot"]
    planned_run_root = run_root.resolve().relative_to(repo).as_posix()
    return {
        "record_id": record_id,
        "record_type": "run_batch",
        "status": "planned",
        "started_at": None,
        "ended_at": None,
        "elapsed_calendar_seconds": None,
        "aggregate_agent_wall_time_ms": None,
        "reason": selection["selection_reason"],
        "authorization": {
            "required": True,
            "status": "granted",
            "id": authorization["authorization_id"],
            "reference": authorization["approval_reference"],
            "budget_summary": proposal.get(
                "approved_maximums", proposal.get("maximums")
            ),
            "note": f"严格绑定预算提案 {proposal['proposal_id']}。",
        },
        "execution": {
            "provider": provider_route["provider_routing"]["logical_provider_id"],
            "transport_provider": provider_route["provider_routing"][
                "transport_provider_id"
            ],
            "provider_descriptor_sha256": provider_route[
                "provider_descriptor_sha256"
            ],
            "model": selection["model"],
            "batch_count": 1,
            "sample_ids": selection["samples"],
            "arm_ids": selection["arms"],
            "repeats_per_arm_per_sample": selection["repeat"],
            "planned_sample_runs": selection["planned_sample_runs"],
            "actual_sample_runs": 0,
            "api_requests": 0,
            "api_requests_evidence_status": "pending",
        },
        "tokens": {"input": 0, "cached_input": 0, "uncached_input": 0, "output": 0},
        "monetary_cost": {
            "status": "planned",
            "currency": pricing["currency"],
            "amount": None,
            "actual_billed_amount": None,
            "components": None,
            "pricing_snapshot": pricing,
            "formula": None,
            "note": "运行后按 provider token 遥测估算。",
        },
        "evidence": {
            "planned_run_root": planned_run_root,
            "subject_commit": proposal["subject_commit"],
            "surface_sha256": proposal["surface_sha256"],
            "proposal_id": proposal["proposal_id"],
            "proposal_contract_sha256": proposal["proposal_sha256"],
            "proposal_path": proposal_path.relative_to(repo).as_posix(),
            "proposal_sha256": file_sha256(proposal_path),
            "authorization_path": authorization_path.relative_to(repo).as_posix(),
            "authorization_sha256": file_sha256(authorization_path),
            "approved_selection": selection,
            "evidence_boundary": proposal["evidence_boundary"],
            "stop_conditions": selection["stop_conditions"],
            "usage_evidence_status": "pending",
            "provider_route_attestation_path": provider_route["artifact_path"],
            "provider_route_attestation_sha256": provider_route["artifact_sha256"],
        },
    }


def settle_entry(entry: dict[str, Any], result: dict[str, Any]) -> None:
    expected_matrix = (
        selection_matrix(entry.get("evidence", {}).get("approved_selection"))
        if result.get("status") == "completed"
        else None
    )
    validate_completed_result_integrity(result, expected_matrix)
    route = validate_route_summary(
        result.get("provider_route_attestation"), entry["execution"]["model"]
    )
    route_identity = route["provider_routing"]
    route_matches = (
        entry["execution"].get("provider") == route_identity["logical_provider_id"]
        and entry["execution"].get("transport_provider")
        == route_identity["transport_provider_id"]
        and entry["execution"].get("provider_descriptor_sha256")
        == route["provider_descriptor_sha256"]
        and entry["evidence"].get("provider_route_attestation_path")
        == route["artifact_path"]
        and entry["evidence"].get("provider_route_attestation_sha256")
        == route["artifact_sha256"]
        and isinstance(result.get("observations"), list)
        and all(
            isinstance(observation, dict)
            and exact_json_equal(observation.get("provider_routing"), route_identity)
            and validate_route_profile_binding(
                observation.get("provider_route_profile"),
                route,
                observation.get("arm"),
            )
            for observation in result["observations"]
        )
    )
    if not route_matches:
        raise ValueError("cache ledger provider route evidence is inconsistent")
    observations = result.get("observations", [])
    attempts = result.get("attempts", [])
    authoritative_request_total, request_accounting_status = (
        summarize_request_accounting(attempts, result.get("actual_sample_runs"))
    )
    accounting_complete = request_accounting_status == "complete"
    usage_complete = (
        bool(observations)
        and accounting_complete
        and (
            result.get("status") == "completed"
            and len(observations) == result.get("actual_sample_runs")
        )
    )
    totals = {
        key: sum(int(observation[key]) for observation in observations)
        for key in (
            "provider_requests",
            "input_tokens",
            "cached_input_tokens",
            "uncached_input_tokens",
            "output_tokens",
        )
    }
    usage_complete = usage_complete and (
        totals["provider_requests"] == authoritative_request_total
    )
    pricing = entry["monetary_cost"]["pricing_snapshot"]
    entry["status"] = (
        "cancelled"
        if result["status"] == "cancelled"
        else "settled"
        if result["status"] == "completed"
        else "failed"
    )
    entry["started_at"] = result["started_at"]
    entry["ended_at"] = result["ended_at"]
    entry["elapsed_calendar_seconds"] = result["elapsed_seconds"]
    entry["execution"]["actual_sample_runs"] = result["actual_sample_runs"]
    if accounting_complete:
        entry["execution"]["api_requests"] = authoritative_request_total
        entry["execution"].pop("api_requests_minimum", None)
        entry["execution"]["api_requests_evidence_status"] = "complete"
    else:
        entry["execution"]["api_requests"] = None
        entry["execution"]["api_requests_minimum"] = authoritative_request_total
        entry["execution"]["api_requests_evidence_status"] = request_accounting_status
    entry["tokens"] = {
        "input": totals["input_tokens"],
        "cached_input": totals["cached_input_tokens"],
        "uncached_input": totals["uncached_input_tokens"],
        "output": totals["output_tokens"],
    }
    entry["monetary_cost"] = settled_monetary_cost(
        entry["tokens"],
        pricing,
        evidence_status=(
            "complete"
            if usage_complete
            else "partial"
            if observations
            else "unavailable"
        ),
    )
    entry["evidence"].update(
        {
            "actual_run_root": result.get("run_root"),
            "result_path": result.get("result_path"),
            "runner_exit_code": result.get("runner_exit_code"),
            "outcome": result.get("status", "failed"),
            "usage_evidence_status": "complete"
            if usage_complete
            else "partial"
            if observations
            else "unavailable",
        }
    )
