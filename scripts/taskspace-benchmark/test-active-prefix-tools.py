#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
CLIENT = SCRIPT_DIR / "app-server-active-prefix.py"
ANALYZER = SCRIPT_DIR / "analyze-active-prefix.py"


def load_analyzer():
    spec = importlib.util.spec_from_file_location("active_prefix_analyzer", ANALYZER)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


FAKE_SERVER = r'''#!/usr/bin/env python3
import json
import sys

for line in sys.stdin:
    request = json.loads(line)
    request_id = request.get("id")
    method = request["method"]
    if request_id is not None:
        print(json.dumps({"id": request_id, "result": {}}), flush=True)
    if method == "thread/compact/start":
        print(json.dumps({
            "method": "turn/completed",
            "params": {"turn": {"id": "compact-turn", "status": "completed"}},
        }), flush=True)
    if method == "turn/start":
        print(json.dumps({
            "method": "item/completed",
            "params": {"item": {"type": "agentMessage", "text": "done"}},
        }), flush=True)
        print(json.dumps({
            "method": "turn/completed",
            "params": {"turn": {"id": "continuation-turn", "status": "completed"}},
        }), flush=True)
'''


class ClientTest(unittest.TestCase):
    def run_client(self, mode: str) -> tuple[dict, str, list[dict]]:
        with tempfile.TemporaryDirectory() as raw_temp:
            temp = Path(raw_temp)
            server = temp / "fake-server.py"
            server.write_text(FAKE_SERVER, encoding="utf-8")
            server.chmod(0o755)
            prompt = temp / "prompt.txt"
            prompt.write_text("continue\n", encoding="utf-8")
            events = temp / "events.jsonl"
            summary = temp / "summary.json"
            last_message = temp / "last-message.md"
            result = subprocess.run(
                [
                    "python3",
                    str(CLIENT),
                    "--binary",
                    str(server),
                    "--thread-id",
                    "thread-1",
                    "--mode",
                    mode,
                    "--prompt",
                    str(prompt),
                    "--events",
                    str(events),
                    "--stderr",
                    str(temp / "stderr.log"),
                    "--summary",
                    str(summary),
                    "--last-message",
                    str(last_message),
                    "--timeout-seconds",
                    "10",
                ],
                check=False,
                text=True,
                capture_output=True,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            return (
                json.loads(summary.read_text(encoding="utf-8")),
                last_message.read_text(encoding="utf-8"),
                [json.loads(line) for line in events.read_text(encoding="utf-8").splitlines()],
            )

    def test_taskspace_flow(self) -> None:
        summary, last_message, events = self.run_client("taskspace")
        self.assertEqual(summary["status"], "completed")
        self.assertEqual(last_message, "done\n")
        self.assertFalse(any(event.get("id") == 3 for event in events))

    def test_standard_flow_sets_mode(self) -> None:
        summary, _, events = self.run_client("standard")
        self.assertEqual(summary["mode"], "standard")
        self.assertTrue(any(event.get("id") == 3 for event in events))


class AnalyzerTest(unittest.TestCase):
    def setUp(self) -> None:
        self.analyzer = load_analyzer()

    def test_projection_reads_snapshot_delta_and_deduplicates_direct_event(self) -> None:
        projection = {
            "id": "trace-1",
            "kind": "projection_budget",
            "tags": [
                "projection_bytes:856",
                "strategy_activation_count:1",
                "projection_bytes_before_strategy:1963",
                "projection_bytes_after_strategy:856",
                "covered_node_count:3",
            ],
        }
        rollout = [
            {
                "type": "event_msg",
                "payload": {
                    "type": "map_runtime",
                    "map_event_type": "snapshot_delta",
                    "patch": [{"op": "add", "value": projection}],
                },
            },
            {
                "type": "event_msg",
                "payload": {
                    "type": "map_runtime",
                    "map_event_type": "taskspace_trace_event_recorded",
                    "traceEventId": "trace-1",
                    "kind": "projection_budget",
                    "tags": projection["tags"],
                },
            },
        ]
        metrics = self.analyzer.projection_metrics(rollout)
        self.assertEqual(metrics["eventCount"], 1)
        self.assertEqual(metrics["activationCount"], 1)
        self.assertEqual(metrics["bytesBeforeStrategy"], 1963)
        self.assertEqual(metrics["bytesAfterStrategy"], 856)

    def test_standard_mode_keeps_historical_map_inactive(self) -> None:
        rollout = [
            {
                "type": "event_msg",
                "payload": {
                    "type": "map_runtime",
                    "map_event_type": "node_status_changed",
                    "nodeId": "run_tests",
                    "currentStatus": "running",
                },
            },
            {
                "type": "event_msg",
                "payload": {
                    "type": "map_runtime",
                    "map_event_type": "mode_changed",
                    "currentMode": "standard",
                },
            },
            {
                "type": "event_msg",
                "payload": {
                    "type": "map_runtime",
                    "map_event_type": "task_context_ownership_changed",
                    "active": False,
                },
            },
        ]
        metrics = self.analyzer.map_metrics(rollout)
        self.assertEqual(metrics["recordedOpenNodeIds"], ["run_tests"])
        self.assertEqual(metrics["activeOpenNodeIds"], [])
        self.assertFalse(metrics["taskContextOwnershipActive"])


if __name__ == "__main__":
    unittest.main()
