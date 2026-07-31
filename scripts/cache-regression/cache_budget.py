#!/usr/bin/env python3
"""Pure budget-plan construction for authorized cache regression smoke runs."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from cache_evidence import canonical_json_sha256, file_sha256
from cache_surface import surface_snapshot


BUDGET_PROPOSAL_SCHEMA_VERSION = "whalecode-cache-budget-proposal-v1"
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
    max_input_tokens_per_run: int,
    max_output_tokens_per_run: int,
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
        (max_input_tokens_per_run, "input token limit"),
        (max_output_tokens_per_run, "output token limit"),
        (max_seconds_per_run, "time limit"),
    ):
        require(isinstance(value, int) and value > 0, f"{label} must be positive")
    require(
        isinstance(retry_sample_run_limit, int) and retry_sample_run_limit >= 0,
        "retry sample-run limit must be non-negative",
    )

    planned_sample_runs = len(samples) * len(arms) * repeat
    maximum_sample_runs = planned_sample_runs + retry_sample_run_limit
    maximum_input_tokens = maximum_sample_runs * max_input_tokens_per_run
    maximum_output_tokens = maximum_sample_runs * max_output_tokens_per_run
    pricing = contract["pricing_snapshot"]
    maximum_cost = (
        maximum_input_tokens / 1_000_000 * pricing["uncached_input_per_million"]
        + maximum_output_tokens / 1_000_000 * pricing["output_per_million"]
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
            "input_tokens": max_input_tokens_per_run,
            "output_tokens": max_output_tokens_per_run,
            "elapsed_seconds": max_seconds_per_run,
        },
        "maximums": {
            "provider_requests": maximum_sample_runs * max_provider_requests_per_run,
            "input_tokens": maximum_input_tokens,
            "output_tokens": maximum_output_tokens,
            "elapsed_seconds": maximum_sample_runs * max_seconds_per_run,
            "estimated_cost": round(maximum_cost, 10),
            "currency": pricing["currency"],
        },
        "pricing_snapshot": pricing,
        "cost_assumption": "all input tokens are charged at the uncached input rate",
        "enforcement_boundary": {
            "hard_before_start": [
                "subject_commit",
                "surface_sha256",
                "proposal_and_authorization_identity",
                "sample_arm_repeat_matrix",
                "no_automatic_retries",
            ],
            "hard_during_run": ["elapsed_seconds"],
            "observed_after_each_run": [
                "provider_requests",
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
