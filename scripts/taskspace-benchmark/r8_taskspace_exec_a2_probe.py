#!/usr/bin/env python3
"""Budgeted DeepSeek probe for A2 per-item hosted node bindings."""

from __future__ import annotations

import argparse
import json
import pathlib
import time
from typing import Any
from urllib import error as urllib_error
from urllib import request as urllib_request

import r8_hosted_container_probe as probe_support


PLAN_VERSION = "taskspace_exec_plan_v2"
CAPABILITY_ID = "r8-a2-hosted-only-v1"
HOSTED_TOOL = "web_search"
EXPECTED_NODES = frozenset({"deepseek-research", "openai-research"})


def request_body(model: str) -> dict[str, Any]:
    return {
        "model": model,
        "instructions": (
            "Work through TaskSpace. The map already has two ready research nodes: "
            "`deepseek-research` and `openai-research`. Use provider-hosted web search "
            "for each node as needed. After all hosted work in this response, call "
            "taskspace_exec exactly once. Its source is a complete "
            f"{PLAN_VERSION} plan. Keep calls empty. Declare every hosted output item "
            "in hosted_bindings, in provider output order, using only its hosted Tool "
            "name and the node_id chosen for that work. Do not copy provider item IDs."
        ),
        "input": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": (
                            "Using official sources, find the current API pricing page "
                            "for DeepSeek and for OpenAI, then briefly compare what each "
                            "page uses as its main pricing unit."
                        ),
                    }
                ],
            }
        ],
        "tools": [
            {"type": HOSTED_TOOL, "external_web_access": True},
            {
                "type": "function",
                "name": "taskspace_exec",
                "description": (
                    "Submit one complete TaskSpace action plan. The source must call "
                    f"taskspace.plan with strict JSON using version {PLAN_VERSION}, "
                    f"capability identity `{CAPABILITY_ID}`, calls, and hosted_bindings. "
                    "For every provider-hosted output item, hosted_bindings contains one "
                    "entry in provider output order with only tool and Agent-selected "
                    "node_id. Do not copy provider item IDs."
                ),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": (
                                "A complete taskspace.plan(<strict JSON>); source value."
                            ),
                        }
                    },
                    "required": ["source"],
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


def decode_plan_source(source: object) -> dict[str, Any] | None:
    if not isinstance(source, str):
        return None
    prefix = "taskspace.plan("
    suffix = ");"
    stripped = source.strip()
    if not stripped.startswith(prefix) or not stripped.endswith(suffix):
        return None
    try:
        value = json.loads(stripped[len(prefix) : -len(suffix)])
    except json.JSONDecodeError:
        return None
    return value if isinstance(value, dict) else None


def analyze(http_status: int, raw: str) -> dict[str, Any]:
    events = probe_support.parse_sse(raw)
    done = [
        (event.get("output_index"), event.get("item", {}))
        for event in events
        if event.get("type") == "response.output_item.done"
        and isinstance(event.get("item"), dict)
    ]
    hosted = sorted(
        [
            {"output_index": index, "id": item.get("id"), "item": item}
            for index, item in done
            if item.get("type") == "web_search_call"
        ],
        key=lambda row: row["output_index"]
        if isinstance(row["output_index"], int)
        else -1,
    )
    outer = [
        {"output_index": index, "item": item}
        for index, item in done
        if item.get("type") == "function_call"
        and item.get("name") == "taskspace_exec"
    ]
    arguments = None
    plan = None
    if len(outer) == 1:
        try:
            arguments = json.loads(outer[0]["item"].get("arguments", ""))
        except json.JSONDecodeError:
            arguments = None
        if isinstance(arguments, dict):
            plan = decode_plan_source(arguments.get("source"))

    bindings = plan.get("hosted_bindings") if isinstance(plan, dict) else None
    bindings = bindings if isinstance(bindings, list) else []
    binding_nodes = [
        binding.get("node_id")
        for binding in bindings
        if isinstance(binding, dict) and isinstance(binding.get("node_id"), str)
    ]
    hosted_indexes = [row["output_index"] for row in hosted]
    hosted_ids = [row["id"] for row in hosted]
    provider_identity_valid = (
        all(isinstance(index, int) and index >= 0 for index in hosted_indexes)
        and len(hosted_indexes) == len(set(hosted_indexes))
        and all(isinstance(item_id, str) and item_id for item_id in hosted_ids)
        and len(hosted_ids) == len(set(hosted_ids))
    )
    binding_shape_valid = all(
        isinstance(binding, dict)
        and set(binding) == {"tool", "node_id"}
        and binding.get("tool") == HOSTED_TOOL
        and binding.get("node_id") in EXPECTED_NODES
        for binding in bindings
    )
    plan_shape_valid = (
        isinstance(plan, dict)
        and set(plan) == {"version", "capability_id", "calls", "hosted_bindings"}
        and plan.get("version") == PLAN_VERSION
        and plan.get("capability_id") == CAPABILITY_ID
        and plan.get("calls") == []
    )
    outer_after_hosted = (
        len(outer) == 1
        and isinstance(outer[0]["output_index"], int)
        and bool(hosted_indexes)
        and outer[0]["output_index"] > max(hosted_indexes)
    )
    exact = (
        http_status == 200
        and len(outer) == 1
        and len(hosted) >= 2
        and provider_identity_valid
        and plan_shape_valid
        and len(bindings) == len(hosted)
        and binding_shape_valid
        and set(binding_nodes) == EXPECTED_NODES
        and outer_after_hosted
    )

    completed = next(
        (
            event.get("response", {})
            for event in events
            if event.get("type") == "response.completed"
        ),
        {},
    )
    usage = completed.get("usage", {}) if isinstance(completed, dict) else {}
    details = usage.get("input_tokens_details", {}) if isinstance(usage, dict) else {}
    input_tokens = int(usage.get("input_tokens", 0) or 0)
    cached_tokens = int(details.get("cached_tokens", 0) or 0)
    return {
        "http_status": http_status,
        "event_count": len(events),
        "hosted_facts": [
            {
                "output_index": row["output_index"],
                "provider_item_id": row["id"],
                "status": row["item"].get("status"),
                "action": row["item"].get("action"),
            }
            for row in hosted
        ],
        "outer_call_count": len(outer),
        "outer_output_index": outer[0]["output_index"] if len(outer) == 1 else None,
        "plan": plan,
        "checks": {
            "provider_identity_valid": provider_identity_valid,
            "plan_shape_valid": plan_shape_valid,
            "binding_count_exact": len(bindings) == len(hosted),
            "binding_shape_valid": binding_shape_valid,
            "both_nodes_declared": set(binding_nodes) == EXPECTED_NODES,
            "outer_after_hosted": outer_after_hosted,
        },
        "a2_v4_exact": exact,
        "usage": {
            "input_tokens": input_tokens,
            "cached_input_tokens": cached_tokens,
            "uncached_input_tokens": max(0, input_tokens - cached_tokens),
            "output_tokens": int(usage.get("output_tokens", 0) or 0),
        },
    }


def invoke(endpoint: str, api_key: str, body: dict[str, Any]) -> tuple[int, str, float]:
    if any(character in api_key for character in "\r\n"):
        raise RuntimeError("DEEPSEEK_API_KEY contains unsupported characters")
    payload = json.dumps(body, ensure_ascii=True, separators=(",", ":")).encode()
    request = urllib_request.Request(
        endpoint,
        data=payload,
        method="POST",
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
            "Accept": "text/event-stream",
        },
    )
    started = time.monotonic()
    try:
        with urllib_request.urlopen(request, timeout=180) as response:
            status = response.status
            raw = response.read().decode("utf-8")
    except urllib_error.HTTPError as error:
        status = error.code
        raw = error.read().decode("utf-8", errors="replace")
    except urllib_error.URLError as error:
        raise RuntimeError(f"provider transport failed: {error.reason}") from error
    return status, raw, time.monotonic() - started


def update_ledger(path: pathlib.Path, record_id: str, update: Any) -> None:
    ledger = json.loads(path.read_text(encoding="utf-8"))
    matches = [entry for entry in ledger["entries"] if entry["record_id"] == record_id]
    if len(matches) != 1:
        raise RuntimeError("probe ledger entry is missing or duplicated")
    update(matches[0])
    ledger["updated_at"] = probe_support.now()
    probe_support.atomic_write_json(path, ledger)


def settle_ledger(
    path: pathlib.Path,
    record_id: str,
    observations: list[dict[str, Any]],
    started_at: str,
    run_root: pathlib.Path,
    result_path: pathlib.Path,
    passed: bool,
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
    pricing = probe_support.PRICING
    amount = (
        total["cached_input_tokens"] * pricing["cached"]
        + total["uncached_input_tokens"] * pricing["uncached"]
        + total["output_tokens"] * pricing["output"]
    ) / 1_000_000

    def update(entry: dict[str, Any]) -> None:
        entry["status"] = "settled" if passed else "failed"
        entry["started_at"] = started_at
        entry["ended_at"] = probe_support.now()
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
                "status": "estimated",
                "amount": amount,
                "components": {
                    "cached_input": total["cached_input_tokens"]
                    * pricing["cached"]
                    / 1_000_000,
                    "uncached_input": total["uncached_input_tokens"]
                    * pricing["uncached"]
                    / 1_000_000,
                    "output": total["output_tokens"] * pricing["output"] / 1_000_000,
                },
                "formula": "cached/1e6*0.02 + uncached/1e6*1 + output/1e6*2",
            }
        )
        entry["evidence"].update(
            {
                "actual_run_root": run_root.as_posix(),
                "result_path": result_path.as_posix(),
                "usage_evidence_status": "complete" if observations else "unavailable",
                "outcome": "a2_v4_passed" if passed else "a2_v4_failed",
            }
        )

    update_ledger(path, record_id, update)


def run(args: argparse.Namespace) -> int:
    repo = pathlib.Path(args.repo).resolve()
    ledger_path = repo / "benchmarks/whale-agent-run-ledger.json"
    run_root = repo / "target/provider-probes" / args.record_id
    result_path = repo / "benchmarks/taskspace/r8/evidence" / f"{args.record_id}.json"
    body = request_body(args.model)
    api_key = probe_support.load_api_key(repo)
    started_at = probe_support.now()

    def mark_running(entry: dict[str, Any]) -> None:
        if entry["status"] != "planned":
            raise RuntimeError("probe ledger entry is not planned")
        entry["status"] = "running"
        entry["started_at"] = started_at

    update_ledger(ledger_path, args.record_id, mark_running)
    observations = []
    for repeat in range(1, args.repeat + 1):
        status, raw, elapsed = invoke(args.endpoint, api_key, body)
        repeat_root = run_root / f"repeat-{repeat}"
        repeat_root.mkdir(parents=True, exist_ok=True)
        probe_support.atomic_write_json(repeat_root / "request.json", body)
        (repeat_root / "response.sse").write_text(raw, encoding="utf-8")
        observation = analyze(status, raw)
        observation.update({"repeat": repeat, "elapsed_seconds": round(elapsed, 3)})
        observations.append(observation)
        if not observation["a2_v4_exact"]:
            break

    passed = len(observations) == args.repeat and all(
        row["a2_v4_exact"] for row in observations
    )
    result = {
        "schema_version": "r8-taskspace-exec-a2-probe-v1",
        "record_id": args.record_id,
        "generated_at": probe_support.now(),
        "model": args.model,
        "endpoint": args.endpoint,
        "repeat_limit": args.repeat,
        "automatic_retries": 0,
        "observations": observations,
        "a2_v4_passed": passed,
    }
    probe_support.atomic_write_json(result_path, result)
    settle_ledger(
        ledger_path,
        args.record_id,
        observations,
        started_at,
        run_root.relative_to(repo),
        result_path.relative_to(repo),
        passed,
    )
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0 if passed else 2


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", default=".")
    parser.add_argument("--record-id", required=True)
    parser.add_argument("--model", default="deepseek-v4-flash")
    parser.add_argument("--endpoint", default="https://api.deepseek.com/responses")
    parser.add_argument("--repeat", type=int, choices=(1, 2), default=2)
    return run(parser.parse_args())


if __name__ == "__main__":
    raise SystemExit(main())
