#!/usr/bin/env python3
"""Atomic ledger claim and settlement for one cache regression authorization."""

from __future__ import annotations

import fcntl
import json
import os
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from cache_evidence import file_sha256


def now() -> str:
    return datetime.now(timezone.utc).astimezone().isoformat(timespec="seconds")


def _locked_ledger(path: Path, update) -> None:
    lock_path = path.with_suffix(path.suffix + ".lock")
    with lock_path.open("a+", encoding="utf-8") as lock:
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
        ledger = json.loads(path.read_text(encoding="utf-8"))
        update(ledger)
        ledger["updated_at"] = now()
        atomic_write_json(path, ledger)


def atomic_write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    temporary_path = Path(temporary)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(value, handle, ensure_ascii=False, indent=2)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary_path, path)
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


def planned_entry(
    record_id: str,
    proposal: dict[str, Any],
    authorization: dict[str, Any],
    proposal_path: Path,
    authorization_path: Path,
    repo: Path,
    run_root: Path,
) -> dict[str, Any]:
    selection = proposal["selection"]
    pricing = proposal["pricing_snapshot"]
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
            "budget_summary": proposal["maximums"],
            "note": f"严格绑定预算提案 {proposal['proposal_id']}。",
        },
        "execution": {
            "provider": "deepseek",
            "model": selection["model"],
            "batch_count": 1,
            "sample_ids": selection["samples"],
            "arm_ids": selection["arms"],
            "repeats_per_arm_per_sample": selection["repeat"],
            "planned_sample_runs": selection["planned_sample_runs"],
            "actual_sample_runs": 0,
            "api_requests": 0,
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
            "planned_run_root": str(run_root),
            "subject_commit": proposal["subject_commit"],
            "surface_sha256": proposal["surface_sha256"],
            "proposal_path": proposal_path.relative_to(repo).as_posix(),
            "proposal_sha256": file_sha256(proposal_path),
            "authorization_path": authorization_path.relative_to(repo).as_posix(),
            "authorization_sha256": file_sha256(authorization_path),
            "stop_conditions": selection["stop_conditions"],
            "usage_evidence_status": "pending",
        },
    }


def settle_entry(entry: dict[str, Any], result: dict[str, Any]) -> None:
    observations = result.get("observations", [])
    usage_complete = bool(observations) and (
        result.get("status") == "completed"
        and len(observations) == result.get("actual_sample_runs")
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
    pricing = entry["monetary_cost"]["pricing_snapshot"]
    components = {
        "cached_input": totals["cached_input_tokens"]
        / 1_000_000
        * pricing["cached_input_per_million"],
        "uncached_input": totals["uncached_input_tokens"]
        / 1_000_000
        * pricing["uncached_input_per_million"],
        "output": totals["output_tokens"] / 1_000_000 * pricing["output_per_million"],
    }
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
    entry["execution"]["api_requests"] = totals["provider_requests"]
    entry["execution"]["api_requests_evidence_status"] = (
        "complete" if usage_complete else "partial" if observations else "unavailable"
    )
    entry["tokens"] = {
        "input": totals["input_tokens"],
        "cached_input": totals["cached_input_tokens"],
        "uncached_input": totals["uncached_input_tokens"],
        "output": totals["output_tokens"],
    }
    entry["monetary_cost"].update(
        {
            "status": (
                "estimated"
                if usage_complete
                else "estimated_partial"
                if observations
                else "unavailable"
            ),
            "amount": round(sum(components.values()), 10) if observations else None,
            "components": components if observations else None,
            "formula": (
                "cached_input/1e6*cached_rate + uncached_input/1e6*miss_rate "
                "+ output/1e6*output_rate"
                if observations
                else None
            ),
            "note": (
                "按完整 provider token 遥测和冻结价格估算。"
                if usage_complete
                else "按已取得的部分 provider token 遥测估算；金额是已知最低值。"
                if observations
                else "无完整 token 证据。"
            ),
        }
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
