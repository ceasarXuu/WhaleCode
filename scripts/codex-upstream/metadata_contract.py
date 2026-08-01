#!/usr/bin/env python3
"""Pure structural validation for Codex sync metadata."""

from __future__ import annotations

import hashlib
import re
from collections import Counter

from classification import CATEGORIES
from tui_baseline import CLASSIFICATIONS

SHA40 = re.compile(r"^[0-9a-f]{40}$")
SHA64 = re.compile(r"^[0-9a-f]{64}$")


def validate_inventory(document: dict) -> list[str]:
    errors: list[str] = []
    required = {
        "schema_version",
        "vendor_path",
        "baseline_commit",
        "baseline_tree",
        "target_commit",
        "source",
        "excluded_control_paths",
        "entries",
        "summary",
    }
    missing = sorted(required - document.keys())
    if missing:
        return [f"inventory missing fields: {', '.join(missing)}"]
    if document["schema_version"] != 1:
        errors.append("inventory schema_version must be 1")
    if document["source"] != "git-index":
        errors.append("inventory source must be git-index")
    if document["excluded_control_paths"] != ["UPSTREAM.md"]:
        errors.append("inventory must exclude only the UPSTREAM.md control path")
    for field in ("baseline_commit", "baseline_tree", "target_commit"):
        if not SHA40.fullmatch(str(document[field])):
            errors.append(f"inventory {field} must be a full SHA")

    entries = document["entries"]
    paths = [entry.get("path") for entry in entries]
    if paths != sorted(paths):
        errors.append("inventory entries must be sorted by path")
    if len(paths) != len(set(paths)):
        errors.append("inventory contains duplicate paths")
    status_counts: Counter[str] = Counter()
    for entry in entries:
        path = entry.get("path", "<missing>")
        status = entry.get("status")
        if status not in {"added", "modified", "deleted"}:
            errors.append(f"{path}: invalid status {status!r}")
        else:
            status_counts[status] += 1
        categories = entry.get("categories", [])
        if categories != sorted(set(categories)) or not categories:
            errors.append(f"{path}: categories must be non-empty, sorted, unique")
        unknown = sorted(set(categories) - set(CATEGORIES))
        if unknown:
            errors.append(f"{path}: unknown categories {unknown}")
        if "unclassified" in categories:
            errors.append(f"{path}: remains unclassified")
        rules = entry.get("matched_rule_ids", [])
        if rules != sorted(set(rules)) or not rules:
            errors.append(f"{path}: matched_rule_ids must be non-empty, sorted, unique")
        commits = entry.get("evidence_commits", [])
        if any(not SHA40.fullmatch(str(commit)) for commit in commits):
            errors.append(f"{path}: invalid evidence commit")
        count = entry.get("evidence_commit_count")
        truncated = entry.get("evidence_truncated")
        if not isinstance(count, int) or count < len(commits):
            errors.append(f"{path}: invalid evidence_commit_count")
        if truncated != (isinstance(count, int) and count > len(commits)):
            errors.append(f"{path}: evidence_truncated does not match count")
        baseline_hash = entry.get("baseline_sha256")
        current_hash = entry.get("current_sha256")
        if baseline_hash is not None and not SHA64.fullmatch(str(baseline_hash)):
            errors.append(f"{path}: invalid baseline_sha256")
        if current_hash is not None and not SHA64.fullmatch(str(current_hash)):
            errors.append(f"{path}: invalid current_sha256")
        if status == "added" and baseline_hash is not None:
            errors.append(f"{path}: added path has a baseline hash")
        if status == "deleted" and current_hash is not None:
            errors.append(f"{path}: deleted path has a current hash")

    expected_summary = {
        "path_count": len(entries),
        "by_status": dict(sorted(status_counts.items())),
        "by_category": dict(sorted(category_counts_from(entries).items())),
    }
    if document["summary"] != expected_summary:
        errors.append("inventory summary does not match entries")
    return errors


def category_counts_from(entries: list[dict]) -> Counter[str]:
    counts: Counter[str] = Counter()
    for entry in entries:
        counts.update(entry.get("categories", []))
    return counts


def validate_ledger(document: dict) -> list[str]:
    errors: list[str] = []
    if document.get("schema_version") != 1:
        errors.append("ledger schema_version must be 1")
    if not SHA40.fullmatch(str(document.get("baseline_commit", ""))):
        errors.append("ledger baseline_commit must be a full SHA")
    expected_policy = {
        "trailer_required_false": "recomputed_2026-08-01_not_historical",
        "trailer_required_true": "historical_commit_trailer",
    }
    if document.get("patch_digest_policy") != expected_policy:
        errors.append("ledger patch_digest_policy is missing or invalid")
    seen_active: set[str] = set()
    for index, entry in enumerate(document.get("entries", [])):
        prefix = f"ledger entry {index}"
        upstream = str(entry.get("upstream_commit", ""))
        local = str(entry.get("local_commit", ""))
        digest = str(entry.get("patch_sha256", ""))
        if not SHA40.fullmatch(upstream):
            errors.append(f"{prefix}: invalid upstream_commit")
        if not SHA40.fullmatch(local):
            errors.append(f"{prefix}: invalid local_commit")
        if not SHA64.fullmatch(digest):
            errors.append(f"{prefix}: invalid patch_sha256")
        status = entry.get("status")
        if status not in {"applied", "reverted", "superseded_by_vendor"}:
            errors.append(f"{prefix}: invalid status {status!r}")
        if status == "applied":
            if upstream in seen_active:
                errors.append(f"{prefix}: duplicate active upstream commit {upstream}")
            seen_active.add(upstream)
        paths = entry.get("paths", [])
        if not paths or paths != sorted(set(paths)):
            errors.append(f"{prefix}: paths must be non-empty, sorted, unique")
        upstream_paths = entry.get("upstream_paths")
        if upstream_paths is not None and (
            not upstream_paths or upstream_paths != sorted(set(upstream_paths))
        ):
            errors.append(f"{prefix}: upstream_paths must be non-empty, sorted, unique")
        verification = entry.get("verification", {})
        if not verification.get("evidence"):
            errors.append(f"{prefix}: verification evidence is required")
        if not isinstance(entry.get("adapted"), bool):
            errors.append(f"{prefix}: adapted must be boolean")
        if not isinstance(entry.get("trailer_required"), bool):
            errors.append(f"{prefix}: trailer_required must be boolean")
    if not document.get("entries"):
        errors.append("ledger must contain at least one entry")
    return errors


def validate_backlog(document: dict) -> list[str]:
    errors: list[str] = []
    if document.get("schema_version") != 1:
        errors.append("backlog schema_version must be 1")
    entries = document.get("entries", [])
    locals_seen: set[str] = set()
    allowed = {
        "inferred_not_verified",
        "inferred_split_from_upstream",
        "source_unproven",
    }
    for index, entry in enumerate(entries):
        prefix = f"backlog entry {index}"
        local = str(entry.get("local_commit", ""))
        inferred = entry.get("inferred_upstream_commit")
        if not SHA40.fullmatch(local):
            errors.append(f"{prefix}: invalid local_commit")
        if local in locals_seen:
            errors.append(f"{prefix}: duplicate local_commit {local}")
        locals_seen.add(local)
        if inferred is not None and not SHA40.fullmatch(str(inferred)):
            errors.append(f"{prefix}: invalid inferred_upstream_commit")
        if entry.get("status") not in allowed:
            errors.append(f"{prefix}: invalid status")
        if not entry.get("scope"):
            errors.append(f"{prefix}: scope is required")
    return errors


def validate_patch_digest(entry: dict, patch: bytes) -> list[str]:
    actual = hashlib.sha256(patch).hexdigest()
    if actual == entry.get("patch_sha256"):
        return []
    return [f"patch digest mismatch for {entry.get('upstream_commit', '<missing>')}"]


def validate_ledger_paths(
    entry: dict,
    baseline_paths: set[str],
    current_paths: set[str],
    upstream_paths: set[str],
) -> list[str]:
    errors: list[str] = []
    upstream = entry.get("upstream_commit", "<missing>")
    for path in entry.get("upstream_paths", entry.get("paths", [])):
        if path not in upstream_paths:
            errors.append(f"path absent from upstream commit tree: {upstream}:{path}")
    for path in entry.get("paths", []):
        if path not in baseline_paths and path not in current_paths:
            errors.append(f"ledger path absent from baseline and current tree: {path}")
    return errors


def validate_tui_baseline(document: dict) -> list[str]:
    errors: list[str] = []
    expected_root = {
        "schema_version": 1,
        "runner": "cargo-nextest",
        "package": "codex-tui",
        "profile": "whale-baseline",
        "environment": {
            "INSTA_UPDATE": "no",
            "RUST_MIN_STACK": "8388608",
        },
    }
    for field, expected in expected_root.items():
        if document.get(field) != expected:
            errors.append(f"TUI baseline {field} is invalid")
    entries = document.get("entries", [])
    names = [entry.get("name") for entry in entries]
    if names != sorted(set(names)):
        errors.append("TUI baseline entries must be sorted and unique")
    counts: Counter[str] = Counter()
    classifications: Counter[str] = Counter()
    for entry in entries:
        name = entry.get("name", "<missing>")
        result = entry.get("result")
        classification = entry.get("classification")
        if result not in {"passed", "failed", "ignored"}:
            errors.append(f"{name}: invalid TUI result")
            continue
        counts[result] += 1
        if result == "failed":
            if classification not in CLASSIFICATIONS:
                errors.append(f"{name}: failed test has invalid classification")
            else:
                classifications[classification] += 1
        elif classification is not None:
            errors.append(f"{name}: non-failed test must not have a classification")
    expected_summary = {
        "test_count": len(entries),
        "by_result": dict(sorted(counts.items())),
        "by_classification": dict(sorted(classifications.items())),
    }
    if document.get("summary") != expected_summary:
        errors.append("TUI baseline summary does not match entries")
    return errors
