#!/usr/bin/env python3
"""R7.1 A2-B0 DeepSeek provider-wire probe.

This script validates transport and model-visible wire behavior only. It never
executes returned tools and does not import the production TaskSpace runtime.

The provider-native Web Search negative scenario is historical and specific to
the former Chat Completions capability surface. DeepSeek's current Responses
surface supports provider-hosted Web Search; use r8_hosted_container_probe.py
for the current contract.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
import pathlib
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any

from r7_a2_b0_provider_wire_contract import control_tool, function_tool


def write_json(path: pathlib.Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, ensure_ascii=True, indent=2) + "\n",
        encoding="utf-8",
    )


PATCH_TEXT = """*** Begin Patch
*** Update File: src/lib.py
@@
-VALUE = 1
+VALUE = 2
*** End Patch"""


@dataclass(frozen=True)
class Scenario:
    name: str
    tools: list[dict[str, Any]]
    prompt: str
    expected_names: list[str]
    expected_action: str
    expected_pairs: list[tuple[str, str]]
    native_negative: bool = False
    required_thinking_negative: bool = False
    expected_patch: str | None = None


def scenarios() -> list[Scenario]:
    read_file = function_tool(
        "read_file", "Read a file.", {"path": {"type": "string"}}, ["path"]
    )
    exec_command = function_tool(
        "exec_command",
        "Run a shell command.",
        {"cmd": {"type": "string"}},
        ["cmd"],
    )
    apply_patch = function_tool(
        "apply_patch",
        "Apply the exact patch text in input.",
        {"input": {"type": "string"}},
        ["input"],
    )
    mcp_lookup = function_tool(
        "mcp__probe__lookup",
        "Synthetic client-side MCP lookup.",
        {"query": {"type": "string"}},
        ["query"],
    )
    tool_search = function_tool(
        "tool_search",
        "Search the client tool catalog.",
        {"query": {"type": "string"}},
        ["query"],
    )
    web_search = function_tool(
        "web_search",
        "Client-sequenced web search adapter function.",
        {"query": {"type": "string"}},
        ["query"],
    )
    control = control_tool()
    return [
        Scenario(
            name="initialize_multi_function",
            tools=[control, read_file, exec_command],
            expected_names=["taskspace_control", "read_file", "exec_command"],
            expected_action="initialize_and_execute",
            expected_pairs=[
                ("inspect", "read_file"),
                ("research", "exec_command"),
            ],
            prompt=(
                "Emit exactly three tool calls in this response and no prose. "
                "First call taskspace_control initialize_and_execute with root "
                "(root, Complete the synthetic task), work_nodes inspect and "
                "research, finish (finish, Finish the task), edges root->inspect, "
                "root->research, inspect->finish, research->finish, and actions "
                "inspect/read_file then research/exec_command. Then call read_file "
                "with path /workspace/src/lib.rs and exec_command with cmd "
                "'rg TODO src', in that order."
            ),
        ),
        Scenario(
            name="execute_patch_and_function",
            tools=[control, apply_patch, exec_command],
            expected_names=["taskspace_control", "apply_patch", "exec_command"],
            expected_action="execute",
            expected_pairs=[
                ("implement", "apply_patch"),
                ("verify", "exec_command"),
            ],
            expected_patch=PATCH_TEXT,
            prompt=(
                "Emit exactly three tool calls in this response and no prose. "
                "First call taskspace_control execute with expected_revision 12, "
                "mutation complete_node inspect, and actions implement/apply_patch "
                "then verify/exec_command. Then call apply_patch with this exact "
                f"input:\n{PATCH_TEXT}\nFinally call exec_command with cmd "
                "'pytest -q'."
            ),
        ),
        Scenario(
            name="execute_mcp_and_tool_search",
            tools=[control, mcp_lookup, tool_search],
            expected_names=[
                "taskspace_control",
                "mcp__probe__lookup",
                "tool_search",
            ],
            expected_action="execute",
            expected_pairs=[
                ("lookup", "mcp__probe__lookup"),
                ("discover", "tool_search"),
            ],
            prompt=(
                "Emit exactly three tool calls in this response and no prose. "
                "First call taskspace_control execute with expected_revision 13, "
                "empty mutations, and actions lookup/mcp__probe__lookup then "
                "discover/tool_search. Then call mcp__probe__lookup with query "
                "account-42 and tool_search with query database-migration, in order."
            ),
        ),
        Scenario(
            name="execute_web_search_function",
            tools=[control, web_search],
            expected_names=["taskspace_control", "web_search"],
            expected_action="execute",
            expected_pairs=[("research", "web_search")],
            prompt=(
                "Emit exactly two tool calls in this response and no prose. First "
                "call taskspace_control execute with expected_revision 14, empty "
                "mutations, and action research/web_search. Then call web_search "
                "with query DeepSeek-function-calling."
            ),
        ),
        Scenario(
            name="provider_native_web_search_negative",
            tools=[{"type": "web_search"}],
            expected_names=[],
            expected_action="",
            expected_pairs=[],
            native_negative=True,
            prompt="Use the provider-native web search tool.",
        ),
        Scenario(
            name="required_tool_choice_thinking_negative",
            tools=[control],
            expected_names=[],
            expected_action="",
            expected_pairs=[],
            required_thinking_negative=True,
            prompt="Call taskspace_control.",
        ),
    ]


def parse_arguments(call: dict[str, Any]) -> dict[str, Any] | None:
    try:
        value = json.loads(call["function"]["arguments"])
    except (KeyError, TypeError, json.JSONDecodeError):
        return None
    return value if isinstance(value, dict) else None


def analyze_response(
    scenario: Scenario, http_status: int, payload: dict[str, Any] | None
) -> dict[str, Any]:
    if scenario.native_negative or scenario.required_thinking_negative:
        rejected = http_status >= 400
        return {
            "http_status": http_status,
            "provider_rejected_expected_request": rejected,
            "dispatch_eligible": False,
            "reason_codes": [] if rejected else ["provider_native_tool_unexpectedly_accepted"],
            "pass": rejected,
        }

    if http_status != 200:
        error = payload.get("error", {}) if isinstance(payload, dict) else {}
        return {
            "http_status": http_status,
            "provider_error": {
                "type": error.get("type", "") if isinstance(error, dict) else "",
                "code": error.get("code", "") if isinstance(error, dict) else "",
                "message": error.get("message", "") if isinstance(error, dict) else "",
            },
            "dispatch_eligible": False,
            "reason_codes": ["provider_request_rejected"],
            "pass": False,
        }

    choices = payload.get("choices", []) if isinstance(payload, dict) else []
    choice = choices[0] if choices else {}
    message = choice.get("message", {}) if isinstance(choice, dict) else {}
    calls = message.get("tool_calls", []) if isinstance(message, dict) else []
    calls = calls if isinstance(calls, list) else []
    names = [
        call.get("function", {}).get("name", "")
        for call in calls
        if isinstance(call, dict)
    ]
    parsed = [parse_arguments(call) for call in calls if isinstance(call, dict)]
    reasons: list[str] = []
    if choice.get("finish_reason") != "tool_calls":
        reasons.append("response_not_complete_tool_manifest")
    if names != scenario.expected_names:
        reasons.append("tool_name_or_order_mismatch")
    if len(parsed) != len(calls) or any(value is None for value in parsed):
        reasons.append("invalid_tool_arguments_json")

    control = parsed[0] if parsed and isinstance(parsed[0], dict) else {}
    actions = control.get("actions", []) if isinstance(control, dict) else []
    pairs = (
        [
            (item.get("node_id", ""), item.get("tool", ""))
            for item in actions
            if isinstance(item, dict)
        ]
        if isinstance(actions, list)
        else []
    )
    sibling_names = names[1:] if names and names[0] == "taskspace_control" else []
    if control.get("action") != scenario.expected_action:
        reasons.append("control_action_mismatch")
    if pairs != scenario.expected_pairs:
        reasons.append("agent_action_manifest_mismatch")
    if [tool for _, tool in pairs] != sibling_names:
        reasons.append("manifest_sibling_pairing_mismatch")
    if any(not node_id for node_id, _ in pairs):
        reasons.append("missing_agent_declared_node_id")

    patch_exact: bool | None = None
    if scenario.expected_patch is not None:
        patch_indexes = [i for i, name in enumerate(names) if name == "apply_patch"]
        patch_exact = bool(
            len(patch_indexes) == 1
            and parsed[patch_indexes[0]] is not None
            and parsed[patch_indexes[0]].get("input") == scenario.expected_patch
        )
        if not patch_exact:
            reasons.append("patch_input_not_exact")

    usage = payload.get("usage", {}) if isinstance(payload, dict) else {}
    reasoning = message.get("reasoning_content") if isinstance(message, dict) else None
    return {
        "http_status": http_status,
        "finish_reason": choice.get("finish_reason", ""),
        "tool_names": names,
        "tool_call_count": len(calls),
        "action_manifest_pairs": [
            {"node_id": node_id, "tool": tool} for node_id, tool in pairs
        ],
        "patch_exact": patch_exact,
        "reasoning_content_present": isinstance(reasoning, str) and bool(reasoning),
        "usage": {
            "input_tokens": int(usage.get("prompt_tokens", 0) or 0),
            "cached_input_tokens": int(usage.get("prompt_cache_hit_tokens", 0) or 0),
            "output_tokens": int(usage.get("completion_tokens", 0) or 0),
        },
        "dispatch_eligible": not reasons,
        "reason_codes": sorted(set(reasons)),
        "pass": not reasons,
    }


def invoke_provider(
    endpoint: str, api_key: str, body: dict[str, Any]
) -> tuple[int, dict[str, Any] | None, str, int]:
    encoded = json.dumps(body, separators=(",", ":")).encode("utf-8")
    request = urllib.request.Request(
        endpoint,
        data=encoded,
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    started = time.monotonic()
    try:
        with urllib.request.urlopen(request, timeout=180) as response:
            text = response.read().decode("utf-8")
            status = response.status
    except urllib.error.HTTPError as error:
        text = error.read().decode("utf-8")
        status = error.code
    duration_ms = round((time.monotonic() - started) * 1000)
    try:
        payload = json.loads(text) if text else None
    except json.JSONDecodeError:
        payload = None
    return status, payload, text, duration_ms


def request_body(model: str, scenario: Scenario) -> dict[str, Any]:
    return {
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": (
                    "Provider wire capability probe. Follow the declared function "
                    "schemas and requested top-level tool-call order exactly."
                ),
            },
            {"role": "user", "content": scenario.prompt},
        ],
        "tools": scenario.tools,
        "tool_choice": "required" if scenario.required_thinking_negative else "auto",
        "parallel_tool_calls": True,
        "thinking": {"type": "enabled"},
        "reasoning_effort": "max",
        "stream": False,
    }


def run_probe(args: argparse.Namespace) -> dict[str, Any]:
    api_key = os.environ.get("DEEPSEEK_API_KEY", "")
    if not api_key:
        raise RuntimeError("DEEPSEEK_API_KEY is required")
    output_path = pathlib.Path(args.output)
    raw_dir = pathlib.Path(args.raw_dir)
    raw_dir.mkdir(parents=True, exist_ok=True)
    tasks = [
        (scenario, repeat)
        for scenario in scenarios()
        for repeat in range(1, args.repeat + 1)
    ]

    def run_one(item: tuple[Scenario, int]) -> dict[str, Any]:
        scenario, repeat = item
        body = request_body(args.model, scenario)
        status, payload, raw_text, duration_ms = invoke_provider(
            args.endpoint, api_key, body
        )
        prefix = f"{scenario.name}-r{repeat:02d}"
        request_path = raw_dir / f"{prefix}-request.json"
        response_path = raw_dir / f"{prefix}-response.json"
        write_json(request_path, body)
        response_path.write_text(raw_text + ("\n" if raw_text else ""), encoding="utf-8")
        observation = analyze_response(scenario, status, payload)
        observation.update(
            {
                "scenario": scenario.name,
                "repeat": repeat,
                "duration_ms": duration_ms,
                "request_sha256": hashlib.sha256(request_path.read_bytes()).hexdigest(),
                "response_sha256": hashlib.sha256(
                    response_path.read_bytes()
                ).hexdigest(),
            }
        )
        return observation

    with concurrent.futures.ThreadPoolExecutor(max_workers=min(5, len(tasks))) as pool:
        observations = list(pool.map(run_one, tasks))
    observations.sort(key=lambda item: (item["scenario"], item["repeat"]))

    scenario_results = []
    for scenario in scenarios():
        rows = [row for row in observations if row["scenario"] == scenario.name]
        scenario_results.append(
            {
                "scenario": scenario.name,
                "requests": len(rows),
                "passed": sum(1 for row in rows if row["pass"]),
                "all_passed": all(row["pass"] for row in rows),
                "reason_codes": sorted(
                    {code for row in rows for code in row["reason_codes"]}
                ),
            }
        )
    total_usage = {
        key: sum(row.get("usage", {}).get(key, 0) for row in observations)
        for key in ("input_tokens", "cached_input_tokens", "output_tokens")
    }
    all_passed = all(row["all_passed"] for row in scenario_results)
    result = {
        "schema_version": "r7-a2-b0-provider-wire-v1",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "repo_commit": args.repo_commit,
        "model": args.model,
        "endpoint": args.endpoint,
        "repeat_per_scenario": args.repeat,
        "execution": "docker",
        "privacy": {
            "api_key_recorded": False,
            "authorization_header_recorded": False,
            "raw_payload_contains_only_synthetic_probe_data": True,
        },
        "official_contract": {
            "tool_type": "function_only",
            "tool_execution": "client_side",
            "tool_calls_url": "https://api-docs.deepseek.com/guides/tool_calls",
            "chat_completion_url": "https://api-docs.deepseek.com/api/create-chat-completion",
        },
        "scenario_results": scenario_results,
        "observations": observations,
        "usage_total": total_usage,
        "capability": {
            "ordered_multi_function_calls": all(
                row["all_passed"]
                for row in scenario_results
                if row["scenario"] in {
                    "initialize_multi_function",
                    "execute_patch_and_function",
                }
            ),
            "agent_manifest_sibling_pairing": all(
                row["all_passed"]
                for row in scenario_results
                if not row["scenario"].endswith("_negative")
            ),
            "apply_patch_function_adapter_shape": next(
                row["all_passed"]
                for row in scenario_results
                if row["scenario"] == "execute_patch_and_function"
            ),
            "mcp_and_tool_search_as_client_functions": next(
                row["all_passed"]
                for row in scenario_results
                if row["scenario"] == "execute_mcp_and_tool_search"
            ),
            "web_search_as_client_function": next(
                row["all_passed"]
                for row in scenario_results
                if row["scenario"] == "execute_web_search_function"
            ),
            "provider_native_non_function_tool": "unsupported_by_deepseek_chat",
        },
        "decision": {
            "overall": "proceed" if all_passed else "pause",
            "b1_allowed": all_passed,
            "production_behavior_changed": False,
            "interpretation": (
                "provider_wire_and_guided_generation_validated"
                if all_passed
                else "provider_wire_or_guided_generation_failed"
            ),
        },
    }
    write_json(output_path, result)
    return result


def main() -> int:
    from r7_a2_b0_provider_wire_cli import main as cli_main

    return cli_main()


if __name__ == "__main__":
    raise SystemExit(main())
