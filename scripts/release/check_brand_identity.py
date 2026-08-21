#!/usr/bin/env python3
"""Guard user-visible Whale surfaces against inherited Codex branding."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


SOURCE_ROOTS = (
    "third_party/codex-cli/codex-rs/cli/src",
    "third_party/codex-cli/codex-rs/exec/src",
    "third_party/codex-cli/codex-rs/login/src",
    "third_party/codex-cli/codex-rs/mcp-server/src",
    "third_party/codex-cli/codex-rs/tui/src",
    "third_party/codex-cli/codex-rs/app-server-daemon/src",
    "third_party/codex-cli/codex-rs/utils/approval-presets/src",
    "third_party/codex-cli/codex-rs/utils/cli/src",
)

EXTRA_FILES = (
    "third_party/codex-cli/codex-rs/tui/tooltips.txt",
    "third_party/codex-cli/codex-rs/models-manager/models.json",
    "third_party/codex-cli/codex-rs/core/gpt-5.1-codex-max_prompt.md",
    "third_party/codex-cli/codex-rs/core/gpt-5.2-codex_prompt.md",
    "third_party/codex-cli/codex-rs/core/gpt_5_1_prompt.md",
    "third_party/codex-cli/codex-rs/core/gpt_5_2_prompt.md",
    "third_party/codex-cli/codex-rs/core/gpt_5_codex_prompt.md",
    "third_party/codex-cli/codex-rs/core/prompt_with_apply_patch_instructions.md",
    "third_party/codex-cli/codex-rs/core/templates/model_instructions/gpt-5.2-codex_instructions_template.md",
    "third_party/codex-cli/codex-rs/prompts/templates/realtime/backend_prompt.md",
    "third_party/codex-cli/codex-rs/config/src/loader/mod.rs",
    "third_party/codex-cli/codex-rs/config/src/tui_keymap.rs",
    "third_party/codex-cli/codex-rs/chatgpt/src/chatgpt_client.rs",
    "third_party/codex-cli/codex-rs/chatgpt/src/connectors.rs",
    "third_party/codex-cli/codex-rs/memories/write/src/workspace.rs",
    "third_party/codex-cli/codex-rs/core/src/session/mod.rs",
    "third_party/codex-cli/codex-rs/linux-sandbox/src/launcher.rs",
    "third_party/codex-cli/codex-rs/feedback/src/lib.rs",
    "third_party/codex-cli/codex-rs/windows-sandbox-rs/src/bin/setup_main/win/setup_runtime_bin.rs",
    "third_party/codex-cli/codex-rs/ext/git-attribution/src/world_state.rs",
    "third_party/codex-cli/codex-rs/git-utils/src/baseline.rs",
)

FORBIDDEN = (
    re.compile(r"OpenAI Codex", re.IGNORECASE),
    re.compile(r"\bCodex Doctor\b", re.IGNORECASE),
    re.compile(r"\bAsk Codex\b", re.IGNORECASE),
    re.compile(r"OpenAI['’]s command-line coding agent", re.IGNORECASE),
    re.compile(r"\bWhale Cloud\b", re.IGNORECASE),
    re.compile(r"~[/\\]\.codex[/\\]config\.toml", re.IGNORECASE),
    re.compile(r"\$CODEX_HOME\b"),
    re.compile(
        r"developers\.openai\.com/codex/(?:memories|mcp|security|windows)",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?<![A-Za-z0-9_-])codex (?:agents|app-server|archive|debug|delete|"
        r"doctor|exec|features|fork|login|mcp|mcp-server|migrate-rollouts|plugin|queue|"
        r"resume|review|sandbox|unarchive)(?![A-Za-z0-9_-])",
        re.IGNORECASE,
    ),
    re.compile(r"(?<!Legacy )(?<!legacy )OpenAI Curated"),
    re.compile(r"\bYou are Codex\b", re.IGNORECASE),
    re.compile(r"\bCodex CLI is an open source project led by OpenAI\b", re.IGNORECASE),
)

REQUIRED_TEXT = {
    "third_party/codex-cli/codex-rs/cli/src/main.rs": (
        'name = "whale"',
        "printenv DEEPSEEK_API_KEY | whale login --with-api-key",
        'anyhow::bail!("Cloud tasks are not available in Whale.")',
        '#[clap(name = "cloud", alias = "cloud-tasks", hide = true)]',
    ),
    "third_party/codex-cli/codex-rs/cli/src/app_cmd.rs": (
        "Whale Desktop is not distributed yet",
    ),
    "third_party/codex-cli/codex-rs/tui/src/onboarding/welcome.rs": (
        "a DeepSeek-first coding agent",
    ),
    "third_party/codex-cli/codex-rs/tui/src/onboarding/auth.rs": (
        "Connect a DeepSeek API key to use Whale",
        "Sign in with legacy ChatGPT",
    ),
    "third_party/codex-cli/codex-rs/tui/src/chatwidget/plugin_catalog.rs": (
        "Legacy OpenAI Curated",
    ),
    "third_party/codex-cli/codex-rs/utils/cli/src/config_override.rs": (
        "~/.whale/config.toml",
    ),
    "third_party/codex-cli/codex-rs/utils/cli/src/shared_options.rs": (
        "$WHALE_HOME/<name>.config.toml",
    ),
    "third_party/codex-cli/codex-rs/models-manager/models.json": (
        "You are Whale",
        "Whale Auto Review",
    ),
    "third_party/codex-cli/codex-rs/prompts/templates/realtime/backend_prompt.md": (
        "You are Whale, a DeepSeek-first general-purpose agentic assistant",
    ),
    "third_party/codex-cli/codex-rs/config/src/loader/mod.rs": (
        'program_data.join("WhaleCode").join("Whale")',
    ),
    "third_party/codex-cli/codex-rs/windows-sandbox-rs/src/bin/setup_main/win/setup_runtime_bin.rs": (
        'local_app_data.join("WhaleCode").join("Whale")',
        'user_profile.join(".cache").join("whale-runtimes")',
    ),
    "third_party/codex-cli/codex-rs/ext/git-attribution/src/world_state.rs": (
        "Co-authored-by: Whale <noreply@whalecode.local>",
        "Generated with [Whale](https://github.com/ceasarXuu/WhaleCode).",
    ),
    "third_party/codex-cli/codex-rs/git-utils/src/baseline.rs": (
        "Initialize Whale git baseline",
        'name: "Whale".into()',
    ),
    "third_party/codex-cli/codex-rs/feedback/src/lib.rs": (
        'DOCTOR_REPORT_ATTACHMENT_FILENAME: &str = "whale-doctor-report.json"',
    ),
}


class BrandIdentityError(ValueError):
    pass


def source_files(repo_root: Path):
    for relative_root in SOURCE_ROOTS:
        root = repo_root / relative_root
        if not root.is_dir():
            raise BrandIdentityError(f"missing brand source root: {relative_root}")
        for path in sorted(root.rglob("*")):
            relative = path.relative_to(repo_root)
            if not path.is_file() or path.suffix not in {".rs", ".html"}:
                continue
            if path.name.endswith("_tests.rs") or "tests" in relative.parts:
                continue
            yield relative, path
    for relative_name in EXTRA_FILES:
        path = repo_root / relative_name
        if not path.is_file():
            raise BrandIdentityError(f"missing brand source file: {relative_name}")
        yield Path(relative_name), path


def is_comment(line: str) -> bool:
    stripped = line.lstrip()
    return stripped.startswith(("//", "/*", "*"))


def validate(repo_root: Path) -> None:
    errors: list[str] = []
    for relative, path in source_files(repo_root):
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except OSError as exc:
            errors.append(f"cannot read {relative}: {exc}")
            continue
        for line_number, line in enumerate(lines, 1):
            if is_comment(line):
                continue
            for pattern in FORBIDDEN:
                match = pattern.search(line)
                if match:
                    errors.append(
                        f"{relative}:{line_number} contains inherited brand exposure: "
                        f"{match.group(0)}"
                    )

    for relative, required_values in REQUIRED_TEXT.items():
        path = repo_root / relative
        try:
            content = path.read_text(encoding="utf-8")
        except OSError as exc:
            errors.append(f"cannot read {relative}: {exc}")
            continue
        for value in required_values:
            if value not in content:
                errors.append(f"{relative} is missing Whale brand contract: {value}")

    if errors:
        raise BrandIdentityError("\n".join(f"- {error}" for error in errors))


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
    except BrandIdentityError as exc:
        print(f"brand identity check FAILED:\n{exc}", file=sys.stderr)
        return 1
    print("brand identity check OK: user-visible surfaces identify Whale")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
