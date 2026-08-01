#!/usr/bin/env python3
"""Compare normalized TUI runs by stable test identity and result."""

from __future__ import annotations

import argparse
import hashlib
import json
import logging
import sys
from pathlib import Path

from tui_baseline import compare_runs


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("runs", nargs="+", type=Path)
    args = parser.parse_args()
    logging.basicConfig(level=logging.INFO)
    if len(args.runs) < 2:
        parser.error("provide at least two normalized run files")
    documents: list[dict] = []
    for path in args.runs:
        content = path.read_bytes()
        documents.append(json.loads(content))
        logging.info("%s sha256=%s", path, hashlib.sha256(content).hexdigest())
    drift = compare_runs(documents)
    if drift:
        for name in drift:
            logging.error("flaky candidate: %s", name)
        logging.error("TUI run comparison found %d drifting test(s)", len(drift))
        return 1
    logging.info("TUI run comparison is stable across %d runs", len(documents))
    return 0


if __name__ == "__main__":
    sys.exit(main())
