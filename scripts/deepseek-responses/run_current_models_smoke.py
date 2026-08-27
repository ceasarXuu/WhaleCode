#!/usr/bin/env python3
"""Run the three explicitly budgeted DeepSeek Responses smoke samples."""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
import subprocess
import tempfile
import time
from pathlib import Path


REQUEST_LIMIT = 12
INPUT_LIMIT = 30_000
OUTPUT_LIMIT = 6_000


def load_api_key(repo: Path) -> str:
    helper = repo / "scripts/cache-regression/cache_run_environment.py"
    spec = importlib.util.spec_from_file_location("cache_run_environment", helper)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load credential helper")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.ensure_deepseek_api_key(repo)


def parse_events(stdout: str) -> tuple[list[dict], dict, str, list[str]]:
    events = []
    for line in stdout.splitlines():
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            events.append(value)
    failures = [event for event in events if event.get("type") in {"error", "turn.failed"}]
    if failures:
        raise RuntimeError(f"Whale emitted terminal failure: {failures[-1]}")
    completed = [event for event in events if event.get("type") == "turn.completed"]
    if len(completed) != 1:
        raise RuntimeError(f"expected one turn.completed event, found {len(completed)}")
    messages = [
        event.get("item", {}).get("text", "")
        for event in events
        if event.get("type") == "item.completed"
        and event.get("item", {}).get("type") == "agent_message"
    ]
    tools = [
        event.get("item", {}).get("type", "")
        for event in events
        if event.get("type") == "item.completed"
        and event.get("item", {}).get("type") in {"command_execution", "file_change"}
    ]
    return events, completed[0]["usage"], (messages[-1] if messages else ""), tools


def run_sample(
    binary: Path,
    workspace: Path,
    evidence_root: Path,
    model: str,
    prompt: str,
    marker: str,
    env: dict[str, str],
    image: Path | None = None,
) -> dict:
    command = [
        str(binary),
        "exec",
        "--json",
        "--ephemeral",
        "--ignore-user-config",
        "--ignore-rules",
        "--skip-git-repo-check",
        "--sandbox",
        "read-only",
        "--cd",
        str(workspace),
        "--model",
        model,
        "-c",
        'approval_policy="never"',
        "-c",
        'model_reasoning_effort="high"',
    ]
    if image is not None:
        command.extend(["--image", str(image)])
    command.extend(["--", prompt])
    started = time.monotonic()
    result = subprocess.run(
        command,
        cwd=workspace,
        env=env,
        capture_output=True,
        text=True,
        timeout=180,
        check=False,
    )
    if result.returncode != 0:
        stderr_tail = result.stderr.splitlines()[-12:]
        raise RuntimeError(f"{model} exited {result.returncode}: {stderr_tail}")
    events, usage, message, tools = parse_events(result.stdout)
    marker_seen = marker in message
    flash_tool_seen = model != "deepseek-v4-flash" or "command_execution" in tools
    validation_errors = []
    if not marker_seen:
        validation_errors.append(f"{model} response did not contain required marker")
    if not flash_tool_seen:
        validation_errors.append("Flash sample did not complete the required command tool call")
    sample = {
        "model": model,
        "status": "failed" if validation_errors else "passed",
        "elapsed_seconds": round(time.monotonic() - started, 3),
        "usage": usage,
        "event_types": sorted({event.get("type", "unknown") for event in events}),
        "completed_tool_types": tools,
        "response_marker": marker,
        "response_marker_seen": marker_seen,
        "final_message": message,
        "validation_errors": validation_errors,
    }
    sample_path = evidence_root / f"{model}.json"
    sample_path.write_text(
        json.dumps(sample, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    if validation_errors:
        raise RuntimeError("; ".join(validation_errors))
    return sample


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--record-id", required=True)
    parser.add_argument(
        "--only-model",
        choices=(
            "deepseek-v4-flash",
            "deepseek-v4-pro",
            "deepseek-v4-flash-vision-exp",
        ),
    )
    parser.add_argument("--request-limit", type=int, default=REQUEST_LIMIT)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    binary = args.binary.resolve()
    if not binary.is_file():
        raise RuntimeError(f"Whale binary not found: {binary}")
    credential_source = load_api_key(repo)
    evidence_root = args.evidence_root.resolve()
    evidence_root.mkdir(parents=True, exist_ok=True)
    request_state = evidence_root / "provider-request-count.txt"
    env = os.environ.copy()
    if args.request_limit < 1 or args.request_limit > REQUEST_LIMIT:
        raise RuntimeError(f"request limit must be within 1..{REQUEST_LIMIT}")
    env["WHALE_PROVIDER_REQUEST_HARD_LIMIT"] = str(args.request_limit)
    env["WHALE_PROVIDER_REQUEST_HARD_LIMIT_STATE_PATH"] = str(request_state)
    started = time.monotonic()
    samples = []
    with tempfile.TemporaryDirectory(prefix="whale-deepseek-smoke-") as temp:
        workspace = Path(temp)
        if args.only_model in {None, "deepseek-v4-flash"}:
            samples.append(
                run_sample(
                    binary,
                    workspace,
                    evidence_root,
                    "deepseek-v4-flash",
                    "Run exactly one shell command `printf WHALE_DS_FLASH_TOOL_OK` without changing files, then respond exactly WHALE_DS_FLASH_OK. Do not use any other tool.",
                    "WHALE_DS_FLASH_OK",
                    env,
                )
            )
        if args.only_model in {None, "deepseek-v4-pro"}:
            samples.append(
                run_sample(
                    binary,
                    workspace,
                    evidence_root,
                    "deepseek-v4-pro",
                    "Do not use tools. Respond exactly WHALE_DS_PRO_OK.",
                    "WHALE_DS_PRO_OK",
                    env,
                )
            )
        if args.only_model in {None, "deepseek-v4-flash-vision-exp"}:
            samples.append(
                run_sample(
                    binary,
                    workspace,
                    evidence_root,
                    "deepseek-v4-flash-vision-exp",
                    "Inspect the attached image. Respond exactly WHALE_DS_VISION_OK if it depicts an open book icon with a red-orange outline; otherwise explain briefly.",
                    "WHALE_DS_VISION_OK",
                    env,
                    repo
                    / "third_party/codex-cli/codex-rs/skills/src/assets/samples/openai-docs/assets/openai.png",
                )
            )
    usage = {
        key: sum(int(sample["usage"].get(key, 0)) for sample in samples)
        for key in (
            "input_tokens",
            "cached_input_tokens",
            "cache_write_input_tokens",
            "output_tokens",
            "reasoning_output_tokens",
        )
    }
    request_count = int(request_state.read_text(encoding="utf-8").strip())
    if request_count > args.request_limit:
        raise RuntimeError("provider request hard limit was exceeded")
    if usage["input_tokens"] > INPUT_LIMIT or usage["output_tokens"] > OUTPUT_LIMIT:
        raise RuntimeError(f"token budget exceeded: {usage}")
    summary = {
        "schema_version": "whalecode-deepseek-responses-smoke-v1",
        "record_id": args.record_id,
        "status": "passed",
        "credential_source": credential_source,
        "api_key_recorded": False,
        "provider_request_count": request_count,
        "elapsed_seconds": round(time.monotonic() - started, 3),
        "usage": usage,
        "samples": samples,
    }
    output = evidence_root / "summary.json"
    output.write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
