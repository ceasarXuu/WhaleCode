#!/usr/bin/env python3
"""Reject new shared workspace defaults and missing high-risk preflights."""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCANNED_SUFFIXES = {".py", ".sh", ".ps1"}


@dataclass(frozen=True)
class Allowance:
    path: str
    rule: str
    maximum: int
    reason: str


RULES = {
    "legacy-whale-bin": re.compile(r"\.whale/bin"),
    "shared-cargo-target": re.compile(r"\$env:CARGO_TARGET_DIR\s*="),
    "shared-bazel-output": re.compile(r"(?:BAZEL_OUTPUT_BASE\s*=|--output[-_]base)"),
}

# Every exception names its retirement boundary. Increasing a maximum requires review.
ALLOWANCES = (
    Allowance(
        "scripts/install-whale-local.sh",
        "legacy-whale-bin",
        2,
        "Explicit --scope user compatibility; workspace scope rejects this destination.",
    ),
    Allowance(
        "scripts/cache-regression/run_cache_hit_regression.ps1",
        "legacy-whale-bin",
        1,
        "Windows migration is deferred to W14.",
    ),
    Allowance(
        "scripts/run-deepseek-reasoning-replay-e2e.ps1",
        "legacy-whale-bin",
        1,
        "Windows migration is deferred to W14.",
    ),
    Allowance(
        "scripts/run-action-map-regression.ps1",
        "shared-cargo-target",
        2,
        "Windows/container migration is deferred to W14.",
    ),
    Allowance(
        "scripts/run-action-map-e2e-scenario.ps1",
        "shared-cargo-target",
        2,
        "Windows/container migration is deferred to W14.",
    ),
)

ENTRYPOINT_CONTRACTS = {
    "scripts/install-whale-local.sh": ("require-ready", "write_binary_attestation.py"),
    "scripts/cache-regression/run_cache_hit_regression.py": (
        "resolve_workspace_binary(",
    ),
    "scripts/taskspace-benchmark/run-active-prefix-matrix.py": ("require_ready(",),
    "scripts/taskspace-benchmark/r7_a2_b0_provider_wire_cli.py": ("require_ready(",),
}


def discover_sources(root: Path) -> dict[str, str]:
    sources: dict[str, str] = {}
    for path in (root / "scripts").rglob("*"):
        relative = path.relative_to(root).as_posix()
        if not path.is_file() or path.suffix not in SCANNED_SUFFIXES:
            continue
        if "__pycache__" in path.parts or "/tests/" in f"/{relative}/":
            continue
        if Path(relative).name.startswith("test-") or Path(relative).name.startswith("test_"):
            continue
        if relative == "scripts/workspace-safety/check_workspace_references.py":
            continue
        sources[relative] = path.read_text(encoding="utf-8")
    return sources


def inspect_sources(
    sources: dict[str, str],
    *,
    allowances: tuple[Allowance, ...] = ALLOWANCES,
    contracts: dict[str, tuple[str, ...]] = ENTRYPOINT_CONTRACTS,
) -> list[str]:
    violations: list[str] = []
    allowance_index = {(item.path, item.rule): item for item in allowances}
    observed: dict[tuple[str, str], int] = {}

    for path, text in sorted(sources.items()):
        for rule_name, pattern in RULES.items():
            matches = list(pattern.finditer(text))
            if not matches:
                continue
            key = (path, rule_name)
            observed[key] = len(matches)
            allowance = allowance_index.get(key)
            allowed = allowance.maximum if allowance else 0
            if len(matches) > allowed:
                violations.append(
                    f"{path}: {rule_name} has {len(matches)} occurrence(s), allowed {allowed}"
                )

    for allowance in allowances:
        if not allowance.reason.strip():
            violations.append(f"{allowance.path}: {allowance.rule} allowance lacks reason")
        count = observed.get((allowance.path, allowance.rule), 0)
        if count > allowance.maximum:
            continue
        if count == 0 and allowance.path in sources:
            violations.append(
                f"{allowance.path}: stale {allowance.rule} allowance; remove it"
            )

    for path, required_tokens in contracts.items():
        text = sources.get(path)
        if text is None:
            violations.append(f"{path}: protected entrypoint missing")
            continue
        for token in required_tokens:
            if token not in text:
                violations.append(f"{path}: missing workspace preflight token {token!r}")
    return violations


def main() -> int:
    violations = inspect_sources(discover_sources(REPO_ROOT))
    if violations:
        print("workspace reference gate: FAIL", file=sys.stderr)
        for violation in violations:
            print(f"- {violation}", file=sys.stderr)
        return 1
    print("workspace reference gate: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
