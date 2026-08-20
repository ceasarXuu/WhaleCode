#!/usr/bin/env python3
"""Shared provider usage fixtures and cache aggregation invariants."""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path
from typing import Any

from cache_json import strict_json_loads

TASKSPACE_BENCHMARK = Path(__file__).resolve().parents[1] / "taskspace-benchmark"
sys.path.insert(0, str(TASKSPACE_BENCHMARK))
from request_facts import build_request_facts_from_events  # noqa: E402


SCHEMA_VERSION = "whalecode-provider-usage-v1"
TOKEN_FIELDS = (
    "input_tokens",
    "cached_input_tokens",
    "output_tokens",
    "reasoning_output_tokens",
    "total_tokens",
)


def _nonnegative_integer(value: Any, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise ValueError(f"{field} must be a nonnegative integer")
    return value


def load_provider_usage_fixture(path: Path) -> dict[str, Any]:
    fixture = json.loads(path.read_text(encoding="utf-8"))
    if fixture.get("schema_version") != SCHEMA_VERSION:
        raise ValueError("unsupported provider usage contract")
    return fixture


def normalized_fixture_cases(
    fixture: dict[str, Any], wire_api: str
) -> dict[str, dict[str, int] | None]:
    if wire_api not in {"chat_completions", "responses"}:
        raise ValueError(f"unsupported fixture wire API: {wire_api}")
    normalized = {}
    for case in fixture[wire_api]:
        case_id = case["id"]
        if case.get("expected_error") is True:
            normalized[case_id] = None
            continue
        expected = case.get("expected")
        if not isinstance(expected, dict):
            raise ValueError(f"fixture case {case_id} has no normalized usage")
        normalized[case_id] = {
            field: _nonnegative_integer(expected.get(field), field)
            for field in TOKEN_FIELDS
        }
    return normalized


def aggregate_usage_records(records: list[dict[str, int] | None]) -> dict[str, Any]:
    if not records:
        raise ValueError("provider usage aggregation requires at least one request")
    if any(record is None for record in records):
        raise ValueError("provider usage is missing or undecodable")
    checked = [
        {
            field: _nonnegative_integer(record.get(field), field)
            for field in TOKEN_FIELDS
        }
        for record in records
        if record is not None
    ]
    for record in checked:
        if record["cached_input_tokens"] > record["input_tokens"]:
            raise ValueError("cached input tokens exceed input tokens")
    request_2_plus = checked[1:]
    request_2_cached = sum(record["cached_input_tokens"] for record in request_2_plus)
    request_2_input = sum(record["input_tokens"] for record in request_2_plus)
    return {
        "provider_request_count": len(checked),
        "input_tokens": sum(record["input_tokens"] for record in checked),
        "cached_input_tokens": sum(record["cached_input_tokens"] for record in checked),
        "output_tokens": sum(record["output_tokens"] for record in checked),
        "request_2_plus_count": len(request_2_plus),
        "request_2_plus_cached_input_tokens": request_2_cached,
        "request_2_plus_uncached_input_tokens": request_2_input - request_2_cached,
        "request_2_plus_hit_rate": (
            request_2_cached / request_2_input if request_2_input else None
        ),
    }


def parse_provider_wire_usage(
    text: str, provider_request_ids: list[str] | None = None
) -> dict[str, Any]:
    events = []
    for line_number, raw_line in enumerate(text.splitlines(), 1):
        if not raw_line.strip():
            continue
        event = strict_json_loads(raw_line)
        if not isinstance(event, dict):
            raise ValueError(f"provider wire event {line_number} must be an object")
        events.append(event)
    facts = build_request_facts_from_events(wire_events=events)
    if not events:
        raise ValueError("provider wire usage requires at least one request")
    if any(facts["availability"][name] != "measured" for name in ("attempt", "completion", "usage")):
        codes = ",".join(sorted({finding["code"] for finding in facts["findings"]}))
        raise ValueError(f"provider wire request facts are not comparable: {codes}")
    rows = facts["rows"]
    if provider_request_ids is not None:
        if (
            not provider_request_ids
            or any(not isinstance(value, str) or not value for value in provider_request_ids)
            or len(set(provider_request_ids)) != len(provider_request_ids)
        ):
            raise ValueError("provider request identity set is invalid")
        rows_by_id = {row["request_id"]: row for row in rows}
        if any(request_id not in rows_by_id for request_id in provider_request_ids):
            raise ValueError("provider boundary request is missing from wire evidence")
        rows = [rows_by_id[request_id] for request_id in provider_request_ids]
    if not rows or any(
        row["terminal_status"] != "response_completed" or row["usage"] is None
        for row in rows
    ):
        raise ValueError("provider wire request did not complete successfully")
    usage_records = [row["usage"] for row in rows]
    aggregate = aggregate_usage_records(usage_records)
    return {
        **aggregate,
        "request_ids": [row["request_id"] for row in rows],
        "provider_payload_sha256": [row["provider_payload_sha256"] for row in rows],
        "request_facts_analyzer_version": facts["analyzer_version"],
    }


def load_provider_wire_usage(
    path: Path, provider_request_ids: list[str] | None = None
) -> dict[str, Any]:
    return parse_provider_wire_usage(
        path.read_text(encoding="utf-8-sig"), provider_request_ids
    )


def validate_cache_artifacts(
    cache: dict[str, Any], request: dict[str, Any]
) -> dict[str, Any]:
    fields = {
        name: _nonnegative_integer(cache.get(name), name)
        for name in (
            "provider_request_count",
            "request_2_plus_count",
            "cache_usage_missing_count",
            "request_2_plus_cached_input_tokens",
            "request_2_plus_uncached_input_tokens",
        )
    }
    input_tokens = _nonnegative_integer(request.get("input_tokens"), "input_tokens")
    cached_tokens = _nonnegative_integer(
        request.get("cached_input_tokens"), "cached_input_tokens"
    )
    output_tokens = _nonnegative_integer(request.get("output_tokens"), "output_tokens")
    if cached_tokens > input_tokens:
        raise ValueError("cached input tokens exceed input tokens")
    if fields["request_2_plus_count"] > fields["provider_request_count"]:
        raise ValueError("request 2+ count exceeds provider request count")
    if fields["cache_usage_missing_count"] > fields["provider_request_count"]:
        raise ValueError("missing usage count exceeds provider request count")

    denominator = (
        fields["request_2_plus_cached_input_tokens"]
        + fields["request_2_plus_uncached_input_tokens"]
    )
    uncached_tokens = input_tokens - cached_tokens
    if fields["request_2_plus_cached_input_tokens"] > cached_tokens:
        raise ValueError("request 2+ cached tokens exceed total cached tokens")
    if fields["request_2_plus_uncached_input_tokens"] > uncached_tokens:
        raise ValueError("request 2+ uncached tokens exceed total uncached tokens")
    if fields["request_2_plus_count"] > 0 and denominator == 0:
        raise ValueError("request 2+ token evidence is missing")
    expected_rate = (
        fields["request_2_plus_cached_input_tokens"] / denominator
        if denominator
        else None
    )
    observed_rate = cache.get("request_2_plus_hit_rate")
    if expected_rate is None:
        if observed_rate is not None:
            raise ValueError("request 2+ hit rate must be null without token evidence")
    elif isinstance(observed_rate, bool) or not isinstance(observed_rate, (int, float)):
        raise ValueError("request 2+ hit rate must be numeric")
    elif not math.isclose(float(observed_rate), expected_rate, abs_tol=0.0000005):
        raise ValueError("request 2+ hit rate does not match token evidence")

    return {
        **fields,
        "request_2_plus_hit_rate": (
            float(observed_rate) if expected_rate is not None else None
        ),
        "input_tokens": input_tokens,
        "cached_input_tokens": cached_tokens,
        "uncached_input_tokens": uncached_tokens,
        "output_tokens": output_tokens,
    }
