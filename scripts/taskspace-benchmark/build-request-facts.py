#!/usr/bin/env python3
"""CLI for the canonical request facts artifact."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from request_facts import build_request_facts


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rollout", type=Path)
    parser.add_argument("--wire", type=Path)
    parser.add_argument("--boundary", type=Path)
    parser.add_argument("--model")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = build_request_facts(args.rollout, args.wire, args.boundary, args.model)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return 0 if all(value != "incomparable" for value in result["availability"].values()) else 3


if __name__ == "__main__":
    raise SystemExit(main())
