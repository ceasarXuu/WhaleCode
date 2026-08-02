#!/usr/bin/env python3
"""Validate Codex vendor provenance, overlay inventory, and backport records."""

from __future__ import annotations

import json
import logging
import sys
from pathlib import Path

from generate_overlay_inventory import (
    BASELINE,
    OUTPUT_PATH,
    TARGET,
    build_inventory,
    render,
)
from git_snapshot import GitError, git, list_tree, resolve_tree
from metadata_contract import (
    validate_backlog,
    validate_candidate,
    validate_inventory,
    validate_ledger,
    validate_ledger_paths,
    validate_patch_digest,
    validate_replay_ledger,
    validate_tui_baseline,
    validate_upstream_delta,
)

LEDGER_PATH = "docs/v0.0.5/codex-upstream-sync/backport-ledger.json"
BACKLOG_PATH = "docs/v0.0.5/codex-upstream-sync/backport-provenance-backlog.json"
TUI_BASELINE_PATH = "docs/v0.0.5/codex-upstream-sync/tui-baseline.json"
CANDIDATE_PATH = "docs/v0.0.5/codex-upstream-sync/upstream-candidate.json"
DELTA_PATH = "docs/v0.0.5/codex-upstream-sync/upstream-delta-inventory.json"
REPLAY_PATH = "docs/v0.0.5/codex-upstream-sync/overlay-replay-ledger.json"
UPSTREAM_PATH = "third_party/codex-cli/UPSTREAM.md"
SCHEMA_PATHS = (
    "scripts/codex-upstream/schemas/overlay-inventory.schema.json",
    "scripts/codex-upstream/schemas/backport-ledger.schema.json",
    "scripts/codex-upstream/schemas/tui-baseline.schema.json",
    "scripts/codex-upstream/schemas/upstream-candidate.schema.json",
    "scripts/codex-upstream/schemas/upstream-delta-inventory.schema.json",
    "scripts/codex-upstream/schemas/overlay-replay-ledger.schema.json",
)


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _load(repo: Path, path: str) -> dict:
    return json.loads((repo / path).read_text(encoding="utf-8"))


def _object_exists(repo: Path, revision: str) -> bool:
    try:
        git(repo, "cat-file", "-e", revision)
        return True
    except GitError:
        return False


def validate_repository(repo: Path) -> list[str]:
    errors: list[str] = []
    for schema_path in SCHEMA_PATHS:
        try:
            _load(repo, schema_path)
        except (OSError, ValueError) as error:
            errors.append(f"invalid schema {schema_path}: {error}")

    inventory = _load(repo, OUTPUT_PATH)
    ledger = _load(repo, LEDGER_PATH)
    backlog = _load(repo, BACKLOG_PATH)
    errors.extend(validate_inventory(inventory))
    errors.extend(validate_ledger(ledger))
    errors.extend(validate_backlog(backlog))
    tui_baseline_path = repo / TUI_BASELINE_PATH
    if tui_baseline_path.exists():
        errors.extend(validate_tui_baseline(_load(repo, TUI_BASELINE_PATH)))
    candidate_path = repo / CANDIDATE_PATH
    if candidate_path.exists():
        candidate = _load(repo, CANDIDATE_PATH)
        errors.extend(validate_candidate(candidate))
        if candidate.get("commit_sha") != TARGET:
            errors.append("candidate commit does not match generator target")
    delta_path = repo / DELTA_PATH
    if delta_path.exists():
        delta = _load(repo, DELTA_PATH)
        errors.extend(validate_upstream_delta(delta))
        if delta.get("baseline_commit") != BASELINE:
            errors.append("upstream delta baseline does not match generator baseline")
        if delta.get("target_commit") != TARGET:
            errors.append("upstream delta target does not match generator target")
    replay_path = repo / REPLAY_PATH
    if replay_path.exists():
        replay = _load(repo, REPLAY_PATH)
        errors.extend(validate_replay_ledger(replay))
        if replay.get("baseline_commit") != BASELINE:
            errors.append("replay baseline does not match generator baseline")
        if replay.get("target_commit") != TARGET:
            errors.append("replay target does not match generator target")
    if ledger.get("baseline_commit") != BASELINE:
        errors.append("ledger baseline does not match generator baseline")
    if inventory.get("baseline_commit") != BASELINE:
        errors.append("inventory baseline does not match generator baseline")
    if inventory.get("target_commit") != TARGET:
        errors.append("inventory target does not match generator target")

    regenerated = render(build_inventory(repo))
    existing = (repo / OUTPUT_PATH).read_text(encoding="utf-8")
    if existing != regenerated:
        errors.append("overlay inventory is stale relative to the Git index")

    baseline_paths = set(list_tree(repo, resolve_tree(repo, BASELINE)))
    current_paths = set(entry[3] for entry in _current_vendor_entries(repo))
    for entry in ledger.get("entries", []):
        upstream = entry["upstream_commit"]
        local = entry["local_commit"]
        if not _object_exists(repo, f"{upstream}^{{commit}}"):
            errors.append(f"missing upstream commit object {upstream}")
            continue
        if not _object_exists(repo, f"{local}^{{commit}}"):
            errors.append(f"missing local commit object {local}")
        patch = git(repo, "show", upstream, "--format=", "--binary")
        upstream_paths = set(list_tree(repo, f"{upstream}^{{tree}}"))
        errors.extend(validate_patch_digest(entry, patch))
        errors.extend(
            validate_ledger_paths(entry, baseline_paths, current_paths, upstream_paths)
        )
        evidence = repo / entry["verification"]["evidence"]
        if not evidence.is_file():
            errors.append(f"missing verification evidence: {evidence}")
        if entry["trailer_required"] and _object_exists(repo, f"{local}^{{commit}}"):
            body = git(repo, "show", "-s", "--format=%B", local).decode(
                "utf-8", "replace"
            )
            if upstream not in body or entry["patch_sha256"] not in body:
                errors.append(f"required trailers missing from {local}")

    for entry in backlog.get("entries", []):
        local = entry["local_commit"]
        inferred = entry["inferred_upstream_commit"]
        if not _object_exists(repo, f"{local}^{{commit}}"):
            errors.append(f"missing backlog local commit {local}")
        if inferred and not _object_exists(repo, f"{inferred}^{{commit}}"):
            errors.append(f"missing inferred upstream object {inferred}")

    upstream_text = (repo / UPSTREAM_PATH).read_text(encoding="utf-8")
    required_text = (
        BASELINE,
        TARGET,
        "overlay-inventory.json",
        "backport-ledger.json",
        "backport-provenance-backlog.json",
        "Responses API",
    )
    for text in required_text:
        if text not in upstream_text:
            errors.append(f"UPSTREAM.md missing required text: {text}")
    forbidden_text = ("1 active Whale overlay", "Chat Completions streaming")
    for text in forbidden_text:
        if text in upstream_text:
            errors.append(f"UPSTREAM.md contains stale text: {text}")
    return errors


def _current_vendor_entries(repo: Path) -> list[tuple[str, str, str, str]]:
    tree = git(repo, "write-tree").decode().strip()
    vendor_tree = (
        git(repo, "rev-parse", f"{tree}:third_party/codex-cli").decode().strip()
    )
    raw = git(repo, "ls-tree", "-rz", "--full-tree", "-r", vendor_tree)
    entries = []
    for record in raw.split(b"\0"):
        if not record:
            continue
        metadata, path = record.split(b"\t", 1)
        mode, object_type, oid = metadata.decode("ascii").split(" ")
        entries.append(
            (mode, object_type, oid, path.decode("utf-8", "surrogateescape"))
        )
    return entries


def main() -> int:
    logging.basicConfig(level=logging.INFO)
    repo = _repo_root()
    try:
        errors = validate_repository(repo)
    except (GitError, KeyError, OSError, TypeError, ValueError) as error:
        logging.error("sync metadata validation could not run: %s", error)
        return 2
    if errors:
        for error in errors:
            logging.error("%s", error)
        logging.error("sync metadata validation failed with %d error(s)", len(errors))
        return 1
    logging.info("sync metadata validation passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
