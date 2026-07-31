#!/usr/bin/env python3
"""Run deterministic cache contracts without contacting a model provider."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any

from cache_payload_contract import compare_snapshot_set
from cache_surface import load_contract, write_json


def validate_free_validation(config: dict[str, Any]) -> None:
    commands = config.get("commands")
    if not isinstance(commands, list) or not commands:
        raise ValueError("free validation must define commands")
    command_ids: set[str] = set()
    for command in commands:
        command_id = command.get("id")
        argv = command.get("argv")
        cwd = command.get("cwd", ".")
        timeout = command.get("timeout_seconds")
        if not isinstance(command_id, str) or not command_id:
            raise ValueError("free validation command id must be non-empty")
        if command_id in command_ids:
            raise ValueError(f"duplicate free validation command id: {command_id}")
        command_ids.add(command_id)
        if (
            not isinstance(argv, list)
            or not argv
            or not all(isinstance(item, str) and item for item in argv)
        ):
            raise ValueError(f"free validation command {command_id} has invalid argv")
        if not isinstance(cwd, str) or Path(cwd).is_absolute():
            raise ValueError(f"free validation command {command_id} has invalid cwd")
        if not isinstance(timeout, int) or timeout <= 0:
            raise ValueError(
                f"free validation command {command_id} has invalid timeout"
            )
        change_report = command.get("change_report")
        if change_report is not None:
            if not isinstance(change_report, dict):
                raise ValueError(
                    f"free validation command {command_id} has invalid change report"
                )
            report_globs = change_report.get("baseline_globs")
            if (
                change_report.get("type") != "final_wire_snapshot_set"
                or set(change_report) != {"type", "baseline_globs"}
                or not isinstance(report_globs, list)
                or not report_globs
                or not all(
                    isinstance(pattern, str) and pattern for pattern in report_globs
                )
            ):
                raise ValueError(
                    f"free validation command {command_id} has invalid change report"
                )
    baseline_globs = config.get("semantic_baseline_globs")
    if (
        not isinstance(baseline_globs, list)
        or not baseline_globs
        or not all(isinstance(pattern, str) and pattern for pattern in baseline_globs)
    ):
        raise ValueError("free validation must define semantic baseline globs")


def _output_tail(text: str, line_limit: int = 40) -> list[str]:
    return text.splitlines()[-line_limit:]


def run_free_validation(repo: Path, config: dict[str, Any]) -> dict[str, Any]:
    validate_free_validation(config)
    results = []
    passed = True
    environment = os.environ.copy()
    environment["INSTA_UPDATE"] = "no"
    environment["CARGO_TERM_COLOR"] = "never"
    for command in config["commands"]:
        command_id = command["id"]
        cwd = (repo / command.get("cwd", ".")).resolve()
        try:
            cwd.relative_to(repo.resolve())
        except ValueError as error:
            raise ValueError(
                f"free validation command {command_id} cwd escapes repository"
            ) from error
        started = time.monotonic()
        change_report = None
        with tempfile.TemporaryDirectory(prefix="whale-cache-report-") as report_dir:
            command_environment = environment.copy()
            if command.get("change_report"):
                command_environment["WHALE_CACHE_CHANGE_REPORT_DIR"] = report_dir
            try:
                completed = subprocess.run(
                    command["argv"],
                    cwd=cwd,
                    env=command_environment,
                    text=True,
                    capture_output=True,
                    timeout=command["timeout_seconds"],
                    check=False,
                )
                exit_code = completed.returncode
                timed_out = False
                output = completed.stdout + completed.stderr
            except subprocess.TimeoutExpired as error:
                exit_code = None
                timed_out = True
                stdout = error.stdout or ""
                stderr = error.stderr or ""
                output = f"{stdout}{stderr}"
            except OSError as error:
                exit_code = None
                timed_out = False
                output = f"failed to start command: {error}"
            if command.get("change_report"):
                change_report = compare_snapshot_set(
                    repo,
                    command["change_report"]["baseline_globs"],
                    Path(report_dir),
                )
        duration_ms = round((time.monotonic() - started) * 1000)
        report_passed = change_report is None or change_report["status"] == "unchanged"
        command_passed = exit_code == 0 and not timed_out and report_passed
        passed = passed and command_passed
        results.append(
            {
                "id": command_id,
                "argv": command["argv"],
                "cwd": command.get("cwd", "."),
                "exit_code": exit_code,
                "timed_out": timed_out,
                "duration_ms": duration_ms,
                "status": "pass" if command_passed else "fail",
                "output_tail": _output_tail(output),
                "change_report": change_report,
            }
        )
        if not command_passed:
            break
    return {
        "status": "pass" if passed else "fail",
        "passed": passed,
        "commands": results,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--contract",
        type=Path,
        default=Path("benchmarks/cache-regression/cache-surface-contract.json"),
    )
    parser.add_argument("--json-output", type=Path)
    args = parser.parse_args()

    repo = args.repo_root.resolve()
    contract_path = args.contract
    if not contract_path.is_absolute():
        contract_path = repo / contract_path
    contract = load_contract(contract_path)
    config = contract.get("free_validation")
    if not isinstance(config, dict):
        raise ValueError("cache contract does not define free validation")
    result = run_free_validation(repo, config)
    if args.json_output:
        output = args.json_output
        if not output.is_absolute():
            output = repo / output
        write_json(output, result)
    print(f"free cache contracts: {result['status'].upper()}")
    for command in result["commands"]:
        print(f"- {command['id']}: {command['status']} ({command['duration_ms']} ms)")
        if command["status"] != "pass":
            if command["change_report"] is not None:
                print(
                    json.dumps(command["change_report"], ensure_ascii=False, indent=2)
                )
            print(json.dumps(command["output_tail"], ensure_ascii=False, indent=2))
    return 0 if result["passed"] else 20


if __name__ == "__main__":
    raise SystemExit(main())
