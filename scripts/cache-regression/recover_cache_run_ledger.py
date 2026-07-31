#!/usr/bin/env python3
"""Idempotently settle a claimed cache-run ledger entry from a durable result."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from cache_evidence import RESULT_SCHEMA_VERSION
from cache_run_ledger import settle_entry, store_entry


def read_json(path: Path) -> dict:
    value = json.loads(path.read_text(encoding="utf-8-sig"))
    if not isinstance(value, dict):
        raise ValueError("cache recovery input must be an object")
    return value


def recover(repo: Path, ledger_path: Path, result_path: Path) -> str:
    result = read_json(result_path)
    if result.get("schema_version") != RESULT_SCHEMA_VERSION:
        raise ValueError("unsupported cache result schema")
    required = ("record_id", "started_at", "ended_at", "elapsed_seconds", "result_path")
    if any(result.get(key) is None for key in required):
        raise ValueError("cache result lacks settlement fields")
    expected_path = result_path.resolve().relative_to(repo).as_posix()
    if result["result_path"] != expected_path:
        raise ValueError("cache result path does not match recovery input")
    ledger = read_json(ledger_path)
    matches = [
        item
        for item in ledger.get("entries", [])
        if item.get("record_id") == result["record_id"]
    ]
    if len(matches) != 1:
        raise ValueError("cache recovery requires one claimed ledger record")
    entry = matches[0]
    if (
        entry.get("status") == "settled"
        and entry.get("evidence", {}).get("result_path") == result["result_path"]
    ):
        return "already_settled"
    if entry.get("status") not in {"planned", "running", "failed", "cancelled"}:
        raise ValueError("cache ledger record cannot be recovered")
    settle_entry(entry, result)
    store_entry(ledger_path, entry)
    return "settled"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("result", type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--ledger",
        type=Path,
        default=Path("benchmarks/whale-agent-run-ledger.json"),
    )
    args = parser.parse_args()
    repo = args.repo_root.resolve()
    result_path = (
        args.result if args.result.is_absolute() else repo / args.result
    ).resolve()
    ledger_path = (
        args.ledger if args.ledger.is_absolute() else repo / args.ledger
    ).resolve()
    try:
        status = recover(repo, ledger_path, result_path)
    except (KeyError, OSError, TypeError, ValueError) as error:
        raise SystemExit(str(error)) from error
    print(f"cache ledger recovery: {status}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
