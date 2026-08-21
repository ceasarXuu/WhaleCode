#!/usr/bin/env python3
"""Require every root GitHub Actions workflow to be manually dispatched."""

import re
import sys
from pathlib import Path


EVENT_RE = re.compile(r"^  ([A-Za-z_][A-Za-z0-9_-]*):(?:\s.*)?$")


def workflow_events(path: Path) -> set[str]:
    lines = path.read_text(encoding="utf-8").splitlines()
    on_indexes = [index for index, line in enumerate(lines) if line.startswith("on:")]
    if len(on_indexes) != 1 or lines[on_indexes[0]].strip() != "on:":
        raise ValueError(f"{path.name}: trigger block must use a single top-level 'on:'")

    events: set[str] = set()
    for line in lines[on_indexes[0] + 1 :]:
        if line and not line.startswith((" ", "\t", "#")):
            break
        match = EVENT_RE.match(line)
        if match:
            events.add(match.group(1))
    return events


def validate(root: Path) -> None:
    workflow_dir = root / ".github/workflows"
    workflows = sorted((*workflow_dir.glob("*.yml"), *workflow_dir.glob("*.yaml")))
    if not workflows:
        raise ValueError("no root GitHub Actions workflows found")

    errors = []
    for path in workflows:
        try:
            events = workflow_events(path)
        except ValueError as exc:
            errors.append(str(exc))
            continue
        if events != {"workflow_dispatch"}:
            errors.append(
                f"{path.name}: only workflow_dispatch is allowed, found {sorted(events)}"
            )
    if errors:
        raise ValueError("\n".join(f"- {error}" for error in errors))


if __name__ == "__main__":
    try:
        validate(Path(__file__).resolve().parents[2])
    except (OSError, ValueError) as exc:
        print(f"manual Actions gate FAILED:\n{exc}", file=sys.stderr)
        raise SystemExit(1)
    print("manual Actions gate OK: all root workflows require workflow_dispatch")
