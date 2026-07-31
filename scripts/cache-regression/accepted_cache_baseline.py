#!/usr/bin/env python3
"""Source-aware validation for cache smoke evidence and accepted baselines."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Any

from cache_budget import validate_budget_proposal, validate_gate_trigger
from cache_evidence import RESULT_SCHEMA_VERSION, canonical_json_sha256
from cache_run_analysis import analyze_artifact_values, budget_observation_exceeded
from cache_run_contract import (
    execution_matrix,
    validate_authorization as validate_run_authorization,
)
from cache_result_envelope import validate_result_envelope
from cache_source_evidence import (
    protected_manifest,
    relative_path,
    source_json,
    source_sha256,
)
from cache_surface import surface_snapshot
from cache_time import parse_timestamp, require_not_future, require_ordered

ACCEPTANCE_SCHEMA_VERSION = "whalecode-cache-baseline-acceptance-v1"
OBSERVATION_KEYS = (
    "arm",
    "provider_usage_contract_version",
    "logical_mode",
    "provider_model",
    "provider_requests",
    "request_2_plus_count",
    "request_2_plus_hit_rate",
    "request_2_plus_cached_input_tokens",
    "request_2_plus_uncached_input_tokens",
    "trace_coverage",
    "cache_usage_missing_count",
    "input_tokens",
    "cached_input_tokens",
    "uncached_input_tokens",
    "output_tokens",
    "business_success",
    "artifacts",
    "artifact_sha256",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def changed_scenarios(report: dict[str, Any]) -> list[dict[str, Any]]:
    scenarios = []
    for command in report["free_validation"]["commands"]:
        change_report = command.get("change_report")
        if change_report:
            scenarios.extend(
                item
                for item in change_report["scenarios"]
                if item["status"] == "changed"
            )
    scenarios.sort(key=lambda item: item["scenario_id"])
    ids = [item["scenario_id"] for item in scenarios]
    require(len(ids) == len(set(ids)), "changed scenarios are invalid")
    if report.get("discovery_state") != "revalidation_requested":
        require(scenarios, "changed scenarios are invalid")
    return scenarios


def validate_proposal(
    repo: Path,
    proposal: dict[str, Any],
    result: dict[str, Any],
    require_current_head: bool,
) -> None:
    validate_budget_proposal(proposal)
    require(
        proposal.get("proposal_id") == result.get("proposal_id")
        and proposal.get("proposal_sha256") == result.get("proposal_sha256"),
        "cache proposal identity mismatch",
    )
    require(
        proposal.get("subject_commit") == result.get("subject_commit")
        and proposal.get("surface_sha256") == result.get("surface_sha256")
        and proposal.get("selection") == result.get("observed_scope"),
        "cache proposal source or scope mismatch",
    )
    if require_current_head:
        head = subprocess.check_output(
            ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True
        ).strip()
        require(
            proposal["subject_commit"] == head, "cache proposal is not current HEAD"
        )


def validate_observation(
    repo: Path,
    result: dict[str, Any],
    observation: dict[str, Any],
    limits: dict[str, Any],
    thresholds: dict[str, Any],
    source: str,
) -> list[str]:
    prefix = (
        f"benchmarks/cache-regression/evidence/{result['record_id']}/"
        f"{observation['run_id']}/"
    )
    artifacts = {
        key: relative_path(repo, observation["artifacts"][key])
        for key in (
            "cache_summary",
            "request_summary",
            "metrics",
            "provider_boundary",
        )
    }
    require(
        all(path.startswith(prefix) for path in artifacts.values()),
        "cache observation evidence is not durable",
    )
    hashes = observation["artifact_sha256"]
    require(
        all(
            source_sha256(repo, path, source) == hashes[key]
            for key, path in artifacts.items()
        ),
        "cache observation evidence digest mismatch",
    )
    cache = source_json(repo, artifacts["cache_summary"], source)
    request = source_json(repo, artifacts["request_summary"], source)["rollout_trace"]
    metrics = source_json(repo, artifacts["metrics"], source)
    boundary = source_json(repo, artifacts["provider_boundary"], source)
    recomputed = analyze_artifact_values(
        cache,
        request,
        metrics,
        boundary,
        observation["arm"],
        result["observed_scope"]["model"],
        artifacts,
        hashes,
    )
    require(
        all(observation.get(key) == recomputed[key] for key in OBSERVATION_KEYS),
        "cache observation metrics mismatch",
    )
    require(
        observation["business_success"]
        and observation["cache_usage_missing_count"] == 0
        and observation["provider_requests"] >= 2
        and observation["request_2_plus_count"] >= 1,
        "cache observation is not promotable",
    )
    exceeded = budget_observation_exceeded(observation, limits, thresholds)
    require(
        observation.get("budget_observation_exceeded") == exceeded == [],
        "cache observation exceeded its approved budget",
    )
    return list(artifacts.values())


def validate_attempts(
    result: dict[str, Any], expected_matrix: list[dict[str, Any]]
) -> None:
    attempts = result.get("attempts")
    require(isinstance(attempts, list), "cache result attempts are missing")
    actual = [
        {key: item[key] for key in ("sample", "arm", "repeat")} for item in attempts
    ]
    require(actual == expected_matrix, "cache result attempt matrix mismatch")
    require(
        all(
            item.get("status") == "completed"
            and item.get("exit_code") == 0
            and item.get("timed_out") is False
            and isinstance(item.get("run_id"), str)
            and item["run_id"].strip()
            and isinstance(item.get("elapsed_seconds"), (int, float))
            and item["elapsed_seconds"] >= 0
            and item.get("post_run_cleanup", {}).get("status")
            in {"verified_absent", "removed_verified"}
            and item.get("post_run_cleanup", {}).get("stable_empty_polls", 0) >= 3
            and "execution_error" not in item
            and "evidence_error" not in item
            for item in attempts
        ),
        "cache result contains a failed attempt",
    )


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
        entry["authorization"].get("budget_summary") == proposal["maximums"],
        "cache ledger budget summary mismatch",
    )
    require(
        execution.get("model") == selection["model"]
        and execution.get("sample_ids") == selection["samples"]
        and execution.get("arm_ids") == selection["arms"]
        and execution.get("repeats_per_arm_per_sample") == selection["repeat"]
        and execution.get("planned_sample_runs") == selection["planned_sample_runs"]
        and execution.get("actual_sample_runs") == result["actual_sample_runs"]
        and execution.get("api_requests") == totals["provider_requests"],
        "cache ledger execution mismatch",
    )
    tokens = entry.get("tokens", {})
    require(
        tokens.get("input") == totals["input_tokens"]
        and tokens.get("cached_input") == totals["cached_input_tokens"]
        and tokens.get("uncached_input") == totals["uncached_input_tokens"]
        and tokens.get("output") == totals["output_tokens"],
        "cache ledger token totals mismatch",
    )
    evidence = entry.get("evidence", {})
    require(
        evidence.get("result_path") == acceptance["result_path"]
        and evidence.get("actual_run_root") == result["run_root"]
        and evidence.get("runner_exit_code") == 0
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
    require(
        entry.get("monetary_cost", {}).get("status") == "estimated",
        "cache ledger cost settlement is incomplete",
    )
    return path


def validate_run_evidence(
    repo: Path,
    contract: dict[str, Any],
    source: str,
    result_path: str,
    acceptance_path: str,
    *,
    require_current_head: bool,
) -> dict[str, Any]:
    result_path = relative_path(repo, result_path)
    acceptance_path = relative_path(repo, acceptance_path)
    result = source_json(repo, result_path, source)
    acceptance = source_json(repo, acceptance_path, source)
    require(
        result.get("schema_version") == RESULT_SCHEMA_VERSION
        and result.get("status") == "completed"
        and result.get("unverified_scope") == [],
        "cache result is incomplete or has unverified scope",
    )
    validate_result_envelope(result, result_path)
    require(
        acceptance.get("schema_version") == ACCEPTANCE_SCHEMA_VERSION
        and acceptance.get("status") == "accepted"
        and acceptance.get("accepted_by") == "user"
        and isinstance(acceptance.get("accepted_at"), str)
        and acceptance["accepted_at"].strip()
        and isinstance(acceptance.get("acceptance_reference"), str)
        and acceptance["acceptance_reference"].strip()
        and acceptance.get("result_path") == result_path
        and acceptance.get("result_sha256") == source_sha256(repo, result_path, source),
        "cache acceptance does not match result",
    )
    started_at = parse_timestamp(result.get("started_at"), "cache result started_at")
    ended_at = parse_timestamp(result.get("ended_at"), "cache result ended_at")
    accepted_at = parse_timestamp(
        acceptance.get("accepted_at"), "cache acceptance timestamp"
    )
    require_ordered(started_at, ended_at, "run start", "run end")
    require_ordered(ended_at, accepted_at, "run end", "acceptance")
    require_not_future(accepted_at, "cache acceptance timestamp")
    proposal_path = relative_path(repo, acceptance.get("proposal_path"))
    authorization_path = relative_path(repo, acceptance.get("authorization_path"))
    proposal = source_json(repo, proposal_path, source)
    authorization = source_json(repo, authorization_path, source)
    validate_proposal(repo, proposal, result, require_current_head)
    if require_current_head:
        actual_surface, _ = surface_snapshot(repo, contract, source)
        require(
            result["surface_sha256"] == actual_surface, "cache result surface is stale"
        )
    require(
        result.get("authorization_sha256")
        == source_sha256(repo, authorization_path, source),
        "cache result authorization digest mismatch",
    )
    validate_run_authorization(proposal, authorization)
    approved_at = parse_timestamp(
        authorization.get("approved_at"), "cache budget approval timestamp"
    )
    require_ordered(approved_at, started_at, "authorization", "run start")
    require(
        authorization["approval_reference"] == result.get("authorization_reference"),
        "cache authorization reference mismatch",
    )
    matrix = execution_matrix(proposal)
    observations = result.get("observations")
    require(isinstance(observations, list), "cache observations are missing")
    observed_matrix = [
        {key: item[key] for key in ("sample", "arm", "repeat")} for item in observations
    ]
    require(
        observed_matrix == matrix and result.get("actual_sample_runs") == len(matrix),
        "cache observation matrix mismatch",
    )
    validate_attempts(result, matrix)
    require(
        all(
            observation.get("run_id") == attempt.get("run_id")
            and observation.get("elapsed_seconds") == attempt.get("elapsed_seconds")
            for observation, attempt in zip(observations, result["attempts"])
        ),
        "cache observation does not match its attempt",
    )
    evidence_paths = []
    for observation in observations:
        evidence_paths.extend(
            validate_observation(
                repo,
                result,
                observation,
                proposal["per_sample_run_limits"],
                proposal["per_sample_run_observation_thresholds"],
                source,
            )
        )
    digest_rows = [
        {**scope, "artifact_sha256": observation["artifact_sha256"]}
        for scope, observation in zip(matrix, observations)
    ]
    require(
        result.get("evidence_sha256") == canonical_json_sha256(digest_rows),
        "cache result evidence digest mismatch",
    )
    gate_path = relative_path(repo, proposal["trigger"]["gate_report_path"])
    require(
        source_sha256(repo, gate_path, source)
        == proposal["trigger"]["gate_report_sha256"],
        "cache gate report digest mismatch",
    )
    gate_report = source_json(repo, gate_path, source)
    failed_commands = validate_gate_trigger(gate_report)
    require(
        gate_report.get("subject_commit") == proposal["subject_commit"]
        and gate_report.get("actual_surface_sha256") == proposal["surface_sha256"]
        and proposal["trigger"].get("failed_free_commands") == failed_commands,
        "cache gate report source or failure set mismatch",
    )
    scenarios = changed_scenarios(gate_report)
    expected_scenarios = [
        {
            "scenario_id": item["scenario_id"],
            "after_payload_sha256": item["after_payload_sha256"],
        }
        for item in scenarios
    ]
    require(
        acceptance.get("accepted_scope") == result["observed_scope"]
        and acceptance.get("acknowledged_unverified_scope") == []
        and acceptance.get("accepted_scenarios") == expected_scenarios,
        "cache acceptance scope or scenarios mismatch",
    )
    ledger_path = validate_ledger(
        repo,
        result,
        acceptance,
        proposal,
        authorization,
        proposal_path,
        authorization_path,
        source,
    )
    return {
        "result": result,
        "acceptance": acceptance,
        "proposal": proposal,
        "authorization": authorization,
        "scenarios": scenarios,
        "evidence_paths": sorted(
            set(
                [
                    result_path,
                    acceptance_path,
                    proposal_path,
                    authorization_path,
                    gate_path,
                    ledger_path,
                    *evidence_paths,
                ]
            )
        ),
    }


def validate_accepted_baseline(
    repo: Path,
    contract: dict[str, Any],
    source: str,
    actual_surface_sha256: str,
) -> dict[str, Any]:
    baseline = contract.get("baseline", {})
    require(baseline.get("status") == "accepted", "baseline status is not accepted")
    smoke = baseline.get("smoke_evidence", {})
    acceptance_evidence = baseline.get("acceptance_evidence", {})
    validated = validate_run_evidence(
        repo,
        contract,
        source,
        smoke.get("result_path"),
        acceptance_evidence.get("path"),
        require_current_head=False,
    )
    require(
        source_sha256(repo, smoke["result_path"], source) == smoke.get("result_sha256")
        and source_sha256(repo, acceptance_evidence["path"], source)
        == acceptance_evidence.get("sha256"),
        "accepted baseline evidence digest mismatch",
    )
    result = validated["result"]
    acceptance = validated["acceptance"]
    require(
        result["subject_commit"] == baseline.get("source_commit")
        and result["surface_sha256"] == baseline.get("surface_sha256")
        and acceptance.get("acceptance_reference")
        == baseline.get("acceptance_reference"),
        "accepted baseline identity mismatch",
    )
    manifest = protected_manifest(repo, contract, source)
    require(
        baseline.get("final_wire_manifest") == manifest
        and baseline.get("final_wire_manifest_sha256")
        == canonical_json_sha256(manifest),
        "accepted final-wire manifest mismatch",
    )
    manifest_by_id = {item["scenario_id"]: item for item in manifest}
    accepted_paths = []
    for item in acceptance["accepted_scenarios"]:
        scenario = manifest_by_id.get(item["scenario_id"])
        require(
            scenario and scenario["payload_sha256"] == item["after_payload_sha256"],
            "accepted scenario does not match final-wire manifest",
        )
        accepted_paths.append(scenario["baseline_path"])
    require(
        len(accepted_paths) == len(set(accepted_paths)),
        "accepted scenario set contains duplicates",
    )
    return {
        "valid": True,
        "surface_matches_current": baseline.get("surface_sha256")
        == actual_surface_sha256,
        "accepted_scenario_paths": sorted(accepted_paths),
        "evidence_paths": validated["evidence_paths"],
    }
