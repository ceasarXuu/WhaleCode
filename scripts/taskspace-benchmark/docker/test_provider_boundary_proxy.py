#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
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


class UpstreamHandler(BaseHTTPRequestHandler):
    authorization = ""

    def log_message(self, format: str, *args: object) -> None:
        del format, args

    def do_POST(self) -> None:
        type(self).authorization = self.headers.get("Authorization", "")
        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length)
        response = json.dumps(
            {"path": self.path, "authorization": type(self).authorization, "body": body.decode()}
        ).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(response)))
        self.end_headers()
        self.wfile.write(response)


class ProviderBoundaryProxyTest(unittest.TestCase):
    def setUp(self) -> None:
        self.upstream = ThreadingHTTPServer(("127.0.0.1", 0), UpstreamHandler)
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
            "PROVIDER_BOUNDARY_EVENTS_PATH": str(
                Path(self.temp.name) / "events.jsonl"
            ),
        }
        self.environment = patch.dict(os.environ, environment, clear=False)
        self.environment.start()
        state = PROXY.BoundaryState()
        self.proxy = ThreadingHTTPServer(
            ("127.0.0.1", 0), PROXY.ProviderBoundaryHandler
        )
        self.proxy.boundary_state = state
        self.proxy_thread = threading.Thread(target=self.proxy.serve_forever, daemon=True)
        self.proxy_thread.start()

    def tearDown(self) -> None:
        self.proxy.shutdown()
        self.proxy.server_close()
        self.upstream.shutdown()
        self.upstream.server_close()
        self.environment.stop()
        self.temp.cleanup()

    def request(self, path: str) -> tuple[int, dict[str, object]]:
        request = urllib.request.Request(
            f"http://127.0.0.1:{self.proxy.server_address[1]}{path}",
            data=b'{"hello":"world"}',
            headers={"Authorization": "Bearer agent-visible-value"},
            method="POST",
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
        first_status, first = self.request("/responses?one=1")
        second_status, _ = self.request("/responses?two=2")
        third_status, third = self.request("/responses?three=3")
        self.assertEqual((first_status, second_status, third_status), (200, 200, 429))
        self.assertEqual(first["authorization"], "Bearer real-secret")
        self.assertEqual(first["path"], "/responses?one=1")
        self.assertNotIn("agent-visible-value", json.dumps(first))
        self.assertIn("hard limit", str(third["error"]))
        events = Path(self.temp.name, "events.jsonl").read_text(encoding="utf-8")
        self.assertEqual(events.count('"event":"provider_request_claimed"'), 2)
        self.assertEqual(events.count('"event":"provider_request_rejected"'), 1)

    def test_health_check_does_not_consume_request_budget(self) -> None:
        health = urllib.request.urlopen(
            f"http://127.0.0.1:{self.proxy.server_address[1]}/healthz", timeout=2
        )
        self.assertEqual(health.status, 200)
        self.assertEqual(self.proxy.boundary_state.budget.count, 0)


if __name__ == "__main__":
    unittest.main()
