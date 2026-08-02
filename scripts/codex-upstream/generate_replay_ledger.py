#!/usr/bin/env python3
"""Generate deterministic disposition decisions for every Whale vendor overlay path."""

from __future__ import annotations

import argparse
import json
import logging
import sys
from collections import Counter
from pathlib import Path

from generate_overlay_inventory import BASELINE, OUTPUT_PATH as OVERLAY_PATH, TARGET
from generate_upstream_delta import OUTPUT_PATH as DELTA_PATH
from generate_upstream_delta import generated_kind
from git_snapshot import index_subtree, list_tree, resolve_tree
from metadata_contract import validate_replay_ledger

VENDOR_PATH = "third_party/codex-cli"
OUTPUT_PATH = "docs/v0.0.5/codex-upstream-sync/overlay-replay-ledger.json"
HIGH_RISK = {
    "cache_observability",
    "multi_agent",
    "provider_model",
    "provider_transport",
    "taskspace_domain",
    "taskspace_host_hooks",
    "wire_sse",
}
CUTOVER_BATCHES = (
    {
        "id": "batch-1-brand-home",
        "depends_on": [],
        "scope": "brand, WHALE_HOME, CLI surface, build and release",
    },
    {
        "id": "batch-2-substrate",
        "depends_on": ["batch-1-brand-home"],
        "scope": "general Codex runtime substrate",
    },
    {
        "id": "batch-3-deepseek-wire",
        "depends_on": ["batch-2-substrate"],
        "scope": "DeepSeek provider, Responses wire, usage and cache semantics",
    },
    {
        "id": "batch-4-taskspace-multi-agent",
        "depends_on": ["batch-2-substrate", "batch-3-deepseek-wire"],
        "scope": "TaskSpace and Multi-Agent domain, projections and host hooks",
    },
    {
        "id": "batch-5-generated-release",
        "depends_on": [
            "batch-1-brand-home",
            "batch-2-substrate",
            "batch-3-deepseek-wire",
            "batch-4-taskspace-multi-agent",
        ],
        "scope": "regenerated artifacts, full gates and release assembly",
    },
)


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def lineage(kind: str | None) -> dict | None:
    if kind in {"app-server-json-schema", "app-server-typescript"}:
        return {
            "command": "just write-app-server-schema",
            "generator": (
                "codex-rs/app-server-protocol/src/bin/write_schema_fixtures.rs"
            ),
            "kind": kind,
        }
    if kind == "config-schema":
        return {
            "command": "just write-config-schema",
            "generator": "codex-rs/core/src/bin/config_schema.rs",
            "kind": kind,
        }
    return None


def cutover_batch(categories: set[str], kind: str | None) -> str:
    if kind is not None:
        return "batch-5-generated-release"
    if categories & {"taskspace_domain", "taskspace_host_hooks", "multi_agent"}:
        return "batch-4-taskspace-multi-agent"
    if categories & {
        "cache_observability",
        "provider_model",
        "provider_transport",
        "wire_sse",
    }:
        return "batch-3-deepseek-wire"
    if categories & {"brand_home", "build_release", "cli_surface"}:
        return "batch-1-brand-home"
    return "batch-2-substrate"


def verification_for(batch: str, disposition: str, lineage_data: dict | None) -> list[str]:
    checks = {
        "batch-1-brand-home": "cargo test -p codex-cli",
        "batch-2-substrate": "cargo nextest run -p codex-core",
        "batch-3-deepseek-wire": "provider wire fixtures and cache regression gate",
        "batch-4-taskspace-multi-agent": "TaskSpace and Multi-Agent targeted nextest suites",
        "batch-5-generated-release": "full workspace tests and generated-tree diff",
    }
    result = [checks[batch], "sync metadata validator"]
    if disposition == "regenerate" and lineage_data is not None:
        result.append(lineage_data["command"])
    if disposition == "reapply-exact":
        result.append("replayed blob SHA-256 matches current Whale blob")
    if disposition == "adopt-upstream":
        result.append("result blob SHA-256 matches target blob")
    return sorted(result)


def decide(
    overlay: dict, delta: dict | None, kind: str | None
) -> tuple[str, list[str], dict | None]:
    target_hash = (
        delta["target_sha256"] if delta is not None else overlay["baseline_sha256"]
    )
    current_hash = overlay["current_sha256"]
    lineage_data = lineage(kind)
    categories = set(overlay["categories"])
    if current_hash == target_hash:
        return "adopt-upstream", ["Whale and target blob SHA-256 are identical"], None
    if current_hash is None and target_hash is None:
        return "adopt-upstream", ["Whale and target both delete the path"], None
    if lineage_data is not None:
        return (
            "regenerate",
            ["authoritative upstream generator and command are known"],
            lineage_data,
        )
    if kind in {"insta-snapshot", "cargo-lock"}:
        return (
            "defer",
            [f"{kind} requires cutover-source integration before regeneration"],
            None,
        )
    if categories & HIGH_RISK:
        return (
            "adapt-semantically",
            ["path belongs to a protected Whale semantic domain"],
            None,
        )
    if delta is None:
        return (
            "reapply-exact",
            ["target leaves the baseline path unchanged"],
            None,
        )
    return (
        "adapt-semantically",
        ["upstream and Whale both change the path"],
        None,
    )


def build_ledger(repo: Path) -> dict:
    overlay = json.loads((repo / OVERLAY_PATH).read_text(encoding="utf-8"))
    delta = json.loads((repo / DELTA_PATH).read_text(encoding="utf-8"))
    delta_by_path = {entry["path"]: entry for entry in delta["entries"]}
    dependencies = {batch["id"]: batch["depends_on"] for batch in CUTOVER_BATCHES}
    entries: list[dict] = []
    for source in overlay["entries"]:
        path = source["path"]
        upstream = delta_by_path.get(path)
        kind = upstream["generated_kind"] if upstream else generated_kind(path)
        categories = set(source["categories"])
        batch = cutover_batch(categories, kind)
        disposition, basis, lineage_data = decide(source, upstream, kind)
        target_hash = (
            upstream["target_sha256"]
            if upstream is not None
            else source["baseline_sha256"]
        )
        entries.append(
            {
                "categories": sorted(categories),
                "current_sha256": source["current_sha256"],
                "cutover_batch": batch,
                "decision_basis": sorted(basis),
                "depends_on": dependencies[batch],
                "disposition": disposition,
                "generated_kind": kind,
                "generated_lineage": lineage_data,
                "owner_domain": batch.removeprefix("batch-").split("-", 1)[1],
                "path": path,
                "target_sha256": target_hash,
                "upstream_status": upstream["status"] if upstream else "unchanged",
                "verification": verification_for(batch, disposition, lineage_data),
                "whale_status": source["status"],
            }
        )
    dispositions = Counter(entry["disposition"] for entry in entries)
    batches = Counter(entry["cutover_batch"] for entry in entries)
    return {
        "schema_version": 1,
        "baseline_commit": BASELINE,
        "target_commit": TARGET,
        "overlay_tree": index_subtree(repo, VENDOR_PATH),
        "cutover_batches": list(CUTOVER_BATCHES),
        "entries": entries,
        "summary": {
            "path_count": len(entries),
            "by_disposition": dict(sorted(dispositions.items())),
            "by_cutover_batch": dict(sorted(batches.items())),
        },
    }


def _lineage_paths_exist(repo: Path, document: dict) -> list[str]:
    target_paths = set(list_tree(repo, resolve_tree(repo, TARGET)))
    return sorted(
        {
            entry["generated_lineage"]["generator"]
            for entry in document["entries"]
            if entry["generated_lineage"] is not None
            and entry["generated_lineage"]["generator"] not in target_paths
        }
    )


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
        document = build_ledger(repo)
        errors = validate_replay_ledger(document)
        missing = _lineage_paths_exist(repo, document)
        errors.extend(f"missing target generator path: {path}" for path in missing)
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
            logging.error("overlay replay ledger is missing or stale")
            return 1
        logging.info("overlay replay ledger is current")
        return 0
    except (OSError, RuntimeError, ValueError) as error:
        logging.error("replay ledger generation failed: %s", error)
        return 2


if __name__ == "__main__":
    sys.exit(main())
