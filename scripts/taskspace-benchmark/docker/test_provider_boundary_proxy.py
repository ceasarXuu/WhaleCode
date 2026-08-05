#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import hashlib
import json
import os
import tempfile
import threading
import unittest
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from unittest.mock import patch


MODULE_PATH = Path(__file__).with_name("provider_boundary_proxy.py")
SPEC = importlib.util.spec_from_file_location("provider_boundary_proxy", MODULE_PATH)
assert SPEC and SPEC.loader
PROXY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROXY)
VERIFIER_PATH = Path(__file__).with_name("verify_provider_boundary.py")
VERIFIER_SPEC = importlib.util.spec_from_file_location(
    "verify_provider_boundary", VERIFIER_PATH
)
assert VERIFIER_SPEC and VERIFIER_SPEC.loader
VERIFIER = importlib.util.module_from_spec(VERIFIER_SPEC)
VERIFIER_SPEC.loader.exec_module(VERIFIER)


class UpstreamHandler(BaseHTTPRequestHandler):
    authorization = ""
    request_count = 0

    def log_message(self, format: str, *args: object) -> None:
        del format, args

    def do_POST(self) -> None:
        type(self).request_count += 1
        type(self).authorization = self.headers.get("Authorization", "")
        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length)
        response = json.dumps(
            {
                "path": self.path,
                "authorization": type(self).authorization,
                "body": body.decode(),
            }
        ).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(response)))
        self.end_headers()
        self.wfile.write(response)


class ProviderBoundaryProxyTest(unittest.TestCase):
    def setUp(self) -> None:
        self.upstream = ThreadingHTTPServer(("127.0.0.1", 0), UpstreamHandler)
        UpstreamHandler.request_count = 0
        self.upstream_thread = threading.Thread(
            target=self.upstream.serve_forever, daemon=True
        )
        self.upstream_thread.start()
        self.temp = tempfile.TemporaryDirectory()
        environment = {
            "DEEPSEEK_API_KEY": "real-secret",
            "PROVIDER_UPSTREAM_BASE_URL": (
                f"http://127.0.0.1:{self.upstream.server_address[1]}"
            ),
            "PROVIDER_REQUEST_HARD_LIMIT": "2",
            "PROVIDER_ALLOWED_MODEL": "deepseek-v4-flash",
            "PROVIDER_BOUNDARY_EVENTS_PATH": str(Path(self.temp.name) / "events.jsonl"),
        }
        self.environment = patch.dict(os.environ, environment, clear=False)
        self.environment.start()
        state = PROXY.BoundaryState()
        self.proxy = ThreadingHTTPServer(
            ("127.0.0.1", 0), PROXY.ProviderBoundaryHandler
        )
        self.proxy.boundary_state = state
        self.proxy_thread = threading.Thread(
            target=self.proxy.serve_forever, daemon=True
        )
        self.proxy_thread.start()

    def tearDown(self) -> None:
        self.proxy.shutdown()
        self.proxy.server_close()
        self.upstream.shutdown()
        self.upstream.server_close()
        self.environment.stop()
        self.temp.cleanup()

    def request(
        self,
        path: str,
        *,
        method: str = "POST",
        model: str = "deepseek-v4-flash",
    ) -> tuple[int, dict[str, object]]:
        body = json.dumps(
            {"model": model, "input": "hello"}, separators=(",", ":")
        ).encode()
        request = urllib.request.Request(
            f"http://127.0.0.1:{self.proxy.server_address[1]}{path}",
            data=body,
            headers={"Authorization": "Bearer agent-visible-value"},
            method=method,
        )
        try:
            response = urllib.request.urlopen(request, timeout=2)
        except urllib.error.HTTPError as error:
            try:
                return error.code, json.loads(error.read())
            finally:
                error.close()
        return response.status, json.loads(response.read())

    def test_injects_supervisor_credential_and_enforces_shared_limit(self) -> None:
        first_status, first = self.request("/responses")
        second_status, _ = self.request("/responses")
        third_status, third = self.request("/responses")
        self.assertEqual((first_status, second_status, third_status), (200, 200, 429))
        self.assertEqual(first["authorization"], "Bearer real-secret")
        self.assertEqual(first["path"], "/responses")
        self.assertNotIn("agent-visible-value", json.dumps(first))
        self.assertIn("hard limit", str(third["error"]))
        events = Path(self.temp.name, "events.jsonl").read_text(encoding="utf-8")
        self.assertEqual(events.count('"event":"provider_request_claimed"'), 2)
        self.assertEqual(events.count('"event":"provider_request_rejected"'), 1)

    def test_rejects_unapproved_route_method_and_model_before_upstream(self) -> None:
        path_status, _ = self.request("/chat/completions")
        method_status, _ = self.request("/responses", method="PUT")
        model_status, _ = self.request("/responses", model="other-model")
        self.assertEqual((path_status, method_status, model_status), (404, 405, 400))
        self.assertEqual(UpstreamHandler.request_count, 0)
        self.assertEqual(self.proxy.boundary_state.budget.count, 0)

    def test_health_check_does_not_consume_request_budget(self) -> None:
        health = urllib.request.urlopen(
            f"http://127.0.0.1:{self.proxy.server_address[1]}/healthz", timeout=2
        )
        self.assertEqual(health.status, 200)
        self.assertEqual(self.proxy.boundary_state.budget.count, 0)

    def test_reconciles_exact_boundary_and_wire_payload_sequence(self) -> None:
        body = b'{"model":"deepseek-v4-flash","input":"hello"}'
        digest = hashlib.sha256(body).hexdigest()
        events = Path(self.temp.name, "reconcile-events.jsonl")
        wire = Path(self.temp.name, "wire.jsonl")
        start_event = {
            "event": "provider_boundary_started",
            "limit": 2,
            "allowed_method": "POST",
            "allowed_path": "/responses",
            "allowed_model": "deepseek-v4-flash",
        }
        first_claim = {
                    "event": "provider_request_claimed",
                    "count": 1,
                    "method": "POST",
                    "path": "/responses",
                    "model": "deepseek-v4-flash",
                    "body_sha256": digest,
                }
        events.write_text(
            "".join(
                json.dumps(event) + "\n"
                for event in (start_event, first_claim, {"event": "provider_boundary_stopped", "request_count": 1})
            ),
            encoding="utf-8",
        )
        wire_events = [
            {
                "schema_version": "provider-chat-wire-trace-v10",
                "status": "payload_captured",
                "request_id": "request-1",
                "logical_request_id": "logical-1",
                "attempt_seq": 1,
                "request_index": 1,
                "provider_payload_sha256": digest,
            },
            {
                "schema_version": "provider-chat-wire-trace-v10",
                "status": "response_completed",
                "request_id": "request-1",
                "logical_request_id": "logical-1",
                "attempt_seq": 1,
                "input_tokens": 10,
                "cached_input_tokens": 0,
                "output_tokens": 2,
                "reasoning_output_tokens": 1,
                "total_tokens": 12,
            },
        ]
        wire.write_text(
            "".join(json.dumps(event) + "\n" for event in wire_events),
            encoding="utf-8",
        )
        result = VERIFIER.reconcile(events, wire, "deepseek-v4-flash")
        self.assertEqual(result["status"], "reconciled")
        self.assertEqual(result["boundary_request_count"], 1)

        second_claim = {
                    "event": "provider_request_claimed",
                    "count": 2,
                    "method": "POST",
                    "path": "/responses",
                    "model": "deepseek-v4-flash",
                    "body_sha256": "f" * 64,
                }
        events.write_text(
            "".join(
                json.dumps(event) + "\n"
                for event in (start_event, first_claim, second_claim, {"event": "provider_boundary_stopped", "request_count": 2})
            ),
            encoding="utf-8",
        )
        mismatch = VERIFIER.reconcile(events, wire, "deepseek-v4-flash")
        self.assertEqual(mismatch["status"], "mismatch")
        self.assertIn("boundary_unattributed", mismatch["errors"])

    def test_parallel_claim_events_remain_in_authoritative_count_order(self) -> None:
        state = self.proxy.boundary_state
        original_record = state.record
        first_waiting = threading.Event()
        release_first = threading.Event()

        def delayed_record(event: str, **fields: object) -> None:
            if event == "provider_request_claimed" and fields.get("count") == 1:
                first_waiting.set()
                self.assertTrue(release_first.wait(timeout=2))
            original_record(event, **fields)

        state.record = delayed_record
        results = []
        first = threading.Thread(
            target=lambda: results.append(self.request("/responses")), daemon=True
        )
        second = threading.Thread(
            target=lambda: results.append(self.request("/responses")), daemon=True
        )
        first.start()
        self.assertTrue(first_waiting.wait(timeout=2))
        second.start()
        second.join(timeout=0.1)
        self.assertTrue(second.is_alive())
        release_first.set()
        first.join(timeout=3)
        second.join(timeout=3)
        self.assertEqual(sorted(status for status, _ in results), [200, 200])
        events = [
            json.loads(line)
            for line in Path(self.temp.name, "events.jsonl")
            .read_text(encoding="utf-8")
            .splitlines()
        ]
        counts = [
            event["count"]
            for event in events
            if event["event"] == "provider_request_claimed"
        ]
        self.assertEqual(counts, [1, 2])


if __name__ == "__main__":
    unittest.main()
