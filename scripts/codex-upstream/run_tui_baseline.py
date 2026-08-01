#!/usr/bin/env python3
"""Run codex-tui with Nextest and normalize its JUnit result."""

from __future__ import annotations

import argparse
import json
import logging
import os
import subprocess
import sys
from pathlib import Path

from tui_baseline import add_ignored_tests, parse_ignored_tests, parse_junit

BASELINE_PATH = "docs/v0.0.5/codex-upstream-sync/tui-baseline.json"
CODEX_ROOT = "third_party/codex-cli/codex-rs"
JUNIT_PATH = "target/nextest/whale-baseline/junit.xml"


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def render(document: dict) -> str:
    return json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def _run_nextest(repo: Path, filter_expr: str | None) -> int:
    codex_root = repo / CODEX_ROOT
    tool_config = (repo / "scripts/codex-upstream/nextest-whale.toml").resolve()
    command = [
        "cargo",
        "nextest",
        "run",
        "--manifest-path",
        str(codex_root / "Cargo.toml"),
        "-p",
        "codex-tui",
        "--profile",
        "whale-baseline",
        "--tool-config-file",
        f"whale:{tool_config}",
        "--no-fail-fast",
    ]
    if filter_expr:
        command.extend(["-E", filter_expr])
    environment = os.environ.copy()
    environment.update({"INSTA_UPDATE": "no", "RUST_MIN_STACK": "8388608"})
    logging.info("running codex-tui via cargo-nextest")
    return subprocess.run(
        command, cwd=codex_root, env=environment, check=False
    ).returncode


def _list_ignored(repo: Path, filter_expr: str | None) -> list[str]:
    codex_root = repo / CODEX_ROOT
    tool_config = (repo / "scripts/codex-upstream/nextest-whale.toml").resolve()
    command = [
        "cargo",
        "nextest",
        "list",
        "-p",
        "codex-tui",
        "--profile",
        "whale-baseline",
        "--tool-config-file",
        f"whale:{tool_config}",
        "--message-format",
        "json",
        "--run-ignored",
        "all",
    ]
    if filter_expr:
        command.extend(["-E", filter_expr])
    completed = subprocess.run(
        command,
        cwd=codex_root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        stderr = completed.stderr.decode("utf-8", "replace")[-2000:]
        raise RuntimeError(f"cargo nextest list failed: {stderr}")
    return parse_ignored_tests(completed.stdout)


def main() -> int:
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--update", action="store_true")
    parser.add_argument("--filter-expr")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--allow-test-failures", action="store_true")
    parser.add_argument("--parse-junit", type=Path)
    args = parser.parse_args()
    logging.basicConfig(level=logging.INFO)
    repo = _repo_root()
    if (args.check or args.update) and args.filter_expr:
        parser.error("--check/--update require an unfiltered full TUI run")
    if args.parse_junit:
        junit = args.parse_junit
        exit_code = 0
    else:
        junit = repo / CODEX_ROOT / JUNIT_PATH
        previous_mtime = junit.stat().st_mtime_ns if junit.exists() else None
        exit_code = _run_nextest(repo, args.filter_expr)
        current_mtime = junit.stat().st_mtime_ns if junit.exists() else None
        if previous_mtime is not None and current_mtime == previous_mtime:
            logging.error(
                "Nextest did not refresh its existing JUnit report: %s", junit
            )
            return 2
    if not junit.is_file():
        logging.error("Nextest JUnit report was not produced: %s", junit)
        return 2
    try:
        document = parse_junit(junit.read_bytes())
        if not args.parse_junit:
            ignored = _list_ignored(repo, args.filter_expr)
            document = add_ignored_tests(document, ignored)
        rendered = render(document)
    except (OSError, RuntimeError, ValueError) as error:
        logging.error("failed to normalize JUnit: %s", error)
        return 2

    baseline = repo / BASELINE_PATH
    if args.check:
        existing = baseline.read_text(encoding="utf-8") if baseline.exists() else ""
        if existing != rendered:
            logging.error("TUI baseline differs from the current full run")
            return 1
    elif args.update:
        baseline.write_text(rendered, encoding="utf-8")
        logging.info("updated TUI baseline: %s", baseline)
    elif args.output:
        output = args.output if args.output.is_absolute() else repo / args.output
        output.write_text(rendered, encoding="utf-8")
        logging.info("wrote normalized TUI result: %s", output)
    else:
        document = json.loads(rendered)
        logging.info("normalized TUI summary: %s", document["summary"])
    if exit_code != 0 and not args.allow_test_failures:
        return exit_code
    return 0


if __name__ == "__main__":
    sys.exit(main())
