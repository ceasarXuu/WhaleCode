#!/usr/bin/env python3
"""Read cache evidence from one explicit Git source."""

from __future__ import annotations

import fnmatch
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

from cache_evidence import canonical_json_sha256
from cache_surface import read_content, tracked_paths


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def relative_path(repo: Path, raw_path: str) -> str:
    require(isinstance(raw_path, str) and raw_path, "evidence path is missing")
    path = Path(raw_path)
    require(not path.is_absolute(), "evidence path must be repository-relative")
    try:
        return (repo / path).resolve().relative_to(repo.resolve()).as_posix()
    except ValueError as error:
        raise ValueError("evidence path escapes repository") from error


def source_bytes(repo: Path, raw_path: str, source: str) -> bytes:
    path = relative_path(repo, raw_path)
    try:
        return read_content(repo, path, source)
    except (OSError, subprocess.CalledProcessError) as error:
        raise ValueError(f"cache evidence is missing: {path}") from error


def source_json(repo: Path, raw_path: str, source: str) -> dict[str, Any]:
    try:
        value = json.loads(source_bytes(repo, raw_path, source).decode("utf-8-sig"))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise ValueError(f"cache evidence is invalid: {raw_path}") from error
    require(isinstance(value, dict), "cache evidence must be an object")
    return value


def source_sha256(repo: Path, raw_path: str, source: str) -> str:
    return hashlib.sha256(source_bytes(repo, raw_path, source)).hexdigest()


def snapshot_payload(content: bytes) -> dict[str, Any]:
    text = content.decode("utf-8")
    separator = "\n---\n"
    require(
        text.startswith("---\n") and separator in text,
        "invalid accepted final-wire snapshot",
    )
    payload = json.loads(text.split(separator, 1)[1])
    require(isinstance(payload, dict), "final-wire snapshot must contain an object")
    return payload


def scenario_id(path: str) -> str:
    return Path(path).name.removesuffix(".snap").rsplit("__", 1)[-1]


def protected_manifest(
    repo: Path, contract: dict[str, Any], source: str
) -> list[dict[str, str]]:
    patterns = contract.get("free_validation", {}).get("semantic_baseline_globs", [])
    paths = sorted(
        path
        for path in tracked_paths(repo, source)
        if any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns)
    )
    manifest = [
        {
            "scenario_id": scenario_id(path),
            "baseline_path": path,
            "payload_sha256": canonical_json_sha256(
                snapshot_payload(source_bytes(repo, path, source))
            ),
        }
        for path in paths
    ]
    ids = [item["scenario_id"] for item in manifest]
    require(manifest and len(ids) == len(set(ids)), "protected scenarios are invalid")
    return manifest
