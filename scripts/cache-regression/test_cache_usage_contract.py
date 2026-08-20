#!/usr/bin/env python3

from __future__ import annotations

import json
import unittest

from cache_usage_contract import parse_provider_wire_usage


def wire_events(statuses: list[str]) -> str:
    rows = []
    for index, status in enumerate(statuses, 1):
        request_id = f"request-{index}"
        identity = {
            "schema_version": "provider-chat-wire-trace-v11",
            "request_id": request_id,
            "logical_request_id": f"logical-{index}",
            "attempt_seq": 1,
        }
        rows.append(
            {
                **identity,
                "status": "payload_captured",
                "request_index": index,
                "provider_payload_sha256": f"{index:064x}",
            }
        )
        terminal = {**identity, "status": status}
        if status == "response_completed":
            terminal.update(
                input_tokens=100 * index,
                cached_input_tokens=50 * (index - 1),
                output_tokens=10 * index,
                reasoning_output_tokens=index,
                total_tokens=110 * index,
            )
        rows.append(terminal)
    return "\n".join(json.dumps(row) for row in rows)


class CacheUsageContractTest(unittest.TestCase):
    def test_strict_mode_rejects_any_failed_local_attempt(self) -> None:
        with self.assertRaisesRegex(ValueError, "did not complete"):
            parse_provider_wire_usage(
                wire_events(["response_completed", "response_failed"])
            )

    def test_boundary_scope_ignores_local_only_failed_attempt(self) -> None:
        usage = parse_provider_wire_usage(
            wire_events(
                ["response_completed", "response_completed", "response_failed"]
            ),
            ["request-1", "request-2"],
        )

        self.assertEqual(usage["provider_request_count"], 2)
        self.assertEqual(usage["input_tokens"], 300)
        self.assertEqual(usage["cached_input_tokens"], 50)
        self.assertEqual(usage["output_tokens"], 30)
        self.assertEqual(usage["request_ids"], ["request-1", "request-2"])

    def test_boundary_scope_rejects_failed_provider_request(self) -> None:
        with self.assertRaisesRegex(ValueError, "did not complete"):
            parse_provider_wire_usage(
                wire_events(["response_completed", "response_failed"]),
                ["request-1", "request-2"],
            )

    def test_boundary_scope_rejects_unknown_or_duplicate_identity(self) -> None:
        text = wire_events(["response_completed"])
        with self.assertRaisesRegex(ValueError, "missing from wire"):
            parse_provider_wire_usage(text, ["request-missing"])
        with self.assertRaisesRegex(ValueError, "identity set is invalid"):
            parse_provider_wire_usage(text, ["request-1", "request-1"])


if __name__ == "__main__":
    unittest.main()
