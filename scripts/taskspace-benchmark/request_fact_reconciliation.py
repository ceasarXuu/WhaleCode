"""Reconcile provider attempts with supervised boundary lifecycle evidence."""

from __future__ import annotations

from collections import Counter
from typing import Any


def _finding(code: str, source: str, **identity: Any) -> dict[str, Any]:
    return {"code": code, "source": source, **identity}


def parse_boundary(
    events: list[dict[str, Any]],
    findings: list[dict[str, Any]],
    expected_model: str | None,
) -> tuple[list[dict[str, Any]], bool, str | None]:
    starts = [event for event in events if event.get("event") == "provider_boundary_started"]
    stops = [event for event in events if event.get("event") == "provider_boundary_stopped"]
    lifecycle_model = starts[0].get("allowed_model") if len(starts) == 1 else None
    lifecycle_valid = (
        len(starts) == 1
        and len(stops) == 1
        and events
        and events[0].get("event") == "provider_boundary_started"
        and events[-1].get("event") == "provider_boundary_stopped"
    )
    if not lifecycle_valid:
        findings.append(
            _finding(
                "boundary_lifecycle_missing",
                "boundary",
                start_line_number=starts[0].get("_request_facts_line_number") if starts else None,
                stop_line_number=stops[0].get("_request_facts_line_number") if stops else None,
            )
        )
    elif (
        starts[0].get("allowed_method") != "POST"
        or starts[0].get("allowed_path") != "/responses"
        or not isinstance(lifecycle_model, str)
        or not lifecycle_model
        or (expected_model is not None and lifecycle_model != expected_model)
    ):
        findings.append(
            _finding(
                "boundary_lifecycle_invalid",
                "boundary",
                start_line_number=starts[0].get("_request_facts_line_number"),
                stop_line_number=stops[0].get("_request_facts_line_number"),
            )
        )
        lifecycle_valid = False

    claims = []
    raw_claims = [event for event in events if event.get("event") == "provider_request_claimed"]
    for event in raw_claims:
        index = len(claims) + 1
        digest = event.get("body_sha256")
        valid = (
            event.get("count") == index
            and event.get("method") == "POST"
            and event.get("path") == "/responses"
            and isinstance(lifecycle_model, str)
            and event.get("model") == lifecycle_model
            and isinstance(digest, str)
            and len(digest) == 64
            and not any(character not in "0123456789abcdef" for character in digest)
        )
        if not valid:
            findings.append(
                _finding(
                    "boundary_claim_invalid",
                    "boundary",
                    boundary_index=index,
                    line_number=event.get("_request_facts_line_number"),
                )
            )
            lifecycle_valid = False
            continue
        claims.append(
            {
                "boundary_index": index,
                "count": event["count"],
                "method": event["method"],
                "path": event["path"],
                "model": event.get("model"),
                "body_sha256": digest,
                "provider_payload_sha256": digest,
                "boundary_line_number": event.get("_request_facts_line_number"),
            }
        )
    if lifecycle_valid and stops[0].get("request_count") != len(raw_claims):
        findings.append(
            _finding(
                "boundary_lifecycle_count_mismatch",
                "boundary",
                line_number=stops[0].get("_request_facts_line_number"),
            )
        )
        lifecycle_valid = False
    return claims, lifecycle_valid, lifecycle_model if isinstance(lifecycle_model, str) else None


def reconcile(
    rows: dict[str, dict[str, Any]],
    claims: list[dict[str, Any]],
    boundary_count_available: bool,
    findings: list[dict[str, Any]],
) -> None:
    attempt_rows = [row for row in rows.values() if "attempt" in row]
    attempt_counts = Counter(row["attempt"]["provider_payload_sha256"] for row in attempt_rows)
    claim_counts = Counter(claim["provider_payload_sha256"] for claim in claims)
    ambiguous = {
        digest for digest in set(attempt_counts) | set(claim_counts)
        if attempt_counts[digest] > 1 or claim_counts[digest] > 1
    }
    for digest in sorted(ambiguous):
        findings.append(_finding("boundary_correlation_ambiguous", "reconcile", digest=digest))
    attempts_by_digest = {
        row["attempt"]["provider_payload_sha256"]: row
        for row in attempt_rows
        if row["attempt"]["provider_payload_sha256"] not in ambiguous
    }
    for row in attempt_rows:
        if row["attempt"]["provider_payload_sha256"] in ambiguous:
            row["boundary_correlation_ambiguous"] = True
    for claim in claims:
        digest = claim["provider_payload_sha256"]
        if digest in ambiguous:
            continue
        row = attempts_by_digest.get(digest)
        if row is None:
            findings.append(
                _finding(
                    "boundary_unattributed",
                    "boundary",
                    boundary_index=claim["boundary_index"],
                    line_number=claim.get("boundary_line_number"),
                )
            )
        else:
            row["boundary"] = claim
    for row in rows.values():
        terminal = row.get("terminal", {})
        ambiguous_row = bool(row.get("boundary_correlation_ambiguous"))
        if boundary_count_available and "attempt" in row and "boundary" not in row and not ambiguous_row:
            if terminal.get("status") == "response_completed":
                findings.append(
                    _finding(
                        "completed_without_boundary",
                        "reconcile",
                        request_id=row["request_id"],
                        wire_line_number=terminal.get("wire_line_number"),
                    )
                )
            elif terminal.get("status") not in {"response_failed", "cancelled", "retry_unauthorized"}:
                findings.append(_finding("boundary_status_unknown", "reconcile", request_id=row["request_id"], wire_line_number=row.get("attempt", {}).get("wire_line_number")))
        if "terminal" in row and "attempt" not in row:
            findings.append(_finding("terminal_without_attempt", "reconcile", request_id=row["request_id"], wire_line_number=terminal.get("wire_line_number")))
        if "attempt" in row and "terminal" not in row:
            findings.append(_finding("terminal_missing", "reconcile", request_id=row["request_id"], wire_line_number=row.get("attempt", {}).get("wire_line_number")))
        rollout_usage = row.get("rollout_usage")
        wire_usage = terminal.get("usage")
        if rollout_usage is not None and wire_usage is not None and rollout_usage != wire_usage:
            row["usage_invalid"] = True
            findings.append(
                _finding(
                    "usage_source_conflict",
                    "reconcile",
                    request_id=row["request_id"],
                    rollout_line_number=row.get("rollout_line_number"),
                    wire_line_number=terminal.get("wire_line_number"),
                )
            )
        if rollout_usage is not None and terminal.get("status") not in {None, "response_completed"}:
            row["usage_invalid"] = True
            findings.append(
                _finding(
                    "usage_terminal_conflict",
                    "reconcile",
                    request_id=row["request_id"],
                    rollout_line_number=row.get("rollout_line_number"),
                    wire_line_number=terminal.get("wire_line_number"),
                )
            )
