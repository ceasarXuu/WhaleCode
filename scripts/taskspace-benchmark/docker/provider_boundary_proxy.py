#!/usr/bin/env python3
"""Fixed-upstream HTTP boundary for provider-budgeted benchmark containers."""

from __future__ import annotations

import hashlib
import http.client
import json
import os
import signal
import threading
import time
from decimal import Decimal, InvalidOperation
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

    def snapshot(self) -> int:
        with self.lock:
            return self.count


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
        self.allowed_model = os.environ.get("PROVIDER_ALLOWED_MODEL", "").strip()
        if not self.allowed_model:
            raise ValueError(
                "PROVIDER_ALLOWED_MODEL is required by the provider boundary"
            )
        self.budget = RequestBudget(
            int(os.environ.get("PROVIDER_REQUEST_HARD_LIMIT", "0"))
        )
        self.input_token_limit = _environment_int("PROVIDER_INPUT_TOKEN_HARD_LIMIT")
        self.output_token_limit = _environment_int("PROVIDER_OUTPUT_TOKEN_HARD_LIMIT")
        self.cost_limit = _environment_decimal("PROVIDER_ESTIMATED_COST_HARD_LIMIT")
        self.currency = os.environ.get("PROVIDER_BUDGET_CURRENCY", "").strip()
        self.cached_input_rate = _environment_decimal(
            "PROVIDER_CACHED_INPUT_RATE_PER_MILLION"
        )
        self.uncached_input_rate = _environment_decimal(
            "PROVIDER_UNCACHED_INPUT_RATE_PER_MILLION"
        )
        self.output_rate = _environment_decimal(
            "PROVIDER_OUTPUT_RATE_PER_MILLION"
        )
        self.usage_budget_enabled = any(
            value > 0
            for value in (
                self.input_token_limit,
                self.output_token_limit,
                self.cost_limit,
            )
        )
        if self.usage_budget_enabled and (
            self.input_token_limit <= 0
            or self.output_token_limit <= 0
            or self.cost_limit <= 0
            or not self.currency
            or self.uncached_input_rate <= 0
            or self.output_rate <= 0
        ):
            raise ValueError("provider token budget is incomplete")
        self.input_tokens = 0
        self.cached_input_tokens = 0
        self.output_tokens = 0
        self.estimated_cost = Decimal("0")
        self.usage_missing = False
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
            stream.write(
                json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n"
            )

    def claim_request(self, **fields: object) -> tuple[bool, int, str | None]:
        with self.budget.lock:
            if self.budget.limit > 0 and self.budget.count >= self.budget.limit:
                return False, self.budget.count, "request_limit_reached"
            if self.usage_budget_enabled and self.usage_missing:
                return False, self.budget.count, "usage_missing"
            if self.usage_budget_enabled and self._usage_budget_reached():
                return False, self.budget.count, "approved_budget_reached"
            self.budget.count += 1
            count = self.budget.count
            self.record("provider_request_claimed", count=count, **fields)
            return True, count, None

    def settle_usage(self, count: int, usage: dict[str, int] | None) -> None:
        if not self.usage_budget_enabled:
            return
        with self.budget.lock:
            if usage is None:
                self.usage_missing = True
                self.record("provider_usage_missing", count=count)
                return
            self.input_tokens += usage["input_tokens"]
            self.cached_input_tokens += usage["cached_input_tokens"]
            self.output_tokens += usage["output_tokens"]
            uncached = usage["input_tokens"] - usage["cached_input_tokens"]
            request_cost = (
                Decimal(usage["cached_input_tokens"])
                * self.cached_input_rate
                + Decimal(uncached) * self.uncached_input_rate
                + Decimal(usage["output_tokens"]) * self.output_rate
            ) / Decimal(1_000_000)
            self.estimated_cost += request_cost
            self.record(
                "provider_usage_settled",
                count=count,
                request_usage=usage,
                input_tokens=self.input_tokens,
                cached_input_tokens=self.cached_input_tokens,
                output_tokens=self.output_tokens,
                estimated_cost=str(self.estimated_cost),
                currency=self.currency,
                budget_reached=self._usage_budget_reached(),
            )

    def _usage_budget_reached(self) -> bool:
        return (
            self.input_tokens >= self.input_token_limit
            or self.output_tokens >= self.output_token_limit
            or self.estimated_cost >= self.cost_limit
        )


def _environment_int(name: str) -> int:
    value = int(os.environ.get(name, "0"))
    if value < 0:
        raise ValueError(f"{name} must be nonnegative")
    return value


def _environment_decimal(name: str) -> Decimal:
    try:
        value = Decimal(os.environ.get(name, "0"))
    except InvalidOperation as error:
        raise ValueError(f"{name} must be numeric") from error
    if not value.is_finite() or value < 0:
        raise ValueError(f"{name} must be a nonnegative finite number")
    return value


class ResponseUsageScanner:
    def __init__(self) -> None:
        self.buffer = b""
        self.usage: dict[str, int] | None = None

    def feed(self, chunk: bytes) -> None:
        self.buffer += chunk
        while b"\n" in self.buffer:
            line, self.buffer = self.buffer.split(b"\n", 1)
            self._line(line.rstrip(b"\r"))

    def finish(self) -> dict[str, int] | None:
        if self.buffer:
            self._line(self.buffer.rstrip(b"\r"))
            self.buffer = b""
        return self.usage

    def _line(self, line: bytes) -> None:
        if not line.startswith(b"data:"):
            return
        data = line[5:].strip()
        if not data or data == b"[DONE]":
            return
        try:
            event = json.loads(data)
        except (UnicodeDecodeError, json.JSONDecodeError):
            return
        if not isinstance(event, dict) or event.get("type") != "response.completed":
            return
        response = event.get("response")
        usage = response.get("usage") if isinstance(response, dict) else None
        self.usage = _normalized_usage(usage)


def _normalized_usage(value: object) -> dict[str, int] | None:
    if not isinstance(value, dict):
        return None
    details = value.get("input_tokens_details")
    cached = details.get("cached_tokens", 0) if isinstance(details, dict) else 0
    tokens = {
        "input_tokens": value.get("input_tokens"),
        "cached_input_tokens": cached,
        "output_tokens": value.get("output_tokens"),
    }
    if any(
        isinstance(token, bool) or not isinstance(token, int) or token < 0
        for token in tokens.values()
    ) or tokens["cached_input_tokens"] > tokens["input_tokens"]:
        return None
    return tokens


class ProviderBoundaryHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server_version = "WhaleProviderBoundary/1"

    @property
    def state(self) -> BoundaryState:
        return self.server.boundary_state  # type: ignore[attr-defined]

    def log_message(self, format: str, *args: object) -> None:
        del format, args

    def do_GET(self) -> None:
        if self.path == "/healthz":
            self._json_response(200, {"status": "ok"})
            return
        self._reject_contract("method_not_allowed", 405)

    def do_POST(self) -> None:
        self._handle()

    def do_PUT(self) -> None:
        self._reject_contract("method_not_allowed", 405)

    def do_PATCH(self) -> None:
        self._reject_contract("method_not_allowed", 405)

    def do_DELETE(self) -> None:
        self._reject_contract("method_not_allowed", 405)

    def _handle(self) -> None:
        if self.path != "/responses":
            self._reject_contract("endpoint_not_allowed", 404)
            return
        try:
            body, model, body_sha256 = self._validated_body()
        except ValueError as error:
            self._reject_contract(str(error), 400)
            return
        request_fields = {
            "method": self.command,
            "path": self.path,
            "model": model,
            "body_sha256": body_sha256,
        }
        allowed, count, reason = self.state.claim_request(**request_fields)
        if not allowed:
            self.state.record(
                "provider_request_rejected",
                count=count,
                reason=reason,
                **request_fields,
            )
            self._json_response(429, {"error": f"provider budget rejected: {reason}"})
            return
        try:
            self._forward(body, count, model, body_sha256)
        except Exception as error:
            self.state.record(
                "provider_request_failed",
                path=self.path,
                model=model,
                body_sha256=body_sha256,
                count=count,
                error=f"{type(error).__name__}: {error}",
            )
            self._json_response(502, {"error": "provider boundary upstream failure"})

    def _validated_body(self) -> tuple[bytes, str, str]:
        try:
            content_length = int(self.headers.get("Content-Length", ""))
        except ValueError as error:
            raise ValueError("content_length_invalid") from error
        if content_length <= 0:
            raise ValueError("request_body_required")
        body = self.rfile.read(content_length)
        try:
            payload = json.loads(body)
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise ValueError("request_json_invalid") from error
        if not isinstance(payload, dict):
            raise ValueError("request_json_object_required")
        model = payload.get("model")
        if model != self.state.allowed_model:
            raise ValueError("model_not_allowed")
        return body, model, hashlib.sha256(body).hexdigest()

    def _forward(self, body: bytes, count: int, model: str, body_sha256: str) -> None:
        headers = {
            name: value
            for name, value in self.headers.items()
            if name.lower()
            not in HOP_BY_HOP | {"host", "authorization", "content-length"}
        }
        headers["Authorization"] = self.state.authorization
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
            connection.request("POST", upstream_path, body=body, headers=headers)
            response = connection.getresponse()
            self.send_response(response.status, response.reason)
            for name, value in response.getheaders():
                if name.lower() not in HOP_BY_HOP | {"content-length"}:
                    self.send_header(name, value)
            self.send_header("Connection", "close")
            self.end_headers()
            scanner = ResponseUsageScanner()
            while chunk := response.read(64 * 1024):
                scanner.feed(chunk)
                self.wfile.write(chunk)
                self.wfile.flush()
            self.close_connection = True
            self.state.record(
                "provider_request_completed",
                path=self.path,
                model=model,
                body_sha256=body_sha256,
                count=count,
                status=response.status,
            )
            self.state.settle_usage(count, scanner.finish())
        finally:
            connection.close()

    def _reject_contract(self, reason: str, status: int) -> None:
        self.state.record(
            "provider_request_contract_rejected",
            method=self.command,
            path=self.path,
            reason=reason,
        )
        self._json_response(status, {"error": reason})

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
    stopping = threading.Event()

    def request_stop(signum: int, frame: object) -> None:
        del signum, frame
        stopping.set()

    signal.signal(signal.SIGTERM, request_stop)
    signal.signal(signal.SIGINT, request_stop)
    state.record(
        "provider_boundary_started",
        limit=state.budget.limit,
        input_token_limit=state.input_token_limit,
        output_token_limit=state.output_token_limit,
        estimated_cost_limit=str(state.cost_limit),
        budget_currency=state.currency,
        allowed_method="POST",
        allowed_path="/responses",
        allowed_model=state.allowed_model,
    )
    server.timeout = 0.2
    try:
        while not stopping.is_set():
            server.handle_request()
    finally:
        server.server_close()
        state.record(
            "provider_boundary_stopped",
            request_count=state.budget.snapshot(),
        )


if __name__ == "__main__":
    main()
