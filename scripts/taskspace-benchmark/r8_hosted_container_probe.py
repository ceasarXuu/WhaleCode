#!/usr/bin/env python3
"""Budgeted DeepSeek Responses probe for hosted output/container coexistence."""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import subprocess
import tempfile
import time
from datetime import datetime
from datetime import timezone
from typing import Any


RECORD_ID = "WAR-20260805-005841-R8-HOSTED-PROBE-DCF750E2"
PRICING = {"cached": 0.02, "uncached": 1.0, "output": 2.0}


def now() -> str:
    return datetime.now().astimezone().isoformat(timespec="seconds")


def atomic_write_json(path: pathlib.Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        "w", encoding="utf-8", dir=path.parent, delete=False
    ) as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write("\n")
        temporary = pathlib.Path(stream.name)
    os.replace(temporary, path)


def load_api_key(repo: pathlib.Path) -> str:
    value = os.environ.get("DEEPSEEK_API_KEY", "").strip()
    if value:
        return value
    for line in (repo / ".env.local").read_text(encoding="utf-8").splitlines():
        key, separator, candidate = line.partition("=")
        if separator and key.strip() == "DEEPSEEK_API_KEY":
            value = candidate.strip().strip("'\"")
            if value:
                return value
    raise RuntimeError("DEEPSEEK_API_KEY is unavailable")


def update_ledger(path: pathlib.Path, update: Any) -> dict[str, Any]:
    ledger = json.loads(path.read_text(encoding="utf-8"))
    matches = [entry for entry in ledger["entries"] if entry["record_id"] == RECORD_ID]
    if len(matches) != 1:
        raise RuntimeError("probe ledger entry is missing or duplicated")
    update(matches[0])
    ledger["updated_at"] = now()
    atomic_write_json(path, ledger)
    return matches[0]


def request_body(model: str) -> dict[str, Any]:
    return {
        "model": model,
        "instructions": (
            "This is a tool protocol capability probe. Use live web search once, then "
            "call taskspace_probe in the same response. Copy the exact provider output "
            "item id for that web_search_call into provider_item_id."
        ),
        "input": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": (
                            "Find the title of the current DeepSeek API pricing page. "
                            "Bind that hosted result to node research-node."
                        ),
                    }
                ],
            }
        ],
        "tools": [
            {"type": "web_search", "external_web_access": True},
            {
                "type": "function",
                "name": "taskspace_probe",
                "description": "Records one Agent-declared node binding for this probe.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "node_id": {"type": "string"},
                        "provider_item_id": {"type": "string"},
                    },
                    "required": ["node_id", "provider_item_id"],
                    "additionalProperties": False,
                },
                "strict": True,
            },
        ],
        "tool_choice": "auto",
        "parallel_tool_calls": True,
        "reasoning": {"effort": "high", "summary": "auto"},
        "store": False,
        "stream": True,
        "include": [],
    }


def parse_sse(raw: str) -> list[dict[str, Any]]:
    events = []
    for line in raw.splitlines():
        if not line.startswith("data: ") or line == "data: [DONE]":
            continue
        try:
            value = json.loads(line[6:])
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            events.append(value)
    return events


def analyze(http_status: int, raw: str) -> dict[str, Any]:
    events = parse_sse(raw)
    done_items = [
        event.get("item", {})
        for event in events
        if event.get("type") == "response.output_item.done"
        and isinstance(event.get("item"), dict)
    ]
    web_ids = [
        item.get("id")
        for item in done_items
        if item.get("type") == "web_search_call" and isinstance(item.get("id"), str)
    ]
    calls = [
        item
        for item in done_items
        if item.get("type") == "function_call"
        and item.get("name") == "taskspace_probe"
    ]
    arguments = None
    if len(calls) == 1:
        try:
            arguments = json.loads(calls[0].get("arguments", ""))
        except json.JSONDecodeError:
            arguments = None
    completed = next(
        (event.get("response", {}) for event in events if event.get("type") == "response.completed"),
        {},
    )
    usage = completed.get("usage", {}) if isinstance(completed, dict) else {}
    details = usage.get("input_tokens_details", {}) if isinstance(usage, dict) else {}
    cached = int(details.get("cached_tokens", 0) or 0)
    input_tokens = int(usage.get("input_tokens", 0) or 0)
    protocol_supported = http_status == 200 and bool(events)
    return {
        "http_status": http_status,
        "event_count": len(events),
        "provider_web_item_ids": web_ids,
        "container_call_count": len(calls),
        "container_arguments": arguments,
        "hosted_and_container_coexist": len(web_ids) >= 1 and len(calls) == 1,
        "provider_id_echo_exact": (
            isinstance(arguments, dict)
            and arguments.get("provider_item_id") in web_ids
        ),
        "node_id_exact": (
            isinstance(arguments, dict)
            and arguments.get("node_id") == "research-node"
        ),
        "protocol_supported": protocol_supported,
        "usage": {
            "input_tokens": input_tokens,
            "cached_input_tokens": cached,
            "uncached_input_tokens": max(0, input_tokens - cached),
            "output_tokens": int(usage.get("output_tokens", 0) or 0),
        },
    }


def invoke(endpoint: str, api_key: str, body: dict[str, Any]) -> tuple[int, str, float]:
    if any(character in api_key for character in "\r\n\""):
        raise RuntimeError("DEEPSEEK_API_KEY contains unsupported characters")
    with tempfile.TemporaryDirectory(prefix="r8-hosted-probe-") as directory:
        root = pathlib.Path(directory)
        config_path = root / "curl.conf"
        request_path = root / "request.json"
        response_path = root / "response.sse"
        config_path.write_text(
            "\n".join(
                [
                    'request = "POST"',
                    f'header = "Authorization: Bearer {api_key}"',
                    'header = "Content-Type: application/json"',
                    'header = "Accept: text/event-stream"',
                    "silent",
                    "show-error",
                ]
            )
            + "\n",
            encoding="utf-8",
        )
        config_path.chmod(0o600)
        request_path.write_text(
            json.dumps(body, ensure_ascii=True, separators=(",", ":")),
            encoding="utf-8",
        )
        started = time.monotonic()
        completed = subprocess.run(
            [
                "curl",
                "--config",
                str(config_path),
                "--max-time",
                "180",
                "--output",
                str(response_path),
                "--write-out",
                "%{http_code}",
                "--data-binary",
                f"@{request_path}",
                endpoint,
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        elapsed = time.monotonic() - started
        if completed.returncode != 0:
            raise RuntimeError(f"curl transport failed: {completed.stderr.strip()}")
        status_text = completed.stdout.strip()
        if not status_text.isdigit():
            raise RuntimeError(f"curl returned invalid HTTP status: {status_text!r}")
        raw = response_path.read_text(encoding="utf-8") if response_path.exists() else ""
        return int(status_text), raw, elapsed


def settle_ledger(
    ledger_path: pathlib.Path,
    observations: list[dict[str, Any]],
    started_at: str,
    run_root: pathlib.Path,
    result_path: pathlib.Path,
    failed: bool,
) -> None:
    total = {
        key: sum(row["usage"][key] for row in observations)
        for key in (
            "input_tokens",
            "cached_input_tokens",
            "uncached_input_tokens",
            "output_tokens",
        )
    }
    amount = (
        total["cached_input_tokens"] * PRICING["cached"]
        + total["uncached_input_tokens"] * PRICING["uncached"]
        + total["output_tokens"] * PRICING["output"]
    ) / 1_000_000

    def update(entry: dict[str, Any]) -> None:
        entry["status"] = "failed" if failed else "settled"
        entry["started_at"] = started_at
        entry["ended_at"] = now()
        entry["elapsed_calendar_seconds"] = round(
            sum(row["elapsed_seconds"] for row in observations), 3
        )
        entry["execution"]["actual_sample_runs"] = len(observations)
        entry["execution"]["api_requests"] = len(observations)
        entry["execution"]["api_requests_evidence_status"] = "complete"
        entry["tokens"] = {
            "input": total["input_tokens"],
            "cached_input": total["cached_input_tokens"],
            "uncached_input": total["uncached_input_tokens"],
            "output": total["output_tokens"],
        }
        entry["monetary_cost"].update(
            {
                "status": "estimated" if not failed else "estimated_partial",
                "amount": amount,
                "components": {
                    "cached_input": total["cached_input_tokens"]
                    * PRICING["cached"]
                    / 1_000_000,
                    "uncached_input": total["uncached_input_tokens"]
                    * PRICING["uncached"]
                    / 1_000_000,
                    "output": total["output_tokens"]
                    * PRICING["output"]
                    / 1_000_000,
                },
                "formula": "cached/1e6*0.02 + uncached/1e6*1 + output/1e6*2",
            }
        )
        entry["evidence"].update(
            {
                "actual_run_root": run_root.as_posix(),
                "result_path": result_path.as_posix(),
                "usage_evidence_status": "complete" if observations else "unavailable",
                "outcome": "protocol_rejected" if failed else "completed",
            }
        )

    update_ledger(ledger_path, update)


def run(args: argparse.Namespace) -> int:
    repo = pathlib.Path(args.repo).resolve()
    ledger_path = repo / "benchmarks/whale-agent-run-ledger.json"
    run_root = repo / "target/provider-probes" / RECORD_ID
    result_path = repo / "benchmarks/taskspace/r8/evidence" / f"{RECORD_ID}.json"
    body = request_body(args.model)
    api_key = load_api_key(repo)
    started_at = now()

    def mark_running(entry: dict[str, Any]) -> None:
        if entry["status"] != "planned":
            raise RuntimeError("probe ledger entry is not planned")
        entry["status"] = "running"
        entry["started_at"] = started_at

    update_ledger(ledger_path, mark_running)
    observations = []
    failed = False
    for repeat in range(1, args.repeat + 1):
        status, raw, elapsed = invoke(args.endpoint, api_key, body)
        repeat_root = run_root / f"repeat-{repeat}"
        atomic_write_json(repeat_root / "request.json", body)
        repeat_root.mkdir(parents=True, exist_ok=True)
        (repeat_root / "response.sse").write_text(raw, encoding="utf-8")
        observation = analyze(status, raw)
        observation.update({"repeat": repeat, "elapsed_seconds": round(elapsed, 3)})
        observations.append(observation)
        if not observation["protocol_supported"]:
            failed = True
            break

    result = {
        "schema_version": "r8-hosted-container-probe-v1",
        "record_id": RECORD_ID,
        "generated_at": now(),
        "model": args.model,
        "endpoint": args.endpoint,
        "repeat_limit": args.repeat,
        "automatic_retries": 0,
        "observations": observations,
        "all_coexist": bool(observations)
        and all(row["hosted_and_container_coexist"] for row in observations),
        "all_provider_ids_exact": bool(observations)
        and all(row["provider_id_echo_exact"] for row in observations),
        "all_node_ids_exact": bool(observations)
        and all(row["node_id_exact"] for row in observations),
    }
    atomic_write_json(result_path, result)
    settle_ledger(
        ledger_path,
        observations,
        started_at,
        run_root.relative_to(repo),
        result_path.relative_to(repo),
        failed,
    )
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 2 if failed else 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", default=".")
    parser.add_argument("--model", default="deepseek-v4-flash")
    parser.add_argument("--endpoint", default="https://api.deepseek.com/responses")
    parser.add_argument("--repeat", type=int, choices=(1, 2), default=2)
    return run(parser.parse_args())


if __name__ == "__main__":
    raise SystemExit(main())
