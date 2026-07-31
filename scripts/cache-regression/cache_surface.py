#!/usr/bin/env python3
"""Shared cache-sensitive surface hashing and contract helpers."""

from __future__ import annotations

import fnmatch
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any


CACHE_CONTROL_PLANE_EXACT_PATHS = frozenset(
    {
        ".githooks/pre-commit",
        "benchmarks/cache-regression/final-wire-comparison-policy.json",
        "scripts/taskspace-benchmark/build-v005-non-agent-gates.ps1",
    }
)
CACHE_CONTROL_PLANE_GLOBS = (
    "third_party/codex-cli/codex-rs/core/tests/common/cache_payload.rs",
    "third_party/codex-cli/codex-rs/core/tests/suite/cache_final_wire.rs",
    "third_party/codex-cli/codex-rs/core/tests/suite/cache_payload*_contract.rs",
)


def run_git(repo: Path, *args: str, text: bool = False) -> bytes | str:
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        check=True,
        capture_output=True,
        text=text,
    )
    return result.stdout


def parse_contract(content: str) -> dict[str, Any]:
    value = json.loads(content)
    if value.get("schema_version") != "whalecode-cache-surface-v1":
        raise ValueError("unsupported cache surface contract")
    return value


def semantic_baseline_changes(
    repo: Path, contract: dict[str, Any], source: str
) -> list[str]:
    free_validation = contract.get("free_validation")
    if not isinstance(free_validation, dict):
        return []
    patterns = free_validation.get("semantic_baseline_globs", [])
    return sorted(
        path
        for path in changed_paths(repo, source)
        if any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns)
    )


def changed_paths_match_worktree(repo: Path, paths: list[str], source: str) -> bool:
    if source != "index" or not paths:
        return True
    raw = run_git(repo, "diff", "--name-only", "-z", "--", *paths)
    return not any(raw.split(b"\0"))


def load_contract(path: Path) -> dict[str, Any]:
    return parse_contract(path.read_text(encoding="utf-8"))


def repository_relative_path(repo: Path, path: Path) -> str:
    repo = repo.resolve()
    absolute_path = path if path.is_absolute() else repo / path
    try:
        return absolute_path.resolve().relative_to(repo).as_posix()
    except ValueError as error:
        raise ValueError(
            "cache surface contract must be inside the repository"
        ) from error


def load_contract_from_source(repo: Path, path: Path, source: str) -> dict[str, Any]:
    relative_path = repository_relative_path(repo, path)
    content = read_content(repo, relative_path, source)
    return parse_contract(content.decode("utf-8"))


def source_matches_worktree(repo: Path, path: Path, source: str) -> bool:
    relative_path = repository_relative_path(repo, path)
    return read_content(repo, relative_path, source) == read_content(
        repo, relative_path, "worktree"
    )


def changed_paths(repo: Path, source: str) -> list[str]:
    if source == "index":
        raw = run_git(
            repo, "diff", "--cached", "--name-only", "--diff-filter=ACMRD", "-z", "HEAD"
        )
    elif source == "worktree":
        raw = run_git(repo, "diff", "--name-only", "--diff-filter=ACMRD", "-z", "HEAD")
    elif source == "head":
        return []
    else:
        raise ValueError(f"unknown source: {source}")
    return sorted(item.decode() for item in raw.split(b"\0") if item)


def is_cache_control_plane_path(path: str) -> bool:
    if path in CACHE_CONTROL_PLANE_EXACT_PATHS:
        return True
    if any(fnmatch.fnmatchcase(path, pattern) for pattern in CACHE_CONTROL_PLANE_GLOBS):
        return True
    candidate = Path(path)
    return (
        candidate.parent.as_posix() == "scripts/cache-regression"
        and candidate.suffix in {".py", ".ps1"}
        and not candidate.name.startswith("test_")
    )


def control_plane_change_summary(
    repo: Path,
    contract_path: Path,
    source: str,
    contract: dict[str, Any],
) -> dict[str, Any]:
    paths = changed_paths(repo, source)
    policy_changes = sorted(path for path in paths if is_cache_control_plane_path(path))
    contract_relative_path = repository_relative_path(repo, contract_path)
    contract_policy_changed = False
    baseline_changed = False
    if contract_relative_path in paths:
        previous = load_contract_from_source(repo, contract_path, "head")
        contract_policy_changed = {
            key: value for key, value in contract.items() if key != "baseline"
        } != {key: value for key, value in previous.items() if key != "baseline"}
        baseline_changed = contract.get("baseline") != previous.get("baseline")
        if contract_policy_changed:
            policy_changes.append(f"{contract_relative_path}#policy")
    return {
        "policy_changes": policy_changes,
        "contract_policy_changed": contract_policy_changed,
        "baseline_changed": baseline_changed,
    }


def release_relevant_changes(
    repo: Path, contract_path: Path, contract: dict[str, Any]
) -> list[dict[str, str]]:
    contract_relative_path = repository_relative_path(repo, contract_path)
    baseline_patterns = contract.get("free_validation", {}).get(
        "semantic_baseline_globs", []
    )
    untracked_raw = run_git(repo, "ls-files", "--others", "--exclude-standard", "-z")
    candidates = [(path, "tracked") for path in changed_paths(repo, "worktree")] + [
        (item.decode(), "untracked") for item in untracked_raw.split(b"\0") if item
    ]
    relevant = []
    for path, state in candidates:
        if (
            path == contract_relative_path
            or is_cache_control_plane_path(path)
            or matching_rules(path, contract)
            or any(fnmatch.fnmatchcase(path, pattern) for pattern in baseline_patterns)
        ):
            relevant.append({"path": path, "state": state})
    return sorted(relevant, key=lambda item: (item["path"], item["state"]))


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
    raw = run_git(repo, "diff", "--cached", "--name-only", "--diff-filter=ACMRD", "-z")
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


def sensitive_changes(
    repo: Path, contract: dict[str, Any], source: str
) -> list[dict[str, Any]]:
    changes = []
    for path in changed_paths(repo, source):
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
