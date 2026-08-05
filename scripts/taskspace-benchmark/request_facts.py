#!/usr/bin/env python3
"""Normalize rollout, provider wire, and boundary evidence into request facts."""

from __future__ import annotations

import json
from collections import Counter
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "whalecode-request-facts-v1"
ANALYZER_VERSION = "i07-w2-v1"
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
    return {
        "path": str(path) if path is not None else None,
        "status": "read" if path is not None and path.is_file() else "unavailable",
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
                {"request_index": index, "provider_payload_sha256": digest},
                findings,
                "wire",
                request_id,
                counters,
            )
            continue
        terminal = {"status": "cancelled" if status == "response_cancelled" else status}
        if status == "response_completed":
            try:
                terminal["usage"] = _usage(event)
            except ValueError as error:
                findings.append(_finding(str(error), "wire", request_id=request_id))
        _put_once(row, "terminal", terminal, findings, "wire", request_id, counters)


def _parse_boundary(
    events: list[dict[str, Any]],
    findings: list[dict[str, Any]],
    expected_model: str | None,
) -> list[dict[str, Any]]:
    claims = []
    for event in events:
        if event.get("event") != "provider_request_claimed":
            continue
        index = len(claims) + 1
        digest = event.get("body_sha256")
        valid = (
            event.get("count") == index
            and event.get("method") == "POST"
            and event.get("path") == "/responses"
            and (expected_model is None or event.get("model") == expected_model)
            and isinstance(digest, str)
            and len(digest) == 64
            and not any(character not in "0123456789abcdef" for character in digest)
        )
        if not valid:
            findings.append(_finding("boundary_claim_invalid", "boundary", boundary_index=index))
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
            }
        )
    digests = [claim["provider_payload_sha256"] for claim in claims]
    if len(digests) != len(set(digests)):
        findings.append(_finding("boundary_digest_ambiguous", "boundary"))
    return claims


def _reconcile(
    rows: dict[str, dict[str, Any]],
    claims: list[dict[str, Any]],
    boundary_available: bool,
    findings: list[dict[str, Any]],
) -> None:
    attempt_rows = [row for row in rows.values() if "attempt" in row]
    digest_counts = Counter(
        row["attempt"]["provider_payload_sha256"] for row in attempt_rows
    )
    for digest, count in digest_counts.items():
        if count > 1:
            findings.append(_finding("attempt_digest_ambiguous", "wire", digest=digest))
    attempts_by_digest = {
        row["attempt"]["provider_payload_sha256"]: row
        for row in attempt_rows
        if digest_counts[row["attempt"]["provider_payload_sha256"]] == 1
    }
    for claim in claims:
        digest = claim["provider_payload_sha256"]
        row = attempts_by_digest.get(digest)
        if row is None:
            findings.append(_finding("boundary_unattributed", "boundary", boundary_index=claim["boundary_index"]))
            continue
        row["boundary"] = claim
    for row in rows.values():
        terminal = row.get("terminal", {})
        if boundary_available and "attempt" in row and "boundary" not in row:
            if terminal.get("status") == "response_completed":
                findings.append(_finding("completed_without_boundary", "reconcile", request_id=row["request_id"]))
            elif terminal.get("status") not in {
                "response_failed",
                "cancelled",
                "retry_unauthorized",
            }:
                findings.append(_finding("boundary_status_unknown", "reconcile", request_id=row["request_id"]))
        if "terminal" in row and "attempt" not in row:
            findings.append(_finding("terminal_without_attempt", "reconcile", request_id=row["request_id"]))
        if "attempt" in row and "terminal" not in row:
            findings.append(_finding("terminal_missing", "reconcile", request_id=row["request_id"]))
        rollout_usage = row.get("rollout_usage")
        wire_usage = terminal.get("usage")
        if rollout_usage is not None and wire_usage is not None and rollout_usage != wire_usage:
            findings.append(_finding("usage_source_conflict", "reconcile", request_id=row["request_id"]))


def _normalized_rows(rows: dict[str, dict[str, Any]], boundary_available: bool) -> list[dict[str, Any]]:
    normalized = []
    for row in rows.values():
        logical_id, attempt_seq = row.get("identity", [None, None])
        terminal = row.get("terminal", {})
        usage = terminal.get("usage") or row.get("rollout_usage")
        normalized.append(
            {
                "request_id": row["request_id"],
                "logical_request_id": logical_id,
                "attempt_seq": attempt_seq,
                "request_index": row.get("attempt", {}).get("request_index"),
                "observation_index": row.get("attempt", {}).get("request_index")
                or row.get("rollout_index"),
                "provider_payload_sha256": row.get("attempt", {}).get("provider_payload_sha256"),
                "attempt_status": "observed" if "attempt" in row else "unavailable",
                "boundary_status": (
                    "observed"
                    if "boundary" in row
                    else "not_observed" if boundary_available and "attempt" in row else "unavailable"
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
    key = lambda row: (row["observation_index"] is None, row["observation_index"] or 0, row["request_id"])
    return sorted(normalized, key=key)


def _percentile(values: list[int], percentile: int) -> int | None:
    if not values:
        return None
    ordered = sorted(values)
    rank = (percentile * len(ordered) + 99) // 100
    return ordered[max(0, min(len(ordered) - 1, rank - 1))]


def _usage_summary(rows: list[dict[str, Any]]) -> dict[str, Any]:
    usage = [row["usage"] for row in rows if row["usage"] is not None]
    inputs = [value["input_tokens"] for value in usage]
    cached = [value["cached_input_tokens"] for value in usage]
    outputs = [value["output_tokens"] for value in usage]
    return {
        "input_tokens": sum(inputs),
        "cached_input_tokens": sum(cached),
        "uncached_input_tokens": sum(inputs) - sum(cached),
        "output_tokens": sum(outputs),
        "reasoning_output_tokens": sum(value["reasoning_output_tokens"] for value in usage),
        "total_tokens": sum(value["total_tokens"] for value in usage),
        "distribution": {
            "first_input_tokens": inputs[0] if inputs else None,
            "last_input_tokens": inputs[-1] if inputs else None,
            "max_input_tokens": max(inputs) if inputs else None,
            "p95_input_tokens": _percentile(inputs, 95),
            "first_output_tokens": outputs[0] if outputs else None,
            "last_output_tokens": outputs[-1] if outputs else None,
            "max_output_tokens": max(outputs) if outputs else None,
            "p95_output_tokens": _percentile(outputs, 95),
            "max_cached_input_tokens": max(cached) if cached else None,
            "p95_cached_input_tokens": _percentile(cached, 95),
        },
    }


def build_request_facts(
    rollout_path: Path | None = None,
    wire_path: Path | None = None,
    boundary_path: Path | None = None,
    expected_model: str | None = None,
) -> dict[str, Any]:
    findings: list[dict[str, Any]] = []
    counters: Counter[str] = Counter()
    rows: dict[str, dict[str, Any]] = {}
    rollout_events = _read_jsonl(rollout_path, "rollout", findings)
    wire_events = _read_jsonl(wire_path, "wire", findings)
    boundary_events = _read_jsonl(boundary_path, "boundary", findings)
    _parse_rollout(rollout_events, rows, findings, counters)
    _parse_wire(wire_events, rows, findings, counters)
    claims = _parse_boundary(boundary_events, findings, expected_model)
    boundary_available = boundary_path is not None and boundary_path.is_file()
    _reconcile(rows, claims, boundary_available, findings)
    normalized = _normalized_rows(rows, boundary_available)
    codes = {finding["code"] for finding in findings}
    wire_source_errors = any(
        finding["source"] == "wire"
        and finding["code"] in {
            "json_invalid",
            "json_object_required",
            "identity_incomplete",
            "identity_conflict",
            "wire_schema_unsupported",
            "attempt_evidence_invalid",
            "attempt_digest_ambiguous",
        }
        for finding in findings
    )
    rollout_source_errors = any(
        finding["source"] == "rollout"
        and finding["code"] in {
            "json_invalid",
            "json_object_required",
            "identity_incomplete",
            "identity_conflict",
            "usage_missing",
            "usage_invalid",
        }
        for finding in findings
    )
    boundary_conflict = bool(
        codes
        & {
            "boundary_claim_invalid",
            "boundary_digest_ambiguous",
            "boundary_unattributed",
            "completed_without_boundary",
            "boundary_status_unknown",
            "attempt_digest_ambiguous",
        }
    ) or any(
        finding["source"] == "boundary"
        and finding["code"] in {"json_invalid", "json_object_required"}
        for finding in findings
    )
    completion_conflict = wire_source_errors or rollout_source_errors or bool(
        codes & {"terminal_without_attempt", "usage_source_conflict"}
    )
    usage_conflict = rollout_source_errors or wire_source_errors or bool(
        codes & {"usage_source_conflict"}
    )
    completed = [row for row in normalized if row["terminal_status"] == "response_completed"]
    usage_rows = [row for row in normalized if row["usage"] is not None]
    attempts = [row for row in normalized if row["attempt_status"] == "observed"]
    boundary_rows = [row for row in normalized if row["boundary_status"] == "observed"]
    failed_rows = [
        row
        for row in normalized
        if row["terminal_status"]
        in {"response_failed", "cancelled", "retry_unauthorized"}
    ]
    logical_ids = {row["logical_request_id"] for row in normalized if row["logical_request_id"]}
    return {
        "schema_version": SCHEMA_VERSION,
        "analyzer_version": ANALYZER_VERSION,
        "sources": {
            "rollout": _source(rollout_path),
            "wire": _source(wire_path),
            "boundary": _source(boundary_path),
        },
        "availability": {
            "attempt": "incomparable" if wire_source_errors else "measured" if wire_events else "unavailable",
            "boundary": "incomparable" if boundary_conflict else "measured" if boundary_available else "unavailable",
            "completion": (
                "incomparable"
                if completion_conflict
                else "partial" if "terminal_missing" in codes
                else "measured" if wire_events or rollout_events
                else "unavailable"
            ),
            "usage": (
                "incomparable"
                if usage_conflict
                else "partial" if "terminal_missing" in codes and usage_rows
                else "measured" if usage_rows
                else "unavailable"
            ),
        },
        "summary": {
            "logical_request_count": len(logical_ids),
            "local_attempt_count": len(attempts),
            "boundary_request_count": len(claims) if boundary_available else None,
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
            "usage": _usage_summary(normalized),
        },
        "boundary_claims": claims,
        "rows": normalized,
        "findings": findings,
    }
