#!/usr/bin/env python3
"""Fail when production scripts read request evidence without inventory."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


SOURCE_PATTERNS = {
    "boundary": re.compile(r"provider_request_claimed"),
    "local_attempt": re.compile(r"payload_captured"),
    "rollout_token": re.compile(r"(?:last_token_usage|token_count|TokenCount)"),
    "terminal": re.compile(
        r"(?:response_completed|response_failed|response_cancelled|retry_unauthorized)"
    ),
}
SCAN_ROOTS = ("scripts/taskspace-benchmark", "scripts/cache-regression")


def _is_production_script(path: Path) -> bool:
    name = path.name
    return (
        path.suffix in {".py", ".ps1"}
        and name != "check-request-fact-consumers.py"
        and not name.startswith("test")
        and not name.endswith("_test_support.py")
        and "fixtures" not in path.parts
    )


def discover(repo_root: Path) -> dict[str, set[str]]:
    found: dict[str, set[str]] = {}
    for relative_root in SCAN_ROOTS:
        root = repo_root / relative_root
        for path in root.rglob("*"):
            if not path.is_file() or not _is_production_script(path):
                continue
            text = path.read_text(encoding="utf-8-sig")
            sources = {
                source for source, pattern in SOURCE_PATTERNS.items() if pattern.search(text)
            }
            if sources:
                found[path.relative_to(repo_root).as_posix()] = sources
    return found


def load_inventory(path: Path) -> dict[str, set[str]]:
    raw: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
    if raw.get("schema_version") != "whalecode-request-fact-consumers-v1":
        raise ValueError("unsupported request fact consumer inventory")
    entries: dict[str, set[str]] = {}
    for item in raw.get("consumers", []):
        item_path = item.get("path")
        sources = item.get("observed_sources")
        if not isinstance(item_path, str) or not isinstance(sources, list):
            raise ValueError("invalid request fact consumer entry")
        if item_path in entries:
            raise ValueError(f"duplicate request fact consumer: {item_path}")
        unknown = set(sources) - set(SOURCE_PATTERNS)
        if unknown:
            raise ValueError(f"unknown observed source for {item_path}: {sorted(unknown)}")
        entries[item_path] = set(sources)
    return entries


def compare(discovered: dict[str, set[str]], inventory: dict[str, set[str]]) -> list[str]:
    errors = []
    for path in sorted(set(discovered) | set(inventory)):
        actual = discovered.get(path)
        expected = inventory.get(path)
        if actual is None:
            errors.append(f"stale inventory entry: {path}")
        elif expected is None:
            errors.append(f"unclassified request fact reader: {path} {sorted(actual)}")
        elif actual != expected:
            errors.append(
                f"request fact source drift: {path} expected={sorted(expected)} "
                f"actual={sorted(actual)}"
            )
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument(
        "--inventory",
        type=Path,
        default=Path(__file__).with_name("request-fact-consumers.json"),
    )
    args = parser.parse_args()
    errors = compare(discover(args.repo_root.resolve()), load_inventory(args.inventory))
    if errors:
        for error in errors:
            print(f"request fact consumer gate: ERROR {error}")
        return 1
    print("request fact consumer gate: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
