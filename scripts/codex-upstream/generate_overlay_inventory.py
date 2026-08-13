#!/usr/bin/env python3
"""Generate or check the deterministic Codex vendor overlay inventory."""

from __future__ import annotations

import argparse
import json
import logging
import sys
from collections import Counter
from pathlib import Path

from classification import classify
from git_snapshot import (
    GitError,
    diff_stats,
    evidence_commits,
    index_subtree,
    list_tree,
    read_blobs,
    resolve_commit,
    resolve_tree,
    sha256,
)

IMPORT_BASELINE = "fed0a8f4faa58db3138488cca77628c1d54a2cd8"
BASELINE = "be6e8eac029b183056b7e4402879f15d2c85f61b"
TARGET = "be6e8eac029b183056b7e4402879f15d2c85f61b"
IMPORT_COMMIT = "8991de2843679e0bbdbb6dc243e632e445cf109d"
VENDOR_PATH = "third_party/codex-cli"
OUTPUT_PATH = "docs/v0.0.5/codex-upstream-sync/overlay-inventory.json"
LEDGER_PATH = "docs/v0.0.5/codex-upstream-sync/backport-ledger.json"
MAX_EVIDENCE_COMMITS = 20
EXCLUDED_CONTROL_PATHS = ("UPSTREAM.md",)


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _backport_paths(repo: Path) -> set[str]:
    ledger = json.loads((repo / LEDGER_PATH).read_text(encoding="utf-8"))
    return {path for entry in ledger["entries"] for path in entry["paths"]}


def build_inventory(repo: Path) -> dict:
    baseline_commit = resolve_commit(repo, BASELINE)
    target_commit = resolve_commit(repo, TARGET)
    head_commit = resolve_commit(repo, "HEAD")
    baseline_tree = resolve_tree(repo, baseline_commit)
    current_tree = index_subtree(repo, VENDOR_PATH)
    baseline_entries = list_tree(repo, baseline_tree)
    current_entries = list_tree(repo, current_tree)
    all_stats = diff_stats(repo, baseline_tree, current_tree)
    stats = {
        path: stat
        for path, stat in all_stats.items()
        if path not in EXCLUDED_CONTROL_PATHS
    }
    changed_oids = {
        entry.oid
        for path in stats
        for entry in (baseline_entries.get(path), current_entries.get(path))
        if entry is not None and entry.object_type == "blob"
    }
    blobs = read_blobs(repo, changed_oids)
    evidence = evidence_commits(repo, IMPORT_COMMIT, head_commit, VENDOR_PATH)
    ledger_paths = _backport_paths(repo)
    entries: list[dict] = []
    status_counts: Counter[str] = Counter()
    category_counts: Counter[str] = Counter()
    for path, stat in stats.items():
        baseline_entry = baseline_entries.get(path)
        current_entry = current_entries.get(path)
        before = blobs.get(baseline_entry.oid) if baseline_entry else None
        after = blobs.get(current_entry.oid) if current_entry else None
        classification = classify(path, before, after, ledger_paths)
        commits = evidence.get(path, [])
        entry = {
            "path": path,
            "status": stat.status,
            "baseline_sha256": sha256(before),
            "current_sha256": sha256(after),
            "additions": stat.additions,
            "deletions": stat.deletions,
            "binary": stat.binary,
            "categories": list(classification.categories),
            "matched_rule_ids": list(classification.rule_ids),
            "evidence_commits": commits[:MAX_EVIDENCE_COMMITS],
            "evidence_commit_count": len(commits),
            "evidence_truncated": len(commits) > MAX_EVIDENCE_COMMITS,
        }
        entries.append(entry)
        status_counts[stat.status] += 1
        category_counts.update(classification.categories)
    return {
        "schema_version": 1,
        "vendor_path": VENDOR_PATH,
        "baseline_commit": baseline_commit,
        "baseline_tree": baseline_tree,
        "target_commit": target_commit,
        "source": "git-index",
        "excluded_control_paths": list(EXCLUDED_CONTROL_PATHS),
        "entries": entries,
        "summary": {
            "path_count": len(entries),
            "by_status": dict(sorted(status_counts.items())),
            "by_category": dict(sorted(category_counts.items())),
        },
    }


def render(document: dict) -> str:
    return json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()
    logging.basicConfig(level=logging.DEBUG if args.verbose else logging.INFO)
    repo = _repo_root()
    output = repo / OUTPUT_PATH
    try:
        rendered = render(build_inventory(repo))
    except (GitError, KeyError, OSError, ValueError) as error:
        logging.error("overlay inventory generation failed: %s", error)
        logging.error(
            "if an object is missing, fetch it explicitly without adding a remote: "
            "git fetch https://github.com/openai/codex.git <commit>"
        )
        return 2
    if args.check:
        existing = output.read_text(encoding="utf-8") if output.exists() else ""
        if existing != rendered:
            logging.error("overlay inventory is stale: %s", output)
            return 1
        logging.info("overlay inventory is reproducible: %s", output)
        return 0
    output.write_text(rendered, encoding="utf-8")
    logging.info("wrote overlay inventory: %s", output)
    return 0


if __name__ == "__main__":
    sys.exit(main())
