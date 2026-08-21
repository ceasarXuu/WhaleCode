#!/usr/bin/env python3
"""Verify six native artifact contracts and assemble one release manifest."""

import argparse
import hashlib
import json
import os
from pathlib import Path

TARGETS = {
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    contracts = []
    for path in args.artifacts.rglob("artifact-contract.json"):
        contract = json.loads(path.read_text())
        archive = path.parent / contract["archive"]
        expected_archive = f"whale-package-{contract['target']}.tar.gz"
        if contract["archive"] != expected_archive:
            raise SystemExit(f"artifact archive contract mismatch: {path}")
        actual = hashlib.sha256(archive.read_bytes()).hexdigest()
        if contract.get("product") != "WhaleCode" or contract.get("version") != args.version:
            raise SystemExit(f"artifact identity mismatch: {path}")
        if contract.get("unsigned") is not True or contract.get("sha256") != actual:
            raise SystemExit(f"artifact integrity mismatch: {path}")
        contracts.append(contract)
    targets = {item["target"] for item in contracts}
    if targets != TARGETS or len(contracts) != len(TARGETS):
        raise SystemExit(f"six-target inventory mismatch: {sorted(targets)}")
    commits = {item.get("commit") for item in contracts}
    expected_commit = os.environ.get("GITHUB_SHA")
    if len(commits) != 1 or (expected_commit and commits != {expected_commit}):
        raise SystemExit(f"artifact commit mismatch: {sorted(str(item) for item in commits)}")
    manifest = {
        "schema_version": 1,
        "product": "WhaleCode",
        "version": args.version,
        "unsigned": True,
        "artifacts": sorted(contracts, key=lambda item: item["target"]),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
