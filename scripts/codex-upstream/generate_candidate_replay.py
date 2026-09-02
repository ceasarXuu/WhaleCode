#!/usr/bin/env python3
"""Generate/check the v0.0.7 replay contract against Codex 0.151."""

from __future__ import annotations

import argparse
import json
import logging
import os
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

from generate_overlay_inventory import BASELINE, VENDOR_PATH, build_inventory
from generate_replay_ledger import build_ledger
from generate_upstream_delta import build_delta
from metadata_contract import (
    validate_inventory,
    validate_replay_ledger,
    validate_upstream_delta,
)
from qualify_candidate import CANDIDATE_TARGET

OUTPUT_DIR = "docs/releases/v0.0.7/codex-upstream-sync"
ARTIFACTS = {
    "overlay-inventory.json": "overlay",
    "upstream-delta-inventory.json": "delta",
    "overlay-replay-ledger.json": "replay",
    "conflict-ledger.json": "conflicts",
}


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _run(
    repo: Path, *args: str, input_bytes: bytes | None = None
) -> subprocess.CompletedProcess[bytes]:
    env = os.environ.copy()
    env["LC_ALL"] = "C"
    return subprocess.run(
        args,
        cwd=repo,
        env=env,
        input=input_bytes,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )


def parse_apply_failures(output: str) -> dict[str, str]:
    prefix = re.escape(f"{VENDOR_PATH}/")
    conflicts = re.findall(rf"Applied patch to '{prefix}(.+)' with conflicts\.", output)
    failed = re.findall(rf"error: patch failed: {prefix}([^:]+):", output)
    absent = re.findall(rf"error: {prefix}([^:]+): does not exist in index", output)
    result = {path: "three-way-conflict" for path in conflicts}
    result.update({path: "patch-apply-failure" for path in failed})
    result.update({path: "index-path-absent" for path in absent})
    return result


def apply_failure_paths(repo: Path) -> dict[str, str]:
    patch = _run(repo, "git", "diff", "--binary", BASELINE, CANDIDATE_TARGET)
    if patch.returncode != 0:
        raise RuntimeError(patch.stdout.decode("utf-8", "replace"))
    applied = _run(
        repo,
        "git",
        "apply",
        "--check",
        "--3way",
        f"--directory={VENDOR_PATH}",
        "-",
        input_bytes=patch.stdout,
    )
    return parse_apply_failures(applied.stdout.decode("utf-8", "replace"))


def build_conflict_ledger(replay: dict, failures: dict[str, str]) -> dict:
    entries = []
    for entry in replay["entries"]:
        if entry["upstream_status"] == "unchanged":
            continue
        status = failures.get(entry["path"], "overlap-clean")
        entries.append(
            {
                "apply_status": status,
                "cutover_batch": entry["cutover_batch"],
                "disposition": entry["disposition"],
                "owner_domain": entry["owner_domain"],
                "path": entry["path"],
                "verification": entry["verification"],
            }
        )
    statuses = Counter(entry["apply_status"] for entry in entries)
    batches = Counter(entry["cutover_batch"] for entry in entries)
    return {
        "schema_version": 1,
        "baseline_commit": BASELINE,
        "target_commit": CANDIDATE_TARGET,
        "source": "git-apply-check-3way-against-index",
        "entries": entries,
        "summary": {
            "path_count": len(entries),
            "by_apply_status": dict(sorted(statuses.items())),
            "by_cutover_batch": dict(sorted(batches.items())),
        },
    }


def build_documents(repo: Path) -> dict[str, dict]:
    overlay = build_inventory(repo, baseline=BASELINE, target=CANDIDATE_TARGET)
    delta = build_delta(repo, baseline=BASELINE, target=CANDIDATE_TARGET)
    replay = build_ledger(
        repo,
        overlay=overlay,
        delta=delta,
        baseline=BASELINE,
        target=CANDIDATE_TARGET,
    )
    conflicts = build_conflict_ledger(replay, apply_failure_paths(repo))
    return {
        "overlay": overlay,
        "delta": delta,
        "replay": replay,
        "conflicts": conflicts,
    }


def validate_documents(documents: dict[str, dict]) -> list[str]:
    errors = []
    errors.extend(validate_inventory(documents["overlay"]))
    errors.extend(validate_upstream_delta(documents["delta"]))
    errors.extend(validate_replay_ledger(documents["replay"]))
    overlap = {
        entry["path"]
        for entry in documents["replay"]["entries"]
        if entry["upstream_status"] != "unchanged"
    }
    recorded = {entry["path"] for entry in documents["conflicts"]["entries"]}
    if overlap != recorded:
        errors.append("conflict ledger does not exactly cover changed-path overlap")
    return errors


def render(document: dict) -> str:
    return json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    logging.basicConfig(level=logging.INFO)
    repo = _repo_root()
    try:
        documents = build_documents(repo)
        errors = validate_documents(documents)
        if errors:
            for error in errors:
                logging.error("%s", error)
            return 1
        output_dir = repo / OUTPUT_DIR
        output_dir.mkdir(parents=True, exist_ok=True)
        for filename, key in ARTIFACTS.items():
            path = output_dir / filename
            rendered = render(documents[key])
            if args.check and (
                not path.is_file() or path.read_text(encoding="utf-8") != rendered
            ):
                logging.error("candidate replay artifact is missing or stale: %s", path)
                return 1
            if args.write:
                path.write_text(rendered, encoding="utf-8")
        logging.info(
            "candidate replay current: overlay=%d delta=%d overlap=%d conflicts=%d",
            documents["overlay"]["summary"]["path_count"],
            documents["delta"]["summary"]["path_count"],
            documents["conflicts"]["summary"]["path_count"],
            documents["conflicts"]["summary"]["by_apply_status"].get(
                "three-way-conflict", 0
            ),
        )
        return 0
    except (OSError, RuntimeError, ValueError) as error:
        logging.error("candidate replay generation failed: %s", error)
        return 2


if __name__ == "__main__":
    sys.exit(main())
