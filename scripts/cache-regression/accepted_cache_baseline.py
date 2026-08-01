#!/usr/bin/env python3
"""Source-aware validation for cache smoke evidence and accepted baselines."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from cache_acceptance_identity import validate_proposal_identity
from cache_budget import validate_gate_trigger
from cache_cleanup_contract import cleanup_verified
from cache_elapsed import is_elapsed_number, validate_elapsed_evidence
from cache_evidence import RESULT_SCHEMA_VERSION, canonical_json_sha256
from cache_arm_identity import validate_arm_identity
from cache_gate_evidence import changed_scenarios
from cache_json import exact_json_equal
from cache_provider_promotion import validate_provider_route_evidence
from cache_run_analysis import (
    CACHE_OBSERVATION_KEYS,
    analyze_artifact_values,
    budget_observation_exceeded,
)
from accepted_cache_ledger import exact_int, validate_ledger
from cache_run_contract import (
    execution_matrix,
    validate_authorization as validate_run_authorization,
)
from cache_result_envelope import validate_result_envelope
from cache_request_accounting import validate_result_request_accounting
from cache_result_integrity import validate_completed_result_integrity
from cache_source_evidence import (
    protected_manifest,
    relative_path,
    require,
    source_json,
    source_sha256,
)
from cache_surface import surface_snapshot
from cache_time import parse_timestamp, require_not_future, require_ordered

ACCEPTANCE_SCHEMA_VERSION = "whalecode-cache-baseline-acceptance-v1"


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
            "execution_argv",
            "logical_mode_map",
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
    validate_arm_identity(
        source_json(repo, artifacts["execution_argv"], source),
        source_json(repo, artifacts["logical_mode_map"], source),
        observation["arm"],
    )
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
        all(
            type(observation.get(key)) is type(recomputed[key])
            and observation.get(key) == recomputed[key]
            for key in CACHE_OBSERVATION_KEYS
        ),
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
    result: dict[str, Any],
    expected_matrix: list[dict[str, Any]],
    limits: dict[str, Any],
) -> None:
    attempts = result.get("attempts")
    require(isinstance(attempts, list), "cache result attempts are missing")
    actual = [
        {key: item[key] for key in ("sample", "arm", "repeat")} for item in attempts
    ]
    require(
        exact_json_equal(actual, expected_matrix),
        "cache result attempt matrix mismatch",
    )
    require(
        all(
            item.get("status") == "completed"
            and exact_int(item.get("exit_code"), 0)
            and item.get("timed_out") is False
            and isinstance(item.get("run_id"), str)
            and item["run_id"].strip()
            and is_elapsed_number(item.get("elapsed_seconds"))
            and item["elapsed_seconds"] >= 0
            and item["elapsed_seconds"] <= limits["elapsed_seconds"]
            and cleanup_verified(item.get("post_run_cleanup", {}))
            and "execution_error" not in item
            and "evidence_error" not in item
            for item in attempts
        ),
        "cache result contains a failed attempt",
    )


def validate_cross_arm_provider_evidence(observations: list[dict[str, Any]]) -> None:
    for index, left in enumerate(observations):
        for right in observations[index + 1 :]:
            if left["arm"] == right["arm"]:
                continue
            require(
                left["provider_payload_sha256"] != right["provider_payload_sha256"],
                "different cache arms share identical provider wire evidence",
            )


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
    route, route_path, route_evidence_paths = validate_provider_route_evidence(
        repo, result, source
    )
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
    preflight_completed_at = parse_timestamp(
        route["preflight_completed_at"], "provider route preflight completion"
    )
    require_ordered(
        preflight_completed_at,
        started_at,
        "provider route preflight completion",
        "run start",
    )
    require_ordered(started_at, ended_at, "run start", "run end")
    require_ordered(ended_at, accepted_at, "run end", "acceptance")
    require_not_future(accepted_at, "cache acceptance timestamp")
    proposal_path = relative_path(repo, acceptance.get("proposal_path"))
    authorization_path = relative_path(repo, acceptance.get("authorization_path"))
    proposal = source_json(repo, proposal_path, source)
    authorization = source_json(repo, authorization_path, source)
    validate_proposal_identity(repo, proposal, result, require_current_head)
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
        exact_json_equal(observed_matrix, matrix)
        and type(result.get("actual_sample_runs")) is int
        and result["actual_sample_runs"] == len(matrix),
        "cache observation matrix mismatch",
    )
    validate_attempts(result, matrix, proposal["per_sample_run_limits"])
    validate_result_request_accounting(result)
    validate_completed_result_integrity(result, matrix)
    validate_elapsed_evidence(
        result,
        result["attempts"],
        started_at,
        ended_at,
        proposal["maximums"]["elapsed_seconds"],
    )
    require(
        all(
            observation.get("run_id") == attempt.get("run_id")
            and observation.get("elapsed_seconds") == attempt.get("elapsed_seconds")
            and attempt.get("provider_boundary_request_count")
            == observation.get("provider_requests")
            and attempt.get("provider_boundary_evidence_path")
            == observation.get("artifacts", {}).get("provider_boundary")
            and attempt.get("provider_boundary_evidence_sha256")
            == observation.get("artifact_sha256", {}).get("provider_boundary")
            for observation, attempt in zip(observations, result["attempts"])
        ),
        "cache observation or provider accounting does not match its attempt",
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
    validate_cross_arm_provider_evidence(observations)
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
        exact_json_equal(acceptance.get("accepted_scope"), result["observed_scope"])
        and acceptance.get("acknowledged_unverified_scope") == []
        and exact_json_equal(acceptance.get("accepted_scenarios"), expected_scenarios),
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
                    route_path,
                    *route_evidence_paths,
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
