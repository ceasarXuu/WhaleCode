#!/usr/bin/env python3
"""Require the Whale npm publishing workflow to remain manual and token-free."""

import re
import sys
from pathlib import Path


REQUIRED = (
    "workflow_dispatch:",
    "actions: read",
    "contents: read",
    "id-token: write",
    "if: github.ref == 'refs/heads/main'",
    "stage_npm_packages.py",
    "whale-native-artifacts",
    "npm publish",
    "--access public",
    "dist-tags.latest",
)
FORBIDDEN = (
    "NODE_AUTH_TOKEN",
    "_authToken",
    "secrets.",
    "contents: write",
    "packages: write",
)


def validate(root: Path) -> None:
    path = root / ".github/workflows/npm-publish.yml"
    text = path.read_text(encoding="utf-8")
    errors: list[str] = []

    for value in REQUIRED:
        if value not in text:
            errors.append(f"missing npm publish contract: {value}")
    for value in FORBIDDEN:
        if value.lower() in text.lower():
            errors.append(f"forbidden persistent credential or permission: {value}")

    if re.search(r"^  (push|pull_request|schedule):", text, re.MULTILINE):
        errors.append("npm publishing must only be manually dispatched")
    platform_loop = text.find("for platform in")
    root_publish = text.find('"$asset_dir/whalecode-npm-${VERSION}.tgz"')
    if platform_loop < 0 or root_publish < 0 or root_publish < platform_loop:
        errors.append("platform versions must be published before the root package")

    if errors:
        raise ValueError("\n".join(f"- {error}" for error in errors))


if __name__ == "__main__":
    try:
        validate(Path(__file__).resolve().parents[2])
    except (OSError, ValueError) as exc:
        print(f"npm publish workflow check FAILED:\n{exc}", file=sys.stderr)
        raise SystemExit(1)
    print("npm publish workflow check OK: manual OIDC publish, platform versions first")
