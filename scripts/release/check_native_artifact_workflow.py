#!/usr/bin/env python3
"""Static release gate for the Whale-owned six-platform artifact workflow."""

import re
import sys
from pathlib import Path

TARGETS = (
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
)
RUNNERS = (
    "ubuntu-24.04",
    "ubuntu-24.04-arm",
    "macos-15-intel",
    "macos-15",
    "windows-2025",
    "windows-11-arm",
)
FORBIDDEN = (
    "npm publish",
    "gh release",
    "packages: write",
    "id-token: write",
    "third_party/codex-cli/.github",
    "rust-release.yml",
)


def validate(root: Path) -> None:
    path = root / ".github/workflows/whale-native-artifacts.yml"
    text = path.read_text(encoding="utf-8")
    errors = []
    for target in TARGETS:
        if text.count(f"target: {target}") != 1:
            errors.append(f"target must appear exactly once: {target}")
    for runner in RUNNERS:
        if len(re.findall(rf"^\s+runner: {re.escape(runner)}$", text, re.MULTILINE)) != 1:
            errors.append(f"runner must appear exactly once: {runner}")
    for value in FORBIDDEN:
        if value.lower() in text.lower():
            errors.append(f"forbidden publish/vendor capability: {value}")
    required = (
        "workflow_dispatch:",
        "contents: read",
        "package_native_artifact.py",
        "build_native_manifest.py",
        "${{ matrix.target }}-unsigned",
        "if: github.ref == 'refs/heads/main'",
    )
    for value in required:
        if value not in text:
            errors.append(f"missing workflow contract: {value}")
    if re.search(r"\bpush\s*:", text):
        errors.append("native candidate workflow must not have a push trigger")
    v8_helper = (root / "scripts/release/prepare_rusty_v8.py").read_text()
    if "github.com/openai/codex/releases/download/rusty-v8-v" not in v8_helper:
        errors.append("V8 input must be pinned to the upstream substrate asset route")
    for forbidden in ("npm", "api.github.com", "gh release"):
        if forbidden in v8_helper.lower():
            errors.append(f"V8 helper contains a distribution capability: {forbidden}")
    if errors:
        raise ValueError("\n".join(f"- {error}" for error in errors))


if __name__ == "__main__":
    try:
        validate(Path(__file__).resolve().parents[2])
    except (OSError, ValueError) as exc:
        print(f"native artifact workflow check FAILED:\n{exc}", file=sys.stderr)
        raise SystemExit(1)
    print("native artifact workflow check OK: six unsigned Whale targets, no publish route")
