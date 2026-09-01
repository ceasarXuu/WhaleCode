#!/usr/bin/env python3
"""Generate/check the current Whale overlay on the imported Codex substrate."""

from __future__ import annotations

import argparse
import logging
import sys
from pathlib import Path

from generate_overlay_inventory import build_inventory, render
from metadata_contract import validate_inventory
from qualify_candidate import CANDIDATE_TARGET

OUTPUT_PATH = "docs/releases/v0.0.7/codex-upstream-sync/current-overlay-inventory.json"


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def build_current_inventory(repo: Path) -> dict:
    return build_inventory(
        repo,
        baseline=CANDIDATE_TARGET,
        target=CANDIDATE_TARGET,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    logging.basicConfig(level=logging.INFO)
    repo = _repo_root()
    try:
        document = build_current_inventory(repo)
        errors = validate_inventory(document)
        if errors:
            for error in errors:
                logging.error("%s", error)
            return 1
        path = repo / OUTPUT_PATH
        rendered = render(document)
        if args.check:
            if not path.is_file() or path.read_text(encoding="utf-8") != rendered:
                logging.error("current overlay inventory is missing or stale: %s", path)
                return 1
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(rendered, encoding="utf-8")
        logging.info(
            "current overlay inventory: %d paths on %s",
            document["summary"]["path_count"],
            CANDIDATE_TARGET,
        )
        return 0
    except (OSError, RuntimeError, ValueError) as error:
        logging.error("current overlay generation failed: %s", error)
        return 2


if __name__ == "__main__":
    sys.exit(main())
