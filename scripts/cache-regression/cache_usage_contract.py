#!/usr/bin/env python3
"""Shared provider usage fixtures and cache aggregation invariants."""

from __future__ import annotations

import json
import math
from pathlib import Path
from typing import Any

from cache_json import strict_json_loads


SCHEMA_VERSION = "whalecode-provider-usage-v1"
PROVIDER_WIRE_SCHEMA_VERSION = "provider-chat-wire-trace-v10"
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


def parse_provider_wire_usage(text: str) -> dict[str, Any]:
    attempts: dict[str, dict[str, Any]] = {}
    request_order: list[str] = []
    for line_number, raw_line in enumerate(text.splitlines(), 1):
        if not raw_line.strip():
            continue
        event = strict_json_loads(raw_line)
        if not isinstance(event, dict):
            raise ValueError(f"provider wire event {line_number} must be an object")
        if event.get("schema_version") != PROVIDER_WIRE_SCHEMA_VERSION:
            raise ValueError("provider wire usage schema is unsupported")
        request_id = event.get("request_id")
        if not isinstance(request_id, str) or not request_id:
            raise ValueError("provider wire request id is missing")
        attempt = attempts.setdefault(request_id, {})
        status = event.get("status")
        if status == "payload_captured":
            if "payload" in attempt:
                raise ValueError("provider wire request has duplicate payload evidence")
            request_index = _nonnegative_integer(
                event.get("request_index"), "request_index"
            )
            digest = event.get("provider_payload_sha256")
            if (
                request_index < 1
                or not isinstance(digest, str)
                or len(digest) != 64
                or any(character not in "0123456789abcdef" for character in digest)
            ):
                raise ValueError("provider wire payload evidence is invalid")
            attempt["payload"] = {
                "request_index": request_index,
                "provider_payload_sha256": digest,
            }
            request_order.append(request_id)
        elif status in {"response_completed", "response_failed", "cancelled"}:
            if "terminal" in attempt:
                raise ValueError(
                    "provider wire request has duplicate terminal evidence"
                )
            attempt["terminal"] = event

    if not request_order:
        raise ValueError("provider wire usage requires at least one request")
    if len(request_order) != len(set(request_order)):
        raise ValueError("provider wire request order contains duplicates")

    usage_records: list[dict[str, int] | None] = []
    payload_hashes: list[str] = []
    for expected_index, request_id in enumerate(request_order, 1):
        attempt = attempts[request_id]
        payload = attempt.get("payload")
        terminal = attempt.get("terminal")
        if not isinstance(payload, dict) or not isinstance(terminal, dict):
            raise ValueError("provider wire request evidence is incomplete")
        if payload["request_index"] != expected_index:
            raise ValueError("provider wire request order is invalid")
        if terminal.get("status") != "response_completed":
            raise ValueError("provider wire request did not complete successfully")
        usage_records.append(
            {
                field: _nonnegative_integer(terminal.get(field), field)
                for field in TOKEN_FIELDS
            }
        )
        payload_hashes.append(payload["provider_payload_sha256"])

    aggregate = aggregate_usage_records(usage_records)
    return {
        **aggregate,
        "request_ids": request_order,
        "provider_payload_sha256": payload_hashes,
    }


def load_provider_wire_usage(path: Path) -> dict[str, Any]:
    return parse_provider_wire_usage(path.read_text(encoding="utf-8-sig"))


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
