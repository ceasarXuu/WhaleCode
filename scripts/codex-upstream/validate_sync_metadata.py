#!/usr/bin/env python3
"""Validate Codex vendor provenance, overlay inventory, and backport records."""

from __future__ import annotations

import json
import logging
import sys
from pathlib import Path

from generate_overlay_inventory import (
    BASELINE,
    IMPORT_BASELINE,
    OUTPUT_PATH,
    TARGET,
)
from generate_current_overlay import OUTPUT_PATH as CURRENT_OVERLAY_PATH
from generate_current_overlay import build_current_inventory
from generate_overlay_inventory import render
from git_snapshot import GitError, git, index_subtree, list_tree, resolve_tree
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
from qualify_candidate import CANDIDATE_TARGET

LEDGER_PATH = "docs/v0.0.5/codex-upstream-sync/backport-ledger.json"
BACKLOG_PATH = "docs/v0.0.5/codex-upstream-sync/backport-provenance-backlog.json"
TUI_BASELINE_PATH = "docs/v0.0.5/codex-upstream-sync/tui-baseline.json"
CANDIDATE_DIR = "docs/releases/v0.0.7/codex-upstream-sync"
CANDIDATE_PATH = f"{CANDIDATE_DIR}/upstream-candidate.json"
CANDIDATE_OVERLAY_PATH = f"{CANDIDATE_DIR}/overlay-inventory.json"
CANDIDATE_DELTA_PATH = f"{CANDIDATE_DIR}/upstream-delta-inventory.json"
CANDIDATE_REPLAY_PATH = f"{CANDIDATE_DIR}/overlay-replay-ledger.json"
LEGACY_DELTA_PATH = "docs/v0.0.5/codex-upstream-sync/upstream-delta-inventory.json"
LEGACY_REPLAY_PATH = "docs/v0.0.5/codex-upstream-sync/overlay-replay-ledger.json"
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
        if candidate.get("commit_sha") != CANDIDATE_TARGET:
            errors.append("candidate commit does not match candidate target")
    for path, validator in (
        (LEGACY_DELTA_PATH, validate_upstream_delta),
        (LEGACY_REPLAY_PATH, validate_replay_ledger),
    ):
        if (repo / path).exists():
            errors.extend(validator(_load(repo, path)))

    candidate_overlay = _load(repo, CANDIDATE_OVERLAY_PATH)
    delta = _load(repo, CANDIDATE_DELTA_PATH)
    replay = _load(repo, CANDIDATE_REPLAY_PATH)
    errors.extend(validate_inventory(candidate_overlay))
    errors.extend(validate_upstream_delta(delta))
    errors.extend(validate_replay_ledger(replay))
    if candidate_overlay.get("baseline_commit") != BASELINE:
        errors.append("candidate overlay baseline does not match cutover baseline")
    if candidate_overlay.get("target_commit") != CANDIDATE_TARGET:
        errors.append("candidate overlay target does not match candidate target")
    if delta.get("baseline_commit") != BASELINE:
        errors.append("candidate delta baseline does not match cutover baseline")
    if delta.get("target_commit") != CANDIDATE_TARGET:
        errors.append("candidate delta target does not match candidate target")
    if replay.get("baseline_commit") != BASELINE:
        errors.append("candidate replay baseline does not match cutover baseline")
    if replay.get("target_commit") != CANDIDATE_TARGET:
        errors.append("candidate replay target does not match candidate target")
    overlay_by_path = {entry["path"]: entry for entry in candidate_overlay["entries"]}
    replay_by_path = {entry["path"]: entry for entry in replay.get("entries", [])}
    if replay_by_path.keys() != overlay_by_path.keys():
        errors.append("candidate replay paths do not exactly cover the candidate overlay")
    delta_by_path = {entry["path"]: entry for entry in delta["entries"]}
    for path in sorted(replay_by_path.keys() & overlay_by_path.keys()):
        source = overlay_by_path[path]
        decision = replay_by_path[path]
        upstream = delta_by_path.get(path)
        expected_target = (
            upstream["target_sha256"]
            if upstream is not None
            else source["baseline_sha256"]
        )
        if decision.get("current_sha256") != source["current_sha256"]:
            errors.append(f"{path}: candidate replay current hash is stale")
        if decision.get("target_sha256") != expected_target:
            errors.append(f"{path}: candidate replay target hash is stale")

    current_inventory = _load(repo, CURRENT_OVERLAY_PATH)
    errors.extend(validate_inventory(current_inventory))
    if current_inventory.get("baseline_commit") != CANDIDATE_TARGET:
        errors.append("current overlay baseline does not match imported substrate")
    if current_inventory.get("target_commit") != CANDIDATE_TARGET:
        errors.append("current overlay target does not match imported substrate")
    regenerated = render(build_current_inventory(repo))
    existing = (repo / CURRENT_OVERLAY_PATH).read_text(encoding="utf-8")
    if existing != regenerated:
        errors.append("current overlay inventory is stale relative to the Git index")
    if ledger.get("baseline_commit") != IMPORT_BASELINE:
        errors.append("ledger baseline does not match generator baseline")
    if inventory.get("baseline_commit") != BASELINE:
        errors.append("inventory baseline does not match generator baseline")
    if inventory.get("target_commit") != TARGET:
        errors.append("inventory target does not match generator target")

    baseline_paths = set(list_tree(repo, resolve_tree(repo, IMPORT_BASELINE)))
    current_paths = set(entry[3] for entry in _current_vendor_entries(repo))
    for entry in ledger.get("entries", []):
        upstream = entry["upstream_commit"]
        local = entry["local_commit"]
        if not _object_exists(repo, f"{upstream}^{{commit}}"):
            errors.append(f"missing upstream commit object {upstream}")
            continue
        if not _object_exists(repo, f"{local}^{{commit}}"):
            errors.append(f"missing local commit object {local}")
        # The historical ledger was created from Git's nine-character index
        # abbreviations.  Make that byte representation explicit: Git's
        # automatic abbreviation grows as the object database grows and would
        # otherwise invalidate immutable historical digests.
        patch = git(repo, "show", upstream, "--format=", "--binary", "--abbrev=9")
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
        IMPORT_BASELINE,
        BASELINE,
        TARGET,
        "overlay-inventory.json",
        "current-overlay-inventory.json",
        "backport-ledger.json",
        "backport-provenance-backlog.json",
        "upstream-candidate.json",
        "upstream-delta-inventory.json",
        "overlay-replay-ledger.json",
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
