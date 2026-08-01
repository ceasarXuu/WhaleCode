#!/usr/bin/env python3
"""Ledger consistency checks for accepted cache regression evidence."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from cache_cost import settled_monetary_cost
from cache_json import exact_json_equal
from cache_source_evidence import require, source_json, source_sha256


def exact_int(value: Any, expected: int) -> bool:
    return type(value) is int and type(expected) is int and value == expected


def validate_ledger(
    repo: Path,
    result: dict[str, Any],
    acceptance: dict[str, Any],
    proposal: dict[str, Any],
    authorization: dict[str, Any],
    proposal_path: str,
    authorization_path: str,
    source: str,
) -> str:
    path = "benchmarks/whale-agent-run-ledger.json"
    ledger = source_json(repo, path, source)
    matches = [
        item
        for item in ledger.get("entries", [])
        if item.get("record_id") == result["record_id"]
    ]
    require(len(matches) == 1, "cache result has no unique ledger entry")
    authorization_matches = [
        item
        for item in ledger.get("entries", [])
        if item.get("authorization", {}).get("id") == authorization["authorization_id"]
    ]
    require(
        len(authorization_matches) == 1,
        "cache authorization is not unique in the ledger",
    )
    entry = matches[0]
    selection = proposal["selection"]
    observations = result["observations"]
    totals = {
        key: sum(int(item[key]) for item in observations)
        for key in (
            "provider_requests",
            "input_tokens",
            "cached_input_tokens",
            "uncached_input_tokens",
            "output_tokens",
        )
    }
    execution = entry.get("execution", {})
    require(
        entry.get("status") == "settled"
        and entry.get("started_at") == result["started_at"]
        and entry.get("ended_at") == result["ended_at"]
        and entry.get("elapsed_calendar_seconds") == result["elapsed_seconds"]
        and entry.get("authorization", {}).get("status") == "granted"
        and entry.get("authorization", {}).get("id")
        == authorization["authorization_id"]
        and entry.get("authorization", {}).get("reference")
        == authorization["approval_reference"],
        "cache ledger status or authorization mismatch",
    )
    require(
        exact_json_equal(
            entry["authorization"].get("budget_summary"), proposal["maximums"]
        ),
        "cache ledger budget summary mismatch",
    )
    require(
        execution.get("model") == selection["model"]
        and execution.get("sample_ids") == selection["samples"]
        and execution.get("arm_ids") == selection["arms"]
        and exact_int(execution.get("repeats_per_arm_per_sample"), selection["repeat"])
        and exact_int(
            execution.get("planned_sample_runs"), selection["planned_sample_runs"]
        )
        and exact_int(execution.get("actual_sample_runs"), result["actual_sample_runs"])
        and exact_int(execution.get("api_requests"), totals["provider_requests"])
        and exact_int(
            execution.get("api_requests_minimum"), totals["provider_requests"]
        )
        and execution.get("api_requests_evidence_status") == "complete",
        "cache ledger execution mismatch",
    )
    tokens = entry.get("tokens", {})
    require(
        exact_int(tokens.get("input"), totals["input_tokens"])
        and exact_int(tokens.get("cached_input"), totals["cached_input_tokens"])
        and exact_int(tokens.get("uncached_input"), totals["uncached_input_tokens"])
        and exact_int(tokens.get("output"), totals["output_tokens"]),
        "cache ledger token totals mismatch",
    )
    evidence = entry.get("evidence", {})
    require(
        evidence.get("result_path") == acceptance["result_path"]
        and evidence.get("actual_run_root") == result["run_root"]
        and exact_int(evidence.get("runner_exit_code"), 0)
        and evidence.get("outcome") == "completed"
        and evidence.get("usage_evidence_status") == "complete"
        and evidence.get("proposal_path") == proposal_path
        and evidence.get("proposal_sha256")
        == source_sha256(repo, proposal_path, source)
        and evidence.get("authorization_path") == authorization_path
        and evidence.get("authorization_sha256")
        == source_sha256(repo, authorization_path, source),
        "cache ledger evidence mismatch",
    )
    expected_cost = settled_monetary_cost(
        tokens, proposal["pricing_snapshot"], evidence_status="complete"
    )
    require(
        entry.get("monetary_cost") == expected_cost,
        "cache ledger cost settlement mismatch",
    )
    return path
