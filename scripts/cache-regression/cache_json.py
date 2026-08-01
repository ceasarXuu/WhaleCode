#!/usr/bin/env python3
"""Strict JSON decoding for durable cache evidence."""

from __future__ import annotations

import json
from typing import Any


def strict_json_loads(value: str) -> Any:
    return json.loads(
        value,
        parse_constant=_reject_nonstandard_constant,
        object_pairs_hook=_reject_duplicate_keys,
    )


def _reject_nonstandard_constant(value: str) -> None:
    raise ValueError(f"non-standard JSON constant is forbidden: {value}")


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key is forbidden: {key}")
        result[key] = value
    return result


def exact_json_equal(left: Any, right: Any) -> bool:
    if type(left) is not type(right):
        return False
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(
            exact_json_equal(left[key], right[key]) for key in left
        )
    if isinstance(left, list):
        return len(left) == len(right) and all(
            exact_json_equal(left_item, right_item)
            for left_item, right_item in zip(left, right)
        )
    return left == right
