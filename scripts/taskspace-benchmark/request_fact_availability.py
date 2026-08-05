"""Derive per-metric availability from canonical request findings."""

from __future__ import annotations

from typing import Any


def classify_availability(
    findings: list[dict[str, Any]],
    normalized: list[dict[str, Any]],
    *,
    rollout_available: bool,
    wire_available: bool,
    boundary_available: bool,
) -> dict[str, str]:
    codes = {finding["code"] for finding in findings}
    wire_errors = {
        "json_invalid",
        "json_object_required",
        "identity_incomplete",
        "identity_conflict",
        "wire_schema_unsupported",
        "attempt_evidence_invalid",
        "attempt_digest_ambiguous",
        "attempt_index_sequence_invalid",
        "logical_attempt_sequence_invalid",
        "logical_completion_sequence_invalid",
    }
    rollout_errors = {
        "json_invalid",
        "json_object_required",
        "identity_incomplete",
        "identity_conflict",
        "usage_missing",
        "usage_invalid",
    }
    wire_invalid = any(
        finding["source"] == "wire" and finding["code"] in wire_errors
        for finding in findings
    )
    rollout_invalid = any(
        finding["source"] == "rollout" and finding["code"] in rollout_errors
        for finding in findings
    )
    boundary_codes = {
        "boundary_claim_invalid",
        "boundary_digest_ambiguous",
        "boundary_unattributed",
        "completed_without_boundary",
        "boundary_status_unknown",
        "attempt_digest_ambiguous",
    }
    boundary_invalid = bool(codes & boundary_codes) or any(
        finding["source"] == "boundary"
        and finding["code"] in {"json_invalid", "json_object_required"}
        for finding in findings
    )
    completion_invalid = wire_invalid or rollout_invalid or bool(
        codes & {"terminal_without_attempt", "usage_source_conflict"}
    )
    usage_invalid = rollout_invalid or wire_invalid or "usage_source_conflict" in codes
    has_usage = any(row["usage"] is not None for row in normalized)
    return {
        "attempt": "incomparable" if wire_invalid else "measured" if wire_available else "unavailable",
        "boundary": "incomparable" if boundary_invalid else "measured" if boundary_available else "unavailable",
        "completion": (
            "incomparable"
            if completion_invalid
            else "partial"
            if "terminal_missing" in codes
            else "measured"
            if wire_available or rollout_available
            else "unavailable"
        ),
        "usage": (
            "incomparable"
            if usage_invalid
            else "partial"
            if "terminal_missing" in codes and has_usage
            else "measured"
            if has_usage
            else "unavailable"
        ),
    }
