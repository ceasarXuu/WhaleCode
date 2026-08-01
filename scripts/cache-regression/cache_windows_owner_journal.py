#!/usr/bin/env python3
"""Durable identity records for suspended Windows benchmark processes."""

from pathlib import Path

from cache_json import strict_json_loads
from cache_run_ledger import atomic_write_json


SCHEMA_VERSION = "whalecode-windows-process-owner-v1"


def owner_directory(cwd: Path) -> Path:
    return cwd / "target/cache-hit-regression/windows-process-owners"


def write_owner_journal(cwd: Path, process_id: int, creation_time: int) -> Path:
    path = owner_directory(cwd) / f"{process_id}-{creation_time}.json"
    atomic_write_json(
        path,
        {
            "schema_version": SCHEMA_VERSION,
            "pid": process_id,
            "creation_time": creation_time,
            "state": "suspended_pre_job",
        },
    )
    return path


def remove_owner_journal(path: Path | None) -> None:
    if path is not None and path.exists():
        path.unlink()


def owner_records(cwd: Path) -> list[tuple[Path, int, int]]:
    directory = owner_directory(cwd)
    if not directory.is_dir():
        return []
    records = []
    for path in sorted(directory.glob("*.json")):
        value = strict_json_loads(path.read_text(encoding="utf-8"))
        process_id = value.get("pid")
        creation_time = value.get("creation_time")
        if (
            value.get("schema_version") != SCHEMA_VERSION
            or type(process_id) is not int
            or process_id <= 0
            or type(creation_time) is not int
            or creation_time < 0
        ):
            raise ValueError(f"invalid Windows process owner journal: {path}")
        records.append((path, process_id, creation_time))
    return records
