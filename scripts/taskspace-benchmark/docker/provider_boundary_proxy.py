#!/usr/bin/env python3
"""Fixed-upstream HTTP boundary for provider-budgeted benchmark containers."""

from __future__ import annotations

import http.client
import json
import os
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit


HOP_BY_HOP = frozenset(
    {
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailer",
        "transfer-encoding",
        "upgrade",
    }
)


class RequestBudget:
    def __init__(self, limit: int) -> None:
        self.limit = limit
        self.count = 0
        self.lock = threading.Lock()

    def claim(self) -> tuple[bool, int]:
        with self.lock:
            if self.limit > 0 and self.count >= self.limit:
                return False, self.count
            self.count += 1
            return True, self.count


class BoundaryState:
    def __init__(self) -> None:
        self.upstream = urlsplit(
            os.environ.get("PROVIDER_UPSTREAM_BASE_URL", "https://api.deepseek.com")
        )
        if self.upstream.scheme not in {"http", "https"} or not self.upstream.hostname:
            raise ValueError("PROVIDER_UPSTREAM_BASE_URL must be an HTTP(S) origin")
        api_key = os.environ.get("DEEPSEEK_API_KEY", "").strip()
        if not api_key:
            raise ValueError("DEEPSEEK_API_KEY is required by the provider boundary")
        self.authorization = f"Bearer {api_key}"
        self.budget = RequestBudget(int(os.environ.get("PROVIDER_REQUEST_HARD_LIMIT", "0")))
        self.events_path = Path(
            os.environ.get("PROVIDER_BOUNDARY_EVENTS_PATH", "/supervisor/events.jsonl")
        )
        self.events_path.parent.mkdir(parents=True, exist_ok=True)
        self.events_lock = threading.Lock()

    def record(self, event: str, **fields: object) -> None:
        value = {
            "schema_version": 1,
            "timestamp_ns": time.time_ns(),
            "event": event,
            **fields,
        }
        with self.events_lock, self.events_path.open("a", encoding="utf-8") as stream:
            stream.write(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n")


class ProviderBoundaryHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server_version = "WhaleProviderBoundary/1"

    @property
    def state(self) -> BoundaryState:
        return self.server.boundary_state  # type: ignore[attr-defined]

    def log_message(self, format: str, *args: object) -> None:
        del format, args

    def do_GET(self) -> None:
        self._handle()

    def do_POST(self) -> None:
        self._handle()

    def do_PUT(self) -> None:
        self._handle()

    def do_PATCH(self) -> None:
        self._handle()

    def do_DELETE(self) -> None:
        self._handle()

    def _handle(self) -> None:
        if self.path == "/healthz":
            self._json_response(200, {"status": "ok"})
            return
        allowed, count = self.state.budget.claim()
        if not allowed:
            self.state.record("provider_request_rejected", path=self.path, count=count)
            self._json_response(429, {"error": "provider request hard limit reached"})
            return
        self.state.record("provider_request_claimed", path=self.path, count=count)
        try:
            self._forward(count)
        except Exception as error:
            self.state.record(
                "provider_request_failed",
                path=self.path,
                count=count,
                error=f"{type(error).__name__}: {error}",
            )
            self._json_response(502, {"error": "provider boundary upstream failure"})

    def _forward(self, count: int) -> None:
        content_length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(content_length) if content_length else None
        headers = {
            name: value
            for name, value in self.headers.items()
            if name.lower() not in HOP_BY_HOP | {"host", "authorization", "content-length"}
        }
        headers["Authorization"] = self.state.authorization
        if body is not None:
            headers["Content-Length"] = str(len(body))
        upstream_path = f"{self.state.upstream.path.rstrip('/')}{self.path}"
        connection_type = (
            http.client.HTTPSConnection
            if self.state.upstream.scheme == "https"
            else http.client.HTTPConnection
        )
        connection = connection_type(
            self.state.upstream.hostname,
            self.state.upstream.port,
            timeout=60,
        )
        try:
            connection.request(self.command, upstream_path, body=body, headers=headers)
            response = connection.getresponse()
            self.send_response(response.status, response.reason)
            for name, value in response.getheaders():
                if name.lower() not in HOP_BY_HOP | {"content-length"}:
                    self.send_header(name, value)
            self.send_header("Connection", "close")
            self.end_headers()
            while chunk := response.read(64 * 1024):
                self.wfile.write(chunk)
                self.wfile.flush()
            self.close_connection = True
            self.state.record(
                "provider_request_completed",
                path=self.path,
                count=count,
                status=response.status,
            )
        finally:
            connection.close()

    def _json_response(self, status: int, value: dict[str, object]) -> None:
        body = json.dumps(value, separators=(",", ":")).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(body)
        self.close_connection = True


def main() -> None:
    state = BoundaryState()
    server = ThreadingHTTPServer(("0.0.0.0", 8080), ProviderBoundaryHandler)
    server.boundary_state = state  # type: ignore[attr-defined]
    state.record("provider_boundary_started", limit=state.budget.limit)
    server.serve_forever()


if __name__ == "__main__":
    main()
