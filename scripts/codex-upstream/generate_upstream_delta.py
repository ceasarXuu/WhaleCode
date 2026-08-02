#!/usr/bin/env python3
"""Generate the deterministic upstream baseline-to-candidate delta inventory."""

from __future__ import annotations

import argparse
import json
import logging
import sys
from collections import Counter
from pathlib import Path

from generate_overlay_inventory import BASELINE, TARGET
from git_snapshot import diff_stats, list_tree, read_blobs, resolve_tree, sha256
from metadata_contract import validate_upstream_delta

OUTPUT_PATH = "docs/v0.0.5/codex-upstream-sync/upstream-delta-inventory.json"


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def crate_owner(path: str) -> str:
    parts = path.split("/")
    if parts[0] != "codex-rs":
        return parts[0]
    if len(parts) < 2 or parts[1] in {
        "Cargo.lock",
        "Cargo.toml",
        "clippy.toml",
        "rust-toolchain.toml",
        "rustfmt.toml",
    }:
        return "codex-rs-workspace"
    return parts[1]


def generated_kind(path: str) -> str | None:
    if path.startswith("codex-rs/app-server-protocol/schema/json/"):
        return "app-server-json-schema"
    if path.startswith("codex-rs/app-server-protocol/schema/typescript/"):
        return "app-server-typescript"
    if path == "codex-rs/core/config.schema.json":
        return "config-schema"
    if "/snapshots/" in path or path.endswith((".snap", ".snap.new")):
        return "insta-snapshot"
    if path == "codex-rs/Cargo.lock":
        return "cargo-lock"
    return None


def build_delta(repo: Path) -> dict:
    baseline_tree = resolve_tree(repo, BASELINE)
    target_tree = resolve_tree(repo, TARGET)
    baseline_entries = list_tree(repo, baseline_tree)
    target_entries = list_tree(repo, target_tree)
    stats = diff_stats(repo, baseline_tree, target_tree)
    required_oids = {
        entry.oid
        for path in stats
        for entry in (baseline_entries.get(path), target_entries.get(path))
        if entry is not None and entry.object_type == "blob"
    }
    blobs = read_blobs(repo, required_oids)
    entries: list[dict] = []
    for path, stat in stats.items():
        before = baseline_entries.get(path)
        after = target_entries.get(path)
        entries.append(
            {
                "additions": stat.additions,
                "baseline_sha256": sha256(blobs[before.oid]) if before else None,
                "binary": stat.binary,
                "crate_owner": crate_owner(path),
                "deletions": stat.deletions,
                "generated_kind": generated_kind(path),
                "path": path,
                "status": stat.status,
                "target_sha256": sha256(blobs[after.oid]) if after else None,
            }
        )
    statuses = Counter(entry["status"] for entry in entries)
    owners = Counter(entry["crate_owner"] for entry in entries)
    generated = Counter(
        entry["generated_kind"]
        for entry in entries
        if entry["generated_kind"] is not None
    )
    return {
        "schema_version": 1,
        "baseline_commit": BASELINE,
        "baseline_tree": baseline_tree,
        "target_commit": TARGET,
        "target_tree": target_tree,
        "source": "git-tree",
        "entries": entries,
        "summary": {
            "path_count": len(entries),
            "by_status": dict(sorted(statuses.items())),
            "by_crate_owner": dict(sorted(owners.items())),
            "by_generated_kind": dict(sorted(generated.items())),
        },
    }


def render(document: dict) -> str:
    return json.dumps(document, indent=2) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    logging.basicConfig(level=logging.INFO)
    repo = _repo_root()
    try:
        document = build_delta(repo)
        errors = validate_upstream_delta(document)
        if errors:
            for error in errors:
                logging.error("%s", error)
            return 1
        output = repo / OUTPUT_PATH
        rendered = render(document)
        if args.write:
            output.write_text(rendered, encoding="utf-8")
            logging.info("wrote %s with %d paths", OUTPUT_PATH, len(document["entries"]))
            return 0
        if not output.is_file() or output.read_text(encoding="utf-8") != rendered:
            logging.error("upstream delta inventory is missing or stale")
            return 1
        logging.info("upstream delta inventory is current")
        return 0
    except (OSError, RuntimeError, ValueError) as error:
        logging.error("upstream delta generation failed: %s", error)
        return 2


if __name__ == "__main__":
    sys.exit(main())
