#!/usr/bin/env python3
"""Offline guard for Whale-owned distribution routes."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


FORBIDDEN = re.compile(
    r"@openai/codex|api\.github\.com/repos/openai/codex|"
    r"github\.com/openai/codex(?:/releases|/issues)?|"
    r"releases\.openai\.com/codex|chatgpt\.com/codex|"
    r"persistent\.oaistatic\.com/codex-app-prod|"
    r"formulae\.brew\.sh/api/cask/codex|OpenAI\.Codex|com\.openai\.codex",
    re.IGNORECASE,
)

ACTIVE_FILES = (
    "third_party/codex-cli/codex-cli/package.json",
    "third_party/codex-cli/codex-cli/bin/whale.js",
    "third_party/codex-cli/codex-cli/scripts/build_npm_package.py",
    "third_party/codex-cli/scripts/stage_npm_packages.py",
    "third_party/codex-cli/scripts/install/install.sh",
    "third_party/codex-cli/scripts/install/install.ps1",
    "third_party/codex-cli/codex-rs/install-context/src/lib.rs",
    "third_party/codex-cli/codex-rs/tui/src/update_action.rs",
    "third_party/codex-cli/codex-rs/tui/src/updates.rs",
    "third_party/codex-cli/codex-rs/tui/src/npm_registry.rs",
    "third_party/codex-cli/codex-rs/tui/src/update_prompt.rs",
    "third_party/codex-cli/codex-rs/tui/src/tooltips.rs",
    "third_party/codex-cli/codex-rs/tui/src/history_cell/notices.rs",
    "third_party/codex-cli/codex-rs/tui/src/bottom_pane/feedback_view.rs",
    "third_party/codex-cli/codex-rs/tui/src/app/history_ui.rs",
    "third_party/codex-cli/codex-rs/cli/src/main.rs",
    "third_party/codex-cli/codex-rs/cli/src/app_cmd.rs",
    "third_party/codex-cli/codex-rs/cli/src/doctor.rs",
    "third_party/codex-cli/codex-rs/cli/src/doctor/updates.rs",
    "third_party/codex-cli/codex-rs/app-server-daemon/src/lib.rs",
    "third_party/codex-cli/codex-rs/app-server-daemon/src/update_loop.rs",
    "third_party/codex-cli/announcement_tip.toml",
    "third_party/codex-cli/codex-cli/scripts/README.md",
)

REQUIRED_TEXT = {
    "third_party/codex-cli/codex-cli/bin/whale.js": (
        "@ceasarxuu/whalecode",
        "WHALE_MANAGED_PACKAGE_ROOT",
        'process.platform === "win32" ? "whale.exe" : "whale"',
    ),
    "third_party/codex-cli/codex-cli/scripts/build_npm_package.py": (
        'WHALE_NPM_NAME = "@ceasarxuu/whalecode"',
        '"whalecode": ["whalecode", *WHALE_PLATFORM_PACKAGES]',
    ),
    "third_party/codex-cli/scripts/stage_npm_packages.py": (
        'GITHUB_REPO = "ceasarXuu/WhaleCode"',
        "automatic vendor workflow discovery is disabled",
    ),
    "third_party/codex-cli/codex-rs/tui/src/update_action.rs": (
        "@ceasarxuu/whalecode@latest",
        "InstallMethod::Brew | InstallMethod::Standalone",
    ),
    "third_party/codex-cli/codex-rs/tui/src/npm_registry.rs": (
        "@ceasarxuu%2fwhalecode",
    ),
    "third_party/codex-cli/codex-rs/cli/src/app_cmd.rs": (
        "Whale Desktop is not distributed yet",
    ),
    "third_party/codex-cli/scripts/install/install.sh": (
        "Whale does not publish a standalone installer yet",
    ),
    "third_party/codex-cli/README.md": (
        "This is a Codex upstream vendor snapshot, not Whale's distribution authority.",
        "@ceasarxuu/whalecode",
    ),
}


class DistributionIdentityError(ValueError):
    pass


def read(repo_root: Path, relative: str) -> str:
    path = repo_root / relative
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        raise DistributionIdentityError(f"missing distribution contract file: {relative}") from exc


def validate(repo_root: Path) -> None:
    errors: list[str] = []
    package_path = "third_party/codex-cli/codex-cli/package.json"
    try:
        package = json.loads(read(repo_root, package_path))
    except json.JSONDecodeError as exc:
        errors.append(f"invalid npm package manifest: {exc}")
        package = {}

    expected_manifest = {
        "name": "@ceasarxuu/whalecode",
        "description": "Whale coding agent CLI.",
        "bin": {"whale": "bin/whale.js"},
    }
    for field, expected in expected_manifest.items():
        if package.get(field) != expected:
            errors.append(f"npm manifest {field} must be {expected!r}")
    repository = package.get("repository")
    if not isinstance(repository, dict) or repository.get("url") != (
        "git+https://github.com/ceasarXuu/WhaleCode.git"
    ):
        errors.append("npm manifest repository must be WhaleCode")
    if (repo_root / "third_party/codex-cli/codex-cli/bin/codex.js").exists():
        errors.append("npm package must not expose bin/codex.js")

    for relative in ACTIVE_FILES:
        try:
            content = read(repo_root, relative)
        except DistributionIdentityError as exc:
            errors.append(str(exc))
            continue
        match = FORBIDDEN.search(content)
        if match:
            errors.append(f"{relative} contains forbidden OpenAI distribution target: {match.group(0)}")

    for relative, required_values in REQUIRED_TEXT.items():
        try:
            content = read(repo_root, relative)
        except DistributionIdentityError as exc:
            errors.append(str(exc))
            continue
        for value in required_values:
            if value not in content:
                errors.append(f"{relative} is missing required Whale contract: {value}")

    builder = read(repo_root, "third_party/codex-cli/codex-cli/scripts/build_npm_package.py")
    for forbidden_package in ('"codex-sdk"', '"codex-responses-api-proxy"'):
        if forbidden_package in builder:
            errors.append(f"Whale npm builder must not publish {forbidden_package}")

    quarantine = read(repo_root, "third_party/codex-cli/DISTRIBUTION_QUARANTINE.md")
    if "@ceasarxuu/whalecode" not in quarantine or "不得调用" not in quarantine:
        errors.append("vendor distribution quarantine contract is incomplete")

    workflow_root = repo_root / ".github/workflows"
    for workflow in sorted(workflow_root.glob("*.y*ml")):
        content = workflow.read_text(encoding="utf-8")
        match = FORBIDDEN.search(content)
        if match:
            errors.append(f"{workflow.relative_to(repo_root)} targets OpenAI: {match.group(0)}")
        if "third_party/codex-cli/.github" in content or "rust-release.yml" in content:
            errors.append(f"{workflow.relative_to(repo_root)} activates quarantined vendor release code")

    if errors:
        raise DistributionIdentityError("\n".join(f"- {error}" for error in errors))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
    )
    args = parser.parse_args()
    try:
        validate(args.repo_root.resolve())
    except DistributionIdentityError as exc:
        print(f"distribution identity check FAILED:\n{exc}", file=sys.stderr)
        return 1
    print("distribution identity check OK: all active routes are Whale-owned")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
