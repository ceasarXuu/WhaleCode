#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import json
import pathlib
import unittest


MODULE_PATH = pathlib.Path(__file__).with_name("r8_hosted_container_probe.py")
SPEC = importlib.util.spec_from_file_location("r8_hosted_container_probe", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)


def event(value: dict[str, object]) -> str:
    return "data: " + json.dumps(value, separators=(",", ":"))


class HostedContainerProbeTest(unittest.TestCase):
    def test_analyze_matches_provider_id_and_node(self) -> None:
        raw = "\n".join(
            [
                event(
                    {
                        "type": "response.output_item.done",
                        "item": {
                            "type": "web_search_call",
                            "id": "ws_123",
                            "status": "completed",
                        },
                    }
                ),
                event(
                    {
                        "type": "response.output_item.done",
                        "item": {
                            "type": "function_call",
                            "id": "fc_123",
                            "call_id": "call_123",
                            "name": "taskspace_probe",
                            "arguments": json.dumps(
                                {
                                    "node_id": "research-node",
                                    "provider_item_id": "ws_123",
                                }
                            ),
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
        result = PROBE.analyze(200, raw)
        self.assertTrue(result["protocol_supported"])
        self.assertTrue(result["hosted_and_container_coexist"])
        self.assertTrue(result["provider_id_echo_exact"])
        self.assertTrue(result["node_id_exact"])
        self.assertEqual(result["usage"]["uncached_input_tokens"], 36)

    def test_request_exposes_only_hosted_search_and_probe_container(self) -> None:
        body = PROBE.request_body("deepseek-v4-flash")
        self.assertEqual(body["tool_choice"], "auto")
        self.assertEqual(
            [(tool["type"], tool.get("name")) for tool in body["tools"]],
            [("web_search", None), ("function", "taskspace_probe")],
        )
        self.assertEqual(body["tools"][1]["parameters"]["required"], [
            "node_id",
            "provider_item_id",
        ])


if __name__ == "__main__":
    unittest.main()
