#!/usr/bin/env python3
"""Pure budget-plan construction for authorized cache regression smoke runs."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from cache_evidence import canonical_json_sha256, file_sha256
from cache_execution_identity import build_execution_identity
from cache_json import exact_json_equal
from cache_surface import surface_snapshot
from cache_time import now_iso, parse_timestamp, require_not_future


BUDGET_PROPOSAL_SCHEMA_VERSION = "whalecode-cache-budget-proposal-v3"
LEGACY_BUDGET_PROPOSAL_SCHEMA_VERSION = "whalecode-cache-budget-proposal-v2"
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
        accepted_validation = report.get("accepted_baseline_validation", {})
        stale_accepted = bool(
            report.get("baseline_status") == "accepted"
            and accepted_validation.get("valid")
            and not accepted_validation.get("manifest_matches_current", True)
        )
        require(
            validation.get("passed") is True
            and (
                report.get("baseline_status") == "live_regression_failed"
                or stale_accepted
            )
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


def validate_budget_selection(selection: object) -> None:
    require(isinstance(selection, dict), "budget proposal selection is invalid")
    model = selection.get("model")
    reason = selection.get("selection_reason")
    samples = selection.get("samples")
    arms = selection.get("arms")
    stop_conditions = selection.get("stop_conditions")
    require(
        isinstance(model, str) and model.strip() == model and bool(model),
        "budget proposal model is invalid",
    )
    require(
        isinstance(reason, str) and reason.strip() == reason and bool(reason),
        "budget proposal selection reason is invalid",
    )
    require(
        isinstance(samples, list)
        and exact_json_equal(samples, _unique_nonempty(samples, "samples")),
        "budget proposal samples are invalid",
    )
    require(
        isinstance(arms, list)
        and exact_json_equal(arms, _unique_nonempty(arms, "arms"))
        and all(arm in SUPPORTED_ARMS for arm in arms),
        "budget proposal contains an unsupported arm",
    )
    require(
        isinstance(stop_conditions, list)
        and exact_json_equal(
            stop_conditions,
            _unique_nonempty(stop_conditions, "stop conditions"),
        )
        and all(
            condition in SUPPORTED_STOP_CONDITIONS for condition in stop_conditions
        ),
        "budget proposal contains an unsupported stop condition",
    )
    repeat = selection.get("repeat")
    planned_runs = selection.get("planned_sample_runs")
    retries = selection.get("retry_sample_run_limit")
    maximum_runs = selection.get("maximum_sample_runs")
    require(
        type(repeat) is int
        and repeat > 0
        and type(planned_runs) is int
        and planned_runs > 0
        and type(retries) is int
        and retries >= 0
        and type(maximum_runs) is int
        and maximum_runs > 0
        and planned_runs == len(samples) * len(arms) * repeat
        and maximum_runs == planned_runs + retries,
        "budget proposal execution matrix is inconsistent",
    )


def selection_matrix(
    selection: dict[str, Any], *, validate: bool = True
) -> list[dict[str, Any]]:
    if validate:
        validate_budget_selection(selection)
    return [
        {"sample": sample, "arm": arm, "repeat": repeat_index}
        for sample in selection["samples"]
        for arm in selection["arms"]
        for repeat_index in range(1, selection["repeat"] + 1)
    ]


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
    created_at: str | None = None,
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
        require(type(value) is int and value > 0, f"{label} must be positive")
    require(
        type(retry_sample_run_limit) is int and retry_sample_run_limit >= 0,
        "retry sample-run limit must be non-negative",
    )

    planned_sample_runs = len(samples) * len(arms) * repeat
    maximum_sample_runs = planned_sample_runs + retry_sample_run_limit
    provider_limits = contract.get("provider_hard_limits", {}).get(model)
    require(isinstance(provider_limits, dict), "model has no provider hard limits")
    require(
        all(
            type(provider_limits.get(key)) is int and provider_limits[key] > 0
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
            type(pricing.get(key)) in (int, float) and pricing[key] >= 0
            for key in (
                "cached_input_per_million",
                "uncached_input_per_million",
                "output_per_million",
            )
        ),
        "pricing snapshot is incomplete",
    )
    hard_request_maximum = maximum_sample_runs * max_provider_requests_per_run
    capacity_input_tokens = (
        hard_request_maximum * provider_limits["max_input_tokens_per_request"]
    )
    capacity_output_tokens = (
        hard_request_maximum * provider_limits["max_output_tokens_per_request"]
    )
    capacity_cost = (
        capacity_input_tokens / 1_000_000 * pricing["uncached_input_per_million"]
        + capacity_output_tokens / 1_000_000 * pricing["output_per_million"]
    )
    approved_input = maximum_sample_runs * observed_input_tokens_per_run
    approved_output = maximum_sample_runs * observed_output_tokens_per_run
    approved_token_cost = (
        approved_input / 1_000_000 * pricing["uncached_input_per_million"]
        + approved_output / 1_000_000 * pricing["output_per_million"]
    )
    per_run_approved_cost = (
        observed_input_tokens_per_run
        / 1_000_000
        * pricing["uncached_input_per_million"]
        + observed_output_tokens_per_run
        / 1_000_000
        * pricing["output_per_million"]
    )
    surface_sha, _ = surface_snapshot(repo, contract, "worktree")
    require(
        gate_report.get("subject_commit") == subject_commit
        and gate_report.get("actual_surface_sha256") == surface_sha,
        "cache gate report does not match the proposed HEAD and surface",
    )
    relative_report = gate_report_path.relative_to(repo).as_posix()
    proposal = {
        "schema_version": BUDGET_PROPOSAL_SCHEMA_VERSION,
        "created_at": created_at or now_iso(),
        "subject_commit": subject_commit,
        "surface_sha256": surface_sha,
        "execution_identity": build_execution_identity(repo, samples),
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
        "per_sample_run_budget_limits": {
            "input_tokens": observed_input_tokens_per_run,
            "output_tokens": observed_output_tokens_per_run,
            "estimated_cost": round(per_run_approved_cost, 10),
            "currency": pricing["currency"],
        },
        "approved_maximums": {
            "provider_requests": hard_request_maximum,
            "input_tokens": approved_input,
            "output_tokens": approved_output,
            "elapsed_seconds": maximum_sample_runs
            * (max_seconds_per_run + RUNNER_CLEANUP_GRACE_SECONDS),
            "estimated_cost": round(approved_token_cost, 10),
            "currency": pricing["currency"],
        },
        "provider_capacity_ceiling": {
            "provider_requests": hard_request_maximum,
            "input_tokens": capacity_input_tokens,
            "output_tokens": capacity_output_tokens,
            "estimated_cost": round(capacity_cost, 10),
            "currency": pricing["currency"],
        },
        "pricing_snapshot": pricing,
        "provider_hard_limits": provider_limits,
        "capacity_cost_assumption": (
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
            "settled_after_each_provider_response": [
                "input_tokens",
                "output_tokens",
                "estimated_cost",
            ],
            "reject_before_next_provider_request": [
                "usage_missing",
                "approved_budget_reached_or_exceeded",
            ],
            "bounded_cleanup_after_run": ["cleanup_grace_seconds"],
            "provider_per_request_capacity": ["input_tokens", "output_tokens"],
        },
        "evidence_boundary": "only the explicitly listed model, samples, arms, repeats, and limits",
    }
    proposal["proposal_id"] = f"CBP-{canonical_json_sha256(proposal)[:16].upper()}"
    proposal["proposal_sha256"] = canonical_json_sha256(proposal)
    return proposal


def validate_budget_proposal(proposal: dict[str, Any]) -> None:
    if proposal.get("schema_version") == LEGACY_BUDGET_PROPOSAL_SCHEMA_VERSION:
        _validate_legacy_budget_proposal(proposal)
        return
    require(
        proposal.get("schema_version") == BUDGET_PROPOSAL_SCHEMA_VERSION,
        "invalid budget proposal schema",
    )
    created_at = parse_timestamp(proposal.get("created_at"), "proposal created_at")
    require_not_future(created_at, "proposal created_at")
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
    validate_budget_selection(selection)
    limits = proposal.get("per_sample_run_limits", {})
    thresholds = proposal.get("per_sample_run_budget_limits", {})
    provider_limits = proposal.get("provider_hard_limits", {})
    pricing = proposal.get("pricing_snapshot", {})
    sample_runs = selection.get("maximum_sample_runs")
    requests_per_run = limits.get("provider_requests")
    require(
        all(
            type(value) is int and value > 0
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
            type(pricing.get(key)) in (int, float) and pricing[key] >= 0
            for key in (
                "cached_input_per_million",
                "uncached_input_per_million",
                "output_per_million",
            )
        ),
        "budget proposal pricing is incomplete",
    )
    hard_requests = sample_runs * requests_per_run
    capacity_input = hard_requests * provider_limits["max_input_tokens_per_request"]
    capacity_output = hard_requests * provider_limits["max_output_tokens_per_request"]
    capacity_cost = round(
        capacity_input / 1_000_000 * pricing["uncached_input_per_million"]
        + capacity_output / 1_000_000 * pricing["output_per_million"],
        10,
    )
    approved_input = sample_runs * thresholds["input_tokens"]
    approved_output = sample_runs * thresholds["output_tokens"]
    approved_cost = round(sample_runs * thresholds.get("estimated_cost"), 10)
    require(
        thresholds.get("currency") == pricing["currency"]
        and approved_cost
        >= round(
            approved_input / 1_000_000 * pricing["uncached_input_per_million"]
            + approved_output / 1_000_000 * pricing["output_per_million"],
            10,
        ),
        "budget cost limit is inconsistent",
    )
    require(
        exact_json_equal(
            proposal.get("approved_maximums"),
            {
                "provider_requests": hard_requests,
                "input_tokens": approved_input,
                "output_tokens": approved_output,
                "elapsed_seconds": sample_runs
                * (limits["elapsed_seconds"] + limits["cleanup_grace_seconds"]),
                "estimated_cost": approved_cost,
                "currency": pricing["currency"],
            },
        ),
        "budget proposal approved maximums are inconsistent",
    )
    require(
        exact_json_equal(
            proposal.get("provider_capacity_ceiling"),
            {
                "provider_requests": hard_requests,
                "input_tokens": capacity_input,
                "output_tokens": capacity_output,
                "estimated_cost": capacity_cost,
                "currency": pricing["currency"],
            },
        ),
        "budget proposal provider capacity ceiling is inconsistent",
    )
    boundary = proposal.get("enforcement_boundary", {})
    require(
        boundary.get("hard_during_run") == ["provider_requests", "elapsed_seconds"]
        and boundary.get("settled_after_each_provider_response")
        == ["input_tokens", "output_tokens", "estimated_cost"]
        and boundary.get("reject_before_next_provider_request")
        == ["usage_missing", "approved_budget_reached_or_exceeded"]
        and boundary.get("provider_per_request_capacity")
        == ["input_tokens", "output_tokens"]
        and boundary.get("bounded_cleanup_after_run") == ["cleanup_grace_seconds"],
        "budget proposal enforcement boundary is incomplete",
    )


def _validate_legacy_budget_proposal(proposal: dict[str, Any]) -> None:
    """Validate immutable v2 evidence without using it for new authorizations."""
    expected = dict(proposal)
    digest = expected.pop("proposal_sha256", None)
    require(digest == canonical_json_sha256(expected), "budget proposal digest mismatch")
    require(
        expected.get("proposal_id")
        == f"CBP-{canonical_json_sha256({key: value for key, value in expected.items() if key != 'proposal_id'})[:16].upper()}",
        "budget proposal id mismatch",
    )
    validate_budget_selection(proposal.get("selection", {}))
    require(
        isinstance(proposal.get("maximums"), dict)
        and isinstance(proposal.get("per_sample_run_observation_thresholds"), dict)
        and isinstance(proposal.get("provider_hard_limits"), dict),
        "legacy budget proposal is incomplete",
    )
