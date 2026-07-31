#!/usr/bin/env python3
"""Deterministic cache regression run-plan and evidence identities."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any


RESULT_SCHEMA_VERSION = "whalecode-cache-hit-regression-v4"


def canonical_json_sha256(value: Any) -> str:
    encoded = json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def file_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()
