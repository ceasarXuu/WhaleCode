#!/usr/bin/env python3
"""Shared cache-sensitive surface hashing and contract helpers."""

from __future__ import annotations

import fnmatch
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any


def run_git(repo: Path, *args: str, text: bool = False) -> bytes | str:
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        check=True,
        capture_output=True,
        text=text,
    )
    return result.stdout


def load_contract(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if value.get("schema_version") != "whalecode-cache-surface-v1":
        raise ValueError("unsupported cache surface contract")
    return value


def matching_rules(path: str, contract: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        rule
        for rule in contract["surface_rules"]
        if any(fnmatch.fnmatchcase(path, pattern) for pattern in rule["globs"])
    ]


def tracked_paths(repo: Path, source: str) -> list[str]:
    if source == "index":
        raw = run_git(repo, "ls-files", "-z")
    elif source == "head":
        raw = run_git(repo, "ls-tree", "-r", "--name-only", "-z", "HEAD")
    elif source == "worktree":
        raw = run_git(repo, "ls-files", "-z")
    else:
        raise ValueError(f"unknown source: {source}")
    return sorted(item.decode() for item in raw.split(b"\0") if item)


def read_content(repo: Path, path: str, source: str) -> bytes:
    if source == "index":
        return run_git(repo, "show", f":{path}")
    if source == "head":
        return run_git(repo, "show", f"HEAD:{path}")
    return (repo / path).read_bytes()


def surface_snapshot(
    repo: Path, contract: dict[str, Any], source: str
) -> tuple[str, list[dict[str, Any]]]:
    entries: list[dict[str, Any]] = []
    digest = hashlib.sha256()
    for path in tracked_paths(repo, source):
        rules = matching_rules(path, contract)
        if not rules:
            continue
        content = read_content(repo, path, source)
        content_sha = hashlib.sha256(content).hexdigest()
        entries.append(
            {
                "path": path,
                "content_sha256": content_sha,
                "rule_ids": [rule["id"] for rule in rules],
            }
        )
        digest.update(path.encode())
        digest.update(b"\0")
        digest.update(content_sha.encode())
        digest.update(b"\n")
    return digest.hexdigest(), entries


def staged_sensitive_changes(
    repo: Path, contract: dict[str, Any]
) -> list[dict[str, Any]]:
    raw = run_git(
        repo, "diff", "--cached", "--name-only", "--diff-filter=ACMRD", "-z"
    )
    changes = []
    for item in raw.split(b"\0"):
        if not item:
            continue
        path = item.decode()
        rules = matching_rules(path, contract)
        if rules:
            changes.append(
                {
                    "path": path,
                    "rules": [
                        {"id": rule["id"], "reason": rule["reason"]} for rule in rules
                    ],
                }
            )
    return changes


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
