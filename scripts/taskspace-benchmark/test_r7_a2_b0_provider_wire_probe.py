from __future__ import annotations

from copy import deepcopy

from r7_a2_b0_provider_wire_contract import control_tool
from r7_a2_b0_provider_wire_probe import PATCH_TEXT
from r7_a2_b0_provider_wire_probe import analyze_response
from r7_a2_b0_provider_wire_probe import scenarios


def call(name: str, arguments: dict) -> dict:
    import json

    return {
        "id": f"call_{name}",
        "type": "function",
        "function": {"name": name, "arguments": json.dumps(arguments)},
    }


def valid_payload(scenario_name: str) -> dict:
    scenario = next(item for item in scenarios() if item.name == scenario_name)
    if scenario.native_negative or scenario.required_thinking_negative:
        return {"error": {"message": "Only function tools are supported"}}
    if scenario.expected_action == "initialize_and_execute":
        control = {
            "action": "initialize_and_execute",
            "root": {"node_id": "root", "goal": "Complete the synthetic task"},
            "work_nodes": [
                {"node_id": "inspect", "goal": "Inspect"},
                {"node_id": "research", "goal": "Research"},
            ],
            "finish": {"node_id": "finish", "goal": "Finish the task"},
            "edges": [
                {"from": "root", "to": "inspect"},
                {"from": "root", "to": "research"},
                {"from": "inspect", "to": "finish"},
                {"from": "research", "to": "finish"},
            ],
            "actions": [
                {"node_id": node_id, "tool": tool}
                for node_id, tool in scenario.expected_pairs
            ],
        }
    else:
        control = {
            "action": "execute",
            "expected_revision": 12,
            "mutations": [],
            "actions": [
                {"node_id": node_id, "tool": tool}
                for node_id, tool in scenario.expected_pairs
            ],
        }
    calls = [call("taskspace_control", control)]
    for name in scenario.expected_names[1:]:
        arguments = {"value": "ok"}
        if name == "apply_patch":
            arguments = {"input": PATCH_TEXT}
        calls.append(call(name, arguments))
    return {
        "choices": [
            {
                "finish_reason": "tool_calls",
                "message": {"content": None, "tool_calls": calls},
            }
        ],
        "usage": {
            "prompt_tokens": 100,
            "prompt_cache_hit_tokens": 50,
            "completion_tokens": 20,
        },
    }


def test_all_positive_fixtures_are_dispatch_eligible() -> None:
    for scenario in scenarios():
        if scenario.native_negative or scenario.required_thinking_negative:
            continue
        result = analyze_response(scenario, 200, valid_payload(scenario.name))
        assert result["pass"], (scenario.name, result["reason_codes"])
        assert result["dispatch_eligible"]


def test_function_parameters_have_provider_required_object_root() -> None:
    parameters = control_tool()["function"]["parameters"]
    assert parameters["type"] == "object"
    assert parameters["anyOf"]


def test_provider_rejection_does_not_masquerade_as_agent_semantic_errors() -> None:
    scenario = next(
        item for item in scenarios() if item.name == "initialize_multi_function"
    )
    result = analyze_response(
        scenario,
        400,
        {
            "error": {
                "type": "invalid_request_error",
                "code": "invalid_request_error",
                "message": "Invalid schema",
            }
        },
    )
    assert result["reason_codes"] == ["provider_request_rejected"]
    assert result["provider_error"]["message"] == "Invalid schema"
    assert not result["dispatch_eligible"]


def test_manifest_count_and_order_mismatch_fail_closed() -> None:
    scenario = next(
        item for item in scenarios() if item.name == "initialize_multi_function"
    )
    payload = valid_payload(scenario.name)
    payload["choices"][0]["message"]["tool_calls"].pop()
    result = analyze_response(scenario, 200, payload)
    assert not result["dispatch_eligible"]
    assert "tool_name_or_order_mismatch" in result["reason_codes"]
    assert "manifest_sibling_pairing_mismatch" in result["reason_codes"]


def test_missing_agent_node_id_fails_closed() -> None:
    scenario = next(
        item for item in scenarios() if item.name == "execute_patch_and_function"
    )
    payload = valid_payload(scenario.name)
    import json

    control_call = payload["choices"][0]["message"]["tool_calls"][0]
    control = json.loads(control_call["function"]["arguments"])
    control["actions"][0]["node_id"] = ""
    control_call["function"]["arguments"] = json.dumps(control)
    result = analyze_response(scenario, 200, payload)
    assert not result["dispatch_eligible"]
    assert "missing_agent_declared_node_id" in result["reason_codes"]


def test_invalid_json_fails_closed() -> None:
    scenario = next(
        item for item in scenarios() if item.name == "execute_mcp_and_tool_search"
    )
    payload = valid_payload(scenario.name)
    payload["choices"][0]["message"]["tool_calls"][1]["function"]["arguments"] = "{"
    result = analyze_response(scenario, 200, payload)
    assert not result["dispatch_eligible"]
    assert "invalid_tool_arguments_json" in result["reason_codes"]


def test_partial_response_fails_closed() -> None:
    scenario = next(
        item for item in scenarios() if item.name == "execute_web_search_function"
    )
    payload = valid_payload(scenario.name)
    payload["choices"][0]["finish_reason"] = "length"
    result = analyze_response(scenario, 200, payload)
    assert not result["dispatch_eligible"]
    assert "response_not_complete_tool_manifest" in result["reason_codes"]


def test_patch_must_be_exact() -> None:
    scenario = next(
        item for item in scenarios() if item.name == "execute_patch_and_function"
    )
    payload = deepcopy(valid_payload(scenario.name))
    import json

    patch_call = payload["choices"][0]["message"]["tool_calls"][1]
    patch_call["function"]["arguments"] = json.dumps({"input": "truncated"})
    result = analyze_response(scenario, 200, payload)
    assert not result["dispatch_eligible"]
    assert "patch_input_not_exact" in result["reason_codes"]


def test_provider_native_non_function_rejection_is_classified() -> None:
    scenario = next(item for item in scenarios() if item.native_negative)
    result = analyze_response(
        scenario,
        400,
        {"error": {"message": "Only function tools are supported"}},
    )
    assert result["pass"]
    assert result["provider_rejected_expected_request"]
    assert not result["dispatch_eligible"]


def test_required_tool_choice_with_thinking_rejection_is_classified() -> None:
    scenario = next(item for item in scenarios() if item.required_thinking_negative)
    result = analyze_response(
        scenario,
        400,
        {"error": {"message": "Thinking mode does not support this tool_choice"}},
    )
    assert result["pass"]
    assert result["provider_rejected_expected_request"]
    assert not result["dispatch_eligible"]
