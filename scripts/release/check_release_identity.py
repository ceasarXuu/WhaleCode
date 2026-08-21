#!/usr/bin/env python3
"""Offline guard against mixing Whale and Codex release identities."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


SEMVER_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$")
WORKSPACE_PACKAGE_RE = re.compile(
    r"(?ms)^\[workspace\.package\]\s*(.*?)(?=^\[|\Z)"
)
VERSION_RE = re.compile(r'(?m)^version\s*=\s*"([^"]+)"')


class IdentityError(ValueError):
    pass


def load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise IdentityError(f"cannot read valid JSON from {path}: {exc}") from exc


def workspace_version(cargo_toml: Path) -> str:
    try:
        content = cargo_toml.read_text(encoding="utf-8")
    except OSError as exc:
        raise IdentityError(f"cannot read {cargo_toml}: {exc}") from exc
    section = WORKSPACE_PACKAGE_RE.search(content)
    version = VERSION_RE.search(section.group(1)) if section else None
    if version is None:
        raise IdentityError("Cargo workspace package version is missing")
    return version.group(1)


def require_string(container: dict, field: str, context: str) -> str:
    value = container.get(field)
    if not isinstance(value, str) or not value:
        raise IdentityError(f"{context}.{field} must be a non-empty string")
    return value


def validate(repo_root: Path, supplied_tag: str | None = None) -> tuple[str, str]:
    manifest_path = (
        repo_root
        / "docs/releases/v0.0.5/release-preparation/release.json"
    )
    manifest = load_json(manifest_path)
    if manifest.get("schema_version") != 1:
        raise IdentityError("release manifest schema_version must be 1")
    if manifest.get("product") != "WhaleCode":
        raise IdentityError("release manifest product must be WhaleCode")

    release = manifest.get("release")
    substrate = manifest.get("upstream_substrate")
    if not isinstance(release, dict) or not isinstance(substrate, dict):
        raise IdentityError("release and upstream_substrate objects are required")

    whale_version = require_string(release, "version", "release")
    whale_tag = require_string(release, "tag", "release")
    if not SEMVER_RE.fullmatch(whale_version):
        raise IdentityError(f"invalid Whale semver: {whale_version}")
    if whale_tag != f"v{whale_version}":
        raise IdentityError(
            f"Whale tag must be v{whale_version}, got {whale_tag}"
        )
    if supplied_tag is not None and supplied_tag != whale_tag:
        raise IdentityError(
            f"supplied Whale tag must be {whale_tag}, got {supplied_tag}"
        )

    cargo_version = workspace_version(
        repo_root / "third_party/codex-cli/codex-rs/Cargo.toml"
    )
    if cargo_version != whale_version:
        raise IdentityError(
            f"Cargo Whale version {cargo_version} != release version {whale_version}"
        )

    substrate_version = require_string(substrate, "version", "upstream_substrate")
    substrate_tag = require_string(substrate, "tag", "upstream_substrate")
    if substrate_tag != f"rust-v{substrate_version}":
        raise IdentityError(
            "Codex substrate tag must be "
            f"rust-v{substrate_version}, got {substrate_tag}"
        )
    if whale_version == substrate_version or whale_tag == substrate_tag:
        raise IdentityError("Whale and Codex substrate identities must remain distinct")

    candidate_rel = require_string(
        substrate, "candidate_manifest", "upstream_substrate"
    )
    candidate = load_json(repo_root / candidate_rel)
    candidate_tag = candidate.get("release_tag")
    if candidate_tag != substrate_tag:
        raise IdentityError(
            f"Codex candidate tag {candidate_tag!r} != registered {substrate_tag}"
        )

    if release.get("status") == "preparing" and release.get("publish_authorized") is not False:
        raise IdentityError("preparing releases must set publish_authorized=false")

    return whale_tag, substrate_tag


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
    )
    parser.add_argument("--tag", help="candidate Whale product tag")
    args = parser.parse_args()
    try:
        whale_tag, substrate_tag = validate(args.repo_root.resolve(), args.tag)
    except IdentityError as exc:
        print(f"release identity check FAILED: {exc}", file=sys.stderr)
        return 1
    print(
        "release identity check OK: "
        f"WhaleCode {whale_tag}; Codex substrate {substrate_tag}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
