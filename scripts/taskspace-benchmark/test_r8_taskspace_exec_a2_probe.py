#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import json
import pathlib
import sys
import unittest


SCRIPT_DIR = pathlib.Path(__file__).parent
sys.path.insert(0, str(SCRIPT_DIR))
MODULE_PATH = SCRIPT_DIR / "r8_taskspace_exec_a2_probe.py"
SPEC = importlib.util.spec_from_file_location("r8_taskspace_exec_a2_probe", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)


def event(value: dict[str, object]) -> str:
    return "data: " + json.dumps(value, separators=(",", ":"))


def plan_source(bindings: list[dict[str, str]]) -> str:
    plan = {
        "version": PROBE.PLAN_VERSION,
        "capability_id": PROBE.CAPABILITY_ID,
        "calls": [],
        "hosted_bindings": bindings,
    }
    return f"taskspace.plan({json.dumps(plan, separators=(',', ':'))});"


def fixture(bindings: list[dict[str, str]]) -> str:
    return "\n".join(
        [
            event(
                {
                    "type": "response.output_item.done",
                    "output_index": 7,
                    "item": {
                        "type": "web_search_call",
                        "id": "ws_openai",
                        "status": "completed",
                        "action": {"type": "search", "queries": ["OpenAI pricing"]},
                    },
                }
            ),
            event(
                {
                    "type": "response.output_item.done",
                    "output_index": 2,
                    "item": {
                        "type": "web_search_call",
                        "id": "ws_deepseek",
                        "status": "completed",
                        "action": {"type": "search", "queries": ["DeepSeek pricing"]},
                    },
                }
            ),
            event(
                {
                    "type": "response.output_item.done",
                    "output_index": 9,
                    "item": {
                        "type": "function_call",
                        "id": "fc_1",
                        "call_id": "call_1",
                        "name": "taskspace_exec",
                        "arguments": json.dumps({"source": plan_source(bindings)}),
                    },
                }
            ),
            event(
                {
                    "type": "response.completed",
                    "response": {
                        "usage": {
                            "input_tokens": 100,
                            "input_tokens_details": {"cached_tokens": 64},
                            "output_tokens": 20,
                        }
                    },
                }
            ),
        ]
    )


class TaskspaceExecA2ProbeTest(unittest.TestCase):
    def test_accepts_exact_multi_node_bindings_in_provider_order(self) -> None:
        result = PROBE.analyze(
            200,
            fixture(
                [
                    {"tool": "web_search", "node_id": "deepseek-research"},
                    {"tool": "web_search", "node_id": "openai-research"},
                ]
            ),
        )

        self.assertTrue(result["a2_v4_exact"])
        self.assertEqual(
            [fact["output_index"] for fact in result["hosted_facts"]], [2, 7]
        )
        self.assertEqual(result["usage"]["uncached_input_tokens"], 36)

    def test_rejects_missing_binding(self) -> None:
        result = PROBE.analyze(
            200,
            fixture(
                [{"tool": "web_search", "node_id": "deepseek-research"}]
            ),
        )

        self.assertFalse(result["a2_v4_exact"])
        self.assertFalse(result["checks"]["binding_count_exact"])

    def test_rejects_one_node_for_all_hosted_items(self) -> None:
        result = PROBE.analyze(
            200,
            fixture(
                [
                    {"tool": "web_search", "node_id": "deepseek-research"},
                    {"tool": "web_search", "node_id": "deepseek-research"},
                ]
            ),
        )

        self.assertFalse(result["a2_v4_exact"])
        self.assertFalse(result["checks"]["both_nodes_declared"])

    def test_rejects_provider_id_copied_into_binding(self) -> None:
        result = PROBE.analyze(
            200,
            fixture(
                [
                    {
                        "tool": "web_search",
                        "node_id": "deepseek-research",
                        "provider_item_id": "ws_deepseek",
                    },
                    {"tool": "web_search", "node_id": "openai-research"},
                ]
            ),
        )

        self.assertFalse(result["a2_v4_exact"])
        self.assertFalse(result["checks"]["binding_shape_valid"])

    def test_request_is_not_spoon_fed_with_expected_plan_json(self) -> None:
        body = PROBE.request_body("deepseek-v4-flash")

        self.assertEqual(
            [(tool["type"], tool.get("name")) for tool in body["tools"]],
            [("web_search", None), ("function", "taskspace_exec")],
        )
        self.assertNotIn("hosted_bindings\":", json.dumps(body))
        self.assertEqual(body["tools"][1]["parameters"]["required"], ["source"])


if __name__ == "__main__":
    unittest.main()
