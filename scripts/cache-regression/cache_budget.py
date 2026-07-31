#!/usr/bin/env python3
"""Pure budget-plan construction for authorized cache regression smoke runs."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from cache_evidence import canonical_json_sha256, file_sha256
from cache_surface import surface_snapshot


BUDGET_PROPOSAL_SCHEMA_VERSION = "whalecode-cache-budget-proposal-v2"
RUNNER_CLEANUP_GRACE_SECONDS = 120
SUPPORTED_ARMS = ("standard", "map-always", "map-append", "map-request")
SUPPORTED_STOP_CONDITIONS = (
    "after_any_run_failure",
    "after_any_business_failure",
    "after_any_usage_gap",
    "after_any_budget_observation_exceeded",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def repository_evidence_path(repo: Path, path: Path) -> Path:
    resolved = (path if path.is_absolute() else repo / path).resolve()
    try:
        resolved.relative_to(repo)
    except ValueError as error:
        raise ValueError("budget evidence must be inside the repository") from error
    require(resolved.is_file(), "budget evidence file does not exist")
    return resolved


def validate_gate_trigger(report: dict[str, Any]) -> list[str]:
    require(
        report.get("schema_version") == "whalecode-cache-regression-gate-v1",
        "budget proposal requires a cache gate report",
    )
    require(report.get("status") == "blocked", "cache gate report is not blocked")
    state = report.get("discovery_state")
    require(
        state in {"changed", "revalidation_requested"},
        "cache gate report has no comparable change or explicit revalidation",
    )
    validation = report.get("free_validation")
    require(isinstance(validation, dict), "cache gate report has no free validation")
    commands = validation.get("commands")
    require(isinstance(commands, list), "free validation commands are missing")
    if state == "revalidation_requested":
        require(
            validation.get("passed") is True
            and report.get("baseline_status") == "live_regression_failed"
            and report.get("revalidation_requested") is True
            and report.get("require_live_baseline") is True
            and report.get("require_clean_subject") is True
            and report.get("sensitive_changes") == [],
            "cache revalidation request is not a clean failed-baseline transition",
        )
        return []
    require(
        validation.get("passed") is False, "free validation did not detect a change"
    )
    failed = [
        command.get("id") for command in commands if command.get("status") == "fail"
    ]
    require(
        failed and all(isinstance(item, str) for item in failed),
        "failed command identity is missing",
    )
    return failed


def _unique_nonempty(values: list[str], label: str) -> list[str]:
    require(
        values and all(isinstance(value, str) and value.strip() for value in values),
        f"{label} must be non-empty strings",
    )
    normalized = [value.strip() for value in values]
    require(
        len(set(normalized)) == len(normalized), f"{label} must not contain duplicates"
    )
    return normalized


def build_budget_proposal(
    *,
    repo: Path,
    contract: dict[str, Any],
    gate_report_path: Path,
    gate_report: dict[str, Any],
    subject_commit: str,
    model: str,
    samples: list[str],
    arms: list[str],
    repeat: int,
    retry_sample_run_limit: int,
    max_provider_requests_per_run: int,
    observed_input_tokens_per_run: int,
    observed_output_tokens_per_run: int,
    max_seconds_per_run: int,
    stop_conditions: list[str],
    selection_reason: str,
) -> dict[str, Any]:
    failed_commands = validate_gate_trigger(gate_report)
    samples = _unique_nonempty(samples, "samples")
    arms = _unique_nonempty(arms, "arms")
    require(
        all(arm in SUPPORTED_ARMS for arm in arms),
        "budget proposal contains an unsupported arm",
    )
    stop_conditions = _unique_nonempty(stop_conditions, "stop conditions")
    require(
        all(condition in SUPPORTED_STOP_CONDITIONS for condition in stop_conditions),
        "budget proposal contains an unsupported stop condition",
    )
    require(model.strip() != "", "model must not be empty")
    require(selection_reason.strip() != "", "selection reason must not be empty")
    for value, label in (
        (repeat, "repeat"),
        (max_provider_requests_per_run, "provider request limit"),
        (observed_input_tokens_per_run, "input token observation threshold"),
        (observed_output_tokens_per_run, "output token observation threshold"),
        (max_seconds_per_run, "time limit"),
    ):
        require(isinstance(value, int) and value > 0, f"{label} must be positive")
    require(
        isinstance(retry_sample_run_limit, int) and retry_sample_run_limit >= 0,
        "retry sample-run limit must be non-negative",
    )

    planned_sample_runs = len(samples) * len(arms) * repeat
    maximum_sample_runs = planned_sample_runs + retry_sample_run_limit
    provider_limits = contract.get("provider_hard_limits", {}).get(model)
    require(isinstance(provider_limits, dict), "model has no provider hard limits")
    require(
        all(
            isinstance(provider_limits.get(key), int) and provider_limits[key] > 0
            for key in (
                "max_input_tokens_per_request",
                "max_output_tokens_per_request",
            )
        ),
        "model provider hard limits are incomplete",
    )
    pricing = contract.get("pricing_snapshot", {})
    require(
        isinstance(pricing.get("currency"), str)
        and pricing["currency"].strip() != ""
        and all(
            isinstance(pricing.get(key), (int, float)) and pricing[key] >= 0
            for key in ("uncached_input_per_million", "output_per_million")
        ),
        "pricing snapshot is incomplete",
    )
    hard_request_maximum = maximum_sample_runs * max_provider_requests_per_run
    maximum_input_tokens = (
        hard_request_maximum * provider_limits["max_input_tokens_per_request"]
    )
    maximum_output_tokens = (
        hard_request_maximum * provider_limits["max_output_tokens_per_request"]
    )
    maximum_cost = (
        maximum_input_tokens / 1_000_000 * pricing["uncached_input_per_million"]
        + maximum_output_tokens / 1_000_000 * pricing["output_per_million"]
    )
    observation_threshold_input = maximum_sample_runs * observed_input_tokens_per_run
    observation_threshold_output = maximum_sample_runs * observed_output_tokens_per_run
    surface_sha, _ = surface_snapshot(repo, contract, "worktree")
    require(
        gate_report.get("subject_commit") == subject_commit
        and gate_report.get("actual_surface_sha256") == surface_sha,
        "cache gate report does not match the proposed HEAD and surface",
    )
    relative_report = gate_report_path.relative_to(repo).as_posix()
    proposal = {
        "schema_version": BUDGET_PROPOSAL_SCHEMA_VERSION,
        "subject_commit": subject_commit,
        "surface_sha256": surface_sha,
        "trigger": {
            "gate_report_path": relative_report,
            "gate_report_sha256": file_sha256(gate_report_path),
            "failed_free_commands": failed_commands,
        },
        "selection": {
            "model": model.strip(),
            "samples": samples,
            "arms": arms,
            "repeat": repeat,
            "planned_sample_runs": planned_sample_runs,
            "retry_sample_run_limit": retry_sample_run_limit,
            "maximum_sample_runs": maximum_sample_runs,
            "stop_conditions": stop_conditions,
            "selection_reason": selection_reason.strip(),
        },
        "per_sample_run_limits": {
            "provider_requests": max_provider_requests_per_run,
            "elapsed_seconds": max_seconds_per_run,
            "cleanup_grace_seconds": RUNNER_CLEANUP_GRACE_SECONDS,
        },
        "per_sample_run_observation_thresholds": {
            "input_tokens": observed_input_tokens_per_run,
            "output_tokens": observed_output_tokens_per_run,
        },
        "maximums": {
            "provider_requests": hard_request_maximum,
            "input_tokens": maximum_input_tokens,
            "output_tokens": maximum_output_tokens,
            "elapsed_seconds": maximum_sample_runs
            * (max_seconds_per_run + RUNNER_CLEANUP_GRACE_SECONDS),
            "estimated_cost": round(maximum_cost, 10),
            "currency": pricing["currency"],
        },
        "observation_threshold_totals": {
            "input_tokens": observation_threshold_input,
            "output_tokens": observation_threshold_output,
            "estimated_cost": round(
                observation_threshold_input
                / 1_000_000
                * pricing["uncached_input_per_million"]
                + observation_threshold_output
                / 1_000_000
                * pricing["output_per_million"],
                10,
            ),
            "currency": pricing["currency"],
        },
        "pricing_snapshot": pricing,
        "provider_hard_limits": provider_limits,
        "cost_assumption": (
            "hard request count multiplied by provider maximum input/output tokens; "
            "all input priced as cache miss"
        ),
        "enforcement_boundary": {
            "hard_before_start": [
                "subject_commit",
                "surface_sha256",
                "proposal_and_authorization_identity",
                "sample_arm_repeat_matrix",
                "no_automatic_retries",
            ],
            "hard_during_run": ["provider_requests", "elapsed_seconds"],
            "bounded_cleanup_after_run": ["cleanup_grace_seconds"],
            "hard_by_provider_per_request": ["input_tokens", "output_tokens"],
            "observed_after_each_run": [
                "input_tokens",
                "output_tokens",
                "estimated_cost",
            ],
        },
        "evidence_boundary": "only the explicitly listed model, samples, arms, repeats, and limits",
    }
    proposal["proposal_id"] = f"CBP-{canonical_json_sha256(proposal)[:16].upper()}"
    proposal["proposal_sha256"] = canonical_json_sha256(proposal)
    return proposal


def validate_budget_proposal(proposal: dict[str, Any]) -> None:
    require(
        proposal.get("schema_version") == BUDGET_PROPOSAL_SCHEMA_VERSION,
        "invalid budget proposal schema",
    )
    expected = dict(proposal)
    digest = expected.pop("proposal_sha256", None)
    require(
        digest == canonical_json_sha256(expected), "budget proposal digest mismatch"
    )
    require(
        expected.get("proposal_id")
        == f"CBP-{canonical_json_sha256({key: value for key, value in expected.items() if key != 'proposal_id'})[:16].upper()}",
        "budget proposal id mismatch",
    )
    selection = proposal.get("selection", {})
    limits = proposal.get("per_sample_run_limits", {})
    thresholds = proposal.get("per_sample_run_observation_thresholds", {})
    provider_limits = proposal.get("provider_hard_limits", {})
    pricing = proposal.get("pricing_snapshot", {})
    sample_runs = selection.get("maximum_sample_runs")
    requests_per_run = limits.get("provider_requests")
    require(
        all(
            isinstance(value, int) and value > 0
            for value in (
                sample_runs,
                requests_per_run,
                limits.get("elapsed_seconds"),
                limits.get("cleanup_grace_seconds"),
                thresholds.get("input_tokens"),
                thresholds.get("output_tokens"),
                provider_limits.get("max_input_tokens_per_request"),
                provider_limits.get("max_output_tokens_per_request"),
            )
        ),
        "budget proposal limits are incomplete",
    )
    require(
        isinstance(pricing.get("currency"), str)
        and pricing["currency"].strip() != ""
        and all(
            isinstance(pricing.get(key), (int, float)) and pricing[key] >= 0
            for key in ("uncached_input_per_million", "output_per_million")
        ),
        "budget proposal pricing is incomplete",
    )
    hard_requests = sample_runs * requests_per_run
    hard_input = hard_requests * provider_limits["max_input_tokens_per_request"]
    hard_output = hard_requests * provider_limits["max_output_tokens_per_request"]
    hard_cost = round(
        hard_input / 1_000_000 * pricing["uncached_input_per_million"]
        + hard_output / 1_000_000 * pricing["output_per_million"],
        10,
    )
    require(
        proposal.get("maximums")
        == {
            "provider_requests": hard_requests,
            "input_tokens": hard_input,
            "output_tokens": hard_output,
            "elapsed_seconds": sample_runs
            * (limits["elapsed_seconds"] + limits["cleanup_grace_seconds"]),
            "estimated_cost": hard_cost,
            "currency": pricing["currency"],
        },
        "budget proposal hard maximums are inconsistent",
    )
    observed_input = sample_runs * thresholds["input_tokens"]
    observed_output = sample_runs * thresholds["output_tokens"]
    require(
        proposal.get("observation_threshold_totals")
        == {
            "input_tokens": observed_input,
            "output_tokens": observed_output,
            "estimated_cost": round(
                observed_input / 1_000_000 * pricing["uncached_input_per_million"]
                + observed_output / 1_000_000 * pricing["output_per_million"],
                10,
            ),
            "currency": pricing["currency"],
        },
        "budget proposal observation thresholds are inconsistent",
    )
    boundary = proposal.get("enforcement_boundary", {})
    require(
        boundary.get("hard_during_run") == ["provider_requests", "elapsed_seconds"]
        and boundary.get("hard_by_provider_per_request")
        == ["input_tokens", "output_tokens"]
        and boundary.get("bounded_cleanup_after_run") == ["cleanup_grace_seconds"],
        "budget proposal enforcement boundary is incomplete",
    )
