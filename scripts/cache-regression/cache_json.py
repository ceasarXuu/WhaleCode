#!/usr/bin/env python3
"""Strict JSON decoding for durable cache evidence."""

from __future__ import annotations

import json
from typing import Any


def strict_json_loads(value: str) -> Any:
    return json.loads(value, parse_constant=_reject_nonstandard_constant)


def _reject_nonstandard_constant(value: str) -> None:
    raise ValueError(f"non-standard JSON constant is forbidden: {value}")
