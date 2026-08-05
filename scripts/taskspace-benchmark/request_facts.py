#!/usr/bin/env python3
"""Normalize rollout, provider wire, and boundary evidence into request facts."""

from __future__ import annotations

import json
import hashlib
from collections import Counter
from pathlib import Path
from typing import Any

from request_fact_availability import classify_availability
from request_fact_diagnostics import build_diagnostics
from request_fact_reconciliation import parse_boundary, reconcile
from request_fact_summary import usage_summary
from request_fact_validation import validate_attempt_sequences


SCHEMA_VERSION = "whalecode-request-facts-v1"
ANALYZER_VERSION = "i07-review-fixes-v2"
WIRE_SCHEMA_VERSION = "provider-chat-wire-trace-v10"
TERMINAL_STATUSES = {"response_completed", "response_failed", "cancelled", "response_cancelled", "retry_unauthorized"}
TOKEN_FIELDS = ("input_tokens", "cached_input_tokens", "output_tokens", "reasoning_output_tokens", "total_tokens")


def _read_jsonl(path: Path | None, source: str, findings: list[dict[str, Any]]) -> list[dict[str, Any]]:
    if path is None or not path.is_file():
        return []
    values = []
    for line_number, raw in enumerate(path.read_text(encoding="utf-8-sig").splitlines(), 1):
        if not raw.strip():
            continue
        try:
            value = json.loads(raw)
        except json.JSONDecodeError:
            findings.append(_finding("json_invalid", source, line_number=line_number))
            continue
        if not isinstance(value, dict):
            findings.append(_finding("json_object_required", source, line_number=line_number))
            continue
        value["_request_facts_line_number"] = line_number
        values.append(value)
    return values
def _finding(code: str, source: str, **identity: Any) -> dict[str, Any]:
    return {"code": code, "source": source, **identity}
def _identity(event: dict[str, Any], prefix: str = "") -> tuple[str, str, int] | None:
    request_id = event.get(f"{prefix}request_id")
    logical_id = event.get(f"{prefix}logical_request_id")
    attempt = event.get(f"{prefix}attempt_seq")
    present = (request_id is not None, logical_id is not None, attempt is not None)
    if not any(present):
        return None
    if (
        not all(present)
        or not isinstance(request_id, str)
        or not request_id
        or not isinstance(logical_id, str)
        or not logical_id
        or isinstance(attempt, bool)
        or not isinstance(attempt, int)
        or attempt < 1
    ):
        raise ValueError("identity_incomplete")
    return request_id, logical_id, attempt


def _usage(value: Any) -> dict[str, int]:
    if not isinstance(value, dict):
        raise ValueError("usage_missing")
    result = {}
    for field in TOKEN_FIELDS:
        token = value.get(field)
        if isinstance(token, bool) or not isinstance(token, int) or token < 0:
            raise ValueError("usage_invalid")
        result[field] = token
    if (
        result["cached_input_tokens"] > result["input_tokens"]
        or result["reasoning_output_tokens"] > result["output_tokens"]
        or result["total_tokens"] != result["input_tokens"] + result["output_tokens"]
    ):
        raise ValueError("usage_invalid")
    return result


def _source(path: Path | None) -> dict[str, Any]:
    available = path is not None and path.is_file()
    return {
        "path": str(path) if path is not None else None,
        "status": "read" if available else "unavailable",
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest() if available else None,
    }


def _put_once(
    target: dict[str, Any],
    key: str,
    value: Any,
    findings: list[dict[str, Any]],
    source: str,
    request_id: str,
    counters: Counter[str],
    count_equal: bool = True,
) -> None:
    if key not in target:
        target[key] = value
    elif target[key] == value:
        if count_equal:
            counters["duplicate_event_count"] += 1
    else:
        findings.append(_finding("identity_conflict", source, request_id=request_id))


def _parse_rollout(
    events: list[dict[str, Any]],
    rows: dict[str, dict[str, Any]],
    findings: list[dict[str, Any]],
    counters: Counter[str],
) -> None:
    for event in events:
        if event.get("type") != "event_msg" or not isinstance(event.get("payload"), dict):
            continue
        payload = event["payload"]
        if payload.get("type") != "token_count":
            continue
        usage_value = (payload.get("info") or {}).get("last_token_usage")
        try:
            identity = _identity(payload, "provider_")
        except ValueError:
            findings.append(_finding("identity_incomplete", "rollout"))
            continue
        if identity is None:
            if usage_value is not None:
                counters["state_snapshot_count"] += 1
            continue
        request_id, logical_id, attempt = identity
        try:
            usage = _usage(usage_value)
        except ValueError as error:
            findings.append(_finding(str(error), "rollout", request_id=request_id))
            continue
        row = rows.setdefault(request_id, {"request_id": request_id})
        if "rollout_index" not in row:
            counters["rollout_request_index"] += 1
            row["rollout_index"] = counters["rollout_request_index"]
            row["rollout_line_number"] = event["_request_facts_line_number"]
        _put_once(
            row,
            "identity",
            [logical_id, attempt],
            findings,
            "rollout",
            request_id,
            counters,
            count_equal=False,
        )
        _put_once(row, "rollout_usage", usage, findings, "rollout", request_id, counters)


def _parse_wire(
    events: list[dict[str, Any]],
    rows: dict[str, dict[str, Any]],
    findings: list[dict[str, Any]],
    counters: Counter[str],
) -> None:
    for event in events:
        status = event.get("status")
        if status != "payload_captured" and status not in TERMINAL_STATUSES:
            continue
        request_id = event.get("request_id")
        try:
            identity = _identity(event)
        except ValueError:
            findings.append(_finding("identity_incomplete", "wire", request_id=request_id))
            continue
        if event.get("schema_version") != WIRE_SCHEMA_VERSION:
            findings.append(_finding("wire_schema_unsupported", "wire", request_id=request_id))
            continue
        if identity is None:
            findings.append(_finding("identity_incomplete", "wire"))
            continue
        request_id, logical_id, attempt = identity
        row = rows.setdefault(request_id, {"request_id": request_id})
        _put_once(
            row,
            "identity",
            [logical_id, attempt],
            findings,
            "wire",
            request_id,
            counters,
            count_equal=False,
        )
        if status == "payload_captured":
            digest = event.get("provider_payload_sha256")
            index = event.get("request_index")
            if (
                not isinstance(digest, str)
                or len(digest) != 64
                or any(character not in "0123456789abcdef" for character in digest)
                or isinstance(index, bool)
                or not isinstance(index, int)
                or index < 1
            ):
                findings.append(_finding("attempt_evidence_invalid", "wire", request_id=request_id))
                continue
            _put_once(
                row,
                "attempt",
                {
                    "request_index": index,
                    "provider_payload_sha256": digest,
                    "wire_line_number": event.get("_request_facts_line_number"),
                },
                findings,
                "wire",
                request_id,
                counters,
            )
            continue
        terminal = {
            "status": "cancelled" if status == "response_cancelled" else status,
            "wire_line_number": event.get("_request_facts_line_number"),
        }
        if status == "response_completed":
            try:
                terminal["usage"] = _usage(event)
            except ValueError as error:
                findings.append(_finding(str(error), "wire", request_id=request_id))
        _put_once(row, "terminal", terminal, findings, "wire", request_id, counters)


def _normalized_rows(
    rows: dict[str, dict[str, Any]], boundary_count_available: bool
) -> list[dict[str, Any]]:
    normalized = []
    for row in rows.values():
        logical_id, attempt_seq = row.get("identity", [None, None])
        terminal = row.get("terminal", {})
        usage = None if row.get("usage_invalid") else terminal.get("usage") or row.get("rollout_usage")
        normalized.append(
            {
                "request_id": row["request_id"],
                "logical_request_id": logical_id,
                "attempt_seq": attempt_seq,
                "request_index": row.get("attempt", {}).get("request_index"),
                "observation_index": row.get("attempt", {}).get("request_index")
                or row.get("rollout_index"),
                "rollout_line_number": row.get("rollout_line_number"),
                "wire_attempt_line_number": row.get("attempt", {}).get("wire_line_number"),
                "wire_terminal_line_number": terminal.get("wire_line_number"),
                "provider_payload_sha256": row.get("attempt", {}).get("provider_payload_sha256"),
                "attempt_status": "observed" if "attempt" in row else "unavailable",
                "boundary_status": (
                    "observed"
                    if "boundary" in row
                    else "unavailable" if row.get("boundary_correlation_ambiguous")
                    else "not_observed" if boundary_count_available and "attempt" in row else "unavailable"
                ),
                "boundary_index": row.get("boundary", {}).get("boundary_index"),
                "terminal_status": terminal.get("status") or (
                    "response_completed" if "rollout_usage" in row else "missing"
                ),
                "usage": usage,
                "usage_source": (
                    "wire_and_rollout"
                    if terminal.get("usage") is not None and row.get("rollout_usage") is not None
                    else "wire" if terminal.get("usage") is not None
                    else "rollout" if row.get("rollout_usage") is not None
                    else None
                ),
            }
        )
    key = lambda row: (
        row["observation_index"] is None,
        row["observation_index"] or 0,
        row["request_id"],
    )
    return sorted(normalized, key=key)

def build_request_facts_from_events(
    rollout_events: list[dict[str, Any]] | None = None,
    wire_events: list[dict[str, Any]] | None = None,
    boundary_events: list[dict[str, Any]] | None = None,
    expected_model: str | None = None,
    sources: dict[str, dict[str, Any]] | None = None,
    initial_findings: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    findings = list(initial_findings or [])
    counters: Counter[str] = Counter()
    rows: dict[str, dict[str, Any]] = {}
    rollout_available = rollout_events is not None
    wire_available = wire_events is not None
    boundary_source_available = boundary_events is not None
    rollout_events = rollout_events or []
    wire_events = wire_events or []
    boundary_events = boundary_events or []
    for events in (rollout_events, wire_events, boundary_events):
        for line_number, event in enumerate(events, 1):
            event.setdefault("_request_facts_line_number", line_number)
    _parse_rollout(rollout_events, rows, findings, counters)
    _parse_wire(wire_events, rows, findings, counters)
    claims, boundary_count_available, boundary_model = parse_boundary(
        boundary_events, findings, expected_model
    ) if boundary_source_available else ([], False, None)
    reconcile(rows, claims, boundary_count_available, findings)
    validate_attempt_sequences(rows, findings, "response_completed")
    normalized = _normalized_rows(rows, boundary_count_available)
    availability = classify_availability(
        findings,
        normalized,
        rollout_available=rollout_available,
        wire_available=wire_available,
        boundary_available=boundary_count_available,
    )
    diagnostics = build_diagnostics(
        normalized,
        claims,
        findings,
        counters,
        {
            "rollout": len(rollout_events),
            "wire": len(wire_events),
            "boundary": len(boundary_events),
        },
    )
    completed = [row for row in normalized if row["terminal_status"] == "response_completed"]
    usage_rows = [row for row in normalized if row["usage"] is not None]
    attempts = [row for row in normalized if row["attempt_status"] == "observed"]
    failed_rows = [
        row
        for row in normalized
        if row["terminal_status"]
        in {"response_failed", "cancelled", "retry_unauthorized"}
    ]
    logical_ids = {
        row["logical_request_id"] for row in normalized if row["logical_request_id"]
    }
    logical_attempts = Counter(
        row["logical_request_id"] for row in attempts if row["logical_request_id"]
    )
    return {
        "schema_version": SCHEMA_VERSION,
        "analyzer_version": ANALYZER_VERSION,
        "sources": sources or {
            "rollout": {"path": None, "status": "provided" if rollout_available else "unavailable"},
            "wire": {"path": None, "status": "provided" if wire_available else "unavailable"},
            "boundary": {"path": None, "status": "provided" if boundary_source_available else "unavailable"},
        },
        "availability": availability,
        "boundary_identity": {
            "expected_model": boundary_model,
            "lifecycle_status": "complete" if boundary_count_available else "unavailable",
        },
        "summary": {
            "logical_request_count": len(logical_ids),
            "retried_logical_request_count": sum(count > 1 for count in logical_attempts.values()),
            "local_attempt_count": len(attempts),
            "boundary_request_count": len(claims) if boundary_count_available else None,
            "completed_response_count": len(completed),
            "failed_or_cancelled_attempt_count": len(failed_rows),
            "usage_record_count": len(usage_rows),
            "state_snapshot_count": counters["state_snapshot_count"],
            "duplicate_event_count": counters["duplicate_event_count"],
            "local_only_attempt_count": len(
                [row for row in attempts if row["boundary_status"] == "not_observed"]
            ),
            "boundary_unattributed_count": sum(
                finding["code"] == "boundary_unattributed" for finding in findings
            ),
            "usage": usage_summary(normalized),
        },
        "boundary_claims": claims,
        "rows": normalized,
        "findings": findings,
        "diagnostics": diagnostics,
    }


def build_request_facts(
    rollout_path: Path | None = None,
    wire_path: Path | None = None,
    boundary_path: Path | None = None,
    expected_model: str | None = None,
) -> dict[str, Any]:
    findings: list[dict[str, Any]] = []
    rollout_available = rollout_path is not None and rollout_path.is_file()
    wire_available = wire_path is not None and wire_path.is_file()
    boundary_available = boundary_path is not None and boundary_path.is_file()
    return build_request_facts_from_events(
        _read_jsonl(rollout_path, "rollout", findings) if rollout_available else None,
        _read_jsonl(wire_path, "wire", findings) if wire_available else None,
        _read_jsonl(boundary_path, "boundary", findings) if boundary_available else None,
        expected_model,
        {
            "rollout": _source(rollout_path),
            "wire": _source(wire_path),
            "boundary": _source(boundary_path),
        },
        findings,
    )
