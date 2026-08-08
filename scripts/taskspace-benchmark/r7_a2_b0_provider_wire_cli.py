#!/usr/bin/env python3
"""Fail-closed CLI boundary for the paid provider-wire probe."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import r7_a2_b0_provider_wire_probe as probe


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "scripts/workspace-safety"))

from workspace_entrypoint import WorkspacePreflightError, require_ready


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", default="deepseek-v4-flash")
    parser.add_argument(
        "--endpoint", default="https://api.deepseek.com/chat/completions"
    )
    parser.add_argument("--repeat", type=int, default=3)
    parser.add_argument("--output", required=True)
    parser.add_argument("--raw-dir", required=True)
    parser.add_argument("--repo-commit", default="")
    args = parser.parse_args()
    if args.repeat < 1:
        parser.error("--repeat must be >= 1")
    return args


def main() -> int:
    args = parse_args()
    try:
        require_ready(REPO_ROOT)
    except WorkspacePreflightError as exc:
        raise SystemExit(str(exc)) from exc
    raise SystemExit("run_ledger_authorization_unavailable")


if __name__ == "__main__":
    raise SystemExit(main())
